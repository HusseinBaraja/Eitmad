using System.Collections.ObjectModel;
using System.Globalization;
using Eitmad.Contracts;
using Definition = Eitmad.Contracts.Furniture;
using DefinitionVariant = Eitmad.Contracts.FurnitureVariant;

namespace Eitmad.WindowsShell.Features.Furniture;

public sealed class FurnitureOptionChoice(Guid id, string name, bool selected) : ObservableObject
{
    private bool selected = selected;
    public Guid Id { get; } = id;
    public string Name { get; } = name;
    public bool Selected { get => selected; set => Set(ref selected, value); }
}

public sealed partial class FurnitureViewModel
{
    internal const string UnsavedCategoryMessage = "احفظ الفئة الجديدة أو اختر فئة موجودة قبل المتابعة.";
    internal sealed class UnsavedCategoryException : Exception;
    private readonly Dictionary<Guid, Definition> records = [];
    private FurnitureCategory[] categories = [];
    private IReadOnlyList<PartCategory> partCategories = [];
    private readonly Dictionary<(Guid, long), Part> compositions = [];
    private Definition? editingRecord;
    private long? editingExpectedRevision;
    private SaveFurniture? pendingSave;
    private decimal reviewedCost;
    private string dataStateText="";
    private bool isLoading;
    public string DataStateText { get=>dataStateText; private set=>Set(ref dataStateText,value); }
    public bool IsLoading { get=>isLoading; set { Set(ref isLoading,value); Raise(nameof(HasNoVisibleFurniture)); if(value) DataStateText="جار تحميل الأثاث..."; } }
    private bool canManage, isBusy, confirmBelowCost;
    public bool CanManage { get => canManage; private set { Set(ref canManage, value); Raise(nameof(CanEditFields)); Raise(nameof(CanSubmit)); } }
    public bool IsBusy { get => isBusy; set { Set(ref isBusy, value); Raise(nameof(CanEditFields)); Raise(nameof(CanSubmit)); } }
    public bool CanSubmit => CanManage && !IsBusy;
    public bool CanEditFields => CanManage && !IsBusy && pendingSave is null;
    public bool ConfirmBelowCost { get => confirmBelowCost; set => Set(ref confirmBelowCost, value); }
    private bool allowCustomization;
    public bool AllowCustomization { get => allowCustomization; set => Set(ref allowCustomization,value); }
    public decimal MinWidth { get; set; } = 120m;
    public decimal MinHeight { get; set; } = 200m;
    public decimal MinDepth { get; set; } = 55m;
    public decimal MaxWidth { get; set; } = 120m;
    public decimal MaxHeight { get; set; } = 200m;
    public decimal MaxDepth { get; set; } = 55m;
    public ObservableCollection<FurnitureOptionChoice> VariantColorChoices { get; } = [];
    public ObservableCollection<FurnitureOptionChoice> VariantHandleChoices { get; } = [];
    public event EventHandler? SearchChanged;
    public event EventHandler? PartSearchChanged;
    public event EventHandler? ReviewRequested;

    /// <summary>Replaces the scoped list projection while preserving unsaved editor fields and the opened revision.</summary>
    public void ApplyDurableData(FurnitureSnapshot snapshot)
    {
        IsLoading=false; DataStateText=""; CanManage = snapshot.CanManage;
        categories = snapshot.Categories.Items; partCategories = snapshot.PartCategories;
        var stagedCategory = EditorCategory;
        EditorCategoryOptions.Clear(); CategoryOptions.Clear(); CategoryOptions.Add(AllCategories);
        foreach (var c in categories.Where(c => !c.Archived)) { EditorCategoryOptions.Add(c.Name); CategoryOptions.Add(c.Name); }
        if (IsEditorOpen) EditorCategory = stagedCategory;
        records.Clear(); furniture.Clear(); availableParts.Clear(); compositions.Clear();
        foreach (var part in snapshot.Parts.Concat(snapshot.Compositions)) compositions[(part.Id, part.Revision)] = part;
        availableParts.AddRange(snapshot.Parts.Where(p => !p.Archived).Select(PartOption));
        foreach (var p in snapshot.Furniture)
        {
            records[p.Id] = p;
            furniture.Add(new(p.Id, p.Name, p.CategoryName, p.Variants.Count(v => !v.Archived), p.Variants.Where(v => !v.Archived).Select(v => (decimal)v.SellingPriceYer).DefaultIfEmpty().Min(), "Wardrobe", p.State == FurnitureState.Archived, p.State == FurnitureState.Draft));
        }
        RefreshVisibleFurniture(); RefreshPartOptions();
    }

