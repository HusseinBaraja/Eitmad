using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.Quotations;

namespace Eitmad.WindowsShell.Tests.Orders;

[TestClass]
public sealed class QuotationConversionTests
{
    [TestMethod]
    public async Task UncertainConversionRetainsExactIntentAcrossSelectionAndRefresh()
    {
        var fixture = new QuotationLifecycleTests.Fixture(); await using var engine = fixture.Engine;
        ((HashSet<string>)engine.SupportedCapabilities!).Add(ProtocolIds.Capabilities.EitmadCapabilityOrdersV1);
        fixture.Record.State = QuotationState.Accepted; fixture.Record.PermittedActions = [QuotationPermittedAction.Convert];
        var other = QuotationDraftTests.Copy(fixture.Record); other.Quotation.Id = Guid.NewGuid();
        var query = engine.QueryHandler;
        engine.QueryHandler = value => value.AsQuotationList() is not null
            ? SalesCatalogAuthorityTests.Response(QueryResult.ForQuotations(new() { Items = [QuotationDraftTests.Copy(fixture.Record), QuotationDraftTests.Copy(other)], ServerAvailable = true }))
            : query!(value);
        await using var drafts = new QuotationDraftClient(engine); await using var orders = new OrderClient(engine);
        var model = new QuotationsViewModel(true); model.AttachDraftClient(drafts); model.AttachOrders(orders);
        await model.ActivateDraftsAsync();
        var sent = new List<(Guid Key, string Intent)>();
        engine.CommandHandler = command => {
            sent.Add((engine.LastIdempotencyKey, JsonSerializer.Serialize(command.AsOrderConvert())));
            throw new IOException("Synthetic lost conversion reply");
        };
        model.OpenQuotation(model.VisibleQuotations.Single(q => q.Id == fixture.Record.Quotation.Id)); await model.ConvertAsync();
        model.OpenQuotation(model.VisibleQuotations.Single(q => q.Id == other.Quotation.Id)); await model.ConvertAsync();
        fixture.Record.Revision = 2; await model.ActivateDraftsAsync();
        model.OpenQuotation(model.VisibleQuotations.Single(q => q.Id == fixture.Record.Quotation.Id)); await model.ConvertAsync();
        Assert.AreEqual(sent[0], sent[2]); Assert.AreNotEqual(sent[0].Key, sent[1].Key);

        engine.CommandHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = ProtocolIds.ErrorCodes.EitmadErrorOrderConflictV1 } } };
        await model.ConvertAsync(); Assert.AreEqual(sent[0].Key, engine.LastIdempotencyKey);
        await model.ConvertAsync(); Assert.AreNotEqual(sent[0].Key, engine.LastIdempotencyKey);
        Assert.AreEqual(2L, engine.LastCommand!.AsOrderConvert()!.ExpectedRevision);

        model.OpenQuotation(model.VisibleQuotations.Single(q => q.Id == other.Quotation.Id));
        var entered = new TaskCompletionSource(); var release = new TaskCompletionSource();
        engine.CommandBarrier = _ => { entered.SetResult(); return release.Task; };
        var pending = model.ConvertAsync(); await entered.Task;
        await model.DeactivateDraftsAsync(); release.SetResult(); await pending;
        Assert.IsNull(model.SelectedQuotation); Assert.IsTrue(model.LifecycleAvailable);
        engine.CommandBarrier = null; await model.ActivateDraftsAsync();
        model.OpenQuotation(model.VisibleQuotations.Single(q => q.Id == other.Quotation.Id)); await model.ConvertAsync();
        Assert.AreNotEqual(sent[1].Key, engine.LastIdempotencyKey);
        await model.DeactivateDraftsAsync();
    }
}
