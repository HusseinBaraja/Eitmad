using System.Text.Json;
using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.Quotations;

public sealed partial class QuotationsViewModel
{
    private string approvalReason = "", decisionNotice = "";
    private bool isApprovalBusy;
    private Guid decisionKey, decisionRequest;
    private string? decisionIntent;
    public string ApprovalReason { get => approvalReason; set => Set(ref approvalReason, value); }
    public string DecisionNotice { get => decisionNotice; private set => Set(ref decisionNotice, value); }
    public bool CanDecideApproval => !isApprovalBusy && !IsReceptionist && SelectedQuotation?.HasPendingDiscountApproval == true;
    public Task LastApprovalDecision { get; private set; } = Task.CompletedTask;

    private async Task DecideApprovalAsync(DiscountDecision decision)
    {
        if (!CanDecideApproval || draftClient is null || SelectedQuotation?.Approval is not { } approval) return;
        var command = Command.ForQuotationApprovalDecide(new() {
            DraftId = approval.Quotation.Id, RequestId = approval.RequestId,
            QuotationRevision = approval.Quotation.Revision, ExpectedRevision = approval.Revision,
            Fingerprint = approval.Fingerprint, Decision = decision,
            Reason = string.IsNullOrWhiteSpace(ApprovalReason) ? null! : ApprovalReason.Trim(),
        });
        var intent = JsonSerializer.Serialize(command);
        if (decisionIntent is not null && intent != decisionIntent) {
            DecisionNotice = "أعد محاولة القرار السابق لتأكيد نتيجته قبل تغييره."; return;
        }
        if (decisionIntent is null) { decisionKey = Guid.NewGuid(); decisionIntent = intent; decisionRequest = approval.RequestId; }
        var session = approvalSession;
        isApprovalBusy = true; Raise(nameof(CanDecideApproval)); DecisionNotice = "جارٍ تأكيد القرار من الخادم...";
        try {
            var result = await draftClient.ApprovalAsync(command, decisionKey);
            if (session != approvalSession || !draftsActive) return;
            DecisionNotice = result.Succeeded ? QuotationDraftClient.ApprovalLabel(result.Value) : QuotationDraftClient.ApprovalMessage(result.Failure);
            if (result.Succeeded || result.Failure != DraftFailure.Unavailable) decisionIntent = null;
            if (result.Failure != DraftFailure.Unavailable) { LastDraftLoad = LoadDraftsAsync(); await LastDraftLoad; }
        }
        finally { if (session == approvalSession) { isApprovalBusy = false; Raise(nameof(CanDecideApproval)); } }
    }
}
