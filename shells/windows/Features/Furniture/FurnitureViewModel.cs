using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Features.Furniture;

/// <summary>Owns transient list and four-step furniture editor state for the Windows preview.</summary>
public sealed class FurnitureViewModel : INotifyPropertyChanged
{
    public const string AllCategories = "كل الفئات";
    public const string AllStatuses = "كل الحالات";
    public const string ActiveStatus = "نشط";
    public const string ArchivedStatus = "مؤرشف";

    private readonly List<FurnitureListItem> furniture;
    private readonly List<FurniturePartOption> availableParts;
    private readonly Dictionary<Guid, List<FurniturePartUsage>> partUsages = [];
    private readonly Dictionary<Guid, List<FurnitureVariant>> productVariants = [];
    private readonly List<FurnitureColorOption> defaultColors =
    [
        new(Guid.NewGuid(), "White", "#F7F4EF", 0m),
        new(Guid.NewGuid(), "Brown", "#8B5A3C", 0m, isActive: false),
        new(Guid.NewGuid(), "Walnut", "#4F2C1D", 10_000m),
    ];
    private readonly List<FurnitureHandleOption> defaultHandles =
    [
        new(Guid.NewGuid(), "Standard Handle", "Standard", 0m),
        new(Guid.NewGuid(), "Black Metal", "BlackMetal", 3_000m),
        new(Guid.NewGuid(), "Brass", "Brass", 5_000m, isActive: false),
    ];
    private readonly Dictionary<Guid, List<FurnitureColorOption>> productColors = [];
    private readonly Dictionary<Guid, List<FurnitureHandleOption>> productHandles = [];
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

