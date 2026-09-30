using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Products;



public enum ProductFailureKind { None, Validation, Reference, Conflict, Denied, Unavailable }
public sealed record ProductResult<T>(T? Value, ProductFailureKind Failure)
{
    public bool Succeeded => Failure == ProductFailureKind.None && Value is not null;
    public static ProductResult<T> Success(T value) => new(value, ProductFailureKind.None);
    public static ProductResult<T> Failed(ProductFailureKind failure) => new(default, failure);
}

public sealed record ProductSnapshot(ProductCategories Categories, IReadOnlyList<Product> Products, bool CanManage, bool CanReadCosts);

/// <summary>Thin typed IPC adapter for ready-made definitions and separate product categories.</summary>
public sealed class ProductClient(IEngineShellBridge engine) : IAsyncDisposable
{
    private sealed class SaveRetry
    {
        public string? Payload;
        public Guid Key;
        public bool Unresolved;
    }
    private readonly SaveRetry productRetry = new();
    private readonly SaveRetry categoryRetry = new();
    private readonly SemaphoreSlim gate = new(1, 1);
    private IEngineSubscription? subscription;
    private CancellationTokenSource? pumpCancellation;
    private SynchronizationContext? uiContext;
    private long generation = -1;
    private bool active;
    private bool disposed;
    private bool connectedAndReady;

    public event EventHandler? Changed;

