using Eitmad.Contracts;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Features.Furniture;

/// <summary>Projects Rust definitions and stages unsaved six-step editor fields.</summary>
public sealed partial class FurnitureViewModel : ObservableObject
{
    public const string AllCategories = "كل الفئات";
    public const string AllStatuses = "كل الحالات";
    public const string ActiveStatus = "نشط";
    public const string ArchivedStatus = "مؤرشف";
    public const string DraftStatus = "مسودة";

    private readonly List<FurnitureListItem> furniture;
    private readonly List<FurniturePartOption> availableParts;
    private readonly List<FurnitureColorOption> defaultColors =
    [
        new(Guid.NewGuid(), "أبيض", "#F7F4EF", 0m),
        new(Guid.NewGuid(), "بني", "#8B5A3C", 0m, isActive: false),
        new(Guid.NewGuid(), "جوزي", "#4F2C1D", 10_000m),
    ];
    private readonly List<FurnitureHandleOption> defaultHandles =
    [
        new(Guid.NewGuid(), "مقبض قياسي", "Standard", 0m),
        new(Guid.NewGuid(), "معدن أسود", "BlackMetal", 3_000m),
        new(Guid.NewGuid(), "نحاسي", "Brass", 5_000m, isActive: false),
    ];
    private FurnitureListItem? editingFurniture;
    private FurnitureVariant? editingVariant;
    private string searchText = string.Empty;
    private string selectedCategory = AllCategories;
    private string selectedStatus = AllStatuses;
    private bool isEditorOpen;
    private bool isCreating;
    private int currentStep = 1;
    private string editorName = string.Empty;
    private string editorCategory = "غرف النوم";
    private string shortDescription = string.Empty;
    private string internalNotes = string.Empty;
    public long ImageEditVersion { get; private set; }
    public void InvalidateImageLoad() => ++ImageEditVersion;
    private Eitmad.Contracts.CatalogImageRef? editorImageReference;
    public Eitmad.Contracts.CatalogImageRef? EditorImageReference => editorImageReference;
    public void SetImportedImage(Eitmad.Contracts.CatalogImageRef? reference, ImageSource? image) { editorImageReference=reference; ProductImage=image; ProductImageName=reference is null ? "" : "صورة مستوردة"; }
    public void ApplyImage(Guid id, ImageSource? image) { var row=furniture.FirstOrDefault(p=>p.Id==id); if(row is not null) row.Image=image; }
    private ImageSource? productImage;
    private string productImageName = string.Empty;
    private string editorError = string.Empty;
    private string feedbackMessage = string.Empty;
    private bool isPartPickerOpen;
    private string partSearchText = string.Empty;
    private bool isVariantEditorOpen;
    private string variantName = string.Empty;
    private decimal variantWidth = 120m;
    private decimal variantHeight = 200m;
    private decimal variantDepth = 55m;
    private bool isColorEditorOpen;
    private string colorName = string.Empty;
    private decimal colorPriceAdjustment;
    private string colorSwatchHex = "#F7F4EF";
    private bool isHandleEditorOpen;
    private string handleName = string.Empty;
    private decimal handlePriceAdjustment;

    /// <summary>Creates empty list and editor projections and wires advisory review requests.</summary>
    public FurnitureViewModel()
    {
        furniture = [];
        availableParts = [];
        CategoryOptions = [AllCategories];
        EditorCategoryOptions = [];
        StatusOptions = [AllStatuses, ActiveStatus, DraftStatus, ArchivedStatus];
        VisibleFurniture = [];
        SelectedParts = [];
        FilteredParts = [];
        Variants = [];
        Variants.CollectionChanged += (_, e) => { if (e.NewItems is not null) foreach (FurnitureVariant v in e.NewItems) v.PropertyChanged += (_, args) => { if (args.PropertyName == nameof(FurnitureVariant.SellingPrice)) ReviewRequested?.Invoke(this, EventArgs.Empty); }; };
        Colors = [];
        Handles = [];

        RefreshVisibleFurniture();
        RefreshPartOptions();
    }