    public FurnitureViewModel()
    {
        furniture =
        [
            new(Guid.Parse("3fc526b4-2b79-45fd-984c-49258f55951d"), "خزانة السكينة", "غرف النوم", 3, 200_000m, "Wardrobe"),
            new(Guid.Parse("0c2ceef3-620b-4d23-a81e-c955572ef440"), "سرير وادي ظهر", "غرف النوم", 2, 145_000m, "Bed"),
            new(Guid.Parse("4246f76c-5476-42cb-9ee6-07694245319f"), "طاولة ضيافة نُحاس", "غرف المعيشة", 4, 78_000m, "Table"),
            new(Guid.Parse("e96abff0-5078-464a-b10f-f73563c311d3"), "مكتب العمل الهادئ", "المكاتب", 2, 115_000m, "Desk"),
            new(Guid.Parse("72123fb5-c48d-4de0-983a-98f3af74250d"), "مقعد المجلس القديم", "المجالس", 1, 62_000m, "Chair", isArchived: true),
        ];

        availableParts =
        [
            new(Guid.Parse("60849186-d13a-4fa2-b441-f7d875176cbf"), "جانب خزانة كامل", "هيكل", 32_000m),
            new(Guid.Parse("693c7248-4276-4d4b-8607-b030def3858f"), "باب بإطار خشبي", "أبواب", 18_500m),
            new(Guid.Parse("699cf31c-c003-462d-b45a-222a29db44e1"), "رف داخلي قابل للتعديل", "رفوف", 7_500m),
            new(Guid.Parse("b31433d3-62b9-4c6f-9346-221577698564"), "قاعدة درج عميق", "أدراج", 12_000m),
            new(Guid.Parse("3114bbf3-e21b-4d70-82bf-a6d38810caf7"), "ظهر سرير منجد", "تنجيد", 38_000m),
        ];

        CategoryOptions = [AllCategories, "غرف النوم", "غرف المعيشة", "المكاتب", "المجالس"];
        EditorCategoryOptions = ["غرف النوم", "غرف المعيشة", "المكاتب", "المجالس"];
        StatusOptions = [AllStatuses, ActiveStatus, ArchivedStatus];
        VisibleFurniture = [];
        SelectedParts = [];
        FilteredParts = [];
        Variants = [];
        Colors = [];
        Handles = [];
        SeedExistingProductDetails();
        RefreshVisibleFurniture();
        RefreshPartOptions();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public IReadOnlyList<string> CategoryOptions { get; }

    public IReadOnlyList<string> EditorCategoryOptions { get; }

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
                RefreshVisibleFurniture();
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
                Raise(nameof(EditorStepDescription));
            }
        }
    }

    public bool IsStepOne => CurrentStep == 1;

    public bool IsStepTwo => CurrentStep == 2;

    public bool IsStepThree => CurrentStep == 3;

    public bool IsStepFour => CurrentStep == 4;

    public string EditorStepDescription => CurrentStep switch
    {
        2 => "اختر الأجزاء، واضبط الكمية، وشاهد التكلفة المحدثة.",
        3 => "أضف المقاسات الثابتة التي يحددها المدير.",
        4 => "حدّد الألوان والمقابض التي يمكن اختيارها لاحقاً.",
        _ => "أنشئ معلومات المنتج الأساسية قبل المتابعة.",
    };

    public string EditorName { get => editorName; set => Set(ref editorName, value ?? string.Empty); }

    public string EditorCategory { get => editorCategory; set => Set(ref editorCategory, value ?? string.Empty); }

    public string ShortDescription { get => shortDescription; set => Set(ref shortDescription, value ?? string.Empty); }

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
                RefreshPartOptions();
            }
        }
    }

    public bool HasSelectedParts => SelectedParts.Count > 0;

    public bool HasNoPartOptions => FilteredParts.Count == 0;

    public decimal CurrentPartsCost => TryCalculatePartsCost(out var total) ? total : 0m;

    public string CurrentPartsCostLabel => TryCalculatePartsCost(out var total)
        ? total.ToString("N0", CultureInfo.InvariantCulture)
        : "—";

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

    public bool HasNoVisibleFurniture => VisibleFurniture.Count == 0;

    public string VisibleCountLabel => $"{VisibleFurniture.Count} من {furniture.Count} منتجات";

    public void BeginCreate()
    {
        editingFurniture = null;
        IsCreating = true;
        EditorName = string.Empty;
        EditorCategory = "غرف النوم";
        ShortDescription = string.Empty;
        InternalNotes = string.Empty;
        ProductImage = null;
        ProductImageName = string.Empty;
        ReplaceSelectedParts([]);
        ReplaceVariants([]);
        ReplaceColors(defaultColors.Select(color => color.Copy()));
        ReplaceHandles(defaultHandles.Select(handle => handle.Copy()));
        ResetEditorState();
    }

    public void BeginEdit(FurnitureListItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        editingFurniture = item;
        IsCreating = false;
        EditorName = item.Name;
        EditorCategory = item.Category;
        ShortDescription = "تصميم أثاث ثابت المقاسات للاستخدام اليومي.";
        InternalNotes = "بيانات معاينة محلية فقط.";
        ProductImage = null;
        ProductImageName = string.Empty;
        ReplaceSelectedParts(partUsages.GetValueOrDefault(item.Id, []).Select(usage => usage.Copy()));
        ReplaceVariants(productVariants.GetValueOrDefault(item.Id, []).Select(CopyVariant));
        ReplaceColors(productColors.GetValueOrDefault(item.Id, defaultColors).Select(color => color.Copy()));
        ReplaceHandles(productHandles.GetValueOrDefault(item.Id, defaultHandles).Select(handle => handle.Copy()));
        ResetEditorState();
    }

    public void CancelEditor()
    {
        IsEditorOpen = false;
        IsPartPickerOpen = false;
        IsVariantEditorOpen = false;
        IsColorEditorOpen = false;
        IsHandleEditorOpen = false;
        EditorError = string.Empty;
    }

    public void SetProductImage(ImageSource image, string fileName)
    {
        ArgumentNullException.ThrowIfNull(image);
        ProductImage = image;
        ProductImageName = fileName;
        EditorError = string.Empty;
    }

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

        EditorError = string.Empty;
        CurrentStep = 2;
        return true;
    }

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

        if (!TryCalculatePartsCost(out _))
        {
            EditorError = "الكمية كبيرة جداً لحساب تكلفة الأجزاء.";
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

    public void RequestNextFromVariants()
    {
        FeedbackMessage = "تنتهي هذه المعاينة عند المقاسات؛ لم تُبنَ خطوة الخيارات بعد.";
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

    public void RequestNextFromOptions()
    {
        FeedbackMessage = "تنتهي هذه المعاينة عند الخيارات؛ لم تُبنَ خطوة التسعير بعد.";
    }

    public void OpenPartPicker()
    {
        PartSearchText = string.Empty;
        RefreshPartOptions();
        IsPartPickerOpen = true;
    }

    public void ClosePartPicker() => IsPartPickerOpen = false;

    public void AddPart(FurniturePartOption part)
    {
        ArgumentNullException.ThrowIfNull(part);
        if (SelectedParts.Any(item => item.Part.Id == part.Id))
        {
            return;
        }

        AddSelectedPart(new FurniturePartUsage(part));
        RefreshPartOptions();
        IsPartPickerOpen = false;
        EditorError = string.Empty;
    }

    public void RemovePart(FurniturePartUsage usage)
    {
        ArgumentNullException.ThrowIfNull(usage);
        usage.PropertyChanged -= SelectedPartChanged;
        SelectedParts.Remove(usage);
        RefreshPartsState();
        RefreshPartOptions();
    }

    public void BeginAddVariant()
    {
        editingVariant = null;
        VariantName = string.Empty;
        VariantWidth = 120m;
        VariantHeight = 200m;
        VariantDepth = 55m;
        EditorError = string.Empty;
        Raise(nameof(VariantEditorTitle));
        IsVariantEditorOpen = true;
    }

    public void BeginEditVariant(FurnitureVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        editingVariant = variant;
        VariantName = variant.Name;
        VariantWidth = variant.Width;
        VariantHeight = variant.Height;
        VariantDepth = variant.Depth;
        EditorError = string.Empty;
        Raise(nameof(VariantEditorTitle));
        IsVariantEditorOpen = true;
    }

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

        var cost = CalculateVariantPreviewCost(VariantWidth, VariantHeight, VariantDepth);
        if (editingVariant is null)
        {
            Variants.Add(new FurnitureVariant(Guid.NewGuid(), VariantName.Trim(), VariantWidth, VariantHeight, VariantDepth, cost));
        }
        else
        {
            var index = Variants.IndexOf(editingVariant);
            Variants[index] = new FurnitureVariant(editingVariant.Id, VariantName.Trim(), VariantWidth, VariantHeight, VariantDepth, cost);
        }

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

    public void DuplicateVariant(FurnitureVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        Variants.Add(variant.Copy($"{variant.Name} — نسخة"));
        Raise(nameof(HasVariants));
        FeedbackMessage = "أُنشئت نسخة من المقاس في المعاينة المحلية.";
    }

    public void RemoveVariant(FurnitureVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        Variants.Remove(variant);
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
        FeedbackMessage = "أُضيف اللون إلى المعاينة المحلية.";
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
        FeedbackMessage = "أُضيف المقبض إلى المعاينة المحلية.";
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

    public FurnitureListItem DuplicateFurniture(FurnitureListItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        var duplicate = new FurnitureListItem(
            Guid.NewGuid(),
            $"{item.Name} — نسخة",
            item.Category,
            item.VariantCount,
            item.SellingPrice,
            item.ThumbnailKind);
        furniture.Add(duplicate);
        partUsages[duplicate.Id] = partUsages.GetValueOrDefault(item.Id, []).Select(usage => usage.Copy()).ToList();
        productVariants[duplicate.Id] = productVariants.GetValueOrDefault(item.Id, []).Select(CopyVariant).ToList();
        productColors[duplicate.Id] = productColors.GetValueOrDefault(item.Id, defaultColors).Select(color => color.Copy()).ToList();
        productHandles[duplicate.Id] = productHandles.GetValueOrDefault(item.Id, defaultHandles).Select(handle => handle.Copy()).ToList();
        RefreshVisibleFurniture();
        FeedbackMessage = "أُنشئت نسخة محلية ويمكن تعديلها الآن.";
        BeginEdit(duplicate);
        return duplicate;
    }

    public void ArchiveFurniture(FurnitureListItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        if (item.IsArchived)
        {
            return;
        }

        item.IsArchived = true;
        FeedbackMessage = "أُرشف المنتج في المعاينة المحلية.";
        RefreshVisibleFurniture();
    }

    public void ClearFeedback() => FeedbackMessage = string.Empty;

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

    private void SeedExistingProductDetails()
    {
        var wardrobe = furniture[0];
        partUsages[wardrobe.Id] =
        [
            new FurniturePartUsage(availableParts[0], 2m),
            new FurniturePartUsage(availableParts[1], 3m),
            new FurniturePartUsage(availableParts[2], 4m),
        ];
        productVariants[wardrobe.Id] =
        [
            new FurnitureVariant(Guid.NewGuid(), "صغير", 120m, 200m, 55m, 160_000m),
            new FurnitureVariant(Guid.NewGuid(), "متوسط", 160m, 220m, 60m, 195_000m),
            new FurnitureVariant(Guid.NewGuid(), "كبير", 200m, 240m, 65m, 235_000m),
        ];

        for (var index = 1; index < furniture.Count; index++)
        {
            var item = furniture[index];
            partUsages[item.Id] = [new FurniturePartUsage(availableParts[index % availableParts.Count], 2m)];
            productVariants[item.Id] = Enumerable.Range(1, item.VariantCount)
                .Select(number => new FurnitureVariant(Guid.NewGuid(), $"مقاس {number}", 100m + (number * 20m), 80m + (number * 15m), 50m, item.SellingPrice * 0.72m))
                .ToList();
        }

        foreach (var item in furniture)
        {
            productColors[item.Id] = defaultColors.Select(color => color.Copy()).ToList();
            productHandles[item.Id] = defaultHandles.Select(handle => handle.Copy()).ToList();
        }
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

    private void SelectedPartChanged(object? sender, PropertyChangedEventArgs eventArgs)
    {
        if (eventArgs.PropertyName is nameof(FurniturePartUsage.Quantity) or nameof(FurniturePartUsage.TotalCost))
        {
            RefreshPartsState();
        }
    }

    private void RefreshPartsState()
    {
        Raise(nameof(HasSelectedParts));
        Raise(nameof(CurrentPartsCost));
        Raise(nameof(CurrentPartsCostLabel));
    }

    private bool TryCalculatePartsCost(out decimal total)
    {
        total = 0m;
        try
        {
            foreach (var usage in SelectedParts)
            {
                if (!usage.TryCalculateTotalCost(out var rowTotal))
                {
                    total = 0m;
                    return false;
                }

                total = checked(total + rowTotal);
            }

            return true;
        }
        catch (OverflowException)
        {
            total = 0m;
            return false;
        }
    }

    private decimal CalculateVariantPreviewCost(decimal width, decimal height, decimal depth)
    {
        var baseCost = Math.Max(CurrentPartsCost, 50_000m);
        var referenceVolume = 120m * 200m * 55m;
        var volumeRatio = decimal.Clamp((width * height * depth) / referenceVolume, 0.5m, 2.5m);
        return decimal.Round(baseCost * (0.72m + (volumeRatio * 0.28m)) / 1_000m, 0, MidpointRounding.AwayFromZero) * 1_000m;
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

    private static FurnitureVariant CopyVariant(FurnitureVariant variant) =>
        new(variant.Id, variant.Name, variant.Width, variant.Height, variant.Depth, variant.CalculatedCost);

    private void RefreshPartOptions()
    {
        var search = NormalizeSearchText(PartSearchText.Trim());
        var selectedIds = SelectedParts.Select(item => item.Part.Id).ToHashSet();
        FilteredParts.Clear();
        foreach (var part in availableParts.Where(item =>
                     !selectedIds.Contains(item.Id)
                     && (search.Length == 0
                         || NormalizeSearchText(item.Name).Contains(search, StringComparison.CurrentCultureIgnoreCase)
                         || NormalizeSearchText(item.Category).Contains(search, StringComparison.CurrentCultureIgnoreCase))))
        {
            FilteredParts.Add(part);
        }

        Raise(nameof(HasNoPartOptions));
    }

    private void RefreshVisibleFurniture()
    {
        var search = NormalizeSearchText(SearchText.Trim());
        var matches = furniture.Where(item =>
            (search.Length == 0
             || NormalizeSearchText(item.Name).Contains(search, StringComparison.CurrentCultureIgnoreCase)
             || NormalizeSearchText(item.Category).Contains(search, StringComparison.CurrentCultureIgnoreCase))
            && (SelectedCategory == AllCategories || item.Category == SelectedCategory)
            && (SelectedStatus == AllStatuses
                || (SelectedStatus == ActiveStatus && !item.IsArchived)
                || (SelectedStatus == ArchivedStatus && item.IsArchived)));

        VisibleFurniture.Clear();
        foreach (var item in matches)
        {
            VisibleFurniture.Add(item);
        }

        Raise(nameof(HasNoVisibleFurniture));
        Raise(nameof(VisibleCountLabel));
    }

    private static string NormalizeSearchText(string value)
    {
        var decomposed = value.Normalize(NormalizationForm.FormD);
        var normalized = new StringBuilder(decomposed.Length);
        foreach (var character in decomposed)
        {
            var category = CharUnicodeInfo.GetUnicodeCategory(character);
            if (character == '\u0640'
                || category is UnicodeCategory.NonSpacingMark
                    or UnicodeCategory.SpacingCombiningMark
                    or UnicodeCategory.EnclosingMark)
            {
                continue;
            }

            normalized.Append(character switch
            {
                '\u0622' or '\u0623' or '\u0625' or '\u0671' => '\u0627',
                '\u0649' => '\u064A',
                '\u0629' => '\u0647',
                _ => character,
            });
        }

        return normalized.ToString();
    }

    private bool Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
        {
            return false;
        }

        field = value;
        Raise(propertyName);
        return true;
    }

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}
