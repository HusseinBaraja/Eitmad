using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Parts;

using Eitmad.WindowsShell.Features.RawMaterials;

public sealed record PartSnapshot(PartCategories Categories, IReadOnlyList<PartProjection> Parts, MaterialSnapshot Materials);

public sealed class PartClient : IAsyncDisposable
{
    private readonly MaterialClient materials;
    private sealed class SaveRetry
    {
        public string? Payload;
        public Guid Key;
        public bool Unresolved;
    }
    private readonly SaveRetry partRetry = new();
    private readonly SaveRetry categoryRetry = new();
    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed changes;

    public PartClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        changes = new EngineChangeFeed(engine, ProtocolIds.Capabilities.EitmadCapabilityPartV1,
            Subscription.ForPartChangedSubscribe(new PartChanges()),
            notice => { if (notice is null || notice.AsPartChangedEvent() is not null) SignalChanged(); });
        materials = new MaterialClient(engine);
        materials.Changed += MaterialChanged;
    }

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
        SubmitAsync(Command.ForPartSave(input), partRetry, cancellationToken);
    public Task<MaterialResult<MaterialSnapshot>> SearchMaterialsAsync(string term, CancellationToken cancellationToken = default) =>
        materials.LoadAsync(term,cancellationToken);
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

    public async Task ActivateAsync()
    {
        await materials.ActivateAsync();
        await changes.ActivateAsync();
    }

    /// <summary>Invalidates advisory part costs when Rust publishes a material change.</summary>
    private void MaterialChanged(object? sender, EventArgs args) => SignalChanged();

    private void SignalChanged() => Changed?.Invoke(this, EventArgs.Empty);

    public async ValueTask DisposeAsync()
    {
        materials.Changed -= MaterialChanged;
        await materials.DisposeAsync();
        await changes.DisposeAsync();
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
