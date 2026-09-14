using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Tests.Products;

[TestClass]
public sealed class SalesCatalogPresentationTests
{
    [TestMethod]
    public void CatalogCombinesActiveItemsAndComposesArabicSearchWithCategories()
    {
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel());
        Assert.HasCount(7, model.VisibleItems);
        model.SearchText = "  مرتبه  ";
        Assert.AreEqual("مرتبة طبية", model.VisibleItems.Single().Name);
        model.SelectedCategory = "غرف النوم";
        Assert.IsTrue(model.IsEmpty);
        model.ClearFilters();
        Assert.HasCount(7, model.VisibleItems);
        model.SearchText = "غرف النوم";
        Assert.HasCount(2, model.VisibleItems);
        model.SearchText = "السكينه";
        Assert.AreEqual("خزانة السكينة", model.VisibleItems.Single().Name);
    }

    [TestMethod]
    public void ReloadUsesManagerCategoriesAndLowestVariantPriceWithoutManagerFilters()
    {
        var products = new ProductsViewModel();
        var furniture = new FurnitureViewModel();
        var model = new SalesCatalogViewModel(furniture, products);
        var mattress = products.VisibleProducts.First();
        products.BeginEdit(mattress);
        products.Variants[2].SellingPrice = 60_000m;
        Assert.IsTrue(products.SaveEditor());
        products.SearchText = "غير موجود";
        furniture.SearchText = "غير موجود";
        model.Reload();
        Assert.HasCount(7, model.VisibleItems);
        Assert.AreEqual("60,000 YER", model.VisibleItems.Single(item => item.Id == mattress.Id).PriceLabel);
        CollectionAssert.AreEquivalent(
            furniture.EditorCategoryOptions.Concat(products.ActiveCategories.Select(category => category.Name)).Distinct().ToArray(),
            model.Categories.Skip(1).ToArray());
        model.Select(model.VisibleItems.First());
        Assert.IsNotNull(model.Selection);
    }
    [TestMethod]
    public void FurnitureSelectionUsesOnlyActiveOptionsAndKeepsQuotationSnapshots()
    {
        var manager = new FurnitureViewModel();
        var model = new SalesCatalogViewModel(manager, new ProductsViewModel());
        model.Select(model.VisibleItems.First());
        var detail = model.Selection!;
        Assert.IsFalse(detail.CanAdd);
        Assert.IsFalse(model.AddSelection());
        Assert.HasCount(3, detail.Sizes);
        Assert.HasCount(2, detail.Colors);
        Assert.HasCount(2, detail.Handles);
        Assert.IsFalse(detail.Colors.Any(c => c.Name == "بني"));
        Assert.IsFalse(detail.Handles.Any(h => h.Name == "نحاسي"));
        detail.SelectedSize = detail.Sizes.Last();
        detail.SelectedColor = detail.Colors.Last();
        detail.SelectedHandle = detail.Handles.Last();
        detail.Quantity = 2;
        Assert.IsTrue(detail.CanAdd);
        Assert.AreEqual(313_000m, detail.UnitPrice);
        Assert.AreEqual(626_000m, detail.LineTotal);
        Assert.IsTrue(model.AddSelection());
        Assert.AreSame(detail, model.Selection);
        detail.Quantity = 3;
        detail.SelectedSize = detail.Sizes.First();
        Assert.AreEqual(626_000m, model.QuotationLines.Single().LineTotal);
        Assert.AreEqual(639_000m, detail.LineTotal);
        detail.Quantity = 0;
        Assert.AreEqual(3, detail.Quantity);
        model.CloseSelection();
        model.Reload();
        Assert.HasCount(1, model.QuotationLines);
    }

    [TestMethod]
    public void UnavailableSizesAndOverflowCannotBeAdded()
    {
        var item = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel()).VisibleItems.First();
        var empty = new FurnitureSelectionViewModel(item, [], [], []);
        Assert.IsFalse(empty.CanAdd);
        var size = new SalesSize(Guid.NewGuid(), "كبير", "200 × 220 × 60 cm", decimal.MaxValue);
        var detail = new FurnitureSelectionViewModel(item, [size], [], []);
        detail.SelectedSize = size;
        Assert.IsTrue(detail.CanAdd);
        detail.Quantity = 2;
        Assert.IsFalse(detail.CanAdd);
        Assert.AreEqual("—", detail.LineTotalLabel);
    }
}
