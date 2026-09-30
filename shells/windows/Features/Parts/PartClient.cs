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
    private string? retryPayload;
    private Guid retryKey;
    private bool unresolvedSave;
    private readonly SemaphoreSlim gate = new(1, 1);
    private IEngineSubscription? subscription;
    private CancellationTokenSource? pumpCancellation;
    private SynchronizationContext? uiContext;
    private long generation = -1;
    private bool active;
    private bool disposed;
    private bool connectedAndReady;

    public event EventHandler? Changed;

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

    public Task<MaterialFailureKind> SaveAsync(SavePart input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForPartSave(input), cancellationToken);
    public Task<MaterialResult<MaterialSnapshot>> SearchMaterialsAsync(string term, CancellationToken cancellationToken = default) =>
        materials.LoadAsync(term,cancellationToken);
    public Task<MaterialFailureKind> SaveAsync(SavePartCategory input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForPartCategorySave(input), cancellationToken);

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

    private async Task<MaterialFailureKind> SubmitAsync(Command command, CancellationToken cancellationToken)
    {
        try
        {
            if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityPartV1)) return MaterialFailureKind.Unavailable;
            var payload = System.Text.Json.JsonSerializer.Serialize(command);
            if (unresolvedSave && payload != retryPayload) return MaterialFailureKind.Conflict;
            if (payload != retryPayload) { retryPayload = payload; retryKey = Guid.NewGuid(); }
            unresolvedSave = true;
            var response = await engine.SubmitCommandAsync(command, retryKey, cancellationToken);
            unresolvedSave = false;
            if (response.Outcome.Status == CommandOutcomeStatus.Succeeded) retryPayload = null;
            return response.Outcome.Status == CommandOutcomeStatus.Succeeded
                ? MaterialFailureKind.None : MapFailure(response.Outcome.Payload.Code);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error)
        {
            var failure = MapFailure(error.ContractError?.Code);
            if (failure != MaterialFailureKind.Unavailable) unresolvedSave = false;
            return failure;
        }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return MaterialFailureKind.Unavailable; }
    }

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

    private void MaterialChanged(object? sender, EventArgs args) => SignalChanged();

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

    private void SignalChanged()
    {
        if (uiContext is null) Changed?.Invoke(this, EventArgs.Empty);
        else uiContext.Post(_ => Changed?.Invoke(this, EventArgs.Empty), null);
    }

    private async Task DropSubscriptionAsync()
    {
        pumpCancellation?.Cancel(); pumpCancellation?.Dispose(); pumpCancellation = null;
        if (subscription is { } current) { subscription = null; await current.DisposeAsync(); }
        generation = -1;
    }

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

    public static string ArabicMessage(MaterialFailureKind failure) => failure switch
    {
        MaterialFailureKind.Validation => "تحقق من الاسم والكميات والبيانات المطلوبة.",
        MaterialFailureKind.Reference => "المرجع غير نشط أو مستخدم. راجع فئة الجزء أو المادة أو الوحدة.",
        MaterialFailureKind.Conflict => "تغيرت البيانات في مكان آخر. لم تُحفظ تعديلاتك. أعد فتح السجل.",
        MaterialFailureKind.Denied => "ليس لديك صلاحية لتعديل الأجزاء.",
        _ => "تعذر الاتصال ببيانات الأجزاء. حاول مرة أخرى.",
    };

    private static MaterialFailureKind MapFailure(string? code) => code switch
    {
        ProtocolIds.ErrorCodes.EitmadErrorPartInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => MaterialFailureKind.Validation,
        ProtocolIds.ErrorCodes.EitmadErrorPartReferenceInvalidV1 => MaterialFailureKind.Reference,
        ProtocolIds.ErrorCodes.EitmadErrorPartRevisionConflictV1 => MaterialFailureKind.Conflict,
        ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => MaterialFailureKind.Denied,
        _ => MaterialFailureKind.Unavailable,
    };
}