    public ObservableCollection<string> CategoryOptions { get; }

    internal bool FixtureSalesCatalog { get; set; }

    /// <summary>Exposes explicit synthetic sales fixtures; private live definitions remain excluded.</summary>
    public IEnumerable<Reception.SalesCatalogItem> GetSalesCatalogItems() =>
        furniture.Where(item => FixtureSalesCatalog && !item.IsArchived && !item.IsDraft).Select(item =>
            new Reception.SalesCatalogItem(item.Id, item.Name, item.Category, records[item.Id].Description,
                item.VariantCountLabel, item.SellingPrice, true, item.ThumbnailKind, null));

    public Reception.FurnitureSelectionViewModel? GetSalesSelection(Guid id)
    {
        var item = GetSalesCatalogItems().FirstOrDefault(item => item.Id == id);
        if (item is null) return null;
        var record = records[id];
        return new(item,
            record.Variants.Where(v => !v.Archived).Select(v => new Reception.SalesSize(v.Id, v.Name,
                new FurnitureVariant(v.Id, v.Name, v.Dimensions.WidthMm / 10m, v.Dimensions.HeightMm / 10m,
                    v.Dimensions.DepthMm / 10m, 0, v.SellingPriceYer).DimensionsLabel, v.SellingPriceYer)).ToArray(),
            record.Colors.Where(c => !c.Archived).Select(c => new Reception.SalesOption(c.Id, c.Name, c.PriceAdjustmentYer,
                new FurnitureColorOption(c.Id, c.Name, c.Visual, c.PriceAdjustmentYer).SwatchBrush)).ToArray(),
            record.Handles.Where(h => !h.Archived).Select(h => new Reception.SalesOption(h.Id, h.Name, h.PriceAdjustmentYer,
                new FurnitureHandleOption(h.Id, h.Name, h.Visual, h.PriceAdjustmentYer).HandleBrush)).ToArray());
    }

    public ObservableCollection<string> EditorCategoryOptions { get; }

    public IReadOnlyList<string> StatusOptions { get; }

    public ObservableCollection<FurnitureListItem> VisibleFurniture { get; }

    public ObservableCollection<FurniturePartUsage> SelectedParts { get; }

    public ObservableCollection<FurniturePartOption> FilteredParts { get; }

    public ObservableCollection<FurnitureVariant> Variants { get; }

    public ObservableCollection<FurnitureColorOption> Colors { get; }

    public ObservableCollection<FurnitureHandleOption> Handles { get; }

    public IReadOnlyList<string> ColorSwatchOptions { get; } =
    [
        "#F7F4EF",
        "#8B5A3C",
        "#4F2C1D",
        "#2E596B",
        "#B89A72",
    ];

    public string SearchText
    {
        get => searchText;
        set
        {
            if (Set(ref searchText, value ?? string.Empty))
            {
                SearchChanged?.Invoke(this, EventArgs.Empty);
            }
        }
    }

    public string SelectedCategory
    {
        get => selectedCategory;
        set
        {
            if (Set(ref selectedCategory, value ?? AllCategories))
            {
                RefreshVisibleFurniture();
            }
        }
    }

    public string SelectedStatus
    {
        get => selectedStatus;
        set
        {
            if (Set(ref selectedStatus, value ?? AllStatuses))
            {
                RefreshVisibleFurniture();
            }
        }
    }

    public bool IsEditorOpen
    {
        get => isEditorOpen;
        private set
        {
            if (Set(ref isEditorOpen, value))
            {
                Raise(nameof(IsListVisible));
            }
        }
    }

    public bool IsListVisible => !IsEditorOpen;

    public bool IsCreating
    {
        get => isCreating;
        private set
        {
            if (Set(ref isCreating, value))
            {
                Raise(nameof(EditorTitle));
            }
        }
    }

    public string EditorTitle => IsCreating ? "إضافة منتج" : "تعديل المنتج";

