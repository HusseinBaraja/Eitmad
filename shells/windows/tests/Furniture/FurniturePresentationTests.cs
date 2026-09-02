using Eitmad.WindowsShell.Features.Furniture;

namespace Eitmad.WindowsShell.Tests.Furniture;

[TestClass]
public sealed class FurniturePresentationTests
{
    [TestMethod]
    public void FurnitureSearchFiltersAndActionsRemainTransient()
    {
        var viewModel = new FurnitureViewModel();

        viewModel.SearchText = "خزانه";
        Assert.HasCount(1, viewModel.VisibleFurniture);
        Assert.AreEqual("خزانة السكينة", viewModel.VisibleFurniture[0].Name);

        viewModel.SearchText = string.Empty;
        viewModel.SelectedCategory = "المكاتب";
        Assert.HasCount(1, viewModel.VisibleFurniture);
        Assert.AreEqual("مكتب العمل الهادئ", viewModel.VisibleFurniture[0].Name);

        viewModel.SelectedCategory = FurnitureViewModel.AllCategories;
        var source = viewModel.VisibleFurniture.First(item => !item.IsArchived);
        var duplicate = viewModel.DuplicateFurniture(source);
        Assert.IsTrue(viewModel.IsEditorOpen);
        Assert.IsTrue(duplicate.Name.EndsWith("— نسخة", StringComparison.Ordinal));

        viewModel.CancelEditor();
        viewModel.ArchiveFurniture(duplicate);
        Assert.IsTrue(duplicate.IsArchived);
        Assert.IsTrue(viewModel.HasFeedback);
    }

    [TestMethod]
    public void EditorCalculatesPartsAndMaintainsFixedVariants()
    {
        var viewModel = new FurnitureViewModel();
        viewModel.BeginCreate();
        viewModel.EditorName = "خزانة اختبار";

        Assert.IsTrue(viewModel.MoveToParts());
        var part = viewModel.FilteredParts[0];
        viewModel.AddPart(part);
        viewModel.SelectedParts[0].Quantity = 3m;
        Assert.AreEqual(part.UnitCost * 3m, viewModel.CurrentPartsCost);

        Assert.IsTrue(viewModel.MoveToVariants());
        viewModel.BeginAddVariant();
        viewModel.VariantName = "صغير";
        viewModel.VariantWidth = 120m;
        viewModel.VariantHeight = 200m;
        viewModel.VariantDepth = 55m;
        Assert.IsTrue(viewModel.SaveVariant());
        Assert.HasCount(1, viewModel.Variants);
        Assert.AreEqual("120 × 200 × 55 cm", viewModel.Variants[0].DimensionsLabel);
        Assert.IsGreaterThan(0m, viewModel.Variants[0].CalculatedCost);

        viewModel.DuplicateVariant(viewModel.Variants[0]);
        Assert.HasCount(2, viewModel.Variants);
        viewModel.RemoveVariant(viewModel.Variants[1]);
        Assert.HasCount(1, viewModel.Variants);

        Assert.IsTrue(viewModel.MoveToOptions());
        Assert.AreEqual(4, viewModel.CurrentStep);
    }

    [TestMethod]
    public void VariantRejectsDimensionsThatOverflowPreviewCost()
    {
        var viewModel = new FurnitureViewModel();
        viewModel.BeginCreate();
        viewModel.BeginAddVariant();
        viewModel.VariantName = "كبير";
        viewModel.VariantWidth = decimal.MaxValue;
        viewModel.VariantHeight = decimal.MaxValue;
        viewModel.VariantDepth = decimal.MaxValue;

        Assert.IsFalse(viewModel.SaveVariant());
        Assert.HasCount(0, viewModel.Variants);
        StringAssert.Contains(viewModel.EditorError, "النطاق المدعوم");
    }

