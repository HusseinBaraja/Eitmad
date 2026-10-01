using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Furniture;



public enum FurnitureFailureKind { None, Validation, Reference, Conflict, Denied, Unavailable }
/// <summary>Carries a typed IPC value or a failure category for presentation recovery.</summary>
public sealed record FurnitureResult<T>(T? Value, FurnitureFailureKind Failure)
{
    public bool Succeeded => Failure == FurnitureFailureKind.None && Value is not null;
    /// <summary>Wraps a received authority value as a successful presentation result.</summary>
    public static FurnitureResult<T> Success(T value) => new(value, FurnitureFailureKind.None);
    /// <summary>Creates a failed presentation result without inventing an authority value.</summary>
    public static FurnitureResult<T> Failed(FurnitureFailureKind failure) => new(default, failure);
}

/// <summary>Groups paged authority records with the management and cost-access flags used by the page.</summary>
public sealed record FurnitureSnapshot(FurnitureCategories Categories, IReadOnlyList<Eitmad.Contracts.Furniture> Furniture, bool CanManage, bool CanReadCosts, IReadOnlyList<Part> Parts, IReadOnlyList<PartCategory> PartCategories, IReadOnlyList<Part> Compositions);

/// <summary>Thin typed IPC adapter for ready-made definitions and separate furniture categories.</summary>
public sealed class FurnitureClient(IEngineShellBridge engine) : IAsyncDisposable
{
    private sealed class SaveRetry
    {
        public string? Payload;
        public Guid Key;
        public bool Unresolved;
    }
    private SaveRetry furnitureRetry = new();
    private SaveRetry categoryRetry = new();
    private readonly SemaphoreSlim gate = new(1, 1);
    private IEngineSubscription? subscription;
    private IEngineSubscription? partSubscription;
    private CancellationTokenSource? pumpCancellation;
    private SynchronizationContext? uiContext;
    private long generation = -1;
    private bool active;
    private bool disposed;
    private bool connectedAndReady;

    public event EventHandler? Changed;
    /// <summary>Requests immediate removal of cached data before any replacement query completes.</summary>
    public event EventHandler? ProjectionInvalidated;

