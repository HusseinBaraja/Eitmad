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
}
