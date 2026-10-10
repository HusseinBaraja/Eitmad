using System.IO;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Quotations;

[TestClass]
public sealed class QuotationLifecycleTests
{
    internal sealed class Fixture
    {
        internal QuotationDraft Draft { get; }
        internal QuotationRecord Record { get; }
        internal FakeEngine Engine { get; }
        internal bool LoseReply;
        internal Guid FirstKey;
        internal Fixture()
        {
            var source = new QuotationDraftTests.Authority();
            var intent = new EvaluateQuotation { Customer = new() { Id = Guid.NewGuid(), Revision = 1 }, DiscountBasisPoints = 500,
                Lines = [new() { Id = Guid.NewGuid(), Configuration = new() { Selection = new() { Target = source.Product.Price.Target, PriceRevision = 1, Quantity = 2 } } }] };
            Draft = new() { Scope = source.Evaluate(intent).Scope, Snapshot = new() { Id = Guid.NewGuid(), Revision = 1, Intent = intent, Evaluation = source.Evaluate(intent), Cancelled = false }, SyncState = SyncState.Confirmed, UpdatedAt = 1791417600000, PermittedActions = [QuotationPermittedAction.Edit, QuotationPermittedAction.Cancel] };
            Record = new() { Scope = Draft.Scope, Quotation = Draft.Snapshot, Revision = 1, DocumentRevision = 1, State = QuotationState.Draft, ValidityDays = 30,
                PermittedActions = [QuotationPermittedAction.Edit, QuotationPermittedAction.Issue, QuotationPermittedAction.Cancel], ChangedAt = Draft.UpdatedAt };
            Engine = new() { SupportedCapabilities = new HashSet<string> { ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1 } };
            Engine.QueryHandler = query => query.AsQuotationList() is not null
                ? SalesCatalogAuthorityTests.Response(QueryResult.ForQuotations(new() { Items = [QuotationDraftTests.Copy(Record)], ServerAvailable = true }))
                : SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationDrafts(new() { Items = [QuotationDraftTests.Copy(Draft)] }));
            Engine.CommandHandler = command => {
                if (FirstKey == Guid.Empty) FirstKey = Engine.LastIdempotencyKey;
                else Assert.AreEqual(FirstKey, Engine.LastIdempotencyKey);
                Assert.IsNotNull(command.AsQuotationIssue());
                if (LoseReply) { LoseReply = false; throw new IOException("Synthetic uncertain issue"); }
                Record.State = QuotationState.Issued; Record.Revision = 2; Record.Number = "QT-2026-00001";
                Record.IssuedAt = 1791417600000; Record.ValidUntil = 1794085199999;
                Record.PermittedActions = [QuotationPermittedAction.Print];
                return new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = CommandResult.ForQuotation(QuotationDraftTests.Copy(Record)) } };
            };
        }
    }
    [TestMethod]
    public async Task UnnumberedLifecycleCannotOpenAnIssuedPrintDocument()
    {
        var fixture = new Fixture(); await using var engine = fixture.Engine; await using var client = new QuotationDraftClient(engine);
        fixture.Record.PermittedActions = [QuotationPermittedAction.Print];
        var model = new QuotationsViewModel(true); model.AttachDraftClient(client); await model.ActivateDraftsAsync(); model.OpenQuotation(model.VisibleQuotations.Single());
        Assert.IsFalse(model.SelectedQuotation!.CanPrint);
        await model.DeactivateDraftsAsync();
    }
    [TestMethod]
    public async Task IssueWaitsForConfirmationAndRetriesTheExactIntent()
    {
        var fixture = new Fixture { LoseReply = true }; await using var engine = fixture.Engine; await using var client = new QuotationDraftClient(engine);
        var model = new QuotationsViewModel(true); model.AttachDraftClient(client); await model.ActivateDraftsAsync(); model.OpenQuotation(model.VisibleQuotations.Single());
        Assert.IsTrue(model.SelectedQuotation!.CanIssue); Assert.IsFalse(model.SelectedQuotation.CanPrint);
        model.Issue(); await model.LastLifecycleAction;
        Assert.AreEqual("غير مرقم", model.SelectedQuotation.Number); Assert.IsTrue(model.CanRetryLifecycle);
        model.RetryLifecycle(); await model.LastLifecycleAction;
        Assert.AreEqual("QT-2026-00001", model.SelectedQuotation.Number); Assert.IsFalse(model.SelectedQuotation.CanIssue);
        Assert.IsTrue(model.SelectedQuotation.CanPrint); Assert.IsFalse(model.SelectedQuotation.CanConvert); Assert.IsFalse(model.CanRetryLifecycle);
        await model.DeactivateDraftsAsync();
    }
    [TestMethod]
    public async Task RustActionsControlManagerAndReceptionistStateAndLateRepliesAreFenced()
    {
        var fixture = new Fixture(); await using var engine = fixture.Engine; await using var client = new QuotationDraftClient(engine);
        fixture.Record.State = QuotationState.Issued; fixture.Record.Number = "QT-2026-00001";
        fixture.Record.PermittedActions = [QuotationPermittedAction.Revise, QuotationPermittedAction.Cancel, QuotationPermittedAction.Print];
        var model = new QuotationsViewModel(); model.AttachDraftClient(client); await model.ActivateDraftsAsync(); model.OpenQuotation(model.VisibleQuotations.Single());
        Assert.IsTrue(model.SelectedQuotation!.CanRevise); Assert.IsTrue(model.SelectedQuotation.CanCancel); Assert.IsFalse(model.SelectedQuotation.CanEdit);
        fixture.Record.State = QuotationState.Draft; fixture.Record.PermittedActions = [QuotationPermittedAction.Issue];
        await model.DeactivateDraftsAsync(); await model.ActivateDraftsAsync(); model.OpenQuotation(model.VisibleQuotations.Single());
        var entered = new TaskCompletionSource(); var release = new TaskCompletionSource();
        engine.CommandBarrier = _ => { entered.SetResult(); return release.Task; };
        model.Issue(); await entered.Task; await model.DeactivateDraftsAsync(); release.SetResult(); await model.LastLifecycleAction;
        Assert.IsNull(model.SelectedQuotation); Assert.HasCount(0,model.VisibleQuotations); Assert.IsFalse(model.CanRetryLifecycle);
    }
    [TestMethod]
    public async Task ReceptionistEditorIssuesConfirmedRevisionAndKeepsUnknownRetryIdentity()
    {
        var authority = new QuotationDraftTests.Authority(); await using var engine = authority.Engine();
        ((HashSet<string>)engine.SupportedCapabilities!).Add(ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1);
        var query = engine.QueryHandler; var save = engine.CommandHandler; QuotationRecord? record = null; Guid key = Guid.Empty; var loseReply = true;
        engine.QueryHandler = q => {
            if (q.AsQuotationList() is null) return query!(q);
            if (record is null && authority.Draft is { } draft) record = new() { Scope = draft.Scope, Quotation = draft.Snapshot, Revision = 1, DocumentRevision = 1, State = QuotationState.Draft, ValidityDays = 30,
                PermittedActions = [QuotationPermittedAction.Edit, QuotationPermittedAction.Issue], ChangedAt = draft.UpdatedAt };
            return SalesCatalogAuthorityTests.Response(QueryResult.ForQuotations(new() { Items = record is null ? [] : [QuotationDraftTests.Copy(record)], ServerAvailable = true }));
        };
        engine.CommandHandler = command => {
            if (command.AsQuotationIssue() is null) return save!(command);
            if (key == Guid.Empty) key = engine.LastIdempotencyKey; else Assert.AreEqual(key, engine.LastIdempotencyKey);
            if (loseReply) { loseReply = false; throw new IOException("Synthetic uncertain issuance"); }
            record!.State = QuotationState.Issued; record.Number = "QT-2026-00001"; record.Revision = 2; record.PermittedActions = [QuotationPermittedAction.Print];
            return new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = CommandResult.ForQuotation(QuotationDraftTests.Copy(record)) } };
        };
        await using var catalog = new Eitmad.WindowsShell.Features.Reception.SalesCatalogClient(engine); await using var client = new QuotationDraftClient(engine);
        var editor = await QuotationDraftTests.Editor(catalog, client);
        try {
            Assert.IsTrue(await editor.SaveDraftAsync()); Assert.IsTrue(editor.CanIssueQuotation);
            Assert.IsFalse(await editor.IssueQuotationAsync()); Assert.IsFalse(editor.CanSaveDraft); Assert.IsTrue(editor.CanIssueQuotation);
            Assert.IsTrue(await editor.IssueQuotationAsync()); Assert.AreEqual("QT-2026-00001", editor.QuotationNumber);
            Assert.IsFalse(editor.CanSaveDraft); Assert.IsFalse(editor.CanIssueQuotation);
        } finally { await editor.DeactivateCatalogAsync(); }
    }

    [TestMethod]
    [DataRow(false)]
    [DataRow(true)]
    public async Task EditorSavedDocumentSurvivesCatalogDenialAndFencesSessionChanges(bool denialDuringRead)
    {
        var authority = new QuotationDraftTests.Authority(); await using var engine = authority.Engine();
        ((HashSet<string>)engine.SupportedCapabilities!).Add(ProtocolIds.Capabilities.EitmadCapabilityCustomerDocumentsV1);
        await using var catalog = new Eitmad.WindowsShell.Features.Reception.SalesCatalogClient(engine); await using var client = new QuotationDraftClient(engine);
        var editor = await QuotationDraftTests.Editor(catalog, client);
        try {
            Assert.IsTrue(await editor.SaveDraftAsync());
            var saved = Rendered.CustomerDocumentsRenderedTests.Saved(); saved.IsDraft = true; saved.CanPrint = false; saved.Number = null!;
            var original = engine.QueryHandler!;
            engine.QueryHandler = query => {
                if (query.AsSalesCatalogList() is not null) return new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 } } };
                if (query.AsQuotationCustomerDocument() is { } document) {
                    Assert.AreEqual(authority.Draft!.Snapshot.Id, document.DraftId);
                    return SalesCatalogAuthorityTests.Response(QueryResult.ForCustomerDocument(saved));
                }
                return original(query);
            };
            var release = new TaskCompletionSource();
            engine.QueryBarrier = query => query.AsQuotationCustomerDocument() is not null ? release.Task : Task.CompletedTask;
            var read = denialDuringRead ? editor.ReadDocumentAsync() : null;
            await editor.ActivateCatalogAsync();
            Assert.IsTrue(editor.CatalogStatus.Contains("صلاحية")); Assert.HasCount(0, editor.VisibleItems);
            Assert.IsTrue(editor.CanPreviewCustomer, "The saved snapshot must remain previewable without catalog access.");
            read ??= editor.ReadDocumentAsync(); release.SetResult();
            Assert.AreSame(saved, await read); Assert.IsFalse(saved.CanPrint);
            var invalidated = 0; editor.DocumentInvalidated += (_, _) => invalidated++;
            engine.QueryHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 } } };
            Assert.IsNull(await editor.ReadDocumentAsync()); Assert.AreEqual(1, invalidated);
            engine.QueryHandler = _ => SalesCatalogAuthorityTests.Response(QueryResult.ForCustomerDocument(saved));
            release = new TaskCompletionSource(); read = editor.ReadDocumentAsync();
            await editor.DeactivateCatalogAsync(); release.SetResult();
            Assert.IsNull(await read); Assert.IsFalse(editor.CanPreviewCustomer);
        } finally { await editor.DeactivateCatalogAsync(); }
    }

    [TestMethod]
    public async Task QuotationDocumentsUseSavedReplyAndRejectDeniedOrLateReads()
    {
        var fixture = new Fixture();
        ((HashSet<string>)fixture.Engine.SupportedCapabilities!).Add(ProtocolIds.Capabilities.EitmadCapabilityCustomerDocumentsV1);
        await using var engine = fixture.Engine; await using var client = new QuotationDraftClient(engine);
        var model = new QuotationsViewModel(true); model.AttachDraftClient(client); await model.ActivateDraftsAsync(); model.OpenQuotation(model.VisibleQuotations.Single());
        var saved = Rendered.CustomerDocumentsRenderedTests.Saved();
        engine.QueryHandler = query => {
            Assert.AreEqual(fixture.Draft.Snapshot.Id, query.AsQuotationCustomerDocument()!.DraftId);
            return SalesCatalogAuthorityTests.Response(QueryResult.ForCustomerDocument(saved));
        };
        Assert.AreSame(saved, await model.ReadDocumentAsync());
        engine.QueryHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 } } };
        Assert.IsNull(await model.ReadDocumentAsync()); Assert.IsTrue(model.ListState.Contains("صلاحية"));
        engine.QueryHandler = _ => SalesCatalogAuthorityTests.Response(QueryResult.ForCustomerDocument(saved));
        var barrier = new TaskCompletionSource(); engine.QueryBarrier = _ => barrier.Task;
        var read = model.ReadDocumentAsync(); model.ClearDrafts(); barrier.SetResult();
        Assert.IsNull(await read); await model.DeactivateDraftsAsync();
    }

}
