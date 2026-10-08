using System.Globalization;

namespace Eitmad.WindowsShell.Features.Reception;

public sealed partial class SalesCatalogViewModel
{
    // Thresholds and approval outcomes are supplied by Rust.
    private Eitmad.Contracts.DiscountApproval? serverApproval;
    private Guid approvalKey;
    private long approvalReadVersion;
    private Eitmad.Contracts.Command? approvalCommand;
    public Task LastApprovalRequest { get; private set; } = Task.CompletedTask;
    private string discountInput = "0";
    private decimal discountPercent;
    private bool isDiscountValid = true;
    private bool discountPending;
    private decimal? previewDiscountOverride;
    private Features.Quotations.QuotationListItem? approvalPreview;
    public Action<SalesCatalogViewModel, bool>? PublishPreview { get; set; }
    public Guid PreviewId { get; set; } = Guid.NewGuid();
    public bool IsDiscountApproved => !hasUnsavedEdits && serverApproval?.State == Eitmad.Contracts.DiscountApprovalState.Approved;
    public bool IsDiscountRejected => serverApproval?.State == Eitmad.Contracts.DiscountApprovalState.Rejected;
    public void ObserveApproval(Features.Quotations.QuotationListItem quotation)
    {
        if (approvalPreview is not null) System.ComponentModel.PropertyChangedEventManager.RemoveHandler(approvalPreview, ApprovalChanged, "");
        approvalPreview = quotation;
        System.ComponentModel.PropertyChangedEventManager.AddHandler(quotation, ApprovalChanged, "");
        discountPending = quotation.HasPendingDiscountApproval;
        RaiseDiscountState();
    }
    private void ApprovalChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        discountPending = approvalPreview?.HasPendingDiscountApproval == true;
        RaiseDiscountState();
    }

    public string DiscountInput
    {
        get => discountInput;
        set
        {
            if (!Set(ref discountInput, value ?? "")) return;
            isDiscountValid = decimal.TryParse(PreviewText.NormalizeNumericInput(discountInput).Trim(),
                NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out discountPercent)
                && discountPercent is >= 0m and <= 100m
                && discountPercent * 100m == decimal.Truncate(discountPercent * 100m);
            InvalidateDiscountRequest();
        }
    }

    internal void SetPreviewDiscount(decimal amount, string input)
    {
        DiscountInput = input;
        previewDiscountOverride = amount;
        RaiseDiscountState();
    }

    public bool IsDiscountValid => isDiscountValid;
    public string DiscountError => IsDiscountValid ? "" : "أدخل نسبة من 0 إلى 100 بمنزلتين عشريتين كحد أقصى";
    public string DiscountAmountLabel => catalogClient is not null && evaluation?.Totals is null ? "—" : IsDiscountValid ? Discount.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public string FinalTotalLabel => catalogClient is not null && evaluation?.Totals is null ? "—" : IsDiscountValid ? FinalTotal.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public bool RequiresDiscountApproval => evaluation?.Totals?.ApprovalRequired == true;
    public bool IsDiscountPending => serverApproval?.State == Eitmad.Contracts.DiscountApprovalState.Pending;
    public bool CanRequestDiscountApproval => draftClient?.SupportsApprovals == true && LifecycleEditable && !IsDraftBusy && !draftConflict && RequiresDiscountApproval && !IsQuotationEmpty && (hasUnsavedEdits || !IsDiscountPending && !IsDiscountApproved);
    public bool CanSaveDraft => LifecycleEditable && !IsDraftBusy && !draftConflict && IsDiscountValid && !IsQuotationEmpty && (catalogClient is null || catalogActive && (uncertainSave || evaluation?.Totals is not null));
    public string DiscountStatus => hasUnsavedEdits && serverApproval is not null ? "تغييرات غير محفوظة — أعد حفظ الشروط" : Features.Quotations.QuotationDraftClient.ApprovalLabel(serverApproval) is { Length: > 0 } status ? status : RequiresDiscountApproval ? "يتطلب موافقة المدير" : "";
    public string DiscountGuidance => IsLiveQuotation ? serverApproval?.State == Eitmad.Contracts.DiscountApprovalState.Rejected ? "سبب الرفض: " + serverApproval.Reason : "احفظ الشروط ثم أرسل طلب الموافقة. أصدر العرض بعد تأكيد الموافقة والشروط." : IsDiscountApproved ? "يمكنك الآن إكمال عرض السعر في المعاينة" : IsDiscountRejected ? "خفّض الخصم أو عدّل العرض ثم اطلب الموافقة مجدداً" : "يمكنك مراجعة العرض وحفظه كمسودة حتى الموافقة على الخصم";
    public string TotalHeading => RequiresDiscountApproval && !IsDiscountApproved ? "الإجمالي بعد الخصم المطلوب" : "الإجمالي النهائي";

    public void RequestDiscountApproval() => LastApprovalRequest = RequestDiscountApprovalAsync();
    private async Task RequestDiscountApprovalAsync()
    {
        if (!CanRequestDiscountApproval || draftClient is null) return;
        if (approvalCommand is null && (savedDraft is null || hasUnsavedEdits)) { if (!await SaveDraftAsync()) return; }
        if (savedDraft is null) return;
        approvalCommand ??= Eitmad.Contracts.Command.ForQuotationApprovalRequest(new() { DraftId = savedDraft.Snapshot.Id, ExpectedRevision = savedDraft.Snapshot.Revision });
        if (approvalKey == Guid.Empty) approvalKey = Guid.NewGuid();
        var session = draftSession; IsDraftBusy = true; QuotationNotice = "جارٍ إرسال طلب الموافقة إلى الخادم...";
        try {
            var result = await draftClient.ApprovalAsync(approvalCommand, approvalKey);
            if (session != draftSession || !catalogActive) return;
            if (result.Succeeded) { serverApproval = result.Value; approvalCommand = null; approvalKey = Guid.Empty; }
            else if (result.Failure != Features.Quotations.DraftFailure.Unavailable) { approvalCommand = null; approvalKey = Guid.Empty; }
            if (result.Succeeded) { await RefreshApprovalAsync(); await RefreshLifecycleAsync(); }
            if (session != draftSession || !catalogActive) return;
            QuotationNotice = result.Succeeded ? Features.Quotations.QuotationDraftClient.ApprovalLabel(serverApproval) : Features.Quotations.QuotationDraftClient.ApprovalMessage(result.Failure);
        }
        finally { if (session == draftSession) { IsDraftBusy = false; RaiseDiscountState(); } }
    }
    private async Task RefreshApprovalAsync()
    {
        if (draftClient?.SupportsApprovals != true || savedDraft is null || !catalogActive) return;
        var id = savedDraft.Snapshot.Id; var session = draftSession; var version = ++approvalReadVersion; Guid? cursor = null;
        do {
            var result = await draftClient.ApprovalsAsync(cursor);
            if (version != approvalReadVersion || session != draftSession || savedDraft?.Snapshot.Id != id || !catalogActive) return;
            if (!result.Succeeded) { serverApproval = null; QuotationNotice = Features.Quotations.QuotationDraftClient.ApprovalMessage(result.Failure); RaiseDiscountState(); return; }
            var value = result.Value!.Items.FirstOrDefault(a => a.Quotation.Id == id);
            if (value is not null) { serverApproval = value; QuotationNotice = Features.Quotations.QuotationDraftClient.ApprovalLabel(value); RaiseDiscountState(); return; }
            cursor = result.Value.Next;
        } while (cursor is not null);
        serverApproval = null; RaiseDiscountState();
    }

    public bool ReviewDraftSave()
    {
        if (draftClient is not null) { QuotationNotice = "استخدم حفظ كمسودة لتأكيد الحفظ من المحرك."; return false; }
        if (!CheckRequiredFields()) return false;
        if (!CanSaveDraft) return false;
        PublishPreview?.Invoke(this, discountPending);
        QuotationNotice = "معاينة المسودة فقط — الحفظ غير متاح بعد، ولم يُحفظ عرض السعر";
        return true;
    }

    private void InvalidateDiscountRequest()
    {
        if (applyingDraft) return;
        hasUnsavedEdits = true; Raise(nameof(DraftState));
        if (approvalPreview is not null) System.ComponentModel.PropertyChangedEventManager.RemoveHandler(approvalPreview, ApprovalChanged, "");
        approvalPreview = null;
        previewDiscountOverride = null;
        discountPending = false;
        QuotationNotice = "";
        RaiseDiscountState();
        QueueQuotationEvaluation();
    }

    private void RaiseDiscountState()
    {
        foreach (var property in new[] { nameof(IsDiscountValid), nameof(DiscountError), nameof(Discount),
            nameof(DiscountAmountLabel), nameof(FinalTotal), nameof(FinalTotalLabel), nameof(RequiresDiscountApproval),
            nameof(IsDiscountPending), nameof(IsDiscountApproved), nameof(IsDiscountRejected), nameof(CanRequestDiscountApproval), nameof(CanIssueQuotation),
            nameof(CanSaveDraft), nameof(DiscountStatus), nameof(DiscountGuidance), nameof(TotalHeading), nameof(CanPreviewCustomer) }) Raise(property);
    }
}
