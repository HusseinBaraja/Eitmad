using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Pricing;

namespace Eitmad.WindowsShell.Features.Reception;

public sealed partial class SalesCatalogViewModel
{
    private SalesCatalogClient? catalogClient;
    private CancellationTokenSource? catalogCancellation, selectionCancellation, checkCancellation;
    private long catalogVersion, selectionVersion, checkVersion;
    private bool applyingCatalog, catalogActive, isCatalogLoading, catalogLoaded;
    private bool addingSelection;
    private readonly Features.CatalogImages.CatalogImageThumbnails catalogThumbnails = new();
    private Guid? nextCatalogPage;
    private string catalogStatus = "بيانات الكتالوج غير متاحة.";
    public bool IsCatalogLoading { get => isCatalogLoading; private set { Set(ref isCatalogLoading, value); Raise(nameof(IsEmpty)); } }
    public string CatalogStatus { get => catalogStatus; private set => Set(ref catalogStatus, value); }
    public bool HasNextPage => nextCatalogPage is not null;
    internal Task LastCatalogOperation { get; private set; } = Task.CompletedTask;
    public void AttachCatalogClient(SalesCatalogClient client)
    {
        catalogClient = client;
        client.Changed += CatalogChanged;
        client.ProjectionInvalidated += CatalogInvalidated;
        ClearCatalog();
    }
    public async Task ActivateCatalogAsync()
    {
        if (catalogClient is null) return;
        catalogActive = true;
        await catalogClient.ActivateAsync();
        LastCatalogOperation = LoadCatalogAsync(); await LastCatalogOperation;
    }
    public async Task DeactivateCatalogAsync()
    {
        catalogActive = false; ClearCatalog(); QuotationLines.Clear();
        if (catalogClient is not null) await catalogClient.DeactivateAsync();
    }
    private void ClearCatalog()
    {
        ++catalogVersion; ++selectionVersion; ++checkVersion;
        catalogCancellation?.Cancel(); selectionCancellation?.Cancel(); checkCancellation?.Cancel();
        catalogLoaded = false;
        applyingCatalog = true;
        CloseSelection(); VisibleItems.Clear(); Categories.Clear(); Categories.Add("الكل");
        selectedCategory = "الكل"; Raise(nameof(SelectedCategory)); applyingCatalog = false;
        catalogThumbnails.Clear();
        nextCatalogPage = null; Raise(nameof(HasNextPage)); IsCatalogLoading = false;
        CatalogStatus = "بيانات الكتالوج غير متاحة."; Raise(nameof(IsEmpty));
    }
    private void CatalogInvalidated(object? sender, EventArgs args) { ClearCatalog(); QuotationLines.Clear(); }
    private void CatalogChanged(object? sender, EventArgs args)
    {
        ++checkVersion; checkCancellation?.Cancel();
        Selection?.Fail("تغير الكتالوج أو الاتصال. حدّث الصنف للتحقق من الاختيار.");
        ProductSelection?.Fail("تغير الكتالوج أو الاتصال. حدّث الصنف للتحقق من الاختيار.");
        if (catalogActive) QueueCatalogLoad();
    }
    private void QueueCatalogLoad() { if (catalogActive) LastCatalogOperation = LoadCatalogAsync(debounce: true); }
    public Task NextPageAsync() => HasNextPage ? LoadCatalogAsync(nextCatalogPage) : Task.CompletedTask;
    public Task RefreshSelectionAsync() => (Selection?.Item ?? ProductSelection?.Item) is { } item ? SelectAsync(item) : Task.CompletedTask;
    private async Task LoadCatalogAsync(Guid? after = null, bool debounce = false)
    {
        if (catalogClient is null || !catalogActive) return;
        catalogCancellation?.Cancel(); catalogCancellation?.Dispose(); catalogCancellation = new();
        var token = catalogCancellation.Token; var version = ++catalogVersion;
        catalogLoaded = false; IsCatalogLoading = true; VisibleItems.Clear(); CatalogStatus = "جارٍ تحميل الكتالوج...";
        try
        {
            if (debounce) await Task.Delay(250, token);
            var result = await catalogClient.LoadAsync(SearchText, SelectedCategory == "الكل" ? null : SelectedCategory, after, token);
            if (version != catalogVersion || token.IsCancellationRequested) return;
            if (!result.Succeeded) { CatalogFailure(result.Failure); return; }
            var page = result.Value!;
            catalogLoaded = true;
            var category = selectedCategory;
            applyingCatalog = true;
            Categories.Clear(); Categories.Add("الكل"); foreach (var name in page.Categories) Categories.Add(name);
            selectedCategory = category;
            if (!Categories.Contains(category)) Categories.Add(category);
            Raise(nameof(SelectedCategory)); applyingCatalog = false;
            foreach (var group in page.Items.GroupBy(e => (SalesCatalogClient.Target(e).Kind, ItemId(e)))) VisibleItems.Add(Item(group.First()));
            nextCatalogPage = page.Next; Raise(nameof(HasNextPage));
            CatalogStatus = Availability(page.ServerAvailable) + " · الاختيارات وعرض السعر غير محفوظة.";
            IsCatalogLoading = false;
            await catalogThumbnails.ApplyAsync(catalogClient.Images, VisibleItems.Where(i => i.Entry?.Image is not null).Select(i => (i.Id, i.Entry!.Image!)), (id, image) => {
                if (version != catalogVersion) return;
                var index = VisibleItems.ToList().FindIndex(i => i.Id == id);
                if (index >= 0) VisibleItems[index] = VisibleItems[index] with { Image = image };
            }, token);
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { }
        finally { if (version == catalogVersion) { IsCatalogLoading = false; Raise(nameof(IsEmpty)); } }
    }
    private void CatalogFailure(PricingFailure failure)
    {
        if (failure == PricingFailure.Denied) { ClearCatalog(); QuotationLines.Clear(); }
        nextCatalogPage = null; Raise(nameof(HasNextPage)); CatalogStatus = SalesCatalogClient.Message(failure);
    }
    internal static string Availability(bool online) => online ? "كتالوج مؤكد من الخادم" : "الخادم غير متصل — آخر كتالوج مؤكد؛ التوفر والأسعار قد تكون قديمة";
    private static Guid ItemId(CatalogEntry entry) => SalesCatalogClient.Target(entry).AsProduct()?.ProductId ?? SalesCatalogClient.Target(entry).AsFurniture()!.FurnitureId;
    private static SalesCatalogItem Item(CatalogEntry entry, bool starting = false) => new(ItemId(entry), entry.Name, entry.CategoryName,
        entry.Description, entry.VariantName, entry.Price.SellingPriceYer, starting, "", null) { Entry = entry };
    private async Task SelectAsync(SalesCatalogItem item)
    {
        if (catalogClient is null || item.Entry is null || !catalogActive) return;
        selectionCancellation?.Cancel(); selectionCancellation?.Dispose(); selectionCancellation = new();
        var token = selectionCancellation.Token; var version = ++selectionVersion;
        ++checkVersion; checkCancellation?.Cancel(); Selection = null; ProductSelection = null;
        SelectionNotice = "جارٍ تحميل المقاسات والخيارات...";
        try
        {
            var result = await catalogClient.ItemAsync(item.Entry.Price.Target, token);
            if (version != selectionVersion || token.IsCancellationRequested) return;
            if (!result.Succeeded) { SelectionNotice = SalesCatalogClient.Message(result.Failure); CatalogFailure(result.Failure); return; }
            var entries = result.Value!.Variants; var current = Item(entries[0], entries.Length > 1);
            if (current.Entry?.Image is { } reference) current = current with { Image = await catalogClient.Images.LoadAsync(reference, 320, token) };
            if (version != selectionVersion || token.IsCancellationRequested) return;
            if (SalesCatalogClient.Target(entries[0]).AsProduct() is not null)
            {
                ProductSelection = new(current, entries.Select(e => new SalesProductVariant(SalesCatalogClient.Target(e).AsProduct()!.VariantId, e.VariantName, e.Price.SellingPriceYer) { Entry = e }).ToArray()) { IsEditing = editingLine is not null };
                ProductSelection.Changed += (_, _) => QueueConfigurationCheck();
                ProductSelection.Fail("اختر النوع / المقاس للتحقق من المنتج.");
            }
            else
            {
                Selection = new(current, entries.Select(e => new SalesSize(SalesCatalogClient.Target(e).AsFurniture()!.VariantId, e.VariantName, DimensionsLabel(e.Dimensions!), e.Price.SellingPriceYer) { Entry = e }).ToArray(), [], []) { IsEditing = editingLine is not null };
                Selection.Changed += (_, _) => QueueConfigurationCheck();
                Selection.Fail("اختر المقاس والخيارات للتحقق من الأثاث.");
            }
            SelectionNotice = Availability(result.Value.ServerAvailable);
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { }
    }
    internal static string DimensionsLabel(FurnitureDimensions d) => FormattableString.Invariant($"{d.WidthMm / 10m:0.#} × {d.HeightMm / 10m:0.#} × {d.DepthMm / 10m:0.#} سم");
    private void QueueConfigurationCheck() => LastCatalogOperation = CheckConfigurationAsync();
    private async Task CheckConfigurationAsync()
    {
        checkCancellation?.Cancel(); checkCancellation?.Dispose(); checkCancellation = new();
        var token = checkCancellation.Token; var version = ++checkVersion;
        var f = Selection; var p = ProductSelection;
        var input = f?.ConfigurationInput() ?? p?.ConfigurationInput();
        if (catalogClient is null || input is null || !catalogActive) return;
        f?.Fail("جارٍ التحقق من الاختيار..."); p?.Fail("جارٍ التحقق من الاختيار...");
        try
        {
            await Task.Delay(200, token);
            var result = await catalogClient.CheckAsync(input, token);
            if (version != checkVersion || token.IsCancellationRequested || f != Selection || p != ProductSelection) return;
            if (result.Succeeded) { f?.Apply(result.Value!); p?.Apply(result.Value!); }
            else { f?.Fail(SalesCatalogClient.Message(result.Failure)); p?.Fail(SalesCatalogClient.Message(result.Failure)); if (result.Failure == PricingFailure.Denied) CatalogFailure(result.Failure); }
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { }
    }

    public async Task<bool> AddValidatedSelectionAsync(bool product)
    {
        if (catalogClient is null) return product ? AddProductSelection() : AddSelection();
        if (addingSelection) return false;
        addingSelection = true;
        try
        {
            var f = Selection; var p = ProductSelection;
            LastCatalogOperation = CheckConfigurationAsync(); await LastCatalogOperation;
            if (f != Selection || p != ProductSelection) return false;
            return product ? AddProductSelection() : AddSelection();
        }
        finally { addingSelection = false; }
    }
}
