using Eitmad.WindowsShell.Features.Parts;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests.Parts;

[TestClass]
public sealed class PartsPresentationTests
{
    [TestMethod]
    public void PartsSearchAndFiltersUpdateVisibleList()
    {
        var model = new PartsViewModel();

        Assert.HasCount(4, model.VisibleParts);
        model.SearchText = "wardrobe";
        Assert.HasCount(1, model.VisibleParts);
        Assert.AreEqual("Wardrobe Side Panel", model.VisibleParts.Single().Name);

        model.SearchText = "خزانه";
        Assert.HasCount(1, model.VisibleParts);

        model.SearchText = string.Empty;
        model.SelectedCategory = "أبواب";
        model.SelectedStatus = PartsViewModel.ArchivedStatus;
        Assert.HasCount(1, model.VisibleParts);
        Assert.IsTrue(model.VisibleParts.Single().IsArchived);

        model.SelectedStatus = PartsViewModel.ActiveStatus;
        Assert.HasCount(1, model.VisibleParts);
        Assert.IsFalse(model.VisibleParts.Single().IsArchived);
    }

    [TestMethod]
    public void PartsActionsRemainNonDestructiveAndEphemeral()
    {
        var model = new PartsViewModel();
        var originalCount = model.VisibleParts.Count;
        var wardrobePanel = model.VisibleParts.Single(item => item.Name == "Wardrobe Side Panel");
        Assert.AreEqual("9,450 YER", wardrobePanel.CostLabel);
        Assert.AreEqual("3 Products", wardrobePanel.UsedInLabel);

        model.Archive(wardrobePanel);
        Assert.HasCount(originalCount, model.VisibleParts);
        Assert.IsTrue(wardrobePanel.IsArchived);

        var shelf = model.VisibleParts.Single(item => item.Name == "رف داخلي قابل للتعديل");
        var duplicate = model.Duplicate(shelf);
        Assert.IsTrue(model.IsEditorOpen);
        Assert.IsTrue(duplicate.Name.EndsWith("نسخة", StringComparison.Ordinal));
        Assert.HasCount(originalCount + 1, model.VisibleParts);

        model.CancelEditor();
        model.BeginCreate();
        model.EditorName = "واجهة درج صغيرة";
        model.EditorCategory = "أدراج";
        model.EditorDescription = "واجهة درج للمقاسات الصغيرة.";
        Assert.IsTrue(model.MoveToMaterials());

        model.OpenMaterialPicker();
        model.AddMaterial(model.FilteredMaterials.Single(item => item.Name == "MDF 18mm"));
        model.SelectedMaterials.Single().Quantity = 1.2m;
        model.OpenMaterialPicker();
        model.AddMaterial(model.FilteredMaterials.Single(item => item.Name == "Edge Band"));
        model.SelectedMaterials.Single(item => item.Material.Name == "Edge Band").Quantity = 3m;

        Assert.AreEqual(9_450m, model.TotalPartCost);
        Assert.AreEqual("9,450", model.TotalPartCostLabel);
        Assert.IsTrue(model.MoveToReview());
        Assert.IsTrue(model.SaveEditor());
        Assert.HasCount(originalCount + 2, model.VisibleParts);
        Assert.AreEqual("9,450 YER", model.VisibleParts.Single(item => item.Name == "واجهة درج صغيرة").CostLabel);
    }

    [TestMethod]
    public void GuidedCreationValidatesStepsAndFiltersMaterialPicker()
    {
        var model = new PartsViewModel();

        model.BeginCreate();
        Assert.AreEqual(1, model.CurrentStep);
        Assert.IsFalse(model.MoveToMaterials());
        Assert.AreEqual("أدخل اسم الجزء.", model.EditorError);

        model.EditorName = "جانب خزانة";
        Assert.IsTrue(model.MoveToMaterials());
        Assert.IsFalse(model.MoveToReview());
        Assert.AreEqual("أضف مادة خام واحدة على الأقل للمتابعة.", model.EditorError);

        model.OpenMaterialPicker();
        model.MaterialSearchText = "edge";
        Assert.HasCount(1, model.FilteredMaterials);
        model.AddMaterial(model.FilteredMaterials.Single());
        Assert.IsFalse(model.IsMaterialPickerOpen);

        var usage = model.SelectedMaterials.Single();
        model.RemoveMaterial(usage);
        Assert.IsFalse(model.HasSelectedMaterials);
        model.OpenMaterialPicker();
        Assert.IsTrue(model.FilteredMaterials.Any(item => item.Name == "Edge Band"));
    }
}