    public int CurrentStep
    {
        get => currentStep;
        private set
        {
            if (Set(ref currentStep, value))
            {
                Raise(nameof(IsStepOne));
                Raise(nameof(IsStepTwo));
                Raise(nameof(IsStepThree));
                Raise(nameof(IsStepFour));
                Raise(nameof(IsStepFive));
                Raise(nameof(IsStepSix));
                Raise(nameof(EditorStepDescription));
            }
        }
    }

    public bool IsStepOne => CurrentStep == 1;

    public bool IsStepTwo => CurrentStep == 2;

    public bool IsStepThree => CurrentStep == 3;

    public bool IsStepFour => CurrentStep == 4;

    public bool IsStepFive => CurrentStep == 5;

    public bool IsStepSix => CurrentStep == 6;

    public string EditorStepDescription => CurrentStep switch
    {
        2 => "اختر الأجزاء، واضبط الكمية، وشاهد التكلفة المحدثة.",
        3 => "أضف المقاسات الثابتة التي يحددها المدير.",
        4 => "حدّد الألوان والمقابض التي يمكن اختيارها لاحقاً.",
        5 => "حدّد سعر بيع كل مقاس وقارن هامش الربح مباشرة.",
        6 => "راجع الأثاث كاملاً قبل حفظ المسودة أو التعريف.",
        _ => "أنشئ معلومات المنتج الأساسية قبل المتابعة.",
    };

    public string EditorName { get => editorName; set => Set(ref editorName, value ?? string.Empty); }

    public string EditorCategory { get => editorCategory; set => Set(ref editorCategory, value ?? string.Empty); }

    public string ShortDescription
    {
        get => shortDescription;
        set
        {
            if (Set(ref shortDescription, value ?? string.Empty))
            {
                Raise(nameof(ReviewDescription));
            }
        }
    }

    public string ReviewDescription => string.IsNullOrWhiteSpace(ShortDescription)
        ? "لا يوجد وصف لهذا المنتج."
        : ShortDescription.Trim();

    public string InternalNotes { get => internalNotes; set => Set(ref internalNotes, value ?? string.Empty); }

    public ImageSource? ProductImage
    {
        get => productImage;
        private set
        {
            if (Set(ref productImage, value))
            {
                Raise(nameof(HasProductImage));
            }
        }
    }

    public bool HasProductImage => ProductImage is not null;

    public string ProductImageName { get => productImageName; private set => Set(ref productImageName, value); }

    public string EditorError
    {
        get => editorError;
        private set
        {
            if (Set(ref editorError, value))
            {
                Raise(nameof(HasEditorError));
            }
        }
    }

    public bool HasEditorError => !string.IsNullOrEmpty(EditorError);

    public string FeedbackMessage
    {
        get => feedbackMessage;
        private set
        {
            if (Set(ref feedbackMessage, value))
            {
                Raise(nameof(HasFeedback));
            }
        }
    }

    public bool HasFeedback => !string.IsNullOrEmpty(FeedbackMessage);

    public bool IsPartPickerOpen { get => isPartPickerOpen; private set => Set(ref isPartPickerOpen, value); }

    public string PartSearchText
    {
        get => partSearchText;
        set
        {
            if (Set(ref partSearchText, value ?? string.Empty))
            {
                PartSearchChanged?.Invoke(this, EventArgs.Empty);
            }
        }
    }

    public bool HasSelectedParts => SelectedParts.Count > 0;

    public bool HasNoPartOptions => FilteredParts.Count == 0;

    public decimal CurrentPartsCost => reviewedCost;

    public string CurrentPartsCostLabel => reviewedCost.ToString("N0", CultureInfo.InvariantCulture);

    public bool HasVariants => Variants.Count > 0;

    public bool IsVariantEditorOpen { get => isVariantEditorOpen; private set => Set(ref isVariantEditorOpen, value); }

    public string VariantEditorTitle => editingVariant is null ? "إضافة مقاس" : "تعديل المقاس";

    public string VariantName { get => variantName; set => Set(ref variantName, value ?? string.Empty); }

    public decimal VariantWidth { get => variantWidth; set => Set(ref variantWidth, value); }

    public decimal VariantHeight { get => variantHeight; set => Set(ref variantHeight, value); }

