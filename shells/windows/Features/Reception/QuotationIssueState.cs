using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Features.Reception;

public sealed partial class SalesCatalogViewModel
{
    private QuotationRecord? quotationLifecycle;
    private Command? issueRetry;
    private Guid issueKey;
    private long lifecycleRead;
    private bool lifecycleOnline;
    public bool CanIssueQuotation => draftClient?.SupportsLifecycle == true && !IsDraftBusy && !draftConflict
        && (issueRetry is not null || !hasUnsavedEdits && lifecycleOnline && quotationLifecycle?.PermittedActions.Contains(QuotationPermittedAction.Issue) == true);
    private bool LifecycleEditable => issueRetry is null && savedDraft?.Snapshot.Cancelled != true
        && (savedDraft is null || savedDraft.PermittedActions?.Contains(QuotationPermittedAction.Edit) == true)
        && (quotationLifecycle is null || quotationLifecycle.PermittedActions.Contains(QuotationPermittedAction.Edit));
    public async Task<bool> IssueQuotationAsync()
    {
        if (!CanIssueQuotation || draftClient is null || savedDraft is null) return false;
        issueRetry ??= Command.ForQuotationIssue(new() { DraftId = savedDraft.Snapshot.Id, ExpectedRevision = quotationLifecycle!.Revision, ExpectedDraftRevision = savedDraft.Snapshot.Revision });
        if (issueKey == Guid.Empty) issueKey = Guid.NewGuid();
        var session = draftSession; IsDraftBusy = true; QuotationNotice = "جارٍ إصدار عرض السعر من الخادم...";
        try {
            var result = await draftClient.TransitionAsync(issueRetry, issueKey);
            if (session != draftSession || !catalogActive) return false;
            if (result.Succeeded) { quotationLifecycle = result.Value; QuotationNumber = result.Value!.Number; }
            if (result.Succeeded || result.Failure != DraftFailure.Unavailable) { issueRetry = null; issueKey = Guid.Empty; }
            QuotationNotice = result.Succeeded ? "صدر عرض السعر " + QuotationNumber : QuotationDraftClient.LifecycleMessage(result.Failure);
            if (result.Succeeded || result.Failure != DraftFailure.Unavailable) await RefreshLifecycleAsync();
            return result.Succeeded;
        }
        finally { if (session == draftSession) { IsDraftBusy = false; RaiseDiscountState(); } }
    }
    private async Task RefreshApprovalAndLifecycleAsync() { await RefreshApprovalAsync(); await RefreshLifecycleAsync(); }
    private void LifecycleChanged(object? sender, EventArgs e) { if (catalogActive) _ = RefreshLifecycleAsync(); }
    private async Task RefreshLifecycleAsync()
    {
        if (draftClient?.SupportsLifecycle != true || savedDraft is null || !catalogActive) return;
        var id = savedDraft.Snapshot.Id; var session = draftSession; var version = ++lifecycleRead; Guid? cursor = null;
        do {
            var result = await draftClient.QuotationsAsync(cursor);
            if (session != draftSession || version != lifecycleRead || savedDraft?.Snapshot.Id != id || !catalogActive) return;
            if (!result.Succeeded) { lifecycleOnline = false; if (result.Failure == DraftFailure.Denied) DraftInvalidated(this, EventArgs.Empty); RaiseDiscountState(); return; }
            lifecycleOnline = result.Value!.ServerAvailable;
            var value = result.Value.Items.FirstOrDefault(q => q.Quotation.Id == id);
            if (value is not null) { quotationLifecycle = value; QuotationNumber = value.Number ?? ""; RaiseDiscountState(); return; }
            cursor = result.Value.Next;
        } while (cursor is not null);
        quotationLifecycle = null; RaiseDiscountState();
    }
}
