using Eitmad.Contracts;
using System.Collections.ObjectModel;
using System.Globalization;

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
    public long ImageEditVersion { get; private set; }
    public void InvalidateImageLoad() => ++ImageEditVersion;
    private CatalogImageRef? editorImageReference;
    public CatalogImageRef? EditorImageReference => editorImageReference;
    private System.Windows.Media.ImageSource? productImage;
    public System.Windows.Media.ImageSource? ProductImage { get => productImage; private set => Set(ref productImage,value); }
    public void SetImportedImage(CatalogImageRef? reference, System.Windows.Media.ImageSource? image) { editorImageReference = reference; ProductImage = image; }
    public void ApplyImage(Guid id, System.Windows.Media.ImageSource? image) { var row=products.FirstOrDefault(p=>p.Id==id); if(row is not null) row.Image=image; }
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

    /// <summary>Initializes empty authority projections and Arabic product filters before the first load.</summary>
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
        records.Clear(); products.Clear(); Categories.Clear(); ActiveCategories.Clear();
        CategoryOptions = new[] { AllCategories }.Concat(data.Categories.Items.Select(c => c.Name)).ToArray();
        Raise(nameof(CategoryOptions));
        foreach (var c in data.Categories.Items)
        {
            var option = new ProductCategoryOption(c.Id, c.Revision, c.Name, c.Archived);
            Categories.Add(option); if (!c.Archived) ActiveCategories.Add(option);
        }
        foreach (var p in data.Products)
        {
            records[p.Id] = p;
            var active = p.Variants.Where(v => !v.Archived).ToArray(); var primary = active.FirstOrDefault() ?? p.Variants.FirstOrDefault();
            products.Add(new ProductListItem(p.Id, p.Name, Categories.FirstOrDefault(c => c.Id == p.CategoryId)?.Name ?? p.CategoryName, primary?.PurchaseCostYer ?? 0, string.Join("، ", active.Select(v => v.Name)), ThumbnailForCategory(p.CategoryName), p.Archived, primary?.PurchaseCostYer is not null));
        }
        if (editingRecord is { } editing && Categories.FirstOrDefault(c => c.Id == editing.CategoryId) is { } retained && !ActiveCategories.Contains(retained)) ActiveCategories.Add(retained);
        EditorCategoryId = selectedCategoryId;
        SelectedCategory = CategoryOptions.Contains(selectedFilter) ? selectedFilter : AllCategories; Raise(nameof(SelectedCategory));
        AvailabilityMessage = ""; RefreshVisibleProducts();
    }
    /// <summary>Shows a load failure and clears visible rows while retaining unsaved edits for transport recovery.</summary>
    public void Unavailable(string message) { AvailabilityMessage = message; products.Clear(); RefreshVisibleProducts(); }
    /// <summary>Removes cached records, restricted fields, staged requests, and management flags after invalidation.</summary>
    public void ClearSession()
    {
        ++ImageEditVersion; SetImportedImage(null,null);
        SavePending = false; IsBusy = false; IsEditorOpen = false; IsCategoryEditorOpen = false; IsCategoryManagerOpen = false; IsArchiveConfirmationOpen = false;
        editingRecord = null; pendingArchiveRecord = null; editorCategoryId = null; editingProduct = null; pendingArchiveProduct = null; editingCategory = null; EditorName = ""; EditorCategory = ""; ShortDescription = ""; Notes = ""; PurchaseCost = 0; Variants.Clear(); CategoryName = ""; CategoryError = ""; EditorError = ""; FeedbackMessage = "";
        searchText = ""; Raise(nameof(SearchText)); SelectedCategory = AllCategories; SelectedStatus = AllStatuses; CategoryOptions = [AllCategories]; Raise(nameof(CategoryOptions)); CanManage = false; CanReadCosts = false; records.Clear(); Categories.Clear(); ActiveCategories.Clear(); Unavailable("جار تحميل المنتجات…");
    }
    /// <summary>Keeps the product editor open and displays the current save failure.</summary>
    public void Fail(string message) => EditorError = message;
    /// <summary>Keeps the category editor open and displays the current save failure.</summary>
    public void FailCategory(string message) => CategoryError = message;
    /// <summary>Closes product dialogs and clears pending-save state after Rust confirms the mutation.</summary>
    public void Saved() { IsEditorOpen = false; IsArchiveConfirmationOpen = false; pendingArchiveProduct = null; SavePending = false; EditorError = ""; FeedbackMessage = "حُفظ المنتج محلياً مع سجل التدقيق."; }
    /// <summary>Selects a newly created category; renaming or archiving keeps the product's selected ID.</summary>
    public void CategorySaved(SaveProductCategory input)
    {
        IsCategoryEditorOpen = false;
        CategoryError = "";
        if (input.Id is null) EditorCategory = input.Name;
        editingCategory = null;
        if (returnToCategoryManager) IsCategoryManagerOpen = true;
    }

    public ObservableCollection<ProductListItem> VisibleProducts { get; }

    // Selling-price publication and sales selection remain owned by pricing/catalog.
    /// <summary>Leaves new sales selections empty until Pricing publishes these definitions.</summary>
    public IEnumerable<Reception.SalesCatalogItem> GetSalesCatalogItems() => [];
    /// <summary>Withholds selection details for definitions that have no published selling price.</summary>
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

    /// <summary>Starts a new unsaved definition only when management is allowed and no exact retry is pending.</summary>
    public void BeginCreate()
    {
        if (SavePending || !CanManage) return;
        ++ImageEditVersion; SetImportedImage(null,null);
        singleVariantId = Guid.NewGuid();
        editingRecord = null; editorCategoryId = null;
        editingProduct = null;
        IsCreating = true;
        EditorName = string.Empty;
        EditorCategory = ActiveCategories.FirstOrDefault()?.Name ?? string.Empty;
        ShortDescription = string.Empty;
        Notes = string.Empty;
        HasVariants = false;
        PurchaseCost = 0m;
        Variants.Clear();
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    /// <summary>Stages the selected definition and retains its reviewed revision and supplier-option identities.</summary>
    public void BeginEdit(ProductListItem product)
    {
        ArgumentNullException.ThrowIfNull(product);
        if (SavePending || !CanManage || !records.ContainsKey(product.Id)) return;
        singleVariantId = records[product.Id].Variants.FirstOrDefault()?.Id ?? Guid.NewGuid();
        editingProduct = product;
        editingRecord = records[product.Id];
        ++ImageEditVersion; SetImportedImage(editingRecord.Image, product.Image);
        var retained = Categories.FirstOrDefault(c => c.Id == editingRecord.CategoryId);
        if (retained is not null && !ActiveCategories.Contains(retained)) ActiveCategories.Add(retained);
        EditorCategoryId = editingRecord.CategoryId;
        IsCreating = false;
        EditorName = product.Name;
        EditorCategory = product.Category;
        PurchaseCost = product.PurchaseCost;
        ShortDescription = editingRecord.Description;
        Notes = editingRecord.Notes;
        Variants.Clear();
        foreach (var variant in editingRecord.Variants)
        {
            Variants.Add(new ProductVariant(variant.Id, variant.Name, variant.PurchaseCostYer ?? 0) { IsArchived = variant.Archived });
        }

        HasVariants = Variants.Count != 1 || Variants[0].Name != "قياسي";
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    /// <summary>Copies active options into a new definition with new product and option identities.</summary>
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

    /// <summary>Closes unsaved editing only when no unknown save outcome requires an exact retry.</summary>
    public void CancelEditor()
    {
        ++ImageEditVersion;
        if (SavePending) return;
        IsEditorOpen = false;
        EditorError = string.Empty;
    }

    /// <summary>Stages a new supplier option with a fresh identity and the current purchase cost.</summary>
    public void AddVariant()
    {
        HasVariants = true;
        Variants.Add(new ProductVariant(Guid.NewGuid(), $"خيار {Variants.Count + 1}", PurchaseCost));
    }

    /// <summary>Archives a saved option to retain its references; removes only unsaved options.</summary>
    public void RemoveVariant(ProductVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        if (editingProduct is not null && editingRecord!.Variants.Any(v => v.Id == variant.Id)) variant.IsArchived = true;
        else Variants.Remove(variant);
    }

    /// <summary>Builds a typed request from staged fields using the reviewed revision and stable category identity.</summary>
    public SaveProduct SaveInput()
    {
        var current = editingRecord;
        var category = Categories.FirstOrDefault(c => c.Name == EditorCategory);
        var variants = HasVariants ? Variants.Select(v => new SaveProductVariant { Id = v.Id, Name = v.Name, PurchaseCostYer = WholeCost(v.PurchaseCost), Archived = v.IsArchived }).ToArray()
            : new[] { new SaveProductVariant { Id = singleVariantId, Name = "قياسي", PurchaseCostYer = WholeCost(PurchaseCost) } };
        return new SaveProduct { Image = editorImageReference!, Id = current?.Id, ExpectedRevision = current?.Revision, Name = EditorName, CategoryId = editorCategoryId ?? category?.Id ?? Guid.Empty, Description = ShortDescription, Notes = Notes, Variants = variants, Archived = current?.Archived ?? false };
    }
    /// <summary>Converts whole-YER input to the contract range and rejects fractions or integer overflow.</summary>
    private static long WholeCost(decimal value) => value == decimal.Truncate(value) ? checked((long)value) : throw new FormatException("whole YER required");
    /// <summary>Builds an archive request from the retained record without replacing its reviewed revision.</summary>
    public SaveProduct ArchiveInput()
    {
        var p = pendingArchiveRecord!;
        return new SaveProduct
        {
            Image = p.Image,
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
    /// <summary>Retains the record for archive confirmation without changing the authority projection.</summary>
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

    /// <summary>Dismisses archive confirmation only when no unknown outcome requires the retained request.</summary>
    public void CancelArchive()
    {
        if (SavePending) return;
        pendingArchiveProduct = null;
        IsArchiveConfirmationOpen = false;
        Raise(nameof(ArchiveConfirmationTitle));
    }

    public void ClearFeedback() => FeedbackMessage = string.Empty;

    /// <summary>Starts a new category draft without changing the open product selection.</summary>
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

    /// <summary>Opens category management only when editing is permitted and no retry is pending.</summary>
    public void BeginManageCategories()
    {
        if (SavePending || !CanManage) return;
        IsCategoryEditorOpen = false;
        IsCategoryManagerOpen = true;
    }

    /// <summary>Stages the category revision and remembers whether to return to category management.</summary>
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

    /// <summary>Builds a category request with its reviewed identity, revision, and archive state.</summary>
    public SaveProductCategory CategoryInput() => new() { Id = editingCategory?.Id, ExpectedRevision = editingCategory?.Revision, Name = CategoryName, Archived = editingCategory?.IsArchived ?? false };
    /// <summary>Builds an archive request for the selected category revision.</summary>
    public SaveProductCategory ArchiveCategoryInput(ProductCategoryOption category) => new() { Id = category.Id, ExpectedRevision = category.Revision, Name = category.Name, Archived = true };

    /// <summary>Returns to category management when appropriate while preserving an unresolved retry.</summary>
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

    /// <summary>Closes category management only after any unknown save outcome is resolved.</summary>
    public void CloseCategoryManager() { if (!SavePending) IsCategoryManagerOpen = false; }

    /// <summary>Filters the current authority projection by category and archive state for the visible list.</summary>
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
