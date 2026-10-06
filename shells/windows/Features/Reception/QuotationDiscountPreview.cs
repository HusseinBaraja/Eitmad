using System.Globalization;

namespace Eitmad.WindowsShell.Features.Reception;

public sealed partial class SalesCatalogViewModel
{
    // Synthetic workflow fixture only. Production limits and approval outcomes must come from Rust.
    private const decimal PreviewDiscountLimit = 5m;
    private string discountInput = "0";
    private decimal discountPercent;
    private bool isDiscountValid = true;
    private bool discountPending;
    private Features.Quotations.QuotationListItem? approvalPreview;
    public Action<SalesCatalogViewModel, bool>? PublishPreview { get; set; }
    public Guid PreviewId { get; set; } = Guid.NewGuid();
    public bool IsDiscountApproved => catalogClient is null && approvalPreview?.ApprovalDecision == Features.Quotations.DiscountApprovalDecision.Approved;
    public bool IsDiscountRejected => catalogClient is null && approvalPreview?.ApprovalDecision == Features.Quotations.DiscountApprovalDecision.Rejected;
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

    public bool IsDiscountValid => isDiscountValid;
    public string DiscountError => IsDiscountValid ? "" : "أدخل نسبة من 0 إلى 100 بمنزلتين عشريتين كحد أقصى";
    public string DiscountAmountLabel => catalogClient is not null && evaluation?.Totals is null ? "—" : IsDiscountValid ? Discount.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public string FinalTotalLabel => catalogClient is not null && evaluation?.Totals is null ? "—" : IsDiscountValid ? FinalTotal.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public bool RequiresDiscountApproval => catalogClient is not null ? evaluation?.Totals?.ApprovalRequired == true : IsDiscountValid && discountPercent > PreviewDiscountLimit;
    public bool IsDiscountPending => catalogClient is null && discountPending;
    public bool CanRequestDiscountApproval => catalogClient is null && RequiresDiscountApproval && !discountPending && !IsDiscountApproved && !IsQuotationEmpty;
    public bool CanSaveQuotation => catalogClient is not null ? IsDiscountValid && evaluation?.Totals is { ApprovalRequired: false } : IsDiscountValid && (!RequiresDiscountApproval || IsDiscountApproved);
    public bool CanSaveDraft => IsDiscountValid && !IsQuotationEmpty && (catalogClient is null || evaluation?.Totals is not null);
    public string DiscountStatus => IsDiscountApproved ? "تمت الموافقة على الخصم — معاينة فقط" : IsDiscountRejected ? "رُفض الخصم — عدّل العرض أو اطلب الموافقة مجدداً" : IsDiscountPending ? "بانتظار موافقة المدير" : RequiresDiscountApproval ? "يتطلب موافقة المدير" : "";
    public string DiscountGuidance => IsDiscountApproved ? "يمكنك الآن إكمال عرض السعر في المعاينة" : IsDiscountRejected ? "خفّض الخصم أو عدّل العرض ثم اطلب الموافقة مجدداً" : "يمكنك مراجعة العرض وحفظه كمسودة حتى الموافقة على الخصم";
    public string TotalHeading => RequiresDiscountApproval && !IsDiscountApproved ? "الإجمالي بعد الخصم المطلوب" : "الإجمالي النهائي";

    public void RequestDiscountApproval()
    {
        if (!CanRequestDiscountApproval || !CheckRequiredFields()) return;
        discountPending = true;
        RaiseDiscountState();
        PublishPreview?.Invoke(this, true);
        QuotationNotice = "طلب مؤقت في موافقات المدير — معاينة فقط، لم يُرسل إلى الخادم";
    }

    public bool ReviewDraftSave()
    {
        if (!CheckRequiredFields()) return false;
        if (!CanSaveDraft) return false;
        PublishPreview?.Invoke(this, discountPending);
        QuotationNotice = "معاينة المسودة فقط — الحفظ غير متاح بعد، ولم يُحفظ عرض السعر";
        return true;
    }

    private void InvalidateDiscountRequest()
    {
        if (approvalPreview is not null) System.ComponentModel.PropertyChangedEventManager.RemoveHandler(approvalPreview, ApprovalChanged, "");
        approvalPreview = null;
        discountPending = false;
        QuotationNotice = "";
        RaiseDiscountState();
        QueueQuotationEvaluation();
    }

    private void RaiseDiscountState()
    {
        foreach (var property in new[] { nameof(IsDiscountValid), nameof(DiscountError), nameof(Discount),
            nameof(DiscountAmountLabel), nameof(FinalTotal), nameof(FinalTotalLabel), nameof(RequiresDiscountApproval),
            nameof(IsDiscountPending), nameof(IsDiscountApproved), nameof(IsDiscountRejected), nameof(CanRequestDiscountApproval), nameof(CanSaveQuotation),
            nameof(CanSaveDraft), nameof(DiscountStatus), nameof(DiscountGuidance), nameof(TotalHeading), nameof(CanPreviewCustomer) }) Raise(property);
    }
}
