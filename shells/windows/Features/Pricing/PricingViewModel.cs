using System.Collections.ObjectModel;
using System.Globalization;
using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.Pricing;

public sealed class PricingViewModel : ObservableObject
{
    public const string AllCategories = "كل الفئات";
    private readonly List<PricingListItem> prices = [];
    private PricingListItem? editingPrice;
    private PriceReview? review;
    private string searchText = "", selectedCategory = AllCategories, editorSellingPrice = "", editorError = "", feedbackMessage = "";
    private string availabilityMessage = "جار تحميل الأسعار…";
    private string syncIssueMessage = "";
    private bool isEditorOpen, isBusy, savePending, confirmBelowCost, canManage, canReadCosts;
    public event EventHandler? SearchChanged;
    public event EventHandler? EditorPriceChanged;
    public ObservableCollection<PricingListItem> VisiblePrices { get; } = [];
    public ObservableCollection<string> Categories { get; } = [AllCategories];
    public string SearchText { get => searchText; set { if (Set(ref searchText, value ?? "")) SearchChanged?.Invoke(this, EventArgs.Empty); } }
    public string SelectedCategory { get => selectedCategory; set { if (Set(ref selectedCategory, value ?? AllCategories)) RefreshVisiblePrices(); } }
    public bool IsEditorOpen { get => isEditorOpen; private set => Set(ref isEditorOpen, value); }
    public bool CanManage { get => canManage; private set { Set(ref canManage, value); Raise(nameof(CanEdit)); } }
    public bool CanReadCosts { get => canReadCosts; private set { Set(ref canReadCosts, value); Raise(nameof(CanEdit)); } }
    public bool IsBusy { get => isBusy; set { Set(ref isBusy, value); Raise(nameof(CanEdit)); Raise(nameof(CanSave)); } }
    public bool SavePending { get => savePending; set { Set(ref savePending, value); Raise(nameof(CanEdit)); } }
    public bool CanEdit => CanManage && CanReadCosts && !IsBusy && !SavePending;
    public bool CanSave => CanManage && CanReadCosts && !IsBusy;
    public string AvailabilityMessage { get => availabilityMessage; private set => Set(ref availabilityMessage, value); }
    public string SyncIssueMessage { get => syncIssueMessage; private set { Set(ref syncIssueMessage, value); Raise(nameof(HasSyncIssues)); } }
    public bool HasSyncIssues => SyncIssueMessage.Length > 0;
    public string EditorProduct => editingPrice?.Product ?? "";
    public string EditorVariant => editingPrice?.Variant ?? "";
    public string EditorCost => PricingListItem.FormatMoney(review?.CostYer ?? editingPrice?.Cost);
    public string EditorMargin => PricingListItem.FormatMoney(review?.MarginYer);
    public bool HasNegativeEditorMargin => review?.BelowCost == true;
    public bool ConfirmBelowCost { get => confirmBelowCost; set => Set(ref confirmBelowCost, value); }
    public string EditorSellingPrice
    {
        get => editorSellingPrice;
        set { if (!Set(ref editorSellingPrice, value ?? "")) return; review = null; ConfirmBelowCost = false; EditorError = ""; RaiseReview(); EditorPriceChanged?.Invoke(this, EventArgs.Empty); }
    }
    public string EditorError { get => editorError; private set { Set(ref editorError, value); Raise(nameof(HasEditorError)); } }
    public bool HasEditorError => EditorError.Length > 0;
    public string FeedbackMessage { get => feedbackMessage; private set { Set(ref feedbackMessage, value); Raise(nameof(HasFeedback)); } }
    public bool HasFeedback => FeedbackMessage.Length > 0;
    public bool HasNoVisiblePrices => VisiblePrices.Count == 0;
    public string VisibleCountLabel => $"{VisiblePrices.Count} من {prices.Count} أسعار";
    public long EditorVersion { get; private set; }

