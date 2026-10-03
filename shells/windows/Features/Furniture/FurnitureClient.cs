using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
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

/// <summary>Contains editor references without reloading the Furniture list.</summary>
public sealed record FurnitureEditorSnapshot(IReadOnlyList<Part> Parts, IReadOnlyList<PartCategory> Categories, IReadOnlyList<Part> Compositions);

/// <summary>Thin typed IPC adapter for ready-made definitions and separate furniture categories.</summary>
public sealed class FurnitureClient : IAsyncDisposable
{
    private sealed class SaveRetry
    {
        public string? Payload;
        public Guid Key;
        public bool Unresolved;
    }
    private SaveRetry furnitureRetry = new();
    private SaveRetry categoryRetry = new();
    private SynchronizationContext? uiContext;
    private bool disposed;
    private readonly SemaphoreSlim loadGate = new(1, 1);
    private FurnitureCategories? cachedCategories;
    private IReadOnlyList<Part>? cachedParts;
    private IReadOnlyList<PartCategory> cachedPartCategories = [];
    private readonly Dictionary<(Guid, long), Part> compositions = [];
    private long projectionEpoch, loadedProjectionEpoch, partsEpoch, loadedPartsEpoch = -1;

    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed changes;

    public FurnitureClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        changes = new EngineChangeFeed(engine, ProtocolIds.Capabilities.EitmadCapabilityFurnitureV1,
            Subscription.ForFurnitureChangedSubscribe(new FurnitureChanges()),
            notice => { if (notice is null || notice.AsFurnitureChangedEvent() is not null) SignalChanged(); }, InvalidateProjection, notifyUnavailable: true);
        partChanges = new EngineChangeFeed(engine, ProtocolIds.Capabilities.EitmadCapabilityPartV1,
            Subscription.ForPartChangedSubscribe(new PartChanges()),
            notice => { if (notice is null || notice.AsPartChangedEvent() is not null) SignalPartsChanged(); },
            InvalidateProjection);
    }

    private readonly EngineChangeFeed partChanges;

    public event EventHandler? Changed;
    public event EventHandler? PartsChanged;
    /// <summary>Requests immediate removal of cached data before any replacement query completes.</summary>
    public event EventHandler? ProjectionInvalidated;

    /// <summary>Loads scoped furnitures and categories through paged Rust queries; cancellation propagates.</summary>
    public async Task<FurnitureResult<FurnitureSnapshot>> LoadAsync(string term, CancellationToken cancellationToken = default, bool reloadCategories = true)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityFurnitureV1))
            return FurnitureResult<FurnitureSnapshot>.Failed(FurnitureFailureKind.Unavailable);
        await loadGate.WaitAsync(cancellationToken);
        var epoch = projectionEpoch;
        try
        {
            ResetInvalidatedCache();
            if (reloadCategories || cachedCategories is null)
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
            cachedCategories = new FurnitureCategories { Items = categoryItems.ToArray() };
            }
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
            if (epoch != projectionEpoch || disposed) return FurnitureResult<FurnitureSnapshot>.Failed(FurnitureFailureKind.Unavailable);
            return FurnitureResult<FurnitureSnapshot>.Success(new FurnitureSnapshot(cachedCategories!, items, canManage, canReadCosts, cachedParts ?? [], cachedPartCategories, compositions.Values.ToArray()));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return FurnitureResult<FurnitureSnapshot>.Failed(MapFailure(error.ContractError?.Code)); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return FurnitureResult<FurnitureSnapshot>.Failed(FurnitureFailureKind.Unavailable); }
        finally { loadGate.Release(); }
    }

    /// <summary>Loads picker data on demand and resolves only the opened definition's immutable references.</summary>
    public async Task<FurnitureResult<FurnitureEditorSnapshot>> LoadEditorAsync(Eitmad.Contracts.Furniture? definition, CancellationToken cancellationToken = default)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityFurnitureV1))
            return FurnitureResult<FurnitureEditorSnapshot>.Failed(FurnitureFailureKind.Unavailable);
        await loadGate.WaitAsync(cancellationToken);
        var epoch = projectionEpoch;
        var partEpoch = partsEpoch;
        try
        {
            ResetInvalidatedCache();
            if (cachedParts is null || loadedPartsEpoch != partEpoch)
            {
            var parts = new List<Part>(); Guid? partAfter = null;
            do {
                var response = await engine.QueryAsync(Query.ForPartList(new ListParts { Term = "", After = partAfter, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsParts() : null;
                if (page is null) return FurnitureResult<FurnitureEditorSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                parts.AddRange(page.Items.Select(p => p.Part)); partAfter = page.Next;
            } while (partAfter is not null);
            var partCategories = new List<PartCategory>(); Guid? pcAfter = null;
            do {
                var response = await engine.QueryAsync(Query.ForPartCategoryList(new ListPartCategories { After = pcAfter, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsPartCategories() : null;
                if (page is null) return FurnitureResult<FurnitureEditorSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                partCategories.AddRange(page.Items); pcAfter = page.Next;
            } while (pcAfter is not null);
            cachedParts = parts; cachedPartCategories = partCategories; loadedPartsEpoch = partEpoch;
            foreach (var part in parts) compositions.TryAdd((part.Id, part.Revision), part);
            }
            foreach (var r in (definition?.Parts ?? []).Select(p => p.Reference).DistinctBy(r => (r.PartId,r.Revision))) {
                if (compositions.ContainsKey((r.PartId, r.Revision))) continue;
                var response = await engine.QueryAsync(Query.ForPartCompositionGet(new GetPartComposition { Reference = r }), cancellationToken);
                var part = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsPartComposition() : null;
                if (part is null) return FurnitureResult<FurnitureEditorSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                compositions[(r.PartId, r.Revision)] = part;
            }
            if (epoch != projectionEpoch || disposed) return FurnitureResult<FurnitureEditorSnapshot>.Failed(FurnitureFailureKind.Unavailable);
            return FurnitureResult<FurnitureEditorSnapshot>.Success(new FurnitureEditorSnapshot(cachedParts!, cachedPartCategories, compositions.Values.ToArray()));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return FurnitureResult<FurnitureEditorSnapshot>.Failed(MapFailure(error.ContractError?.Code)); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return FurnitureResult<FurnitureEditorSnapshot>.Failed(FurnitureFailureKind.Unavailable); }
        finally { loadGate.Release(); }
    }

    /// <summary>Drops all cost-bearing references after authority invalidation, under the load gate.</summary>
    private void ResetInvalidatedCache()
    {
        if (loadedProjectionEpoch == projectionEpoch) return;
        cachedCategories = null; cachedParts = null; cachedPartCategories = []; compositions.Clear();
        loadedProjectionEpoch = projectionEpoch;
    }

    /// <summary>Queries Rust for matching Part choices without reloading Furniture or its categories.</summary>
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

    /// <summary>Requests Rust cost and validation results for staged editor fields.</summary>
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

    public async Task ActivateAsync()
    {
        uiContext = SynchronizationContext.Current;
        await changes.ActivateAsync();
        await partChanges.ActivateAsync();
    }

    /// <summary>Discards cost-bearing retry payloads and clears the UI before requesting a fresh projection.</summary>
    private void InvalidateProjection()
    {
        Interlocked.Increment(ref projectionEpoch);
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

    private void SignalChanged() => Changed?.Invoke(this, EventArgs.Empty);

    private void SignalPartsChanged()
    {
        Interlocked.Increment(ref partsEpoch);
        PartsChanged?.Invoke(this, EventArgs.Empty);
    }

    public async ValueTask DisposeAsync()
    {
        disposed = true;
        await changes.DisposeAsync();
        await partChanges.DisposeAsync();
        await loadGate.WaitAsync();
        try { cachedCategories = null; cachedParts = null; cachedPartCategories = []; compositions.Clear(); }
        finally { loadGate.Release(); }
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
