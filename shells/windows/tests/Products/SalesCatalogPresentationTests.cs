using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Products;

[TestClass]
public sealed class SalesCatalogPresentationTests
{
    /// <summary>Verifies catalog browsing is allowed before customer details but draft save requires them.</summary>
    [TestMethod]
    public void DraftSaveRequiresCustomerNameAndPhoneAfterBrowsing()
    {
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel());
        AddHistoricalProductLine(model,12000);
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

    /// <summary>Verifies discount approval applies only to the exact reviewed quotation fields.</summary>
    [TestMethod]
    public void DiscountPreviewGatesSavingAndInvalidatesChangedRequests()
    {
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel());
        AddHistoricalProductLine(model,12000);
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

    /// <summary>Verifies quotation editing, cancellation, totals, and customer selection through the presentation flow.</summary>
    [TestMethod]
    public async Task CurrentQuotationEditingCancellationTotalsAndCustomerFlow()
    {
        await using var engine = new FakeEngine();
        await using var customers = new CustomerClient(engine);
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel(), customers);
        AddHistoricalProductLine(model,105000,"مزدوج");
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
        model.QuotationLines.Remove(model.QuotationLines[0]);
        Assert.IsFalse(model.ReviewSave());
        model.AttachCustomer(new PreviewCustomer("عميل تجريبي", "000000000", "عنوان تجريبي", ""));
        model.BeginNewCustomer();
        Assert.IsFalse(await model.SaveNewCustomerAsync());
        model.CancelNewCustomer();
        Assert.AreEqual("عميل تجريبي", model.CustomerName);
        model.BeginNewCustomer();
        model.CustomerName = "عميل معاينة جديد"; model.Phone = "000000001";
        Assert.IsTrue(await model.SaveNewCustomerAsync());
        Assert.IsTrue(model.ReviewSave());
        Assert.IsTrue(model.QuotationNotice.Contains("لم يُحفظ عرض السعر"));
    }

    /// <summary>Verifies historical product lines remain editable without offering current unpriced definitions.</summary>
    [TestMethod]
    public void HistoricalProductSnapshotRemainsEditableWithoutCurrentCatalogSelection() {
        var model=new SalesCatalogViewModel(new FurnitureViewModel(),new ProductsViewModel());AddHistoricalProductLine(model,105000,"مزدوج");
        var line=model.QuotationLines.Single();model.EditLine(line);Assert.AreEqual("مزدوج",model.ProductSelection!.SelectedVariant!.Name);model.ProductSelection.Quantity=2;Assert.IsTrue(model.AddProductSelection());Assert.AreEqual(210000m,model.QuotationLines.Single().LineTotal);
    }

    /// <summary>Verifies Arabic search and category filtering compose across active catalog items.</summary>
    [TestMethod]
    public void CatalogCombinesActiveItemsAndComposesArabicSearchWithCategories()
    {
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel());
        Assert.HasCount(4, model.VisibleItems);
        model.SearchText = "  مرتبه  ";
        Assert.IsTrue(model.IsEmpty);
        model.SelectedCategory = "غرف النوم";
        Assert.IsTrue(model.IsEmpty);
        model.ClearFilters();
        Assert.HasCount(4, model.VisibleItems);
        model.SearchText = "غرف النوم";
        Assert.HasCount(2, model.VisibleItems);
        model.SearchText = "السكينه";
        Assert.AreEqual("خزانة السكينة", model.VisibleItems.Single().Name);
    }

    /// <summary>Verifies reloading the catalog does not expose definitions without published selling prices.</summary>
    [TestMethod]
    public void ReloadExcludesUnpricedProductDefinitions() {
        var products=new ProductsViewModel();products.ApplyDurableData(ProductsPresentationTests.Data());
        var model=new SalesCatalogViewModel(new FurnitureViewModel(),products);
        Assert.IsFalse(model.VisibleItems.Any(item=>item.Name=="مرتبة طبية"));
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
    /// <summary>Adds a synthetic stored product snapshot without depending on a current catalog selection.</summary>
    internal static void AddHistoricalProductLine(SalesCatalogViewModel model,decimal price=12000,string variantName="") {
        var item=new SalesCatalogItem(Guid.NewGuid(),"منتج تاريخي","منتجات","",variantName,price,variantName.Length>0,"Pillow",null);
        var variant=new SalesProductVariant(Guid.NewGuid(),variantName,price);
        model.QuotationLines.Add(new(new ProductSelectionViewModel(item,variantName.Length>0?[variant]:[]){SelectedVariant=variant}));
    }

}
