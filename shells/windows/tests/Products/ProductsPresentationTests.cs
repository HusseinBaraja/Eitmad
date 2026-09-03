using System.Globalization;
using Eitmad.WindowsShell.Features.Products;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests.Products;

[TestClass]
public sealed class ProductsPresentationTests
{
    [TestMethod]
    public void ProductSearchAndFiltersKeepReadyMadeCatalogRowsDistinct()
    {
        var model = new ProductsViewModel();

        Assert.HasCount(4, model.VisibleProducts);
        model.SearchText = "مرتبه";
        Assert.AreEqual("مرتبة طبية", model.VisibleProducts.Single().Name);

        model.SearchText = string.Empty;
        model.SelectedCategory = "الإضاءة";
        model.SelectedStatus = ProductsViewModel.ArchivedStatus;
        Assert.HasCount(1, model.VisibleProducts);
        Assert.AreEqual("مصباح قراءة", model.VisibleProducts.Single().Name);
        Assert.IsTrue(model.VisibleProducts.Single().IsArchived);
    }

    [TestMethod]
    public void ProductEditorSavesDirectPricingAndOptionalSupplierVariants()
    {
        var model = new ProductsViewModel();

        model.BeginCreate();
        model.EditorName = "غطاء وسادة قطني";
        model.EditorCategory = "الوسائد";
        model.PurchaseCost = 3_000m;
        model.SellingPrice = 5_500m;
        Assert.AreEqual("2,500", model.MarginLabel);
        Assert.IsTrue(model.SaveEditor());

        var fixedProduct = model.VisibleProducts.Single(product => product.Name == "غطاء وسادة قطني");
        Assert.AreEqual("بدون خيارات", fixedProduct.VariantSummary);
        Assert.AreEqual("3,000", fixedProduct.PurchaseCostLabel);
        Assert.AreEqual("5,500", fixedProduct.SellingPriceLabel);

        model.BeginCreate();
        model.EditorName = "مرتبة اقتصادية";
        model.EditorCategory = "المراتب";
        model.HasVariants = true;
        model.AddVariant();
        model.Variants[0].Name = "مفرد";
        model.Variants[0].PurchaseCost = 45_000m;
        model.Variants[0].SellingPrice = 61_000m;
        model.AddVariant();
        model.Variants[1].Name = "مزدوج";
        model.Variants[1].PurchaseCost = 70_000m;
        model.Variants[1].SellingPrice = 92_000m;

        Assert.AreEqual("16,000", model.Variants[0].MarginLabel);
        Assert.IsTrue(model.SaveEditor());
        var mattress = model.VisibleProducts.Single(product => product.Name == "مرتبة اقتصادية");
        Assert.AreEqual("مفرد +1", mattress.VariantSummary);
        Assert.AreEqual(45_000m, mattress.PurchaseCost);
        Assert.AreEqual(61_000m, mattress.SellingPrice);
    }

    [TestMethod]
    public void DuplicateAndArchiveRemainConfirmedAndEphemeral()
    {
        var model = new ProductsViewModel();
        var source = model.VisibleProducts.Single(product => product.Name == "وسادة فندقية");

        model.BeginDuplicate(source);
        Assert.IsTrue(model.IsCreating);
        Assert.IsTrue(model.EditorName.EndsWith("نسخة", StringComparison.Ordinal));
        Assert.IsTrue(model.SaveEditor());
        Assert.HasCount(5, model.VisibleProducts);

        model.RequestArchive(source);
        Assert.IsTrue(model.IsArchiveConfirmationOpen);
        Assert.IsFalse(source.IsArchived);
        model.CancelArchive();
        Assert.IsFalse(source.IsArchived);

        model.RequestArchive(source);
        model.ConfirmArchive();
        Assert.IsTrue(source.IsArchived);
        Assert.IsFalse(model.IsArchiveConfirmationOpen);
    }

    [TestMethod]
    public void ProductCategoryUsesTheEstablishedInlineManagementFlow()
    {
        var model = new ProductsViewModel();

        model.BeginAddCategory();
        model.CategoryName = "عطور منزلية";
        Assert.IsTrue(model.SaveCategory());
        Assert.AreEqual("عطور منزلية", model.EditorCategory);
        Assert.IsTrue(model.ActiveCategories.Any(category => category.Name == "عطور منزلية"));

        var category = model.ActiveCategories.Single(item => item.Name == "عطور منزلية");
        model.BeginManageCategories();
        model.BeginEditCategory(category);
        model.CategoryName = "روائح منزلية";
        Assert.IsTrue(model.SaveCategory());
        Assert.IsTrue(model.IsCategoryManagerOpen);
        Assert.IsTrue(model.CategoryOptions.Contains("روائح منزلية"));
    }

    [TestMethod]
    public void ProductCurrencyLabelsIgnoreTheAmbientCulture()
    {
        var originalCulture = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("ar-YE");
            var model = new ProductsViewModel();
            var mattress = model.VisibleProducts.Single(product => product.Name == "مرتبة طبية");

            Assert.AreEqual("55,000", mattress.PurchaseCostLabel);
            Assert.AreEqual("75,000", mattress.SellingPriceLabel);
        }
        finally
        {
            CultureInfo.CurrentCulture = originalCulture;
        }
    }
}