    public decimal VariantDepth { get => variantDepth; set => Set(ref variantDepth, value); }

    public bool IsColorEditorOpen { get => isColorEditorOpen; private set => Set(ref isColorEditorOpen, value); }

    public string ColorName { get => colorName; set => Set(ref colorName, value ?? string.Empty); }

    public decimal ColorPriceAdjustment { get => colorPriceAdjustment; set => Set(ref colorPriceAdjustment, value); }

    public string ColorSwatchHex { get => colorSwatchHex; set => Set(ref colorSwatchHex, value ?? "#F7F4EF"); }

    public bool IsHandleEditorOpen { get => isHandleEditorOpen; private set => Set(ref isHandleEditorOpen, value); }

    public string HandleName { get => handleName; set => Set(ref handleName, value ?? string.Empty); }

    public decimal HandlePriceAdjustment { get => handlePriceAdjustment; set => Set(ref handlePriceAdjustment, value); }

    public bool HasColors => Colors.Count > 0;

    public bool HasHandles => Handles.Count > 0;

    public bool HasNoVisibleFurniture => !IsLoading && VisibleFurniture.Count == 0;

    public string VisibleCountLabel => $"{VisibleFurniture.Count} من {furniture.Count} منتجات";

    /// <summary>Starts unsaved input only when management is allowed and no save outcome is pending.</summary>
    public void BeginCreate()
    {
        if (!CanManage || IsBusy || pendingSave is not null) return;
        editingFurniture = null;
        IsCreating = true;
        EditorName = string.Empty;
        editingRecord = null; editingExpectedRevision=null; pendingSave = null; reviewedCost=0; RefreshPartsState();
        EditorCategory = EditorCategoryOptions.FirstOrDefault() ?? "";
        ConfirmBelowCost = false;
        ShortDescription = string.Empty;
        InternalNotes = string.Empty;
        ++ImageEditVersion; SetImportedImage(null,null);
        ReplaceSelectedParts([]);
        ReplaceVariants([]);
        ReplaceColors(defaultColors.Select(color => new FurnitureColorOption(Guid.NewGuid(),color.Name,color.SwatchHex,color.PriceAdjustment,color.IsActive)));
        ReplaceHandles(defaultHandles.Select(handle => new FurnitureHandleOption(Guid.NewGuid(),handle.Name,handle.HandleKind,handle.PriceAdjustment,handle.IsActive)));
        ResetEditorState();
    }

    /// <summary>Stages a saved definition with its original revision and immutable Part references.</summary>
    public bool BeginEdit(FurnitureListItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        if (!CanEditFields) return false;
        var record = records.GetValueOrDefault(item.Id);
        if (record is null || record.Parts.Any(u => !compositions.ContainsKey((u.Reference.PartId, u.Reference.Revision))))
        {
            DataStateText = "السجل أو أجزاؤه غير متاحة. أعد تحميل القائمة.";
            return false;
        }
        editingRecord = record; editingExpectedRevision=record.Revision; pendingSave = null;
        ConfirmBelowCost = false;
        editingFurniture = item;
        IsCreating = false;
        EditorName = editingRecord.Name;
        EditorCategory = editingRecord.CategoryName;
        ShortDescription = editingRecord.Description;
        InternalNotes = editingRecord.Notes;
        ++ImageEditVersion; SetImportedImage(editingRecord.Image,item.Image);
        ReplaceSelectedParts(editingRecord.Parts.Select(u =>
        {
            var part = compositions[(u.Reference.PartId, u.Reference.Revision)];
            return new FurniturePartUsage(PartOption(part), u.Quantity);
        }));
        ReplaceVariants(editingRecord.Variants.Where(v => !v.Archived).Select(v =>
            new FurnitureVariant(v.Id, v.Name, v.Dimensions.WidthMm / 10m, v.Dimensions.HeightMm / 10m,
                v.Dimensions.DepthMm / 10m, editingRecord.PartsCostYer, v.SellingPriceYer)
                { Customization = v.Customization, ColorIds = v.ColorIds.ToArray(), HandleIds = v.HandleIds.ToArray() }));
        ReplaceColors(editingRecord.Colors.Select(c => new FurnitureColorOption(c.Id, c.Name, c.Visual, c.PriceAdjustmentYer, !c.Archived)));
        ReplaceHandles(editingRecord.Handles.Select(h => new FurnitureHandleOption(h.Id, h.Name, h.Visual, h.PriceAdjustmentYer, !h.Archived)));
        ResetEditorState();
        return true;
    }

