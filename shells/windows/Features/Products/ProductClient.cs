using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Products;

public enum ProductFailureKind { None, Validation, Reference, Conflict, Denied, Unavailable }
/// <summary>Carries a typed IPC value or a failure category for presentation recovery.</summary>
public sealed record ProductResult<T>(T? Value, ProductFailureKind Failure)
{
    public bool Succeeded => Failure == ProductFailureKind.None && Value is not null;
    /// <summary>Wraps a received authority value as a successful presentation result.</summary>
    public static ProductResult<T> Success(T value) => new(value, ProductFailureKind.None);
    /// <summary>Creates a failed presentation result without inventing an authority value.</summary>
    public static ProductResult<T> Failed(ProductFailureKind failure) => new(default, failure);
}

/// <summary>Groups paged authority records with the management and cost-access flags used by the page.</summary>
public sealed record ProductSnapshot(ProductCategories Categories, IReadOnlyList<Product> Products, bool CanManage, bool CanReadCosts);

/// <summary>Thin typed IPC adapter for ready-made definitions and separate product categories.</summary>
public sealed class ProductClient : IAsyncDisposable
{
    private sealed class SaveRetry
    {
        public string? Payload;
        public Guid Key;
        public bool Unresolved;
    }
    private SaveRetry productRetry = new();
    private SaveRetry categoryRetry = new();
    private SynchronizationContext? uiContext;
    private bool disposed;
    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed changes;

    public ProductClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        changes = new EngineChangeFeed(engine, ProtocolIds.Capabilities.EitmadCapabilityProductV1,
            Subscription.ForProductChangedSubscribe(new ProductChanges()),
            notice => { if (notice is null || notice.AsProductChangedEvent() is not null) SignalChanged(); }, InvalidateProjection, notifyUnavailable: true);
    }

    public event EventHandler? Changed;
    /// <summary>Requests immediate removal of cached data before any replacement query completes.</summary>
    public event EventHandler? ProjectionInvalidated;

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
            var failure = response.Outcome.Status == CommandOutcomeStatus.Succeeded
                ? ProductFailureKind.None : MapFailure(response.Outcome.Payload.Code);
            retry.Unresolved = failure == ProductFailureKind.Unavailable;
            if (failure == ProductFailureKind.None) retry.Payload = null;
            return failure;
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

    public async Task ActivateAsync()
    {
        uiContext = SynchronizationContext.Current;
        await changes.ActivateAsync();
    }

    /// <summary>Discards cost-bearing retry payloads and clears the UI before requesting a fresh projection.</summary>
    private void InvalidateProjection()
    {
        productRetry.Payload = null;
        categoryRetry.Payload = null;
        productRetry = new SaveRetry();
        categoryRetry = new SaveRetry();
        void Notify()
        {
            if (disposed) return;
            ProjectionInvalidated?.Invoke(this, EventArgs.Empty);
            Changed?.Invoke(this, EventArgs.Empty);
        }
        if (uiContext is null) Notify();
        else uiContext.Post(_ => Notify(), null);
    }

    private void SignalChanged() => Changed?.Invoke(this, EventArgs.Empty);

    public async ValueTask DisposeAsync()
    {
        disposed = true;
        await changes.DisposeAsync();
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
