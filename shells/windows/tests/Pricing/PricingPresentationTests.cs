using Eitmad.WindowsShell.Features.Pricing;

namespace Eitmad.WindowsShell.Tests.Pricing;

[TestClass]
public sealed class PricingPresentationTests
{
    [TestMethod]
    public void SearchCategoryAndArabicNormalizationFilterProductVariants()
    {
        var viewModel = new PricingViewModel();

        viewModel.SearchText = "خزانه";
        Assert.HasCount(2, viewModel.VisiblePrices);

        viewModel.SelectedCategory = "غرف الطعام";
        Assert.HasCount(0, viewModel.VisiblePrices);

        viewModel.SearchText = "طاولة";
        Assert.HasCount(1, viewModel.VisiblePrices);
        Assert.AreEqual("طاولة طعام", viewModel.VisiblePrices[0].Product);
    }

    [TestMethod]
    public void QuickEditValidatesAndUpdatesOnlyTheSellingPricePreview()
    {
        var viewModel = new PricingViewModel();
        var item = viewModel.VisiblePrices[0];
        var originalCost = item.Cost;

        viewModel.BeginEdit(item);
        viewModel.EditorSellingPrice = "غير صالح";
        Assert.IsFalse(viewModel.SaveEditor());
        Assert.IsTrue(viewModel.IsEditorOpen);
        StringAssert.Contains(viewModel.EditorError, "سعر بيع صالحاً");

        viewModel.EditorSellingPrice = "٢٢٠٬٠٠٠";
        Assert.AreEqual("60,000 YER", viewModel.EditorMargin);
        Assert.IsTrue(viewModel.SaveEditor());

        Assert.AreEqual(220_000m, item.SellingPrice);
        Assert.AreEqual(originalCost, item.Cost);
        Assert.AreEqual("60,000 YER", item.MarginLabel);
        StringAssert.Contains(viewModel.FeedbackMessage, "المعاينة المحلية فقط");
    }
}
