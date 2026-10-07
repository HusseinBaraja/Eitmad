using System.Globalization;
using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.Quotations;

public sealed partial class QuotationsViewModel
{
    private Command? lifecycleRetry;
    private Guid lifecycleKey;
    private bool lifecycleBusy;
    private string validityInput = "30", cancellationReason = "", lifecycleNotice = "";
    public string ValidityInput { get => validityInput; set => Set(ref validityInput, value); }
    public string CancellationReason { get => cancellationReason; set => Set(ref cancellationReason, value); }
    public string LifecycleNotice { get => lifecycleNotice; private set => Set(ref lifecycleNotice, value); }
    public bool LifecycleAvailable => !lifecycleBusy;
    public bool CanRetryLifecycle => lifecycleRetry is not null && !lifecycleBusy;
    public Task LastLifecycleAction { get; private set; } = Task.CompletedTask;
    public void Issue() => LastLifecycleAction = ActAsync(QuotationPermittedAction.Issue);
    public void Revise() => LastLifecycleAction = ActAsync(QuotationPermittedAction.Revise);
    public void SetValidity() => LastLifecycleAction = ActAsync(QuotationPermittedAction.ManageValidity);
    public void Cancel() => LastLifecycleAction = ActAsync(QuotationPermittedAction.Cancel);
    public void RetryLifecycle() => LastLifecycleAction = SendLifecycleAsync();

    private async Task ActAsync(QuotationPermittedAction action)
    {
        if (draftClient is null || lifecycleBusy || SelectedQuotation is not { } row || !row.Permits(action)) return;
        if (lifecycleRetry is not null) { LifecycleNotice = "أعد محاولة العملية السابقة لتأكيد نتيجتها أولاً."; return; }
        var record = row.Lifecycle;
        if (action == QuotationPermittedAction.Cancel && record?.Number is null && row.Draft is { } draft)
            lifecycleRetry = Command.ForQuotationDraftCancel(new() { DraftId = row.Id, ExpectedRevision = draft.Snapshot.Revision });
        else if (record is not null) {
            if (action == QuotationPermittedAction.Issue) lifecycleRetry = Command.ForQuotationIssue(new() { DraftId = row.Id, ExpectedRevision = record.Revision, ExpectedDraftRevision = record.Quotation.Revision });
            else if (action == QuotationPermittedAction.Cancel) lifecycleRetry = Command.ForQuotationCancel(new() { DraftId = row.Id, ExpectedRevision = record.Revision, Reason = CancellationReason });
            else {
                if (!uint.TryParse(PreviewText.NormalizeNumericInput(ValidityInput), NumberStyles.None, CultureInfo.InvariantCulture, out var days)) { LifecycleNotice = "أدخل مدة الصلاحية بالأيام."; return; }
                var value = new SetQuotationValidity { DraftId = row.Id, ExpectedRevision = record.Revision, ValidityDays = days };
                lifecycleRetry = action == QuotationPermittedAction.Revise ? Command.ForQuotationRevise(value) : Command.ForQuotationValidity(value);
            }
        }
        if (lifecycleRetry is null) return;
        lifecycleKey = Guid.NewGuid(); await SendLifecycleAsync();
    }
    private async Task SendLifecycleAsync()
    {
        if (draftClient is null || lifecycleRetry is null || lifecycleBusy) return;
        var session = approvalSession; var command = lifecycleRetry;
        lifecycleBusy = true; RaiseLifecycle(); LifecycleNotice = "جارٍ تأكيد العملية...";
        try {
            bool succeeded; DraftFailure failure;
            if (command.AsQuotationDraftCancel() is not null) {
                var result = await draftClient.SaveAsync(command, lifecycleKey); succeeded = result.Succeeded; failure = result.Failure;
            } else {
                var result = await draftClient.TransitionAsync(command, lifecycleKey); succeeded = result.Succeeded; failure = result.Failure;
            }
            if (session != approvalSession || !draftsActive) return;
            LifecycleNotice = succeeded ? "تم تأكيد العملية. حُفظ سجل العرض." : QuotationDraftClient.LifecycleMessage(failure);
            if (succeeded || failure != DraftFailure.Unavailable) lifecycleRetry = null;
            if (failure != DraftFailure.Unavailable) { LastDraftLoad = LoadDraftsAsync(); await LastDraftLoad; }
        }
        finally { if (session == approvalSession) { lifecycleBusy = false; RaiseLifecycle(); } }
    }
    private void ClearLifecycle() { lifecycleRetry = null; lifecycleKey = Guid.Empty; lifecycleBusy = false; LifecycleNotice = ""; RaiseLifecycle(); }
    private void RaiseLifecycle() { Raise(nameof(LifecycleAvailable)); Raise(nameof(CanRetryLifecycle)); }
}
