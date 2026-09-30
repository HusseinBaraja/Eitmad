using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Parts;

using Eitmad.WindowsShell.Features.RawMaterials;

public sealed record PartSnapshot(PartCategories Categories, IReadOnlyList<PartProjection> Parts, MaterialSnapshot Materials);

/// <summary>Thin typed IPC adapter for parts and their material references.</summary>
public sealed class PartClient(IEngineShellBridge engine) : IAsyncDisposable
{
    private readonly MaterialClient materials = new(engine);
    private sealed class SaveRetry
    {
        public string? Payload;
        public Guid Key;
        public bool Unresolved;
    }
    private readonly SaveRetry partRetry = new();
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

    /// <summary>Loads scoped parts, categories, and material references through paged Rust queries; cancellation propagates.</summary>
    public async Task<MaterialResult<PartSnapshot>> LoadAsync(string term, CancellationToken cancellationToken = default)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityPartV1))
            return MaterialResult<PartSnapshot>.Failed(MaterialFailureKind.Unavailable);
        try
        {
            var categoryItems = new List<PartCategory>();
            Guid? categoryAfter = null;
            do
            {
                var response = await engine.QueryAsync(Query.ForPartCategoryList(new ListPartCategories { After = categoryAfter, Limit = 100 }),cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsPartCategories() : null;
                if (page is null) return MaterialResult<PartSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                categoryItems.AddRange(page.Items); categoryAfter = page.Next;
            } while (categoryAfter is not null);
            var categories = new PartCategories { Items = categoryItems.ToArray() };
            var materialData = await materials.LoadAsync(string.Empty, cancellationToken);
            if (!materialData.Succeeded) return MaterialResult<PartSnapshot>.Failed(materialData.Failure);
            var items = new List<PartProjection>();
            Guid? after = null;
            do
            {
                var response = await engine.QueryAsync(Query.ForPartList(new ListParts { Term = term, After = after, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsParts() : null;
                if (page is null) return MaterialResult<PartSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                items.AddRange(page.Items); after = page.Next;
            } while (after is not null);
            return MaterialResult<PartSnapshot>.Success(new PartSnapshot(categories, items, materialData.Value!));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return MaterialResult<PartSnapshot>.Failed(MapFailure(error.ContractError?.Code)); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return MaterialResult<PartSnapshot>.Failed(MaterialFailureKind.Unavailable); }
    }

    /// <summary>Submits a typed save with the retry state reserved for its record kind.</summary>
    public Task<MaterialFailureKind> SaveAsync(SavePart input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForPartSave(input), partRetry, cancellationToken);
    /// <summary>Passes search text to Rust so the shell does not duplicate Arabic matching rules.</summary>
    public Task<MaterialResult<MaterialSnapshot>> SearchMaterialsAsync(string term, CancellationToken cancellationToken = default) =>
        materials.LoadAsync(term,cancellationToken);
    /// <summary>Submits a typed save with the retry state reserved for its record kind.</summary>
    public Task<MaterialFailureKind> SaveAsync(SavePartCategory input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForPartCategorySave(input), categoryRetry, cancellationToken);

    /// <summary>Requests authoritative current costs, including retained references for an existing part.</summary>
    public async Task<MaterialResult<PartCost>> CostAsync(PartUsage[] usages, Guid? partId = null, CancellationToken cancellationToken = default)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityPartV1))
            return MaterialResult<PartCost>.Failed(MaterialFailureKind.Unavailable);
        try
        {
            var response = await engine.QueryAsync(Query.ForPartCost(new CalculatePartCost { Usages = usages, PartId = partId }), cancellationToken);
            var cost = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsPartCost() : null;
            return cost is null ? MaterialResult<PartCost>.Failed(MapFailure(response.Outcome.Payload.Code)) : MaterialResult<PartCost>.Success(cost);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return MaterialResult<PartCost>.Failed(MapFailure(error.ContractError?.Code)); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return MaterialResult<PartCost>.Failed(MaterialFailureKind.Unavailable); }
    }

    /// <summary>Freezes the payload and key after an unknown outcome; only an exact retry can resolve that record kind.</summary>
    private async Task<MaterialFailureKind> SubmitAsync(Command command, SaveRetry retry, CancellationToken cancellationToken)
    {
        try
        {
            if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityPartV1)) return MaterialFailureKind.Unavailable;
            var payload = System.Text.Json.JsonSerializer.Serialize(command);
            if (retry.Unresolved && payload != retry.Payload) return MaterialFailureKind.Conflict;
            if (payload != retry.Payload) { retry.Payload = payload; retry.Key = Guid.NewGuid(); }
            retry.Unresolved = true;
            var response = await engine.SubmitCommandAsync(command, retry.Key, cancellationToken);
            retry.Unresolved = false;
            if (response.Outcome.Status == CommandOutcomeStatus.Succeeded) retry.Payload = null;
            return response.Outcome.Status == CommandOutcomeStatus.Succeeded
                ? MaterialFailureKind.None : MapFailure(response.Outcome.Payload.Code);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error)
        {
            var failure = MapFailure(error.ContractError?.Code);
            if (failure != MaterialFailureKind.Unavailable) retry.Unresolved = false;
            return failure;
        }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return MaterialFailureKind.Unavailable; }
    }

    /// <summary>Captures the UI context and starts part and material change subscriptions once.</summary>
    public async Task ActivateAsync()
    {
        if (disposed || active) return;
        active = true;
        uiContext = SynchronizationContext.Current;
        var snapshot = engine.Snapshot;
        connectedAndReady = snapshot.IpcHealth == EngineIpcHealthState.Connected
            && snapshot.LastLifecycle?.Ready == true;
        engine.StateChanged += EngineStateChanged;
        materials.Changed += MaterialChanged;
        await materials.ActivateAsync();
        await RefreshSubscriptionAsync();
    }

    /// <summary>Invalidates advisory part costs when Rust publishes a material change.</summary>
    private void MaterialChanged(object? sender, EventArgs args) => SignalChanged();

    /// <summary>Refreshes on connection restoration and replaces subscriptions after an engine generation change.</summary>
    private void EngineStateChanged(EngineSupervisionSnapshot snapshot)
    {
        if (!active) return;
        var ready = snapshot.IpcHealth == EngineIpcHealthState.Connected
            && snapshot.LastLifecycle?.Ready == true;
        if (!ready)
        {
            connectedAndReady = false;
            return;
        }

        var restored = !connectedAndReady;
        connectedAndReady = true;
        if (snapshot.Generation != generation) _ = RefreshSubscriptionAsync();
        else if (restored) SignalChanged();
    }

    /// <summary>Serializes subscription replacement and starts an acknowledged event pump for the current generation.</summary>
    private async Task RefreshSubscriptionAsync()
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityPartV1)) return;
        await gate.WaitAsync();
        try
        {
            if (!active || disposed || generation == engine.Snapshot.Generation && subscription is not null) return;
            await DropSubscriptionAsync();
            var current = await engine.SubscribeAsync(Subscription.ForPartChangedSubscribe(new PartChanges()));
            subscription = current;
            generation = engine.Snapshot.Generation;
            pumpCancellation = new CancellationTokenSource();
            current.ResyncRequired += SignalChanged;
            _ = PumpAsync(current, pumpCancellation.Token);
            SignalChanged();
        }
        catch (EngineIpcException) { SignalChanged(); }
        finally { gate.Release(); }
    }

    /// <summary>Signals refresh for typed part events and acknowledges delivery; transport failure requests resynchronization.</summary>
    private async Task PumpAsync(IEngineSubscription current, CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var delivered in current.ReadAllAsync(cancellationToken))
            {
                if (EngineContractCodec.DecodeEvent(delivered).AsPartChangedEvent() is not null) SignalChanged();
                current.Acknowledge(delivered);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { }
        catch (Exception error) when (error is EngineIpcException or IOException or InvalidDataException)
        { SignalChanged(); }
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

    /// <summary>Detaches change handlers and releases subscriptions and synchronization resources once.</summary>
    public async ValueTask DisposeAsync()
    {
        if (disposed) return;
        disposed = true; active = false;
        engine.StateChanged -= EngineStateChanged;
        materials.Changed -= MaterialChanged;
        await materials.DisposeAsync();
        await gate.WaitAsync();
        try { await DropSubscriptionAsync(); }
        finally { gate.Release(); gate.Dispose(); }
    }

    /// <summary>Maps typed failure categories to Arabic recovery text without displaying transport diagnostics.</summary>
    public static string ArabicMessage(MaterialFailureKind failure) => failure switch
    {
        MaterialFailureKind.Validation => "تحقق من الاسم والكميات والبيانات المطلوبة.",
        MaterialFailureKind.Reference => "المرجع غير نشط أو مستخدم. راجع فئة الجزء أو المادة أو الوحدة.",
        MaterialFailureKind.Conflict => "تغيرت البيانات في مكان آخر. لم تُحفظ تعديلاتك. أعد فتح السجل.",
        MaterialFailureKind.Denied => "ليس لديك صلاحية لتعديل الأجزاء.",
        _ => "تعذر الاتصال ببيانات الأجزاء. حاول مرة أخرى.",
    };

    /// <summary>Classifies Rust error identifiers; unknown or transport failures remain unavailable.</summary>
    private static MaterialFailureKind MapFailure(string? code) => code switch
    {
        ProtocolIds.ErrorCodes.EitmadErrorPartInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => MaterialFailureKind.Validation,
        ProtocolIds.ErrorCodes.EitmadErrorPartReferenceInvalidV1 => MaterialFailureKind.Reference,
        ProtocolIds.ErrorCodes.EitmadErrorPartRevisionConflictV1 => MaterialFailureKind.Conflict,
        ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => MaterialFailureKind.Denied,
        _ => MaterialFailureKind.Unavailable,
    };
}