    private FurniturePartOption PartOption(Part part) => new(part.Id, part.Name,
        partCategories.FirstOrDefault(c => c.Id == part.CategoryId)?.Name ?? "", part.Cost.TotalCostYer)
        { Reference = part.Composition };

    /// <summary>Returns the authority record whose references must resolve before opening an editor.</summary>
    internal Definition? RecordFor(FurnitureListItem? item) => item is null ? null : records.GetValueOrDefault(item.Id);

    /// <summary>Updates picker data and saved usages without changing unsaved editor fields or revisions.</summary>
    public void ApplyEditorReferences(FurnitureEditorSnapshot snapshot)
    {
        partCategories = snapshot.Categories;
        foreach (var part in snapshot.Compositions) compositions[(part.Id, part.Revision)] = part;
        ApplyPartChoices(snapshot.Parts);
    }

    /// <summary>Replaces picker choices using Rust search results and projected category names.</summary>
    public void ApplyPartChoices(IReadOnlyList<Part> parts)
    {
        availableParts.Clear();
        availableParts.AddRange(parts.Where(p => !p.Archived).Select(PartOption));
        RefreshPartOptions();
    }
    /// <summary>Removes restricted records, costs, retry input, and editor fields when authority ends.</summary>
    public void ClearSession()
    {
        IsLoading=false; DataStateText=""; FixtureSalesCatalog=false; CanManage = false; IsBusy = false; pendingSave = null; editingRecord = null; editingFurniture = null;
        records.Clear(); furniture.Clear(); compositions.Clear(); categories = []; partCategories = [];
        availableParts.Clear();
        EditorCategoryOptions.Clear(); CategoryOptions.Clear(); CategoryOptions.Add(AllCategories);
        SelectedParts.Clear(); Variants.Clear(); Colors.Clear(); Handles.Clear(); FilteredParts.Clear(); VisibleFurniture.Clear();
        EditorName = ""; ShortDescription = ""; InternalNotes = ""; ProductImage = null; ProductImageName = ""; EditorCategory = "";
        CancelEditor(); reviewedCost = 0; RefreshPartsState(); RefreshVisibleFurniture();
    }
    /// <summary>Shows an editor recovery message without changing staged input.</summary>
    public void Fail(string text) => EditorError = text;
    /// <summary>Disables management when authoritative Furniture data cannot be loaded.</summary>
    public void Unavailable(string text) { IsLoading=false; CanManage = false; DataStateText=text; FeedbackMessage = text; }
    /// <summary>Closes the editor only after a confirmed save and shows local persistence feedback.</summary>
    public void Saved(bool draft)
    {
        pendingSave = null; CancelEditor();
        FeedbackMessage = draft ? "حُفظت مسودة الأثاث محلياً." : "حُفظ تعريف الأثاث محلياً.";
        Raise(nameof(CanEditFields));
    }
    /// <summary>Freezes the original request while its save outcome remains unknown.</summary>
    public void SaveUnconfirmed(SaveFurniture input) { pendingSave = input; Raise(nameof(CanEditFields)); }
    /// <summary>Releases frozen input after the authority returns a definite save outcome.</summary>
    public void SaveResolved() { pendingSave = null; Raise(nameof(CanEditFields)); }