    /// <summary>Closes unsaved input unless an unresolved save must first be retried.</summary>
    public void CancelEditor()
    {
        ++ImageEditVersion;
        if (pendingSave is not null) { EditorError = "أعد محاولة الحفظ لحسم النتيجة أولاً."; return; }
        IsEditorOpen = false;
        IsPartPickerOpen = false;
        IsVariantEditorOpen = false;
        IsColorEditorOpen = false;
        IsHandleEditorOpen = false;
        EditorError = string.Empty;
    }

    /// <summary>Requires a name and saved category before advancing to composition input.</summary>
    public bool MoveToParts()
    {
        if (EditorName.Trim().Length == 0)
        {
            EditorError = "أدخل اسم الأثاث.";
            return false;
        }

        if (EditorCategory.Trim().Length == 0)
        {
            EditorError = "اختر فئة الأثاث.";
            return false;
        }

        if (!categories.Any(c => c.Name == EditorCategory.Trim()))
        {
            EditorError = UnsavedCategoryMessage;
            return false;
        }

        EditorError = string.Empty;
        CurrentStep = 2;
        return true;
    }

    /// <summary>Requires selected Parts with positive quantities before advancing.</summary>
    public bool MoveToVariants()
    {
        if (SelectedParts.Count == 0)
        {
            EditorError = "أضف جزءاً واحداً على الأقل للمتابعة.";
            return false;
        }

        if (SelectedParts.Any(item => item.Quantity <= 0m))
        {
            EditorError = "أدخل كمية أكبر من صفر لكل جزء.";
            return false;
        }

        EditorError = string.Empty;
        CurrentStep = 3;
        return true;
    }

    public void MoveToPreviousStep()
    {
        if (CurrentStep > 1)
        {
            CurrentStep--;
            EditorError = string.Empty;
        }
    }

    public bool MoveToOptions()
    {
        if (!HasVariants)
        {
            EditorError = "أضف مقاساً ثابتاً واحداً على الأقل للمتابعة.";
            return false;
        }

        EditorError = string.Empty;
        CurrentStep = 4;
        return true;
    }

    public bool MoveToPricing()
    {
        if (!HasVariants)
        {
            EditorError = "أضف مقاساً ثابتاً واحداً على الأقل للمتابعة.";
            return false;
        }

        EditorError = string.Empty;
        CurrentStep = 5;
        return true;
    }

    /// <summary>Opens the final summary after the view completes authoritative review.</summary>
    public bool MoveToReview()
    {
        ClearFeedback();
        EditorError = string.Empty;
        CurrentStep = 6;
        return true;
    }

    public void ReportPricingInputError() =>
        EditorError = "صحّح سعر البيع غير الصالح قبل المتابعة.";

    public void OpenPartPicker()
    {
        PartSearchText = string.Empty;
        RefreshPartOptions();
        IsPartPickerOpen = true;
    }

    public void ClosePartPicker() => IsPartPickerOpen = false;

    /// <summary>Adds a picker reference to staged composition and requests Rust costing.</summary>
    public void AddPart(FurniturePartOption part)
    {
        ArgumentNullException.ThrowIfNull(part);
        if (SelectedParts.Any(item => item.Part.Id == part.Id))
        {
            return;
        }

        AddSelectedPart(new FurniturePartUsage(part));
        ReviewRequested?.Invoke(this,EventArgs.Empty);
        RefreshPartOptions();
        IsPartPickerOpen = false;
        EditorError = string.Empty;
    }

