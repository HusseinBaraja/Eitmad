using Eitmad.WindowsShell.Features.Parts;

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
        model.EditorCost = 2_750m;
        model.EditorUsedInCount = 2;
        Assert.IsTrue(model.SaveEditor());
        Assert.HasCount(originalCount + 2, model.VisibleParts);
    }
}
