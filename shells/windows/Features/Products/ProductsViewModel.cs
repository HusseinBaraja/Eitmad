using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Features.Products;

/// <summary>
/// Owns transient ready-made product list and editor state for the Windows preview.
/// Durable validation, authorization, audit, storage, and synchronization remain Rust responsibilities.
/// </summary>
public sealed class ProductsViewModel : INotifyPropertyChanged
{
    public const string AllCategories = "كل الفئات";
    public const string AllStatuses = "كل الحالات";
    public const string ActiveStatus = "نشط";
    public const string ArchivedStatus = "مؤرشف";

    private readonly List<ProductListItem> products;
    private readonly Dictionary<Guid, ProductDraftDetails> details = [];
    private ProductListItem? editingProduct;
    private ProductListItem? pendingArchiveProduct;
    private ProductCategoryOption? editingCategory;
    private string searchText = string.Empty;
    private string selectedCategory = AllCategories;
    private string selectedStatus = AllStatuses;
    private bool isEditorOpen;
    private bool isCreating;
    private string editorName = string.Empty;
    private string editorCategory = "المراتب";
    private string shortDescription = string.Empty;
    private string notes = string.Empty;
    private ImageSource? productImage;
    private string productImageName = string.Empty;
    private bool hasVariants;
    private decimal purchaseCost;
    private decimal sellingPrice;
    private string editorError = string.Empty;
    private string feedbackMessage = string.Empty;
    private bool isArchiveConfirmationOpen;
    private bool isCategoryEditorOpen;
    private bool isCategoryManagerOpen;
    private bool returnToCategoryManager;
    private string categoryName = string.Empty;
    private string categoryError = string.Empty;