    // These conversions preserve exact input. Rust determines domain validity.
    /// <summary>Converts integer money exactly and rejects fractional or overflowing input.</summary>
    internal static long WholeMoney(decimal value) => value == decimal.Truncate(value) ? checked((long)value) : throw new FormatException();
    /// <summary>Converts centimetres exactly and rejects unsupported precision or overflow.</summary>
    internal static uint ToMillimetres(decimal cm) => cm * 10m == decimal.Truncate(cm * 10m) ? checked((uint)(cm * 10m)) : throw new FormatException();
    /// <summary>Creates the Rust dimension DTO from exact presentation values.</summary>
    internal static FurnitureDimensions Dimensions(decimal w, decimal h, decimal d) => new() { WidthMm = ToMillimetres(w), HeightMm = ToMillimetres(h), DepthMm = ToMillimetres(d) };
    /// <summary>Builds a typed save or returns frozen retry input; reports an unsaved category separately.</summary>
    public SaveFurniture SaveInput(FurnitureState state)
    {
        if (pendingSave is not null) return pendingSave;
        var category = categories.FirstOrDefault(c => c.Name == EditorCategory.Trim()) ?? throw new UnsavedCategoryException();
        return new SaveFurniture
        {
            Id = editingRecord?.Id, ExpectedRevision = editingExpectedRevision,
            Name = EditorName.Trim(), CategoryId = category.Id, Description = ShortDescription, Notes = InternalNotes,
            State = editingRecord?.State == FurnitureState.Archived ? FurnitureState.Archived : state, ConfirmBelowCost = ConfirmBelowCost,
            Parts = SelectedParts.Select(u => new FurniturePart { Reference = u.Part.Reference ?? throw new FormatException(), Quantity = u.Quantity == decimal.Truncate(u.Quantity) ? checked((uint)u.Quantity) : throw new FormatException() }).ToArray(),
            Variants = Variants.Select(v => new DefinitionVariant { Id = v.Id, Name = v.Name, Dimensions = Dimensions(v.Width, v.Height, v.Depth), Customization = v.Customization!, SellingPriceYer = WholeMoney(v.SellingPrice), Archived = v.IsArchived, ColorIds = v.ColorIds, HandleIds = v.HandleIds }).ToArray(),
            Colors = Colors.Select(c => new FurnitureOption { Id = c.Id, Name = c.Name, Visual = c.SwatchHex, PriceAdjustmentYer = WholeMoney(c.PriceAdjustment), Archived = !c.IsActive }).ToArray(),
            Handles = Handles.Select(h => new FurnitureOption { Id = h.Id, Name = h.Name, Visual = h.HandleKind, PriceAdjustmentYer = WholeMoney(h.PriceAdjustment), Archived = !h.IsActive }).ToArray(),
        };
    }
    /// <summary>Stages the saved definition for an audited archive revision.</summary>
    public SaveFurniture ArchiveInput(FurnitureListItem item)
    {
        BeginEdit(item);
        var input = SaveInput(FurnitureState.Archived);
        return input;
    }
    /// <summary>Displays Rust cost and margin results without calculating domain values.</summary>
    public void ApplyReview(FurnitureReview review)
    {
        reviewedCost = review.PartsCostYer;
        for (var i = 0; i < SelectedParts.Count && i < review.RowCostsYer.Length; i++) SelectedParts[i].ApplyRowCost(review.RowCostsYer[i]);
        for (var i = 0; i < Variants.Count && i < review.MarginsYer.Length; i++) Variants[i].ApplyReview(review.PartsCostYer, review.MarginsYer[i]);
        RefreshPartsState();
    }
    /// <summary>Stages permitted bounds and named options for a variant dialog.</summary>
    private void PrepareVariantChoices(FurnitureVariant? v)
    {
        AllowCustomization = v?.Customization is not null;
        MinWidth = v?.Customization?.Minimum.WidthMm / 10m ?? v?.Width ?? 120;
        MinHeight = v?.Customization?.Minimum.HeightMm / 10m ?? v?.Height ?? 200;
        MinDepth = v?.Customization?.Minimum.DepthMm / 10m ?? v?.Depth ?? 55;
        MaxWidth = v?.Customization?.Maximum.WidthMm / 10m ?? MinWidth;
        MaxHeight = v?.Customization?.Maximum.HeightMm / 10m ?? MinHeight;
        MaxDepth = v?.Customization?.Maximum.DepthMm / 10m ?? MinDepth;
        VariantColorChoices.Clear(); VariantHandleChoices.Clear();
        foreach (var c in Colors.Where(c => c.IsActive)) VariantColorChoices.Add(new(c.Id, c.Name, v?.ColorIds.Contains(c.Id) == true));
        foreach (var h in Handles.Where(h => h.IsActive)) VariantHandleChoices.Add(new(h.Id, h.Name, v?.HandleIds.Contains(h.Id) == true));
        foreach (var name in new[] { nameof(AllowCustomization), nameof(MinWidth), nameof(MinHeight), nameof(MinDepth), nameof(MaxWidth), nameof(MaxHeight), nameof(MaxDepth) }) Raise(name);
    }
}
