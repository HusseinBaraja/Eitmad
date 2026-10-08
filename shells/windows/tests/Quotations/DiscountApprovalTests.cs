using System.IO;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.TestDoubles;
using Eitmad.WindowsShell.Tests.Products;

namespace Eitmad.WindowsShell.Tests.Quotations;

[TestClass]
public sealed class DiscountApprovalTests
{
    internal sealed class ConfirmedAuthority
    {
        internal readonly QuotationDraftTests.Authority Drafts = new();
        internal DiscountApproval? Approval;
        internal readonly Dictionary<Guid, DiscountApproval> Receipts = [];
        internal FakeEngine Engine()
        {
            var engine = Drafts.Engine();
            ((HashSet<string>)engine.SupportedCapabilities).Add(ProtocolIds.Capabilities.EitmadCapabilityQuotationApprovalV1);
            var queries = engine.QueryHandler!; var commands = engine.CommandHandler!;
            engine.QueryHandler = query => query.AsQuotationApprovalList() is not null
                ? SalesCatalogAuthorityTests.Response(QueryResult.ForDiscountApprovals(new() { Items = Approval is null ? [] : [QuotationDraftTests.Copy(Approval)] })) : queries(query);
            engine.CommandHandler = command => {
                if (command.AsQuotationApprovalRequest() is null && command.AsQuotationApprovalDecide() is null) return commands(command);
                if (!Receipts.TryGetValue(engine.LastIdempotencyKey, out var result)) {
                    if (command.AsQuotationApprovalRequest() is { } request) {
                        Assert.AreEqual(request.ExpectedRevision, Drafts.Draft!.Snapshot.Revision);
                        result = new() { Quotation = QuotationDraftTests.Copy(Drafts.Draft.Snapshot), Scope = Drafts.Draft.Scope,
                            RequestId = Guid.NewGuid(), Revision = 1, Fingerprint = "server-commercial-fingerprint", State = DiscountApprovalState.Pending,
                            Requester = Guid.NewGuid(), RequestedAt = 1791244800000, ValidityDays = 30, ProposedValidUntil = 1793836799999 };
                    } else {
                        var decision = command.AsQuotationApprovalDecide()!;
                        Assert.AreEqual(Approval!.RequestId, decision.RequestId); Assert.AreEqual(Approval.Fingerprint, decision.Fingerprint);
                        Assert.AreEqual(Approval.Quotation.Revision, decision.QuotationRevision); Assert.AreEqual(decision.ExpectedRevision, Approval.Revision);
                        result = QuotationDraftTests.Copy(Approval); result.Revision++;
                        result.State = decision.Decision == DiscountDecision.Approve ? DiscountApprovalState.Approved : DiscountApprovalState.Rejected;
                        result.Reason = decision.Reason; result.Decider = Guid.NewGuid(); result.DecidedAt = 1791244801000;
                    }
                    Receipts.Add(engine.LastIdempotencyKey, QuotationDraftTests.Copy(result)); Approval = QuotationDraftTests.Copy(result);
                }
                return new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = CommandResult.ForDiscountApproval(QuotationDraftTests.Copy(result)) } };
            };
            return engine;
        }
        internal void Publish(FakeEngine engine) => engine.Publish(Subscription.QuotationApprovalChangedSubscribeKind, new EventEnvelope { Event = new Dictionary<string, object> {
            ["kind"] = Event.QuotationApprovalChangedEventKind, ["payload"] = new DiscountApprovalNotice { Scope = Approval!.Scope, DraftId = Approval.Quotation.Id, Revision = Approval.Revision },
        } });
    }
    [TestMethod]
    [Timeout(30000)]
    [DataRow(DiscountDecision.Approve)]
    [DataRow(DiscountDecision.Reject)]
    public async Task CrossClientScreensUseConfirmedRequestAndDecisionOnly(DiscountDecision decision)
    {
        var authority = new ConfirmedAuthority(); await using var reception = authority.Engine(); await using var manager = authority.Engine();
        await using var catalog = new SalesCatalogClient(reception); await using var receptionDrafts = new QuotationDraftClient(reception); await using var managerDrafts = new QuotationDraftClient(manager);
        await receptionDrafts.ActivateAsync();
        var editor = await QuotationDraftTests.Editor(catalog, receptionDrafts);
        var model = new QuotationsViewModel(); model.AttachDraftClient(managerDrafts); await model.ActivateDraftsAsync();
        editor.DiscountInput = "6"; await editor.LastQuotationEvaluation;
        editor.RequestDiscountApproval(); await editor.LastApprovalRequest;
        Assert.IsTrue(editor.IsDiscountPending); Assert.HasCount(0, model.VisibleQuotations);
        authority.Publish(manager); await QuotationDraftTests.WaitFor(() => model.VisibleQuotations.Count == 1);
        model.OpenQuotation(model.VisibleQuotations.Single()); model.ApprovalReason = "الخصم مرتفع";
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        manager.CommandBarrier = _ => { entered.SetResult(); return release.Task; };
        if (decision == DiscountDecision.Approve) model.ApproveDiscount(); else model.RejectDiscount();
        await entered.Task; Assert.IsTrue(model.SelectedQuotation!.HasPendingDiscountApproval); Assert.IsFalse(model.CanDecideApproval); Assert.IsTrue(editor.IsDiscountPending);
        release.SetResult(); await model.LastApprovalDecision;
        Assert.AreEqual(decision == DiscountDecision.Approve ? DiscountApprovalState.Approved : DiscountApprovalState.Rejected, model.SelectedQuotation!.Approval!.State);
        Assert.IsTrue(editor.IsDiscountPending);
        authority.Publish(reception); await QuotationDraftTests.WaitFor(() => decision == DiscountDecision.Approve ? editor.IsDiscountApproved : editor.IsDiscountRejected);
        Assert.AreEqual(QuotationDraftClient.ApprovalLabel(authority.Approval), editor.QuotationNotice);
        Assert.IsFalse(editor.CanIssueQuotation);
        if (decision == DiscountDecision.Reject) StringAssert.Contains(editor.DiscountGuidance, "الخصم مرتفع");
        editor.DiscountInput = "7"; await editor.LastQuotationEvaluation; Assert.IsFalse(editor.IsDiscountApproved); Assert.IsTrue(editor.CanRequestDiscountApproval);
        await model.DeactivateDraftsAsync(); await editor.DeactivateCatalogAsync();
    }
    [TestMethod]
    [Timeout(30000)]
    public async Task LostDecisionReplyReusesItsKeyAndLateReplyCannotRestoreSignedOutContent()
    {
        var authority = new ConfirmedAuthority(); await using var engine = authority.Engine(); await using var catalog = new SalesCatalogClient(engine); await using var drafts = new QuotationDraftClient(engine);
        var editor = await QuotationDraftTests.Editor(catalog, drafts); editor.DiscountInput = "6"; await editor.LastQuotationEvaluation; editor.RequestDiscountApproval(); await editor.LastApprovalRequest;
        var model = new QuotationsViewModel(); model.AttachDraftClient(drafts); await model.ActivateDraftsAsync(); model.OpenQuotation(model.VisibleQuotations.Single());
        var handler = engine.CommandHandler!; engine.CommandHandler = command => { handler(command); throw new IOException("Synthetic lost decision reply"); };
        model.ApproveDiscount(); await model.LastApprovalDecision; var key = engine.LastIdempotencyKey;
        Assert.IsTrue(model.SelectedQuotation!.HasPendingDiscountApproval); Assert.IsTrue(model.CanDecideApproval);
        engine.CommandHandler = handler; model.ApproveDiscount(); await model.LastApprovalDecision;
        Assert.AreEqual(key, engine.LastIdempotencyKey); Assert.AreEqual(DiscountApprovalState.Approved, model.SelectedQuotation!.Approval!.State);
        await model.DeactivateDraftsAsync(); await editor.DeactivateCatalogAsync();
        // Start another pending request and close the session while its server reply is in flight.
        authority.Approval!.State = DiscountApprovalState.Pending; await model.ActivateDraftsAsync(); model.OpenQuotation(model.VisibleQuotations.Single());
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously); var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.CommandBarrier = _ => { entered.SetResult(); return release.Task; };
        model.RejectDiscount(); await entered.Task; await model.DeactivateDraftsAsync(); release.SetResult(); await model.LastApprovalDecision;
        Assert.HasCount(0, model.VisibleQuotations); Assert.IsNull(model.SelectedQuotation); Assert.AreEqual("", model.DecisionNotice);
    }
}