    [TestMethod]
    public void OptionsExposePriceAdjustmentsAndTransientActiveState()
    {
        var viewModel = new FurnitureViewModel();
        viewModel.BeginCreate();
        viewModel.EditorName = "خزانة خيارات";
        Assert.IsTrue(viewModel.MoveToParts());
        viewModel.AddPart(viewModel.FilteredParts[0]);
        Assert.IsTrue(viewModel.MoveToVariants());
        viewModel.BeginAddVariant();
        viewModel.VariantName = "صغير";
        Assert.IsTrue(viewModel.SaveVariant());

        Assert.IsTrue(viewModel.MoveToOptions());
        Assert.AreEqual(4, viewModel.CurrentStep);
        Assert.HasCount(3, viewModel.Colors);
        Assert.HasCount(3, viewModel.Handles);
        Assert.AreEqual("مشمول", viewModel.Colors.First(color => color.Name == "أبيض").PriceAdjustmentLabel);
        Assert.AreEqual("+10,000 YER", viewModel.Colors.First(color => color.Name == "جوزي").PriceAdjustmentLabel);
        Assert.IsFalse(viewModel.Colors.First(color => color.Name == "بني").IsActive);

        var brown = viewModel.Colors.First(color => color.Name == "بني");
        viewModel.ToggleColor(brown);
        Assert.IsTrue(brown.IsActive);
        Assert.AreEqual("تعطيل", brown.ToggleActionLabel);

        viewModel.BeginAddColor();
        viewModel.ColorName = "أزرق";
        viewModel.ColorPriceAdjustment = 2_500m;
        Assert.IsTrue(viewModel.SaveColor());
        Assert.AreEqual("+2,500 YER", viewModel.Colors[^1].PriceAdjustmentLabel);

        viewModel.BeginAddHandle();
        viewModel.HandleName = "فولاذي";
        viewModel.HandlePriceAdjustment = 4_000m;
        Assert.IsTrue(viewModel.SaveHandle());
        Assert.AreEqual("+4,000 YER", viewModel.Handles[^1].PriceAdjustmentLabel);

        Assert.IsTrue(viewModel.MoveToPricing());
        Assert.AreEqual(5, viewModel.CurrentStep);
    }

    [TestMethod]
    public void PricingCalculatesMarginAndFinalReviewCompletesTransientFlow()
    {
        var viewModel = new FurnitureViewModel();
        var wardrobe = viewModel.VisibleFurniture.First(item => item.Name == "خزانة السكينة");
        viewModel.BeginEdit(wardrobe);

        Assert.IsTrue(viewModel.MoveToParts());
        Assert.IsTrue(viewModel.MoveToVariants());
        Assert.IsTrue(viewModel.MoveToOptions());
        Assert.IsTrue(viewModel.MoveToPricing());

        var small = viewModel.Variants.First(variant => variant.Name == "صغير");
        Assert.AreEqual("200,000", small.SellingPriceInput);
        Assert.AreEqual(40_000m, small.Margin);
        Assert.AreEqual("40,000", small.MarginLabel);
        Assert.AreEqual("هامش الربح", small.MarginCaption);

        small.SellingPrice = 150_000m;
        Assert.IsTrue(small.HasNegativeMargin);
        Assert.AreEqual("خسارة متوقعة", small.MarginCaption);
        small.SellingPrice = 200_000m;

        Assert.IsTrue(viewModel.MoveToReview());
        Assert.AreEqual(6, viewModel.CurrentStep);
        viewModel.SaveDraftPreview();

        Assert.IsTrue(viewModel.IsListVisible);
        Assert.IsTrue(wardrobe.IsDraft);
        Assert.AreEqual("مسودة", wardrobe.StatusLabel);
        Assert.AreEqual(200_000m, wardrobe.SellingPrice);
        StringAssert.Contains(viewModel.FeedbackMessage, "المعاينة المحلية");
    }

    [TestMethod]
    public void SellingPriceAcceptsArabicIndicNumerals()
    {
        var variant = new FurnitureVariant(Guid.NewGuid(), "صغير", 120m, 200m, 55m, 160_000m);

        variant.SellingPriceInput = "٢٠٠٬٠٠٠";

        Assert.AreEqual(200_000m, variant.SellingPrice);
    }

    [TestMethod]
    [DataRow(true)]
    [DataRow(false)]
    public void SavingAnArchivedFurnitureEditPreservesArchiveState(bool saveAsDraft)
    {
        var viewModel = new FurnitureViewModel();
        var archived = viewModel.VisibleFurniture.First(item => item.IsArchived);
        viewModel.BeginEdit(archived);

        if (saveAsDraft)
        {
            viewModel.SaveDraftPreview();
        }
        else
        {
            viewModel.PublishPreview();
        }

        Assert.IsTrue(archived.IsArchived);
        Assert.AreEqual(saveAsDraft, archived.IsDraft);
    }
}