    public ProductsViewModel()
    {
        products =
        [
            new(Guid.Parse("5e084704-f318-4563-9f23-22fc0a7cbe61"), "مرتبة طبية", "المراتب", 55_000m, 75_000m, "مفرد +2", "Mattress"),
            new(Guid.Parse("9b5b2782-e342-45d3-971e-b10e22870c25"), "وسادة فندقية", "الوسائد", 8_000m, 12_000m, "قياسي", "Pillow"),
            new(Guid.Parse("bd408138-8410-471f-8de9-fb33c7a0ff18"), "مزهرية رملية", "الديكور", 14_000m, 22_000m, "بدون خيارات", "Vase"),
            new(Guid.Parse("02b38a86-778a-4d34-bf74-3ca79bf10b24"), "مصباح قراءة", "الإضاءة", 21_000m, 31_000m, "أسود", "Lamp", isArchived: true),
        ];

        Categories =
        [
            new("المراتب"),
            new("الوسائد"),
            new("الديكور"),
            new("الإضاءة"),
            new("الإكسسوارات"),
        ];
        ActiveCategories = new ObservableCollection<ProductCategoryOption>(Categories);
        CategoryOptions = [AllCategories, .. Categories.Select(category => category.Name)];
        StatusOptions = [AllStatuses, ActiveStatus, ArchivedStatus];
        VisibleProducts = [];
        Variants = [];

        details[products[0].Id] = new(
            "مرتبة جاهزة بدعم طبي وطبقة علوية مريحة.",
            "متوفرة من المورد خلال يومي عمل.",
            null,
            string.Empty,
            [
                new(Guid.NewGuid(), "مفرد", 55_000m, 75_000m),
                new(Guid.NewGuid(), "مزدوج", 80_000m, 105_000m),
                new(Guid.NewGuid(), "كينغ", 105_000m, 135_000m),
            ]);
        details[products[1].Id] = new("وسادة جاهزة بحشوة ناعمة.", string.Empty, null, string.Empty, []);
        details[products[2].Id] = new("قطعة ديكور جاهزة بلون رملي محايد.", string.Empty, null, string.Empty, []);
        details[products[3].Id] = new("مصباح قراءة معدني جاهز.", string.Empty, null, string.Empty, []);
        RefreshVisibleProducts();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public ObservableCollection<ProductListItem> VisibleProducts { get; }

    public ObservableCollection<ProductVariant> Variants { get; }

    public ObservableCollection<ProductCategoryOption> Categories { get; }

    public ObservableCollection<ProductCategoryOption> ActiveCategories { get; }

    public ObservableCollection<string> CategoryOptions { get; }

    public IReadOnlyList<string> StatusOptions { get; }

    public string SearchText
    {
        get => searchText;
        set
        {
            if (Set(ref searchText, value ?? string.Empty))
            {
                RefreshVisibleProducts();
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
                RefreshVisibleProducts();
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
                RefreshVisibleProducts();
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
                Raise(nameof(CanArchiveFromEditor));
            }
        }
    }

    public string EditorTitle => IsCreating ? "إضافة منتج" : "تعديل المنتج";

    public bool CanArchiveFromEditor => !IsCreating && editingProduct?.CanArchive == true;

    public string EditorName { get => editorName; set => Set(ref editorName, value ?? string.Empty); }

    public string EditorCategory { get => editorCategory; set => Set(ref editorCategory, value ?? string.Empty); }

    public string ShortDescription { get => shortDescription; set => Set(ref shortDescription, value ?? string.Empty); }

    public string Notes { get => notes; set => Set(ref notes, value ?? string.Empty); }

    public ImageSource? ProductImage
    {
        get => productImage;
        set
        {
            if (Set(ref productImage, value))
            {
                Raise(nameof(HasProductImage));
            }
        }
    }

    public bool HasProductImage => ProductImage is not null;

    public string ProductImageName { get => productImageName; set => Set(ref productImageName, value ?? string.Empty); }

    public bool HasVariants
    {
        get => hasVariants;
        set
        {
            if (Set(ref hasVariants, value))
            {
                Raise(nameof(HasNoVariants));
            }
        }
    }

    public bool HasNoVariants
    {
        get => !HasVariants;
        set
        {
            if (value)
            {
                HasVariants = false;
            }
        }
    }

    public decimal PurchaseCost
    {
        get => purchaseCost;
        set
        {
            if (Set(ref purchaseCost, value))
            {
                Raise(nameof(Margin));
                Raise(nameof(MarginLabel));
            }
        }
    }

    public decimal SellingPrice
    {
        get => sellingPrice;
        set
        {
            if (Set(ref sellingPrice, value))
            {
                Raise(nameof(Margin));
                Raise(nameof(MarginLabel));
            }
        }
    }

    public decimal Margin => SellingPrice - PurchaseCost;

    public string MarginLabel => Margin.ToString("N0", CultureInfo.InvariantCulture);

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

    public bool HasNoVisibleProducts => VisibleProducts.Count == 0;

    public string VisibleCountLabel => $"{VisibleProducts.Count} من {products.Count} منتجات";

    public bool IsArchiveConfirmationOpen
    {
        get => isArchiveConfirmationOpen;
        private set => Set(ref isArchiveConfirmationOpen, value);
    }

    public string ArchiveConfirmationTitle => pendingArchiveProduct is null
        ? "أرشفة المنتج"
        : $"أرشفة «{pendingArchiveProduct.Name}»؟";

    public bool IsCategoryEditorOpen
    {
        get => isCategoryEditorOpen;
        private set => Set(ref isCategoryEditorOpen, value);
    }

    public bool IsCategoryManagerOpen
    {
        get => isCategoryManagerOpen;
        private set => Set(ref isCategoryManagerOpen, value);
    }

    public string CategoryEditorTitle => editingCategory is null ? "إضافة فئة جديدة" : "تعديل الفئة";

    public string CategoryName { get => categoryName; set => Set(ref categoryName, value ?? string.Empty); }

    public string CategoryError
    {
        get => categoryError;
        private set
        {
            if (Set(ref categoryError, value))
            {
                Raise(nameof(HasCategoryError));
            }
        }
    }

    public bool HasCategoryError => !string.IsNullOrEmpty(CategoryError);

    public void BeginCreate()
    {
        editingProduct = null;
        IsCreating = true;
        EditorName = string.Empty;
        EditorCategory = ActiveCategories.FirstOrDefault()?.Name ?? string.Empty;
        ShortDescription = string.Empty;
        Notes = string.Empty;
        ProductImage = null;
        ProductImageName = string.Empty;
        HasVariants = false;
        PurchaseCost = 0m;
        SellingPrice = 0m;
        Variants.Clear();
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    public void BeginEdit(ProductListItem product)
    {
        ArgumentNullException.ThrowIfNull(product);
        editingProduct = product;
        IsCreating = false;
        EditorName = product.Name;
        EditorCategory = product.Category;
        PurchaseCost = product.PurchaseCost;
        SellingPrice = product.SellingPrice;
        var productDetails = details[product.Id];
        ShortDescription = productDetails.Description;
        Notes = productDetails.Notes;
        ProductImage = productDetails.Image;
        ProductImageName = productDetails.ImageName;
        Variants.Clear();
        foreach (var variant in productDetails.Variants)
        {
            Variants.Add(variant.Copy());
        }

        HasVariants = Variants.Count > 0;
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    public void BeginDuplicate(ProductListItem product)
    {
        BeginEdit(product);
        editingProduct = null;
        IsCreating = true;
        EditorName = $"{product.Name} — نسخة";
    }

    public void CancelEditor()
    {
        IsEditorOpen = false;
        EditorError = string.Empty;
    }

    public void AddVariant()
    {
        HasVariants = true;
        Variants.Add(new ProductVariant(Guid.NewGuid(), $"خيار {Variants.Count + 1}", PurchaseCost, SellingPrice));
    }

    public void RemoveVariant(ProductVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        Variants.Remove(variant);
    }

    public bool SaveEditor()
    {
        var normalizedName = EditorName.Trim();
        if (normalizedName.Length == 0)
        {
            EditorError = "أدخل اسم المنتج.";
            return false;
        }

        if (!ActiveCategories.Any(category => category.Name == EditorCategory))
        {
            EditorError = "اختر فئة نشطة للمنتج.";
            return false;
        }

        if (HasVariants)
        {
            if (Variants.Count == 0)
            {
                EditorError = "أضف خياراً واحداً على الأقل أو اختر «لا».";
                return false;
            }

            if (Variants.Any(variant => string.IsNullOrWhiteSpace(variant.Name)))
            {
                EditorError = "أدخل اسماً لكل خيار.";
                return false;
            }

            if (Variants.Any(variant => variant.PurchaseCost < 0m || variant.SellingPrice < 0m))
            {
                EditorError = "يجب ألا تكون أسعار الخيارات سالبة.";
                return false;
            }
        }
        else if (PurchaseCost < 0m || SellingPrice < 0m)
        {
            EditorError = "يجب ألا تكون الأسعار سالبة.";
            return false;
        }

        var primaryVariant = HasVariants ? Variants[0] : null;
        var rowPurchaseCost = primaryVariant?.PurchaseCost ?? PurchaseCost;
        var rowSellingPrice = primaryVariant?.SellingPrice ?? SellingPrice;
        var variantSummary = HasVariants
            ? Variants.Count == 1 ? Variants[0].Name.Trim() : $"{Variants[0].Name.Trim()} +{Variants.Count - 1}"
            : "بدون خيارات";

        if (editingProduct is null)
        {
            editingProduct = new ProductListItem(
                Guid.NewGuid(),
                normalizedName,
                EditorCategory,
                rowPurchaseCost,
                rowSellingPrice,
                variantSummary,
                ThumbnailForCategory(EditorCategory),
                ProductImage);
            products.Add(editingProduct);
            FeedbackMessage = "أضيف المنتج إلى المعاينة المحلية فقط.";
        }
        else
        {
            editingProduct.Name = normalizedName;
            editingProduct.Category = EditorCategory;
            editingProduct.PurchaseCost = rowPurchaseCost;
            editingProduct.SellingPrice = rowSellingPrice;
            editingProduct.VariantSummary = variantSummary;
            editingProduct.Image = ProductImage;
            FeedbackMessage = "حُدث المنتج في المعاينة المحلية فقط.";
        }

        details[editingProduct.Id] = new(
            ShortDescription.Trim(),
            Notes.Trim(),
            ProductImage,
            ProductImageName,
            HasVariants ? Variants.Select(variant => variant.Copy()).ToList() : []);
        IsEditorOpen = false;
        EditorError = string.Empty;
        RefreshVisibleProducts();
        return true;
    }

    public void RequestArchive(ProductListItem product)
    {
        ArgumentNullException.ThrowIfNull(product);
        if (!product.CanArchive)
        {
            return;
        }

        pendingArchiveProduct = product;
        Raise(nameof(ArchiveConfirmationTitle));
        IsArchiveConfirmationOpen = true;
    }

    public void RequestArchiveFromEditor()
    {
        if (editingProduct is not null)
        {
            RequestArchive(editingProduct);
        }
    }

    public void CancelArchive()
    {
        pendingArchiveProduct = null;
        IsArchiveConfirmationOpen = false;
        Raise(nameof(ArchiveConfirmationTitle));
    }

    public void ConfirmArchive()
    {
        if (pendingArchiveProduct is null)
        {
            return;
        }

        pendingArchiveProduct.IsArchived = true;
        FeedbackMessage = "أُرشف المنتج في المعاينة المحلية فقط.";
        pendingArchiveProduct = null;
        IsArchiveConfirmationOpen = false;
        IsEditorOpen = false;
        Raise(nameof(ArchiveConfirmationTitle));
        RefreshVisibleProducts();
    }

    public void ClearFeedback() => FeedbackMessage = string.Empty;

    public void BeginAddCategory()
    {
        editingCategory = null;
        returnToCategoryManager = false;
        CategoryName = string.Empty;
        CategoryError = string.Empty;
        IsCategoryManagerOpen = false;
        IsCategoryEditorOpen = true;
        Raise(nameof(CategoryEditorTitle));
    }

    public void BeginManageCategories()
    {
        IsCategoryEditorOpen = false;
        IsCategoryManagerOpen = true;
    }

    public void BeginEditCategory(ProductCategoryOption category)
    {
        ArgumentNullException.ThrowIfNull(category);
        editingCategory = category;
        returnToCategoryManager = IsCategoryManagerOpen;
        CategoryName = category.Name;
        CategoryError = string.Empty;
        IsCategoryManagerOpen = false;
        IsCategoryEditorOpen = true;
        Raise(nameof(CategoryEditorTitle));
    }

    public bool SaveCategory()
    {
        var normalizedName = CategoryName.Trim();
        if (normalizedName.Length == 0)
        {
            CategoryError = "أدخل اسم الفئة.";
            return false;
        }

        if (Categories.Any(category => category != editingCategory
            && string.Equals(category.Name, normalizedName, StringComparison.CurrentCultureIgnoreCase)))
        {
            CategoryError = "اسم الفئة مستخدم بالفعل.";
            return false;
        }

        if (editingCategory is null)
        {
            var added = new ProductCategoryOption(normalizedName);
            Categories.Add(added);
            ActiveCategories.Add(added);
            CategoryOptions.Add(normalizedName);
            EditorCategory = normalizedName;
        }
        else
        {
            var previousName = editingCategory.Name;
            editingCategory.Name = normalizedName;
            var filterIndex = CategoryOptions.IndexOf(previousName);
            if (filterIndex >= 0)
            {
                CategoryOptions[filterIndex] = normalizedName;
            }

            foreach (var product in products.Where(product => product.Category == previousName))
            {
                product.Category = normalizedName;
            }

            if (EditorCategory == previousName)
            {
                EditorCategory = normalizedName;
            }

            if (SelectedCategory == previousName)
            {
                SelectedCategory = normalizedName;
            }
        }

        CategoryError = string.Empty;
        IsCategoryEditorOpen = false;
        if (returnToCategoryManager)
        {
            IsCategoryManagerOpen = true;
        }

        RefreshVisibleProducts();
        return true;
    }

    public void CancelCategoryEditor()
    {
        IsCategoryEditorOpen = false;
        CategoryError = string.Empty;
        if (returnToCategoryManager)
        {
            IsCategoryManagerOpen = true;
        }
    }

    public void CloseCategoryManager() => IsCategoryManagerOpen = false;

    public void ArchiveCategory(ProductCategoryOption category)
    {
        ArgumentNullException.ThrowIfNull(category);
        if (category.IsArchived || ActiveCategories.Count == 1)
        {
            if (ActiveCategories.Count == 1)
            {
                CategoryError = "يجب إبقاء فئة نشطة واحدة على الأقل.";
            }

            return;
        }

        category.IsArchived = true;
        ActiveCategories.Remove(category);
        if (EditorCategory == category.Name)
        {
            EditorCategory = ActiveCategories[0].Name;
        }
    }

    private void RefreshVisibleProducts()
    {
        var normalizedSearch = NormalizeSearchText(SearchText.Trim());
        var matches = products.Where(product =>
            MatchesSearch(product, normalizedSearch)
            && (SelectedCategory == AllCategories || product.Category == SelectedCategory)
            && (SelectedStatus == AllStatuses
                || (SelectedStatus == ActiveStatus && !product.IsArchived)
                || (SelectedStatus == ArchivedStatus && product.IsArchived)));

        VisibleProducts.Clear();
        foreach (var product in matches)
        {
            VisibleProducts.Add(product);
        }

        Raise(nameof(HasNoVisibleProducts));
        Raise(nameof(VisibleCountLabel));
    }

    private static bool MatchesSearch(ProductListItem product, string search) =>
        search.Length == 0
        || NormalizeSearchText(product.Name).Contains(search, StringComparison.CurrentCultureIgnoreCase)
        || NormalizeSearchText(product.Category).Contains(search, StringComparison.CurrentCultureIgnoreCase)
        || NormalizeSearchText(product.VariantSummary).Contains(search, StringComparison.CurrentCultureIgnoreCase);

    private static string ThumbnailForCategory(string category) => category switch
    {
        "المراتب" => "Mattress",
        "الوسائد" => "Pillow",
        "الإضاءة" => "Lamp",
        _ => "Vase",
    };

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
