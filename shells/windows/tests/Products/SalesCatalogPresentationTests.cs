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
        Assert.IsTrue(model.SelectionNotice.Contains("للمعاينة فقط"));
    }
}
