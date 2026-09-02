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
    public void ThreeStepEditorCalculatesPartsAndMaintainsFixedVariants()
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

        viewModel.RequestNextFromVariants();
        Assert.AreEqual(3, viewModel.CurrentStep);
        StringAssert.Contains(viewModel.FeedbackMessage, "لم تُبنَ خطوة الخيارات");
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
        Assert.AreEqual("Included", viewModel.Colors.First(color => color.Name == "White").PriceAdjustmentLabel);
        Assert.AreEqual("+10,000 YER", viewModel.Colors.First(color => color.Name == "Walnut").PriceAdjustmentLabel);
        Assert.IsFalse(viewModel.Colors.First(color => color.Name == "Brown").IsActive);

        var brown = viewModel.Colors.First(color => color.Name == "Brown");
        viewModel.ToggleColor(brown);
        Assert.IsTrue(brown.IsActive);
        Assert.AreEqual("تعطيل", brown.ToggleActionLabel);

        viewModel.BeginAddColor();
        viewModel.ColorName = "Blue";
        viewModel.ColorPriceAdjustment = 2_500m;
        Assert.IsTrue(viewModel.SaveColor());
        Assert.AreEqual("+2,500 YER", viewModel.Colors[^1].PriceAdjustmentLabel);

        viewModel.BeginAddHandle();
        viewModel.HandleName = "Steel";
        viewModel.HandlePriceAdjustment = 4_000m;
        Assert.IsTrue(viewModel.SaveHandle());
        Assert.AreEqual("+4,000 YER", viewModel.Handles[^1].PriceAdjustmentLabel);

        viewModel.RequestNextFromOptions();
        Assert.AreEqual(4, viewModel.CurrentStep);
        StringAssert.Contains(viewModel.FeedbackMessage, "لم تُبنَ خطوة التسعير");
    }
}