    /// <summary>Removes staged Part usage and requests updated Rust costing.</summary>
    public void RemovePart(FurniturePartUsage usage)
    {
        ArgumentNullException.ThrowIfNull(usage);
        usage.PropertyChanged -= SelectedPartChanged;
        SelectedParts.Remove(usage);
        ReviewRequested?.Invoke(this,EventArgs.Empty);
        RefreshPartsState();
        RefreshPartOptions();
    }

    /// <summary>Opens an unsaved fixed-size dialog with option choices.</summary>
    public void BeginAddVariant()
    {
        editingVariant = null;
        PrepareVariantChoices(null);
        VariantName = string.Empty;
        VariantWidth = 120m;
        VariantHeight = 200m;
        VariantDepth = 55m;
        EditorError = string.Empty;
        Raise(nameof(VariantEditorTitle));
        IsVariantEditorOpen = true;
    }

    /// <summary>Stages a variant and its permitted bounds and option identities.</summary>
    public void BeginEditVariant(FurnitureVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        editingVariant = variant;
        PrepareVariantChoices(variant);
        VariantName = variant.Name;
        VariantWidth = variant.Width;
        VariantHeight = variant.Height;
        VariantDepth = variant.Depth;
        EditorError = string.Empty;
        Raise(nameof(VariantEditorTitle));
        IsVariantEditorOpen = true;
    }

    /// <summary>Stages exact dimensions, permitted bounds, and compatible options without committing.</summary>
    public bool SaveVariant()
    {
        if (VariantName.Trim().Length == 0)
        {
            EditorError = "أدخل اسم المقاس.";
            return false;
        }

        if (VariantWidth <= 0m || VariantHeight <= 0m || VariantDepth <= 0m)
        {
            EditorError = "أدخل أبعاداً أكبر من صفر.";
            return false;
        }

        decimal cost = reviewedCost;
        try { _ = ToMillimetres(VariantWidth); _ = ToMillimetres(VariantHeight); _ = ToMillimetres(VariantDepth); }
        catch (Exception e) when (e is OverflowException or FormatException) { EditorError = "أدخل الأبعاد بالسنتيمتر بمنزلة عشرية واحدة ضمن النطاق المدعوم."; return false; }
        FurnitureCustomization? customization;
        try { customization = AllowCustomization ? new FurnitureCustomization { Minimum = Dimensions(MinWidth, MinHeight, MinDepth), Maximum = Dimensions(MaxWidth, MaxHeight, MaxDepth) } : null; }
        catch (Exception e) when (e is FormatException or OverflowException) { EditorError = "صحّح حدود المقاس."; return false; }
        if (editingVariant is null)
        {
            Variants.Add(new FurnitureVariant(Guid.NewGuid(), VariantName.Trim(), VariantWidth, VariantHeight, VariantDepth, cost));
        }
        else
        {
            var index = Variants.IndexOf(editingVariant);
            Variants[index] = new FurnitureVariant(
                editingVariant.Id,
                VariantName.Trim(),
                VariantWidth,
                VariantHeight,
                VariantDepth,
                cost,
                editingVariant.SellingPrice);
        }

        var savedVariant = editingVariant is null ? Variants[^1] : Variants.First(v => v.Id == editingVariant.Id);
        savedVariant.Customization = customization;
        savedVariant.ColorIds = VariantColorChoices.Where(c => c.Selected).Select(c => c.Id).ToArray();
        savedVariant.HandleIds = VariantHandleChoices.Where(c => c.Selected).Select(c => c.Id).ToArray();
        ReviewRequested?.Invoke(this, EventArgs.Empty);
        IsVariantEditorOpen = false;
        EditorError = string.Empty;
        Raise(nameof(HasVariants));
        return true;
    }

    public void CancelVariantEditor()
    {
        IsVariantEditorOpen = false;
        EditorError = string.Empty;
    }

    /// <summary>Creates an unsaved variant copy with a new identity.</summary>
    public void DuplicateVariant(FurnitureVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        Variants.Add(variant.Copy($"{variant.Name} — نسخة"));
        ReviewRequested?.Invoke(this, EventArgs.Empty);
        Raise(nameof(HasVariants));
        FeedbackMessage = "أُضيف مقاس إلى المحرر. لم يُحفظ بعد.";
    }

