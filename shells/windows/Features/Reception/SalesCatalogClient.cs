using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Pricing;

namespace Eitmad.WindowsShell.Features.Reception;

/// <summary>Public catalog IPC only. No manager definitions or cost queries.</summary>
public sealed class SalesCatalogClient : IAsyncDisposable
{
    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed changes;
    private SynchronizationContext? context;
    internal Features.CatalogImages.CatalogImageClient Images { get; }
    /// <summary>Connects public catalog queries, image reads, and catalog change notifications to the engine bridge.</summary>
    public SalesCatalogClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        Images = new(engine);
        changes = new(engine, ProtocolIds.Capabilities.EitmadCapabilitySalesCatalogV1,
            Subscription.ForPricingChangedSubscribe(new PriceChanges()), _ => Changed?.Invoke(this, EventArgs.Empty),
            Invalidate, refreshOnStart: false, notifyUnavailable: true);
    }
    public event EventHandler? Changed;
    public event EventHandler? ProjectionInvalidated;
    /// <summary>Starts the change feed and captures the UI context for projection invalidation.</summary>
    public Task ActivateAsync() { context = SynchronizationContext.Current; return changes.ActivateAsync(); }
    /// <summary>Delivers projection invalidation on the captured UI context after access or session loss.</summary>
    private void Invalidate() { if (context is null) ProjectionInvalidated?.Invoke(this, EventArgs.Empty); else context.Post(_ => ProjectionInvalidated?.Invoke(this, EventArgs.Empty), null); }
    /// <summary>Stops catalog notifications when the receptionist session ends.</summary>
    public Task DeactivateAsync() => changes.DeactivateAsync();
    /// <summary>Releases the catalog change feed after its owning view is closed.</summary>
    public ValueTask DisposeAsync() => changes.DisposeAsync();
    /// <summary>Requests one bounded public page; Rust owns search normalization, filtering, and scope authorization.</summary>
    public Task<PricingResult<SalesCatalogPage>> LoadAsync(string term, string? category, Guid? after, CancellationToken token) =>
        QueryAsync(Query.ForSalesCatalogList(new ListSalesCatalog { Term = term, Category = category!, After = after, Limit = 30 }), r => r.AsSalesCatalog(), token);
    /// <summary>Reads the current published variants for the selected public target.</summary>
    public Task<PricingResult<SalesCatalogDetails>> ItemAsync(Dictionary<string, object> target, CancellationToken token) =>
        QueryAsync(Query.ForSalesCatalogGet(new GetSalesCatalogItem { Target = target }), r => r.AsSalesCatalogItem(), token);
    /// <summary>Decodes a public target with the Rust-generated union binding.</summary>
    internal static PriceTarget Target(CatalogEntry entry) => JsonSerializer.Deserialize<PriceTarget>(JsonSerializer.Serialize(entry.Price.Target))!;
    /// <summary>Asks Rust to validate options, dimensions, quantity, and the expected selling-price revision.</summary>
    public Task<PricingResult<SalesConfiguration>> CheckAsync(CheckSalesConfiguration input, CancellationToken token) =>
        QueryAsync(Query.ForSalesCatalogCheck(input), r => r.AsSalesConfiguration(), token);
    /// <summary>Sends quotation intent only; Rust derives descriptions, amounts, and approval requirements.</summary>
    public Task<PricingResult<QuotationEvaluation>> EvaluateAsync(EvaluateQuotation input, CancellationToken token) =>
        engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityQuotationEvaluationV1)
            ? QueryAsync(Query.ForQuotationEvaluate(input), r => r.AsQuotationEvaluation(), token)
            : Task.FromResult(new PricingResult<QuotationEvaluation>(null, PricingFailure.Unconfirmed));
    /// <summary>Requires the negotiated catalog capability and maps contract failures without exposing engine diagnostics.</summary>
    private async Task<PricingResult<T>> QueryAsync<T>(Query query, Func<QueryResult, T?> read, CancellationToken token) where T : class
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilitySalesCatalogV1)) return new(null, PricingFailure.Unconfirmed);
        try
        {
            var response = await engine.QueryAsync(query, token);
            if (response.Outcome.Status == CommandOutcomeStatus.Succeeded && read(response.Outcome.Payload) is { } value) return new(value, PricingFailure.None);
            return new(null, Map(response.Outcome.Payload.Code));
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { throw; }
        catch (EngineIpcException e) { return new(null, Map(e.ContractError?.Code)); }
        catch (Exception e) when (e is IOException or InvalidOperationException or ObjectDisposedException or JsonException) { return new(null, PricingFailure.Unconfirmed); }
    }
    /// <summary>Classifies contract failures for denial, invalid configuration, changed definitions, and stale prices.</summary>
    private static PricingFailure Map(string? code) => code switch
            {
                ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => PricingFailure.Denied,
                ProtocolIds.ErrorCodes.EitmadErrorPricingInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => PricingFailure.Invalid,
                ProtocolIds.ErrorCodes.EitmadErrorPricingReferenceInvalidV1 => PricingFailure.Reference,
                ProtocolIds.ErrorCodes.EitmadErrorPricingRevisionConflictV1 => PricingFailure.Conflict,
                _ => PricingFailure.Unconfirmed
            };
    /// <summary>Provides Arabic recovery guidance for each catalog failure without displaying raw contract errors.</summary>
    public static string Message(PricingFailure failure) => failure switch
    {
        PricingFailure.Denied => "ليس لديك صلاحية لعرض الكتالوج.",
        PricingFailure.Invalid => "تحقق من المقاسات والكمية والخيارات المسموحة.",
        PricingFailure.Reference => "الصنف مؤرشف أو تغير تعريفه أو خياراته. حدّث الصنف واختر من جديد.",
        PricingFailure.Conflict => "تغير سعر البيع. حدّث الصنف وراجع السعر الجديد.",
        _ => "تعذر الاتصال بالمحرك. الاختيار غير متحقق؛ أعد المحاولة."
    };
}