    /// <summary>Loads scoped furnitures and categories through paged Rust queries; cancellation propagates.</summary>
    public async Task<FurnitureResult<FurnitureSnapshot>> LoadAsync(string term, CancellationToken cancellationToken = default)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityFurnitureV1))
            return FurnitureResult<FurnitureSnapshot>.Failed(FurnitureFailureKind.Unavailable);
        try
        {
            var categoryItems = new List<FurnitureCategory>();
            Guid? categoryAfter = null;
            do
            {
                var response = await engine.QueryAsync(Query.ForFurnitureCategoryList(new ListFurnitureCategories { After = categoryAfter, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsFurnitureCategories() : null;
                if (page is null) return FurnitureResult<FurnitureSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                categoryItems.AddRange(page.Items); categoryAfter = page.Next;
            } while (categoryAfter is not null);
            var categories = new FurnitureCategories { Items = categoryItems.ToArray() };
            var items = new List<Eitmad.Contracts.Furniture>();
            Guid? after = null;
            var canManage = true; var canReadCosts = true;
            do
            {
                var response = await engine.QueryAsync(Query.ForFurnitureList(new ListFurnitures { Term = term, After = after, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsFurnitures() : null;
                if (page is null) return FurnitureResult<FurnitureSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                items.AddRange(page.Items); after = page.Next; canManage &= page.CanManage; canReadCosts &= page.CanReadCosts;
            } while (after is not null);
            var parts = new List<Part>(); Guid? partAfter = null;
            do {
                var response = await engine.QueryAsync(Query.ForPartList(new ListParts { Term = "", After = partAfter, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsParts() : null;
                if (page is null) return FurnitureResult<FurnitureSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                parts.AddRange(page.Items.Select(p => p.Part)); partAfter = page.Next;
            } while (partAfter is not null);
            var partCategories = new List<PartCategory>(); Guid? pcAfter = null;
            do {
                var response = await engine.QueryAsync(Query.ForPartCategoryList(new ListPartCategories { After = pcAfter, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsPartCategories() : null;
                if (page is null) return FurnitureResult<FurnitureSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                partCategories.AddRange(page.Items); pcAfter = page.Next;
            } while (pcAfter is not null);
            var compositions = new List<Part>();
            foreach (var r in items.SelectMany(p => p.Parts).Select(p => p.Reference).DistinctBy(r => (r.PartId,r.Revision))) {
                var response = await engine.QueryAsync(Query.ForPartCompositionGet(new GetPartComposition { Reference = r }), cancellationToken);
                var part = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsPartComposition() : null;
                if (part is null) return FurnitureResult<FurnitureSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                compositions.Add(part);
            }
            return FurnitureResult<FurnitureSnapshot>.Success(new FurnitureSnapshot(categories, items, canManage, canReadCosts, parts, partCategories, compositions));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return FurnitureResult<FurnitureSnapshot>.Failed(MapFailure(error.ContractError?.Code)); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return FurnitureResult<FurnitureSnapshot>.Failed(FurnitureFailureKind.Unavailable); }
    }

    public async Task<FurnitureResult<IReadOnlyList<Part>>> SearchPartsAsync(string term, CancellationToken cancellationToken = default)
    {
        try {
            var parts=new List<Part>(); Guid? after=null;
            do {
                var response=await engine.QueryAsync(Query.ForPartList(new ListParts { Term=term,After=after,Limit=100 }),cancellationToken);
                var page=response.Outcome.Status==CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsParts() : null;
                if (page is null) return FurnitureResult<IReadOnlyList<Part>>.Failed(MapFailure(response.Outcome.Payload.Code));
                parts.AddRange(page.Items.Select(p=>p.Part));after=page.Next;
            } while(after is not null);
            return FurnitureResult<IReadOnlyList<Part>>.Success(parts);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return FurnitureResult<IReadOnlyList<Part>>.Failed(MapFailure(error.ContractError?.Code)); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException) { return FurnitureResult<IReadOnlyList<Part>>.Failed(FurnitureFailureKind.Unavailable); }
    }

    public async Task<FurnitureResult<FurnitureReview>> ReviewAsync(SaveFurniture input, CancellationToken cancellationToken = default)
    {
        try {
            var response = await engine.QueryAsync(Query.ForFurnitureReview(input), cancellationToken);
            var value = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsFurnitureReview() : null;
            return value is null ? FurnitureResult<FurnitureReview>.Failed(MapFailure(response.Outcome.Payload.Code)) : FurnitureResult<FurnitureReview>.Success(value);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return FurnitureResult<FurnitureReview>.Failed(MapFailure(error.ContractError?.Code)); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException) { return FurnitureResult<FurnitureReview>.Failed(FurnitureFailureKind.Unavailable); }
    }

    /// <summary>Submits a typed save with the retry state reserved for its record kind.</summary>
    public Task<FurnitureFailureKind> SaveAsync(SaveFurniture input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForFurnitureSave(input), furnitureRetry, cancellationToken);
    /// <summary>Submits a typed save with the retry state reserved for its record kind.</summary>
    public Task<FurnitureFailureKind> SaveAsync(SaveFurnitureCategory input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForFurnitureCategorySave(input), categoryRetry, cancellationToken);

    /// <summary>Freezes the payload and key after an unknown outcome; only an exact retry can resolve that record kind.</summary>
    private async Task<FurnitureFailureKind> SubmitAsync(Command command, SaveRetry retry, CancellationToken cancellationToken)
    {
        try
        {
            if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityFurnitureV1)) return FurnitureFailureKind.Unavailable;
            var payload = System.Text.Json.JsonSerializer.Serialize(command);
            if (retry.Unresolved && payload != retry.Payload) return FurnitureFailureKind.Conflict;
            if (payload != retry.Payload) { retry.Payload = payload; retry.Key = Guid.NewGuid(); }
            retry.Unresolved = true;
            var response = await engine.SubmitCommandAsync(command, retry.Key, cancellationToken);
            var failure = response.Outcome.Status == CommandOutcomeStatus.Succeeded
                ? FurnitureFailureKind.None : MapFailure(response.Outcome.Payload.Code);
            retry.Unresolved = failure == FurnitureFailureKind.Unavailable;
            if (failure == FurnitureFailureKind.None) retry.Payload = null;
            return failure;
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error)
        {
            var failure = MapFailure(error.ContractError?.Code);
            if (failure != FurnitureFailureKind.Unavailable) retry.Unresolved = false;
            return failure;
        }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return FurnitureFailureKind.Unavailable; }
    }

    /// <summary>Captures the UI context and starts furniture change subscriptions once.</summary>
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
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityFurnitureV1)) return;
        await gate.WaitAsync();
        try
        {
            if (!active || disposed || generation == engine.Snapshot.Generation && subscription is not null) return;
            await DropSubscriptionAsync();
            var current = await engine.SubscribeAsync(Subscription.ForFurnitureChangedSubscribe(new FurnitureChanges()));
            subscription = current;
            partSubscription = await engine.SubscribeAsync(Subscription.ForPartChangedSubscribe(new PartChanges()));
            generation = engine.Snapshot.Generation;
            pumpCancellation = new CancellationTokenSource();
            current.ResyncRequired += SignalChanged;
            _ = PumpAsync(current, pumpCancellation.Token);
            partSubscription.ResyncRequired += SignalChanged;
            _ = PumpAsync(partSubscription, pumpCancellation.Token);
            SignalChanged();
        }
        catch (Exception error) when (error is EngineIpcException or IOException or ObjectDisposedException) { SignalChanged(); }
        finally { gate.Release(); }
    }

    /// <summary>Signals refresh for typed furniture events and acknowledges delivery; transport failure requests resynchronization.</summary>
    private async Task PumpAsync(IEngineSubscription current, CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var delivered in current.ReadAllAsync(cancellationToken))
            {
                var decoded = EngineContractCodec.DecodeEvent(delivered);
                if (decoded.AsFurnitureChangedEvent() is not null || decoded.AsPartChangedEvent() is not null) SignalChanged();
                current.Acknowledge(delivered);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { }
        catch (Exception error) when (error is EngineIpcException or IOException or InvalidDataException)
        { await RecoverSubscriptionAsync(current, error is EngineIpcException { Kind: EngineIpcFailureKind.SessionChanged }); }
    }

    /// <summary>Replaces a failed stream; policy closure first removes the prior cost-bearing projection.</summary>
    private async Task RecoverSubscriptionAsync(IEngineSubscription current, bool invalidateProjection)
    {
        if (disposed) return;
        await gate.WaitAsync();
        try
        {
            if (disposed || !ReferenceEquals(subscription, current) && !ReferenceEquals(partSubscription, current)) return;
            if (invalidateProjection) InvalidateProjection();
            await DropSubscriptionAsync();
        }
        finally { gate.Release(); }
        if (!invalidateProjection) SignalChanged();
        if (active && !disposed) await RefreshSubscriptionAsync();
    }

    /// <summary>Discards cost-bearing retry payloads and clears the UI before requesting a fresh projection.</summary>
    private void InvalidateProjection()
    {
        furnitureRetry.Payload = null;
        categoryRetry.Payload = null;
        furnitureRetry = new SaveRetry();
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
        if (partSubscription is { } parts) { partSubscription = null; await parts.DisposeAsync(); }
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
    public static string ArabicMessage(FurnitureFailureKind failure) => failure switch
    {
        FurnitureFailureKind.Validation => "تحقق من الأجزاء والمقاسات والخيارات والأسعار.",
        FurnitureFailureKind.Reference => "المرجع غير نشط أو غير متوافق. راجع الأجزاء والخيارات.",
        FurnitureFailureKind.Conflict => "تغيرت البيانات في مكان آخر. لم تُحفظ تعديلاتك. أعد فتح السجل.",
        FurnitureFailureKind.Denied => "ليس لديك صلاحية لتعديل الأثاث.",
        _ => "تعذر الاتصال ببيانات الأثاث. حاول مرة أخرى.",
    };

    /// <summary>Classifies Rust error identifiers; unknown or transport failures remain unavailable.</summary>
    private static FurnitureFailureKind MapFailure(string? code) => code switch
    {
        ProtocolIds.ErrorCodes.EitmadErrorFurnitureInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => FurnitureFailureKind.Validation,
        ProtocolIds.ErrorCodes.EitmadErrorFurnitureReferenceInvalidV1 => FurnitureFailureKind.Reference,
        ProtocolIds.ErrorCodes.EitmadErrorFurnitureRevisionConflictV1 => FurnitureFailureKind.Conflict,
        ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => FurnitureFailureKind.Denied,
        _ => FurnitureFailureKind.Unavailable,
    };
}