    /// <summary>Removes a staged variant and invalidates reviews of the previous variant order.</summary>
    public void RemoveVariant(FurnitureVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        Variants.Remove(variant);
        ReviewRequested?.Invoke(this, EventArgs.Empty);
        Raise(nameof(HasVariants));
    }

    public void BeginAddColor()
    {
        ColorName = string.Empty;
        ColorPriceAdjustment = 0m;
        ColorSwatchHex = ColorSwatchOptions[0];
        EditorError = string.Empty;
        IsColorEditorOpen = true;
    }

    /// <summary>Stages a named color and integer price adjustment in the editor.</summary>
    public bool SaveColor()
    {
        if (ColorName.Trim().Length == 0)
        {
            EditorError = "أدخل اسم اللون.";
            return false;
        }

        if (ColorPriceAdjustment < 0m)
        {
            EditorError = "أدخل تعديلاً سعرياً يساوي صفراً أو أكثر.";
            return false;
        }

        Colors.Add(new FurnitureColorOption(Guid.NewGuid(), ColorName.Trim(), ColorSwatchHex, ColorPriceAdjustment));
        IsColorEditorOpen = false;
        EditorError = string.Empty;
        Raise(nameof(HasColors));
        FeedbackMessage = "أُضيف لون إلى المحرر. لم يُحفظ بعد.";
        return true;
    }

    public void CancelColorEditor()
    {
        IsColorEditorOpen = false;
        EditorError = string.Empty;
    }

    public void ToggleColor(FurnitureColorOption color)
    {
        ArgumentNullException.ThrowIfNull(color);
        color.IsActive = !color.IsActive;
    }

    public void BeginAddHandle()
    {
        HandleName = string.Empty;
        HandlePriceAdjustment = 0m;
        EditorError = string.Empty;
        IsHandleEditorOpen = true;
    }

    /// <summary>Stages a named handle and integer price adjustment in the editor.</summary>
    public bool SaveHandle()
    {
        if (HandleName.Trim().Length == 0)
        {
            EditorError = "أدخل اسم المقبض.";
            return false;
        }

        if (HandlePriceAdjustment < 0m)
        {
            EditorError = "أدخل تعديلاً سعرياً يساوي صفراً أو أكثر.";
            return false;
        }

        var handleKind = defaultHandles[Handles.Count % defaultHandles.Count].HandleKind;
        Handles.Add(new FurnitureHandleOption(Guid.NewGuid(), HandleName.Trim(), handleKind, HandlePriceAdjustment));
        IsHandleEditorOpen = false;
        EditorError = string.Empty;
        Raise(nameof(HasHandles));
        FeedbackMessage = "أُضيف مقبض إلى المحرر. لم يُحفظ بعد.";
        return true;
    }

    public void CancelHandleEditor()
    {
        IsHandleEditorOpen = false;
        EditorError = string.Empty;
    }

    public void ToggleHandle(FurnitureHandleOption handle)
    {
        ArgumentNullException.ThrowIfNull(handle);
        handle.IsActive = !handle.IsActive;
    }

    /// <summary>Creates unsaved input with new Furniture, variant, and option identities.</summary>
    public void DuplicateFurniture(FurnitureListItem item)
    {
        if (!CanEditFields || !records.ContainsKey(item.Id)) return;
        if (!BeginEdit(item)) return;
        editingRecord = null; editingExpectedRevision=null; editingFurniture = null; pendingSave = null;
        EditorName = $"{item.Name} — نسخة"; IsCreating = true;
        ReplaceVariants(Variants.Select(v => v.Copy(v.Name)).ToArray());
        ReplaceColors(Colors.Select(c => new FurnitureColorOption(Guid.NewGuid(), c.Name,c.SwatchHex,c.PriceAdjustment,c.IsActive)).ToArray());
        ReplaceHandles(Handles.Select(h => new FurnitureHandleOption(Guid.NewGuid(),h.Name,h.HandleKind,h.PriceAdjustment,h.IsActive)).ToArray());
        foreach (var v in Variants) { v.ColorIds=[];v.HandleIds=[]; }
        FeedbackMessage = "نسخة غير محفوظة.";
    }

