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

    public string DiscountInput
    {
        get => discountInput;
        set
        {
            if (!Set(ref discountInput, value ?? "")) return;
            isDiscountValid = decimal.TryParse(PreviewText.NormalizeNumericInput(discountInput).Trim(),
                NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out discountPercent)
                && discountPercent is >= 0m and <= 100m;
            InvalidateDiscountRequest();
        }
    }

    public bool IsDiscountValid => isDiscountValid;
    public string DiscountError => IsDiscountValid ? "" : "أدخل نسبة من 0 إلى 100";
    public string DiscountAmountLabel => IsDiscountValid ? Discount.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public string FinalTotalLabel => IsDiscountValid ? FinalTotal.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public bool RequiresDiscountApproval => IsDiscountValid && discountPercent > PreviewDiscountLimit;
    public bool IsDiscountPending => discountPending;
    public bool CanRequestDiscountApproval => RequiresDiscountApproval && !discountPending && !IsQuotationEmpty;
    public bool CanSaveQuotation => IsDiscountValid && !RequiresDiscountApproval;
    public bool CanSaveDraft => IsDiscountValid && !IsQuotationEmpty;
    public string DiscountStatus => discountPending ? "بانتظار موافقة المدير" : RequiresDiscountApproval ? "يتطلب موافقة المدير" : "";
    public string TotalHeading => RequiresDiscountApproval ? "الإجمالي بعد الخصم المطلوب" : "الإجمالي النهائي";

    public void RequestDiscountApproval()
    {
        if (!CanRequestDiscountApproval) return;
        discountPending = true;
        RaiseDiscountState();
        QuotationNotice = "معاينة حالة الانتظار فقط — لم يُرسل طلب الموافقة";
    }

    public bool ReviewDraftSave()
    {
        if (!CanSaveDraft) return false;
        QuotationNotice = "معاينة المسودة فقط — الحفظ غير متاح بعد، ولم تُحفظ البيانات";
        return true;
    }

    private void InvalidateDiscountRequest()
    {
        discountPending = false;
        QuotationNotice = "";
        RaiseDiscountState();
    }

    private void RaiseDiscountState()
    {
        foreach (var property in new[] { nameof(IsDiscountValid), nameof(DiscountError), nameof(Discount),
            nameof(DiscountAmountLabel), nameof(FinalTotal), nameof(FinalTotalLabel), nameof(RequiresDiscountApproval),
            nameof(IsDiscountPending), nameof(CanRequestDiscountApproval), nameof(CanSaveQuotation),
            nameof(CanSaveDraft), nameof(DiscountStatus), nameof(TotalHeading), nameof(CanPreviewCustomer) }) Raise(property);
    }
}