    public void ApplyDurableData(PricePage page)
    {
        CanManage = page.CanManage; CanReadCosts = page.CanReadCosts;
        if (!CanManage || !CanReadCosts) CancelEditor(force: true);
        prices.Clear();
        foreach (var record in page.Items)
        {
            if (!CanReadCosts) { record.CostYer = null; record.MarginYer = null; }
            if (CanManage || record.Published is not null) prices.Add(new(record));
        }
        Categories.Clear(); Categories.Add(AllCategories);
        foreach (var category in prices.Select(p => p.Category).Distinct()) Categories.Add(category);
        if (!Categories.Contains(SelectedCategory)) SelectedCategory = AllCategories;
        AvailabilityMessage = page.ServerAvailable ? "أسعار مؤكدة من الخادم — القيم الداخلية حسب الصلاحية" : "الخادم غير متاح — عرض آخر أسعار مؤكدة، والنشر يحتاج اتصالاً.";
        SyncIssueMessage = CanManage && page.CatalogSyncIssues is { Length: > 0 } issues
            ? "تحتاج مزامنة هذه التعريفات إلى مراجعة: " + string.Join("، ", issues.Take(3).Select(issue => issue.Name))
                + (issues.Length > 3 ? " وغيرها" : "") + ". صحّح التعريف واحفظ إصداراً جديداً؛ تبقى التعريفات التي تعتمد على الإصدار المرفوض معلّقة."
            : "";
        RefreshVisiblePrices();
    }
    public void ClearSession()
    {
        CancelEditor(force: true); prices.Clear(); VisiblePrices.Clear(); Categories.Clear(); Categories.Add(AllCategories);
        CanManage = false; CanReadCosts = false; SavePending = false; IsBusy = false; FeedbackMessage = "";
        SyncIssueMessage = "";
        AvailabilityMessage = "بيانات الأسعار غير متاحة."; Raise(nameof(VisibleCountLabel));
    }
    public void Unavailable(string message) { ClearSession(); AvailabilityMessage = message; }
    public void BeginEdit(PricingListItem item)
    {
        if (!CanEdit || !prices.Contains(item)) return;
        ++EditorVersion; editingPrice = item; review = null; ConfirmBelowCost = false;
        EditorSellingPrice = (item.SellingPrice ?? 0).ToString("N0", CultureInfo.InvariantCulture);
        EditorError = ""; IsEditorOpen = true; Raise(nameof(EditorProduct)); Raise(nameof(EditorVariant)); RaiseReview();
        EditorPriceChanged?.Invoke(this, EventArgs.Empty);
    }
    public void CancelEditor(bool force = false)
    {
        if (!force && (IsBusy || SavePending)) return;
        ++EditorVersion; IsEditorOpen = false; EditorError = ""; editingPrice = null; review = null;
        editorSellingPrice = ""; ConfirmBelowCost = false; Raise(nameof(EditorSellingPrice)); RaiseReview();
    }
    public ReviewPrice? ReviewInput() => editingPrice is not null && TryParsePrice(EditorSellingPrice, out var price)
        ? new() { Target = editingPrice.Record.Target, SellingPriceYer = price } : null;
    public void ApplyReview(PriceReview value) { review = value; RaiseReview(); }
    public PublishPrice? SaveInput()
    {
        if (editingPrice is null || !TryParsePrice(EditorSellingPrice, out var price) || price <= 0) { Fail(PricingClient.ArabicMessage(PricingFailure.Invalid)); return null; }
        if (HasNegativeEditorMargin && !ConfirmBelowCost) { Fail(PricingClient.ArabicMessage(PricingFailure.BelowCost)); return null; }
        return new() { Target = editingPrice.Record.Target, ExpectedRevision = editingPrice.Record.Published?.Revision, SellingPriceYer = price, ConfirmBelowCost = ConfirmBelowCost };
    }
    public void Saved() { SavePending = false; CancelEditor(force: true); FeedbackMessage = "نُشر سعر البيع بتأكيد الخادم وحُفظ سجل التغيير."; }
    public void Fail(string message) => EditorError = message;
    public void ClearFeedback() => FeedbackMessage = "";
    private void RaiseReview() { Raise(nameof(EditorCost)); Raise(nameof(EditorMargin)); Raise(nameof(HasNegativeEditorMargin)); }
    private void RefreshVisiblePrices()
    {
        VisiblePrices.Clear();
        foreach (var item in prices.Where(p => SelectedCategory == AllCategories || p.Category == SelectedCategory)) VisiblePrices.Add(item);
        Raise(nameof(HasNoVisiblePrices)); Raise(nameof(VisibleCountLabel));
    }
    private static bool TryParsePrice(string value, out long result) => long.TryParse(PreviewText.NormalizeNumericInput(value).Trim(), NumberStyles.AllowThousands, CultureInfo.InvariantCulture, out result) && result >= 0;
}