    public void ClearFeedback() => FeedbackMessage = string.Empty;

    public void ReportImageLoadError() =>
        FeedbackMessage = "تعذر فتح الصورة. اختر ملف صورة صالحاً بحجم مناسب.";

    private void ResetEditorState()
    {
        EditorError = string.Empty;
        CurrentStep = 1;
        IsPartPickerOpen = false;
        IsVariantEditorOpen = false;
        IsColorEditorOpen = false;
        IsHandleEditorOpen = false;
        IsEditorOpen = true;
    }

    private void ReplaceSelectedParts(IEnumerable<FurniturePartUsage> usages)
    {
        foreach (var existing in SelectedParts)
        {
            existing.PropertyChanged -= SelectedPartChanged;
        }

        SelectedParts.Clear();
        foreach (var usage in usages)
        {
            AddSelectedPart(usage);
        }

        RefreshPartsState();
        RefreshPartOptions();
    }

    private void AddSelectedPart(FurniturePartUsage usage)
    {
        usage.PropertyChanged += SelectedPartChanged;
        SelectedParts.Add(usage);
        RefreshPartsState();
    }

    /// <summary>Requests authority costing when a staged quantity changes.</summary>
    private void SelectedPartChanged(object? sender, PropertyChangedEventArgs eventArgs)
    {
        if (eventArgs.PropertyName is nameof(FurniturePartUsage.Quantity) or nameof(FurniturePartUsage.TotalCost))
        {
            RefreshPartsState();
            if (eventArgs.PropertyName == nameof(FurniturePartUsage.Quantity)) ReviewRequested?.Invoke(this, EventArgs.Empty);
        }
    }

    private void RefreshPartsState()
    {
        Raise(nameof(HasSelectedParts));
        Raise(nameof(CurrentPartsCost));
        Raise(nameof(CurrentPartsCostLabel));
    }

    private void ReplaceVariants(IEnumerable<FurnitureVariant> variants)
    {
        Variants.Clear();
        foreach (var variant in variants)
        {
            Variants.Add(variant);
        }

        Raise(nameof(HasVariants));
    }

    private void ReplaceColors(IEnumerable<FurnitureColorOption> colors)
    {
        Colors.Clear();
        foreach (var color in colors)
        {
            Colors.Add(color);
        }

        Raise(nameof(HasColors));
    }

    private void ReplaceHandles(IEnumerable<FurnitureHandleOption> handles)
    {
        Handles.Clear();
        foreach (var handle in handles)
        {
            Handles.Add(handle);
        }

        Raise(nameof(HasHandles));
    }

    /// <summary>Projects picker category filters and excludes already selected Parts.</summary>
    private void RefreshPartOptions()
    {
        var selectedIds = SelectedParts.Select(item => item.Part.Id).ToHashSet();
        FilteredParts.Clear();
        foreach (var part in availableParts.Where(item => !selectedIds.Contains(item.Id)))
        {
            FilteredParts.Add(part);
        }

        Raise(nameof(HasNoPartOptions));
    }

    /// <summary>Projects local status and category filters over the Rust search result.</summary>
    private void RefreshVisibleFurniture()
    {
        var matches = furniture.Where(item =>
            (SelectedCategory == AllCategories || item.Category == SelectedCategory)
            && (SelectedStatus == AllStatuses
                || (SelectedStatus == ActiveStatus && !item.IsArchived && !item.IsDraft)
                || (SelectedStatus == DraftStatus && !item.IsArchived && item.IsDraft)
                || (SelectedStatus == ArchivedStatus && item.IsArchived)));

        VisibleFurniture.Clear();
        foreach (var item in matches)
        {
            VisibleFurniture.Add(item);
        }

        Raise(nameof(HasNoVisibleFurniture));
        Raise(nameof(VisibleCountLabel));
    }
}
