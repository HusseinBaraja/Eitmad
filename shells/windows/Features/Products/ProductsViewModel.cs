using Eitmad.Contracts;
using System.Collections.ObjectModel;
using System.Globalization;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Features.Products;

/// <summary>
/// Projects Rust ready-made definitions and holds unsaved editor fields.
/// Durable validation, authorization, audit, storage, and synchronization remain Rust responsibilities.
/// </summary>
public sealed class ProductsViewModel : ObservableObject
{
    public const string AllCategories = "كل الفئات";
    public const string AllStatuses = "كل الحالات";
    public const string ActiveStatus = "نشط";
    public const string ArchivedStatus = "مؤرشف";

    private readonly List<ProductListItem> products = [];
    private readonly Dictionary<Guid, Product> records = [];
    public event EventHandler? SearchChanged;
    private Guid singleVariantId = Guid.NewGuid();
    private bool isBusy, savePending;
    public bool IsBusy { get => isBusy; set { Set(ref isBusy, value); Raise(nameof(CanInteract)); Raise(nameof(CanEdit)); } }
    public bool SavePending { get => savePending; set { Set(ref savePending, value); Raise(nameof(CanEdit)); Raise(nameof(CanInteract)); } }
    private bool canManage, canReadCosts;
    public bool CanManage { get => canManage; private set { Set(ref canManage, value); Raise(nameof(CanEdit)); } }
    public bool CanReadCosts { get => canReadCosts; private set => Set(ref canReadCosts, value); }
    public bool CanInteract => !IsBusy;
    public bool CanEdit => !IsBusy && !SavePending && CanManage;
    private string availabilityMessage = "جار تحميل المنتجات…";
    public string AvailabilityMessage { get => availabilityMessage; private set { Set(ref availabilityMessage, value); Raise(nameof(PageSubtitle)); } }
    public string PageSubtitle => AvailabilityMessage.Length > 0 ? AvailabilityMessage : "المنتجات محفوظة محلياً — سعر البيع من التسعير";
    private readonly Dictionary<Guid, ProductDraftDetails> details = [];
    private ProductListItem? editingProduct;
    private Product? editingRecord, pendingArchiveRecord;
    private Guid? editorCategoryId;
    public Guid? EditorCategoryId { get => editorCategoryId; set { Set(ref editorCategoryId, value); var c = Categories.FirstOrDefault(c => c.Id == value); if (c is not null) { editorCategory = c.Name; Raise(nameof(EditorCategory)); } } }
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
        Categories = []; ActiveCategories = []; CategoryOptions = [AllCategories];
        StatusOptions = [AllStatuses, ActiveStatus, ArchivedStatus]; VisibleProducts = []; Variants = [];
    }

    /// <summary>Projects generated Rust records without changing an open unsaved editor.</summary>
    public void ApplyDurableData(ProductSnapshot data)
    {
        if (!data.CanManage || !data.CanReadCosts) { IsEditorOpen = false; IsCategoryEditorOpen = false; IsCategoryManagerOpen = false; IsArchiveConfirmationOpen = false; editingRecord = null; pendingArchiveRecord = null; Notes = ""; PurchaseCost = 0; Variants.Clear(); SavePending = false; }
        CanManage = data.CanManage; CanReadCosts = data.CanReadCosts;
        var selectedCategoryId = editorCategoryId;
        var selectedFilter = SelectedCategory;
        records.Clear(); products.Clear(); details.Clear(); Categories.Clear(); ActiveCategories.Clear();
        CategoryOptions = new[] { AllCategories }.Concat(data.Categories.Items.Select(c => c.Name)).ToArray();
        Raise(nameof(CategoryOptions));
        foreach (var c in data.Categories.Items)
        {
            var option = new ProductCategoryOption(c.Name) { Id = c.Id, Revision = c.Revision, IsArchived = c.Archived };
            Categories.Add(option); if (!c.Archived) ActiveCategories.Add(option);
        }
        foreach (var p in data.Products)
        {
            records[p.Id] = p;
            var active = p.Variants.Where(v => !v.Archived).ToArray(); var primary = active.FirstOrDefault() ?? p.Variants.FirstOrDefault();
            products.Add(new ProductListItem(p.Id, p.Name, Categories.FirstOrDefault(c => c.Id == p.CategoryId)?.Name ?? p.CategoryName, primary?.PurchaseCostYer ?? 0, string.Join("، ", active.Select(v => v.Name)), ThumbnailForCategory(p.CategoryName), isArchived: p.Archived) { HasPurchaseCost = primary?.PurchaseCostYer is not null });
            details[p.Id] = new(p.Description, p.Notes, null, "", p.Variants.Select(v => new ProductVariant(v.Id, v.Name, v.PurchaseCostYer ?? 0) { IsArchived = v.Archived }).ToArray());
        }
        if (editingRecord is { } editing && Categories.FirstOrDefault(c => c.Id == editing.CategoryId) is { } retained && !ActiveCategories.Contains(retained)) ActiveCategories.Add(retained);
        EditorCategoryId = selectedCategoryId;
        SelectedCategory = CategoryOptions.Contains(selectedFilter) ? selectedFilter : AllCategories; Raise(nameof(SelectedCategory));
        AvailabilityMessage = ""; RefreshVisibleProducts();
    }
    public void Unavailable(string message) { AvailabilityMessage = message; products.Clear(); RefreshVisibleProducts(); }
    public void ClearSession()
    {
        SavePending = false; IsBusy = false; IsEditorOpen = false; IsCategoryEditorOpen = false; IsCategoryManagerOpen = false; IsArchiveConfirmationOpen = false;
        editingRecord = null; pendingArchiveRecord = null; editorCategoryId = null; editingProduct = null; pendingArchiveProduct = null; editingCategory = null; EditorName = ""; EditorCategory = ""; ShortDescription = ""; Notes = ""; PurchaseCost = 0; Variants.Clear(); ProductImage = null; ProductImageName = ""; CategoryName = ""; CategoryError = ""; EditorError = ""; FeedbackMessage = "";
        searchText = ""; Raise(nameof(SearchText)); SelectedCategory = AllCategories; SelectedStatus = AllStatuses; CategoryOptions = [AllCategories]; Raise(nameof(CategoryOptions)); CanManage = false; CanReadCosts = false; records.Clear(); details.Clear(); Categories.Clear(); ActiveCategories.Clear(); Unavailable("جار تحميل المنتجات…");
    }
    public void Fail(string message) => EditorError = message;
    public void FailCategory(string message) => CategoryError = message;
    public void Saved() { IsEditorOpen = false; IsArchiveConfirmationOpen = false; pendingArchiveProduct = null; SavePending = false; EditorError = ""; FeedbackMessage = "حُفظ المنتج محلياً مع سجل التدقيق."; }
    public void CategorySaved(string name) { IsCategoryEditorOpen = false; CategoryError = ""; EditorCategory = name; if (returnToCategoryManager) IsCategoryManagerOpen = true; }

    public ObservableCollection<ProductListItem> VisibleProducts { get; }

    // Selling-price publication and sales selection remain owned by pricing/catalog.
    public IEnumerable<Reception.SalesCatalogItem> GetSalesCatalogItems() => [];
    public Reception.ProductSelectionViewModel? GetSalesSelection(Guid id) => null;

    public ObservableCollection<ProductVariant> Variants { get; }

    public ObservableCollection<ProductCategoryOption> Categories { get; }

    public ObservableCollection<ProductCategoryOption> ActiveCategories { get; }

    public IReadOnlyList<string> CategoryOptions { get; private set; }

    public IReadOnlyList<string> StatusOptions { get; }

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

    public string EditorCategory { get => editorCategory; set { Set(ref editorCategory, value ?? string.Empty); EditorCategoryId = Categories.FirstOrDefault(c => c.Name == value)?.Id ?? editorCategoryId; } }

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
            }
        }
    }

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
        if (SavePending || !CanManage) return;
        singleVariantId = Guid.NewGuid();
        editingRecord = null; editorCategoryId = null;
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
        Variants.Clear();
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    public void BeginEdit(ProductListItem product)
    {
        ArgumentNullException.ThrowIfNull(product);
        if (SavePending || !CanManage || !records.ContainsKey(product.Id)) return;
        singleVariantId = records[product.Id].Variants.FirstOrDefault()?.Id ?? Guid.NewGuid();
        editingProduct = product;
        editingRecord = records[product.Id];
        var retained = Categories.FirstOrDefault(c => c.Id == editingRecord.CategoryId);
        if (retained is not null && !ActiveCategories.Contains(retained)) ActiveCategories.Add(retained);
        EditorCategoryId = editingRecord.CategoryId;
        IsCreating = false;
        EditorName = product.Name;
        EditorCategory = product.Category;
        PurchaseCost = product.PurchaseCost;
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

        HasVariants = Variants.Count != 1 || Variants[0].Name != "قياسي";
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    public void BeginDuplicate(ProductListItem product)
    {
        if (SavePending || !CanManage) return;
        BeginEdit(product);
        var copies = Variants.Where(v => !v.IsArchived).Select(v => new ProductVariant(Guid.NewGuid(), v.Name, v.PurchaseCost)).ToArray();
        Variants.Clear(); foreach (var v in copies) Variants.Add(v); singleVariantId = Guid.NewGuid();
        editingRecord = null; editorCategoryId = null;
        editingProduct = null;
        IsCreating = true;
        EditorCategoryId = Categories.FirstOrDefault(c => c.Name == EditorCategory)?.Id;
        EditorName = $"{product.Name} — نسخة";
    }

    public void CancelEditor()
    {
        if (SavePending) return;
        IsEditorOpen = false;
        EditorError = string.Empty;
    }

    public void AddVariant()
    {
        HasVariants = true;
        Variants.Add(new ProductVariant(Guid.NewGuid(), $"خيار {Variants.Count + 1}", PurchaseCost));
    }

    public void RemoveVariant(ProductVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        if (editingProduct is not null && editingRecord!.Variants.Any(v => v.Id == variant.Id)) variant.IsArchived = true;
        else Variants.Remove(variant);
    }

    public SaveProduct SaveInput()
    {
        var current = editingRecord;
        var category = Categories.FirstOrDefault(c => c.Name == EditorCategory);
        var variants = HasVariants ? Variants.Select(v => new SaveProductVariant { Id = v.Id, Name = v.Name, PurchaseCostYer = WholeCost(v.PurchaseCost), Archived = v.IsArchived }).ToArray()
            : new[] { new SaveProductVariant { Id = singleVariantId, Name = "قياسي", PurchaseCostYer = WholeCost(PurchaseCost) } };
        return new SaveProduct { Id = current?.Id, ExpectedRevision = current?.Revision, Name = EditorName, CategoryId = editorCategoryId ?? category?.Id ?? Guid.Empty, Description = ShortDescription, Notes = Notes, Variants = variants, Archived = current?.Archived ?? false };
    }
    private static long WholeCost(decimal value) => value == decimal.Truncate(value) ? checked((long)value) : throw new FormatException("whole YER required");
    public SaveProduct ArchiveInput()
    {
        var p = pendingArchiveRecord!;
        return new SaveProduct
        {
            Id = p.Id,
            ExpectedRevision = p.Revision,
            Name = p.Name,
            CategoryId = p.CategoryId,
            Description = p.Description,
            Notes = p.Notes,
            Archived = true,
            Variants = p.Variants.Select(v => new SaveProductVariant { Id = v.Id, Name = v.Name, PurchaseCostYer = v.PurchaseCostYer ?? 0, Archived = v.Archived }).ToArray()
        };
    }
    public void RequestArchive(ProductListItem product)
    {
        ArgumentNullException.ThrowIfNull(product);
        if (!product.CanArchive)
        {
            return;
        }

        if (SavePending || !records.TryGetValue(product.Id, out var record)) return;
        pendingArchiveRecord = editingProduct?.Id == product.Id ? editingRecord : record;
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
        if (SavePending) return;
        pendingArchiveProduct = null;
        IsArchiveConfirmationOpen = false;
        Raise(nameof(ArchiveConfirmationTitle));
    }

    public void ClearFeedback() => FeedbackMessage = string.Empty;

    public void SetProductImage(ImageSource image, string fileName)
    {
        ArgumentNullException.ThrowIfNull(image);
        ProductImage = image;
        ProductImageName = fileName;
        EditorError = string.Empty;
    }

    public void ReportImageLoadError() =>
        FeedbackMessage = "تعذر فتح الصورة. اختر ملف صورة صالحاً بحجم مناسب.";

    public void BeginAddCategory()
    {
        if (SavePending || !CanManage) return;
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
        if (SavePending || !CanManage) return;
        IsCategoryEditorOpen = false;
        IsCategoryManagerOpen = true;
    }

    public void BeginEditCategory(ProductCategoryOption category)
    {
        if (SavePending || !CanManage) return;
        ArgumentNullException.ThrowIfNull(category);
        editingCategory = category;
        returnToCategoryManager = IsCategoryManagerOpen;
        CategoryName = category.Name;
        CategoryError = string.Empty;
        IsCategoryManagerOpen = false;
        IsCategoryEditorOpen = true;
        Raise(nameof(CategoryEditorTitle));
    }

    public SaveProductCategory CategoryInput() => new() { Id = editingCategory?.Id, ExpectedRevision = editingCategory?.Revision, Name = CategoryName, Archived = editingCategory?.IsArchived ?? false };
    public SaveProductCategory ArchiveCategoryInput(ProductCategoryOption category) => new() { Id = category.Id, ExpectedRevision = category.Revision, Name = category.Name, Archived = true };

    public void CancelCategoryEditor()
    {
        if (SavePending) return;
        IsCategoryEditorOpen = false;
        CategoryError = string.Empty;
        if (returnToCategoryManager)
        {
            IsCategoryManagerOpen = true;
        }
    }

    public void CloseCategoryManager() { if (!SavePending) IsCategoryManagerOpen = false; }

    private void RefreshVisibleProducts()
    {
        var matches = products.Where(product =>
             (SelectedCategory == AllCategories || product.Category == SelectedCategory)
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

    private static string ThumbnailForCategory(string category) => category switch
    {
        "المراتب" => "Mattress",
        "الوسائد" => "Pillow",
        "الإضاءة" => "Lamp",
        _ => "Vase",
    };
}
