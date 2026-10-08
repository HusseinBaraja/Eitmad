using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Products;

[TestClass]
public sealed class QuotationEvaluationTests
{
    internal static FakeEngine Engine(CatalogEntry entry) => new() {
        SupportedCapabilities = new HashSet<string> { ProtocolIds.Capabilities.EitmadCapabilitySalesCatalogV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationEvaluationV1 },
        QueryHandler = q => q.AsQuotationEvaluate() is { } input ? Evaluation(input) : SalesCatalogAuthorityTests.Handle(q, entry),
    };

    internal static QueryResponseEnvelope Evaluation(EvaluateQuotation input, long total = 959, bool approval = false) => SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationEvaluation(new() {
        Scope = new() { Kind = "branch", Id = Guid.NewGuid() }, Customer = new() { Id = input.Customer.Id, Revision = input.Customer.Revision, Name = "عميل تجريبي", Phone = "777123456" },
        Lines = [], Currency = "YER", DiscountBasisPoints = input.DiscountBasisPoints, Errors = [],
        Totals = new() { SubtotalYer = 1010, DiscountYer = 51, TotalYer = total, ApprovalRequired = approval }, ServerAvailable = false,
    }));

    internal static async Task<SalesCatalogViewModel> Model(SalesCatalogClient client)
    {
        var model = new SalesCatalogViewModel(new Features.Furniture.FurnitureViewModel(), new Features.Products.ProductsViewModel());
        model.AttachCatalogClient(client); await model.ActivateCatalogAsync();
        model.Select(model.VisibleItems[0]); await model.LastCatalogOperation;
        model.ProductSelection!.SelectedVariant = model.ProductSelection.Variants[0]; await model.LastCatalogOperation;
        Assert.IsTrue(await model.AddValidatedSelectionAsync(true)); model.CloseSelection();
        model.AttachCustomer(new("عميل تجريبي", "777123456", "", "", Guid.NewGuid(), 1));
        await model.LastQuotationEvaluation;
        return model;
    }

    [TestMethod]
    public async Task LiveQuotationUsesReturnedTotalsAndRejectsAnObsoleteReplyAndFixtureApproval()
    {
        var entry = SalesCatalogAuthorityTests.Entry(); await using var engine = Engine(entry); await using var client = new SalesCatalogClient(engine);
        var model = await Model(client);
        Assert.AreEqual(1010m, model.Subtotal); Assert.AreEqual(51m, model.Discount); Assert.AreEqual(959m, model.FinalTotal);
        Assert.AreEqual(25554m, model.QuotationLines[0].LineTotal); // Distinct values detect shell recalculation.
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.QueryBarrier = q => q.AsQuotationEvaluate()?.DiscountBasisPoints == 600 ? Block() : Task.CompletedTask;
        Task Block() { entered.TrySetResult(); return release.Task; }
        engine.QueryHandler = q => q.AsQuotationEvaluate() is { } input ? Evaluation(input, input.DiscountBasisPoints == 600 ? 1 : 900, approval: true) : SalesCatalogAuthorityTests.Handle(q, entry);
        model.DiscountInput = "6"; var old = model.LastQuotationEvaluation;
        Assert.AreEqual("—", model.FinalTotalLabel); Assert.IsFalse(model.CanIssueQuotation);
        await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));
        model.DiscountInput = "7"; await model.LastQuotationEvaluation;
        Assert.AreEqual(900m, model.FinalTotal); Assert.IsTrue(model.RequiresDiscountApproval); Assert.IsFalse(model.CanIssueQuotation);
        release.SetResult(); await old;
        Assert.AreEqual(900m, model.FinalTotal); Assert.IsFalse(model.IsDiscountApproved);
        model.DiscountInput = "5.001"; Assert.IsFalse(model.IsDiscountValid); Assert.IsNull(model.Evaluation);
        await model.DeactivateCatalogAsync(); Assert.IsNull(model.Evaluation); Assert.AreEqual("—", model.FinalTotalLabel);
    }

    [TestMethod]
    public async Task InvalidFieldsWithholdTotalsAndSessionEndDiscardsPendingEvaluation()
    {
        var entry = SalesCatalogAuthorityTests.Entry(); await using var engine = Engine(entry); await using var client = new SalesCatalogClient(engine);
        var model = await Model(client);
        engine.QueryHandler = q => q.AsQuotationEvaluate() is { } input ? SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationEvaluation(new() {
            Scope = new() { Kind = "branch", Id = Guid.NewGuid() }, Currency = "YER", Lines = [], DiscountBasisPoints = input.DiscountBasisPoints,
            Errors = [new() { LineId = input.Lines[0].Id, Field = QuotationField.PriceRevision, Issue = QuotationIssue.Stale }],
        })) : SalesCatalogAuthorityTests.Handle(q, entry);
        model.DiscountInput = "1"; await model.LastQuotationEvaluation;
        Assert.IsNull(model.Evaluation!.Totals); Assert.IsFalse(model.CanSaveDraft); Assert.IsTrue(model.QuotationNotice.Contains("تغير سعر البيع"));
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.QueryBarrier = q => q.AsQuotationEvaluate() is not null ? Block() : Task.CompletedTask;
        Task Block() { entered.TrySetResult(); return release.Task; }
        model.DiscountInput = "2"; var old = model.LastQuotationEvaluation; await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));
        await model.DeactivateCatalogAsync(); release.SetResult(); await old;
        Assert.IsNull(model.Evaluation); Assert.HasCount(0, model.QuotationLines);
    }
}
