using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Quotations;

[TestClass]
public sealed class QuotationDraftTests
{
    internal sealed class Authority
    {
        internal readonly CatalogEntry Product = SalesCatalogAuthorityTests.Entry();
        internal readonly CatalogEntry Furniture = SalesCatalogAuthorityTests.Entry(true);
        internal QuotationDraft? Draft;
        internal readonly Dictionary<Guid, QuotationDraft> Replays = [];
        internal QuotationEvaluation Evaluate(EvaluateQuotation input) => new() {
            Scope = new() { Kind = "branch", Id = Guid.Parse("fdccf778-90ec-467b-a787-a409fc7ef903") }, Currency = "YER", Errors = [],
            Customer = input.Customer is { } customer ? new() { Id = customer.Id, Revision = customer.Revision, Name = "عميل تجريبي", Phone = "777123456", Address = "عدن" } : null!,
            DiscountBasisPoints = input.DiscountBasisPoints,
            Lines = input.Lines.Select(line => {
                var entry = line.Configuration.Dimensions is null ? Product : Furniture;
                return new EvaluatedQuotationLine { Id = line.Id, Name = entry.Name, VariantName = entry.VariantName, Description = entry.Description,
                    Dimensions = line.Configuration.Dimensions!, Quantity = line.Configuration.Selection.Quantity,
                    Price = new() { Snapshot = entry.Price, UnitPriceYer = 12777, TotalYer = 25554 } };
            }).ToArray(),
            Totals = new() { SubtotalYer = 51108, DiscountYer = 2555, TotalYer = 48553, ApprovalRequired = input.DiscountBasisPoints > 500 },
        };
        internal FakeEngine Engine()
        {
            var engine = new FakeEngine { SupportedCapabilities = new HashSet<string> {
                ProtocolIds.Capabilities.EitmadCapabilitySalesCatalogV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationEvaluationV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1,
            } };
            engine.QueryHandler = q => {
                if (q.AsQuotationEvaluate() is { } input) return SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationEvaluation(Evaluate(input)));
                if (q.AsQuotationDraftList() is not null) return SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationDrafts(new() { Items = Draft is null ? [] : [Copy(Draft)] }));
                if (q.AsQuotationDraftGet() is not null) return SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationDraft(Copy(Draft!)));
                if (q.AsSalesCatalogList() is not null) return SalesCatalogAuthorityTests.Response(QueryResult.ForSalesCatalog(new() { Items = [Product, Furniture], Categories = ["مراتب", "خزائن"] }));
                var entry = q.AsSalesCatalogGet()?.Target ?? q.AsSalesCatalogCheck()?.Selection.Target;
                var isFurniture = entry is not null && JsonSerializer.Deserialize<PriceTarget>(JsonSerializer.Serialize(entry))!.AsFurniture() is not null;
                return SalesCatalogAuthorityTests.Handle(q, isFurniture ? Furniture : Product);
            };
            engine.CommandHandler = command => {
                if (Replays.TryGetValue(engine.LastIdempotencyKey, out var prior)) return Success(Copy(prior), command);
                var input = command.AsQuotationDraftCreate()?.Intent ?? command.AsQuotationDraftUpdate()!.Intent;
                if (command.AsQuotationDraftUpdate() is { } update && update.ExpectedRevision != Draft!.Snapshot.Revision)
                    return Failed(ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftConflictV1);
                Draft = new() { Scope = Evaluate(input).Scope, Snapshot = new() { Id = Draft?.Snapshot.Id ?? Guid.NewGuid(), Revision = (Draft?.Snapshot.Revision ?? 0) + 1, Intent = Copy(input), Evaluation = Copy(Evaluate(input)) }, SyncState = SyncState.Pending, UpdatedAt = 1791244800000 };
                Replays.Add(engine.LastIdempotencyKey, Copy(Draft)); return Success(Copy(Draft), command);
            };
            return engine;
        }
    }
    internal static T Copy<T>(T value) => JsonSerializer.Deserialize<T>(JsonSerializer.Serialize(value))!;
    private static CommandResponseEnvelope Success(QuotationDraft draft, Command command) => new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded,
        Payload = command.AsQuotationDraftCreate() is not null ? CommandResult.ForQuotationDraftCreated(draft) : CommandResult.ForQuotationDraftUpdated(draft) } };
    private static CommandResponseEnvelope Failed(string code) => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = code } } };
    internal static async Task<SalesCatalogViewModel> Editor(SalesCatalogClient catalog, QuotationDraftClient drafts)
    {
        var model = new SalesCatalogViewModel(new Features.Furniture.FurnitureViewModel(), new Features.Products.ProductsViewModel());
        model.AttachCatalogClient(catalog); model.AttachDraftClient(drafts); await model.ActivateCatalogAsync();
        foreach (var item in model.VisibleItems.ToArray())
        {
            model.Select(item); await model.LastCatalogOperation;
            if (model.ProductSelection is { } p) p.SelectedVariant = p.Variants[0];
            if (model.Selection is { } f) f.SelectedSize = f.Sizes[0];
            await model.LastCatalogOperation;
            Assert.IsTrue(await model.AddValidatedSelectionAsync(model.ProductSelection is not null)); model.CloseSelection();
        }
        model.AttachCustomer(new("عميل تجريبي", "777123456", "عدن", "", Guid.NewGuid(), 1)); model.DiscountInput = "5";
        await model.LastQuotationEvaluation; return model;
    }
    [TestMethod]
    public async Task MixedDraftReopensRetainsConfigurationAndManagerRefreshesOnlyFromSubscription()
    {
        var authority = new Authority(); await using var receptionEngine = authority.Engine(); await using var managerEngine = authority.Engine();
        await using var catalog = new SalesCatalogClient(receptionEngine); await using var drafts = new QuotationDraftClient(receptionEngine); await using var managerDrafts = new QuotationDraftClient(managerEngine);
        var model = await Editor(catalog, drafts); var manager = new QuotationsViewModel(); manager.AttachDraftClient(managerDrafts); await manager.ActivateDraftsAsync();
        Assert.HasCount(0, manager.VisibleQuotations); Assert.IsTrue(await model.SaveDraftAsync()); Assert.HasCount(0, manager.VisibleQuotations);
        managerEngine.Publish(Subscription.QuotationDraftChangedSubscribeKind, new EventEnvelope { Event = new Dictionary<string, object> {
            ["kind"] = Event.QuotationDraftChangedEventKind,
            ["payload"] = new QuotationDraftChangeNotice { DraftId = authority.Draft!.Snapshot.Id, Scope = authority.Draft.Scope, Revision = 1, ChangeId = Guid.NewGuid(), ChangedAt = authority.Draft.UpdatedAt },
        } });
        await WaitFor(() => manager.VisibleQuotations.Count == 1);
        var row = manager.VisibleQuotations.Single(); Assert.AreEqual(authority.Draft!.Snapshot.Id, row.Id); Assert.AreEqual(48553m, row.FinalTotal);
        Assert.IsFalse(row.CanPrint); Assert.IsFalse(row.HasPendingDiscountApproval); Assert.IsFalse(manager.ShowManagerApproval);
        var savedInput = JsonSerializer.Serialize(authority.Draft.Snapshot.Intent);
        model.DiscountInput = "6"; await model.LastQuotationEvaluation;
        Assert.AreEqual(500, authority.Draft.Snapshot.Intent.DiscountBasisPoints);
        Assert.IsTrue(await model.OpenDraftAsync(row.Id)); Assert.AreEqual("5", model.DiscountInput); Assert.AreEqual(48553m, model.FinalTotal);
        Assert.AreEqual(savedInput, JsonSerializer.Serialize(new EvaluateQuotation { Customer = authority.Draft.Snapshot.Intent.Customer, Lines = model.QuotationLines.Select(l => new QuotationLineIntent { Id = l.Id, Configuration = l.Intent! }).ToArray(), DiscountBasisPoints = 500 }));
        Assert.IsTrue(model.QuotationLines.Any(l => l.IsFurniture)); Assert.IsTrue(model.QuotationLines.Any(l => !l.IsFurniture));
        var original = model.QuotationLines.First(); model.EditLine(original); await model.LastCatalogOperation;
        Assert.AreEqual(original.Quantity, model.ProductSelection!.Quantity); model.CloseSelection(); Assert.AreSame(original, model.QuotationLines.First());
        await manager.DeactivateDraftsAsync(); await model.DeactivateCatalogAsync();
    }
    [TestMethod]
    public async Task LostReplyReplaysOriginalSaveAndConflictPreservesLocalEdits()
    {
        var authority = new Authority(); await using var engine = authority.Engine(); await using var catalog = new SalesCatalogClient(engine); await using var drafts = new QuotationDraftClient(engine);
        var model = await Editor(catalog, drafts); var handler = engine.CommandHandler!;
        engine.CommandHandler = c => { handler(c); throw new IOException("Synthetic lost reply"); };
        Assert.IsFalse(await model.SaveDraftAsync()); var key = engine.LastIdempotencyKey; var id = authority.Draft!.Snapshot.Id;
        Assert.IsTrue(model.DraftState.Contains("لم يُؤكد")); Assert.HasCount(2, model.QuotationLines);
        model.DiscountInput = "6"; await model.LastQuotationEvaluation; engine.CommandHandler = handler;
        Assert.IsTrue(await model.SaveDraftAsync()); Assert.AreEqual(key, engine.LastIdempotencyKey); Assert.AreEqual(id, authority.Draft.Snapshot.Id);
        Assert.AreEqual("6", model.DiscountInput); Assert.IsTrue(model.DraftState.Contains("غير محفوظة")); Assert.HasCount(1, authority.Replays);
        authority.Draft.Snapshot.Revision++;
        Assert.IsFalse(await model.SaveDraftAsync()); Assert.IsTrue(model.QuotationNotice.Contains("تغيرت المسودة")); Assert.AreEqual("6", model.DiscountInput); Assert.IsFalse(model.CanSaveDraft);
        Assert.IsTrue(await model.ReloadDraftAsync()); Assert.AreEqual("5", model.DiscountInput); Assert.IsTrue(model.CanSaveDraft);
        await model.DeactivateCatalogAsync();
    }
    [TestMethod]
    public async Task ChangedPriceIsReportedAtSaveAndExplicitLineReviewPreservesOtherConfiguration()
    {
        var authority = new Authority(); await using var engine = authority.Engine(); await using var catalog = new SalesCatalogClient(engine); await using var drafts = new QuotationDraftClient(engine);
        var model = await Editor(catalog, drafts); Assert.IsTrue(await model.SaveDraftAsync());
        var id = authority.Draft!.Snapshot.Id; Assert.IsTrue(await model.OpenDraftAsync(id));
        var original = model.QuotationLines.First(); var second = model.QuotationLines[1];
        authority.Product.Price.Revision = 2; authority.Product.Price.SellingPriceYer = 20000;
        var handler = engine.CommandHandler!;
        engine.CommandHandler = command => command.AsQuotationDraftUpdate()!.Intent.Lines[0].Configuration.Selection.PriceRevision == 1
            ? new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() {
                Code = ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftInvalidV1,
                Detail = new() { Kind = DetailKind.QuotationDraftValidation, Payload = new() { Errors = [new() { LineId = original.Id, Field = QuotationField.PriceRevision, Issue = QuotationIssue.Stale }] } },
            } } } : handler(command);
        Assert.IsFalse(await model.SaveDraftAsync()); Assert.IsTrue(model.QuotationNotice.Contains("تغير سعر البيع")); Assert.IsFalse(model.CanSaveDraft);
        Assert.AreEqual(1, authority.Draft.Snapshot.Revision); Assert.AreEqual(1, authority.Draft.Snapshot.Intent.Lines[0].Configuration.Selection.PriceRevision);
        model.EditLine(original); await model.LastCatalogOperation; Assert.AreEqual(original.Quantity, model.ProductSelection!.Quantity);
        Assert.AreEqual(2, model.ProductSelection.SelectedVariant!.Entry!.Price.Revision);
        Assert.AreSame(original, model.QuotationLines[0]); Assert.AreSame(second, model.QuotationLines[1]);
        Assert.IsTrue(await model.AddValidatedSelectionAsync(true)); await model.LastQuotationEvaluation;
        Assert.AreEqual(original.Id, model.QuotationLines[0].Id); Assert.AreEqual(2, model.QuotationLines[0].Intent!.Selection.PriceRevision);
        Assert.IsTrue(await model.SaveDraftAsync()); Assert.AreEqual(2, authority.Draft.Snapshot.Revision);
        await model.DeactivateCatalogAsync();
    }
    [TestMethod]
    public async Task LateSaveCannotRestorePreviousSessionAndValidationDoesNotClaimSavedState()
    {
        var authority = new Authority(); await using var engine = authority.Engine(); await using var catalog = new SalesCatalogClient(engine); await using var drafts = new QuotationDraftClient(engine);
        var model = await Editor(catalog, drafts);
        engine.CommandHandler = _ => Failed(ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftInvalidV1);
        Assert.IsFalse(await model.SaveDraftAsync()); Assert.HasCount(2, model.QuotationLines); Assert.IsTrue(model.DraftState.Contains("لم يُؤكد"));
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously); var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.CommandBarrier = _ => { entered.SetResult(); return release.Task; }; engine.CommandHandler = _ => Failed(ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftUnavailableV1);
        var pending = model.SaveDraftAsync(); await entered.Task;
        await model.DeactivateCatalogAsync(); release.SetResult(); Assert.IsFalse(await pending); Assert.HasCount(0, model.QuotationLines); Assert.IsNull(model.SelectedCustomer);
    }
    internal static async Task WaitFor(Func<bool> condition)
    {
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        while (!condition()) await Task.Delay(10, timeout.Token);
    }
}
