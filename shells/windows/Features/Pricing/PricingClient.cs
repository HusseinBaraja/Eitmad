using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Pricing;

public enum PricingFailure { None, Invalid, Reference, Conflict, BelowCost, Denied, Unconfirmed }
public sealed record PricingResult<T>(T? Value, PricingFailure Failure) where T : class
{
    public bool Succeeded => Failure == PricingFailure.None && Value is not null;
}

public sealed class PricingClient : IAsyncDisposable
{
    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed changes;
    private readonly EngineChangeFeed products;
    private readonly EngineChangeFeed furniture;
    private string? pendingPayload;
    private Guid pendingKey;
    private bool unresolved;

    public PricingClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        changes = new(engine, ProtocolIds.Capabilities.EitmadCapabilityPricingV1,
            Subscription.ForPricingChangedSubscribe(new PriceChanges()), _ => Changed?.Invoke(this, EventArgs.Empty), Invalidate, notifyUnavailable: true);
        products = new(engine, ProtocolIds.Capabilities.EitmadCapabilityProductV1,
            Subscription.ForProductChangedSubscribe(new ProductChanges()), _ => Changed?.Invoke(this, EventArgs.Empty), Invalidate);
        furniture = new(engine, ProtocolIds.Capabilities.EitmadCapabilityFurnitureV1,
            Subscription.ForFurnitureChangedSubscribe(new FurnitureChanges()), _ => Changed?.Invoke(this, EventArgs.Empty), Invalidate);
    }
    public event EventHandler? Changed;
    public event EventHandler? ProjectionInvalidated;

    public async Task ActivateAsync()
    {
        await changes.ActivateAsync();
        await products.ActivateAsync();
        await furniture.ActivateAsync();
    }
    private void Invalidate()
    {
        pendingPayload = null;
        unresolved = false;
        ProjectionInvalidated?.Invoke(this, EventArgs.Empty);
    }
    public async Task<PricingResult<PricePage>> LoadAsync(string term, CancellationToken cancellationToken = default)
    {
        var items = new List<PriceItem>();
        string? after = null;
        bool manage = true, costs = true, serverAvailable = false;
        CatalogSyncIssue[] syncIssues = [];
        do
        {
            var result = await QueryAsync(Query.ForPricingList(new ListPrices { Term = term, After = after!, Limit = 100 }), p => p.AsPrices(), cancellationToken);
            if (!result.Succeeded) return result;
            var page = result.Value!;
            if (after is null) { serverAvailable = page.ServerAvailable; syncIssues = page.CatalogSyncIssues ?? []; }
            items.AddRange(page.Items);
            manage &= page.CanManage;
            costs &= page.CanReadCosts;
            if (page.Next == after && after is not null) return new(null, PricingFailure.Unconfirmed);
            after = page.Next;
        } while (after is not null);
        if (!costs) foreach (var item in items) { item.CostYer = null; item.MarginYer = null; }
        if (!manage) items.RemoveAll(item => item.Published is null);
        return new(new PricePage { Items = items.ToArray(), CanManage = manage, CanReadCosts = costs, ServerAvailable = serverAvailable, CatalogSyncIssues = manage ? syncIssues : [] }, PricingFailure.None);
    }
    public Task<PricingResult<PriceReview>> ReviewAsync(ReviewPrice input, CancellationToken cancellationToken = default) =>
        QueryAsync(Query.ForPricingReview(input), p => p.AsPriceReview(), cancellationToken);

    private async Task<PricingResult<T>> QueryAsync<T>(Query query, Func<QueryResult, T?> read, CancellationToken cancellationToken) where T : class
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityPricingV1)) return new(null, PricingFailure.Unconfirmed);
        try
        {
            var response = await engine.QueryAsync(query, cancellationToken);
            var value = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? read(response.Outcome.Payload) : null;
            return value is null ? new(null, Map(response.Outcome.Payload.Code)) : new(value, PricingFailure.None);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException e) { return new(null, Map(e.ContractError?.Code)); }
        catch (Exception e) when (e is IOException or InvalidOperationException or ObjectDisposedException) { return new(null, PricingFailure.Unconfirmed); }
    }
    public async Task<PricingResult<PublishedPrice>> PublishAsync(PublishPrice input, CancellationToken cancellationToken = default)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityPricingV1)) return new(null, PricingFailure.Unconfirmed);
        var command = Command.ForPricingPublish(input);
        var payload = JsonSerializer.Serialize(command);
        if (unresolved && payload != pendingPayload) return new(null, PricingFailure.Conflict);
        if (payload != pendingPayload) { pendingPayload = payload; pendingKey = Guid.NewGuid(); }
        unresolved = true;
        try
        {
            var response = await engine.SubmitCommandAsync(command, pendingKey, cancellationToken);
            var value = response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsPricePublished() : null;
            var failure = value is null ? Map(response.Outcome.Payload.Code) : PricingFailure.None;
            unresolved = failure == PricingFailure.Unconfirmed;
            if (failure == PricingFailure.None) pendingPayload = null;
            return new(value, failure);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException e) { var failure = Map(e.ContractError?.Code); unresolved = failure == PricingFailure.Unconfirmed; return new(null, failure); }
        catch (Exception e) when (e is IOException or InvalidOperationException or ObjectDisposedException) { return new(null, PricingFailure.Unconfirmed); }
    }
    public async ValueTask DisposeAsync()
    {
        await changes.DisposeAsync();
        await products.DisposeAsync();
        await furniture.DisposeAsync();
        pendingPayload = null;
    }
    public static string ArabicMessage(PricingFailure failure) => failure switch
    {
        PricingFailure.Invalid => "أدخل سعر بيع صحيحاً بالريال اليمني دون كسور وأكبر من صفر.",
        PricingFailure.Reference => "تغير تعريف المنتج أو المقاس. أعد تحميل الأسعار وافتح السجل مجدداً.",
        PricingFailure.Conflict => "تغير السعر في مكان آخر. أعد تحميل الأسعار قبل التعديل.",
        PricingFailure.BelowCost => "السعر أقل من التكلفة. أكد النشر بعد مراجعة التحذير.",
        PricingFailure.Denied => "ليس لديك صلاحية لتنفيذ هذه العملية.",
        _ => "لم يتأكد نشر السعر من الخادم. أعد المحاولة بنفس البيانات.",
    };
    private static PricingFailure Map(string? code) => code switch
    {
        ProtocolIds.ErrorCodes.EitmadErrorPricingInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => PricingFailure.Invalid,
        ProtocolIds.ErrorCodes.EitmadErrorPricingReferenceInvalidV1 => PricingFailure.Reference,
        ProtocolIds.ErrorCodes.EitmadErrorPricingRevisionConflictV1 => PricingFailure.Conflict,
        ProtocolIds.ErrorCodes.EitmadErrorPricingBelowCostV1 => PricingFailure.BelowCost,
        ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => PricingFailure.Denied,
        _ => PricingFailure.Unconfirmed,
    };
}