    /// <summary>Loads scoped products and categories through paged Rust queries; cancellation propagates.</summary>
    public async Task<ProductResult<ProductSnapshot>> LoadAsync(string term, CancellationToken cancellationToken = default)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityProductV1))
            return ProductResult<ProductSnapshot>.Failed(ProductFailureKind.Unavailable);
        try
        {
            var categoryItems = new List<ProductCategory>();
            Guid? categoryAfter = null;
            do
            {
                var response = await engine.QueryAsync(Query.ForProductCategoryList(new ListProductCategories { After = categoryAfter, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsProductCategories() : null;
                if (page is null) return ProductResult<ProductSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                categoryItems.AddRange(page.Items); categoryAfter = page.Next;
            } while (categoryAfter is not null);
            var categories = new ProductCategories { Items = categoryItems.ToArray() };
            var items = new List<Product>();
            Guid? after = null;
            var canManage = true; var canReadCosts = true;
            do
            {
                var response = await engine.QueryAsync(Query.ForProductList(new ListProducts { Term = term, After = after, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsProducts() : null;
                if (page is null) return ProductResult<ProductSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                items.AddRange(page.Items); after = page.Next; canManage &= page.CanManage; canReadCosts &= page.CanReadCosts;
            } while (after is not null);
            // A permission change between pages must also clear the earlier projection.
            if (!canReadCosts)
                foreach (var item in items)
                {
                    item.Notes = "";
                    foreach (var variant in item.Variants) variant.PurchaseCostYer = null;
                }
            return ProductResult<ProductSnapshot>.Success(new ProductSnapshot(categories, items, canManage, canReadCosts));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return ProductResult<ProductSnapshot>.Failed(MapFailure(error.ContractError?.Code)); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return ProductResult<ProductSnapshot>.Failed(ProductFailureKind.Unavailable); }
    }

    /// <summary>Submits a typed save with the retry state reserved for its record kind.</summary>
    public Task<ProductFailureKind> SaveAsync(SaveProduct input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForProductSave(input), productRetry, cancellationToken);
    /// <summary>Submits a typed save with the retry state reserved for its record kind.</summary>
    public Task<ProductFailureKind> SaveAsync(SaveProductCategory input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForProductCategorySave(input), categoryRetry, cancellationToken);

    /// <summary>Freezes the payload and key after an unknown outcome; only an exact retry can resolve that record kind.</summary>
    private async Task<ProductFailureKind> SubmitAsync(Command command, SaveRetry retry, CancellationToken cancellationToken)
    {
        try
        {
            if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityProductV1)) return ProductFailureKind.Unavailable;
            var payload = System.Text.Json.JsonSerializer.Serialize(command);
            if (retry.Unresolved && payload != retry.Payload) return ProductFailureKind.Conflict;
            if (payload != retry.Payload) { retry.Payload = payload; retry.Key = Guid.NewGuid(); }
            retry.Unresolved = true;
            var response = await engine.SubmitCommandAsync(command, retry.Key, cancellationToken);
            retry.Unresolved = false;
            if (response.Outcome.Status == CommandOutcomeStatus.Succeeded) retry.Payload = null;
            return response.Outcome.Status == CommandOutcomeStatus.Succeeded
                ? ProductFailureKind.None : MapFailure(response.Outcome.Payload.Code);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error)
        {
            var failure = MapFailure(error.ContractError?.Code);
            if (failure != ProductFailureKind.Unavailable) retry.Unresolved = false;
            return failure;
        }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return ProductFailureKind.Unavailable; }
    }

    /// <summary>Captures the UI context and starts product change subscriptions once.</summary>
    public async Task ActivateAsync()
    {
        if (disposed || active) return;
        active = true;
        uiContext = SynchronizationContext.Current;
        var snapshot = engine.Snapshot;
        connectedAndReady = snapshot.IpcHealth == EngineIpcHealthState.Connected
            && snapshot.LastLifecycle?.Ready == true;
        engine.StateChanged += EngineStateChanged;
        await RefreshSubscriptionAsync();
    }

    /// <summary>Refreshes on connection restoration and replaces subscriptions after an engine generation change.</summary>
    private void EngineStateChanged(EngineSupervisionSnapshot snapshot)
    {
        if (!active) return;
        var ready = snapshot.IpcHealth == EngineIpcHealthState.Connected
            && snapshot.LastLifecycle?.Ready == true;
        if (!ready)
        {
            var wasReady = connectedAndReady; connectedAndReady = false;
            if (wasReady) SignalChanged(); return;
        }

        var restored = !connectedAndReady;
        connectedAndReady = true;
        if (snapshot.Generation != generation) _ = RefreshSubscriptionAsync();
        else if (restored) SignalChanged();
    }

    /// <summary>Serializes subscription replacement and starts an acknowledged event pump for the current generation.</summary>
    private async Task RefreshSubscriptionAsync()
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityProductV1)) return;
        await gate.WaitAsync();
        try
        {
            if (!active || disposed || generation == engine.Snapshot.Generation && subscription is not null) return;
            await DropSubscriptionAsync();
            var current = await engine.SubscribeAsync(Subscription.ForProductChangedSubscribe(new ProductChanges()));
            subscription = current;
            generation = engine.Snapshot.Generation;
            pumpCancellation = new CancellationTokenSource();
            current.ResyncRequired += SignalChanged;
            _ = PumpAsync(current, pumpCancellation.Token);
            SignalChanged();
        }
        catch (Exception error) when (error is EngineIpcException or IOException or ObjectDisposedException) { SignalChanged(); }
        finally { gate.Release(); }
    }

    /// <summary>Signals refresh for typed product events and acknowledges delivery; transport failure requests resynchronization.</summary>
    private async Task PumpAsync(IEngineSubscription current, CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var delivered in current.ReadAllAsync(cancellationToken))
            {
                if (EngineContractCodec.DecodeEvent(delivered).AsProductChangedEvent() is not null) SignalChanged();
                current.Acknowledge(delivered);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { }
        catch (Exception error) when (error is EngineIpcException or IOException or InvalidDataException)
        { await RecoverSubscriptionAsync(current); }
    }

    private async Task RecoverSubscriptionAsync(IEngineSubscription current)
    {
        if (disposed) return;
        await gate.WaitAsync();
        try { if (disposed || !ReferenceEquals(subscription, current)) return; await DropSubscriptionAsync(); }
        finally { gate.Release(); }
        SignalChanged(); if (active && !disposed) await RefreshSubscriptionAsync();
    }

    /// <summary>Delivers invalidation on the captured UI context when available.</summary>
    private void SignalChanged()
    {
        if (uiContext is null) Changed?.Invoke(this, EventArgs.Empty);
        else uiContext.Post(_ => Changed?.Invoke(this, EventArgs.Empty), null);
    }

    /// <summary>Cancels the event pump and releases the subscription before resetting its generation.</summary>
    private async Task DropSubscriptionAsync()
    {
        pumpCancellation?.Cancel(); pumpCancellation?.Dispose(); pumpCancellation = null;
        if (subscription is { } current) { subscription = null; await current.DisposeAsync(); }
        generation = -1;
    }

    /// <summary>Detaches change handlers and releases subscriptions once.</summary>
    public async ValueTask DisposeAsync()
    {
        if (disposed) return;
        disposed = true; active = false;
        engine.StateChanged -= EngineStateChanged;
        await gate.WaitAsync();
        try { await DropSubscriptionAsync(); }
        finally { gate.Release(); }
    }

    /// <summary>Maps typed failure categories to Arabic recovery text without displaying transport diagnostics.</summary>
    public static string ArabicMessage(ProductFailureKind failure) => failure switch
    {
        ProductFailureKind.Validation => "تحقق من الاسم وتكلفة الشراء والبيانات المطلوبة.",
        ProductFailureKind.Reference => "المرجع غير نشط أو مستخدم. راجع فئة المنتج أو خيارات المورد.",
        ProductFailureKind.Conflict => "تغيرت البيانات في مكان آخر. لم تُحفظ تعديلاتك. أعد فتح السجل.",
        ProductFailureKind.Denied => "ليس لديك صلاحية لتعديل المنتجات.",
        _ => "تعذر الاتصال ببيانات المنتجات. حاول مرة أخرى.",
    };

    /// <summary>Classifies Rust error identifiers; unknown or transport failures remain unavailable.</summary>
    private static ProductFailureKind MapFailure(string? code) => code switch
    {
        ProtocolIds.ErrorCodes.EitmadErrorProductInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => ProductFailureKind.Validation,
        ProtocolIds.ErrorCodes.EitmadErrorProductReferenceInvalidV1 => ProductFailureKind.Reference,
        ProtocolIds.ErrorCodes.EitmadErrorProductRevisionConflictV1 => ProductFailureKind.Conflict,
        ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => ProductFailureKind.Denied,
        _ => ProductFailureKind.Unavailable,
    };
}
