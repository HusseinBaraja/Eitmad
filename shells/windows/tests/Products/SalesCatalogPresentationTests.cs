using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Tests.Products;

[TestClass]
public sealed class SalesCatalogPresentationTests
{
    [TestMethod]
    public void DraftSaveRequiresCustomerNameAndPhoneAfterBrowsing()
    {
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel());
        model.Select(model.VisibleItems.Single(item => item.Name == "وسادة فندقية"));
        Assert.IsTrue(model.AddProductSelection());
        Assert.IsFalse(model.IsReviewingQuotation);
        Assert.IsFalse(model.ReviewDraftSave());
        Assert.AreEqual("أدخل اسم العميل", model.CustomerNameError);
        model.CustomerName = "عميل تجريبي";
        Assert.IsFalse(model.ReviewDraftSave());
        Assert.AreEqual("أدخل رقم الهاتف", model.PhoneError);
        model.Phone = "000000000";
        model.DiscountInput = "10";
        model.RequestDiscountApproval();
        Assert.IsTrue(model.ReviewDraftSave());
        Assert.IsTrue(model.IsDiscountPending);
        Assert.IsFalse(model.ReviewSave());
    }

    [TestMethod]
    public void DiscountPreviewGatesSavingAndInvalidatesChangedRequests()
    {
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel());
        model.Select(model.VisibleItems.Single(item => item.Name == "وسادة فندقية"));
        model.AddProductSelection();
        model.CustomerName = "عميل تجريبي"; model.Phone = "000000000";
        model.DiscountInput = "٥";
        Assert.AreEqual(600m, model.Discount);
        Assert.AreEqual(11_400m, model.FinalTotal);
        Assert.IsTrue(model.ReviewSave());
        model.DiscountInput = "٥٫٥";
        Assert.AreEqual(660m, model.Discount);
        Assert.IsTrue(model.CanRequestDiscountApproval);
        Assert.IsFalse(model.ReviewSave());
        model.RequestDiscountApproval();
        Assert.IsTrue(model.IsDiscountPending);
        Assert.IsFalse(model.CanRequestDiscountApproval);
        Assert.IsTrue(model.ReviewDraftSave());
        Assert.IsTrue(model.IsDiscountPending);
        Assert.IsFalse(model.ReviewSave());
        model.DuplicateLine(model.QuotationLines[0]);
        Assert.IsFalse(model.IsDiscountPending);
        Assert.AreEqual(1320m, model.Discount);
        model.RequestDiscountApproval();
        model.DiscountInput = "۶";
        Assert.IsFalse(model.IsDiscountPending);
        foreach (var invalid in new[] { "", "abc", "-1", "101", "1,5" })
        {
            model.DiscountInput = invalid;
            Assert.IsFalse(model.CanSaveQuotation);
            Assert.IsFalse(model.ReviewDraftSave());
            Assert.IsFalse(model.CanRequestDiscountApproval);
            Assert.AreEqual("—", model.FinalTotalLabel);
        }
        model.DiscountInput = "0";
        Assert.AreEqual(model.Subtotal, model.FinalTotal);
        Assert.IsTrue(model.ReviewSave());
    }

    [TestMethod]
    public void CurrentQuotationEditingCancellationTotalsAndCustomerFlow()
    {
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel());
        model.Select(model.VisibleItems.Single(item => item.Name == "مرتبة طبية"));
        model.ProductSelection!.SelectedVariant = model.ProductSelection.Variants[1];
        model.AddProductSelection();
        var original = model.QuotationLines.Single();
        model.EditLine(original);
        Assert.AreEqual(original.Variant, model.ProductSelection!.SelectedVariant!.Name);
        model.ProductSelection.Quantity = 3;
        model.CloseSelection();
        Assert.AreEqual(1, original.Quantity);
        Assert.IsTrue(model.IsReviewingQuotation);
        model.EditLine(original);
        model.ProductSelection!.Quantity = 2;
        model.AddProductSelection();
        Assert.AreEqual(original.Id, model.QuotationLines.Single().Id);
        Assert.AreEqual(210_000m, model.Subtotal);
        Assert.IsTrue(model.IsReviewingQuotation);
        model.DuplicateLine(model.QuotationLines.Single());
        Assert.AreNotEqual(model.QuotationLines[0].Id, model.QuotationLines[1].Id);
        Assert.AreEqual(420_000m, model.FinalTotal);
        model.RemoveLine(model.QuotationLines[0]);
        Assert.IsFalse(model.ReviewSave());
        model.CustomerName = "عميل";
        Assert.HasCount(1, model.CustomerMatches);
        model.AttachCustomer(model.CustomerMatches[0]);
        model.BeginNewCustomer();
        Assert.IsFalse(model.SaveNewCustomer());
        model.CancelNewCustomer();
        Assert.AreEqual("عميل تجريبي", model.CustomerName);
        model.BeginNewCustomer();
        model.CustomerName = "عميل معاينة جديد"; model.Phone = "000000001";
        Assert.IsTrue(model.SaveNewCustomer());
        Assert.IsTrue(model.ReviewSave());
        Assert.IsTrue(model.QuotationNotice.Contains("لم تُحفظ"));
    }

    [TestMethod]
    public void ReadyMadeSelectionsRequireVariantsAndKeepQuotationSnapshots()
    {
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel());
        model.Select(model.VisibleItems.Single(item => item.Name == "مرتبة طبية"));
        var detail = model.ProductSelection!;
        Assert.IsFalse(model.AddProductSelection());
        detail.SelectedVariant = detail.Variants[1];
        detail.Quantity = 2;
        Assert.IsTrue(model.AddProductSelection());
        Assert.AreEqual(210_000m, model.QuotationLines.Single().LineTotal);
        Assert.AreEqual("مزدوج", model.QuotationLines.Single().Variant);
        Assert.IsNull(model.QuotationLines.Single().Color);
        detail.SelectedVariant = detail.Variants[2];
        Assert.AreEqual(210_000m, model.QuotationLines.Single().LineTotal);
        detail.Quantity = 0;
        Assert.AreEqual(2, detail.Quantity);
        model.CloseSelection();
        model.Select(model.VisibleItems.Single(item => item.Name == "وسادة فندقية"));
        Assert.IsFalse(model.ProductSelection!.HasVariants);
        Assert.IsTrue(model.AddProductSelection());
        Assert.AreEqual(12_000m, model.QuotationLines.Last().UnitPrice);
        Assert.AreEqual(string.Empty, model.QuotationLines.Last().Variant);
        var item = model.ProductSelection.Item;
        var variant = new SalesProductVariant(Guid.NewGuid(), "قياسي", decimal.MaxValue);
        var overflow = new ProductSelectionViewModel(item, [variant]) { SelectedVariant = variant, Quantity = 2 };
        Assert.IsFalse(overflow.CanAdd);
    }

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
