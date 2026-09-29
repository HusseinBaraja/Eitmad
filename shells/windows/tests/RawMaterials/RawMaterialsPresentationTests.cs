using System.Globalization;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.RawMaterials;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests.RawMaterials;

[TestClass]
public sealed class RawMaterialsPresentationTests
{
    [TestMethod]
    public void DurableProjectionKeepsArchivedReferenceNamesWithoutOfferingThemForNewMaterials()
    {
        var model = new RawMaterialsViewModel();
        model.EnableDurableMode();
        var scope = new ScopeRef { Kind = "organization", Id = Guid.NewGuid() };
        var category = new MaterialCategory { Id = Guid.NewGuid(), Scope = scope,
            Name = "أخشاب طبيعية", Archived = true, Revision = 2 };
        var unit = new MaterialUnit { Id = Guid.NewGuid(), Scope = scope,
            Name = "متر", Symbol = "م", Dimension = UnitDimension.Length,
            Numerator = 1000, Denominator = 1, Archived = true, Revision = 2 };
        model.ApplyDurableData(new MaterialReferences { Categories = [category], Units = [unit] },
            [new Material { Id = Guid.NewGuid(), Scope = scope, Name = "خشب زان",
                CategoryId = category.Id, UnitId = unit.Id, CurrentCostYer = 8_000, Revision = 1 }]);

        Assert.AreEqual("أخشاب طبيعية", model.VisibleMaterials.Single().Category);
        Assert.AreEqual("متر", model.VisibleMaterials.Single().Unit);
        Assert.HasCount(0, model.ActiveCategories);
        Assert.HasCount(0, model.ActiveUnits);
        Assert.AreEqual(RawMaterialsViewModel.AllCategories, model.SelectedCategory);
        model.BeginEdit(model.VisibleMaterials.Single());
        Assert.AreEqual(category.Id, model.EditorCategories.Single().Id);
        Assert.AreEqual(unit.Id, model.EditorUnits.Single().Id);
        model.CancelEditor();
        model.BeginCreate();
        Assert.HasCount(0, model.EditorCategories);
        Assert.HasCount(0, model.EditorUnits);
    }

    [TestMethod]
    public void DurableEditorKeepsReferenceIdsWhenNamesChangeAndAreReused()
    {
        var model = new RawMaterialsViewModel();
        model.EnableDurableMode();
        var scope = new ScopeRef { Kind = "organization", Id = Guid.NewGuid() };
        var categoryId = Guid.NewGuid();
        var unitId = Guid.NewGuid();
        var material = new Material { Id = Guid.NewGuid(), Scope = scope, Name = "خشب زان",
            CategoryId = categoryId, UnitId = unitId, CurrentCostYer = 8_000, Revision = 1 };
        model.ApplyDurableData(new MaterialReferences
        {
            Categories = [new MaterialCategory { Id = categoryId, Scope = scope, Name = "أخشاب", Revision = 1 }],
            Units = [new MaterialUnit { Id = unitId, Scope = scope, Name = "متر", Revision = 1 }],
        }, [material]);
        model.BeginEdit(model.VisibleMaterials.Single());

        model.ApplyDurableData(new MaterialReferences
        {
            Categories =
            [
                new MaterialCategory { Id = categoryId, Scope = scope, Name = "أخشاب طبيعية", Revision = 2 },
                new MaterialCategory { Id = Guid.NewGuid(), Scope = scope, Name = "أخشاب", Revision = 1 },
            ],
            Units =
            [
                new MaterialUnit { Id = unitId, Scope = scope, Name = "متر طولي", Revision = 2 },
                new MaterialUnit { Id = Guid.NewGuid(), Scope = scope, Name = "متر", Revision = 1 },
            ],
        }, [material]);

        Assert.AreEqual(categoryId, model.EditorCategoryId);
        Assert.AreEqual(unitId, model.EditorUnitId);
        Assert.AreEqual("أخشاب طبيعية", model.EditorCategory);
        Assert.AreEqual("متر طولي", model.EditorUnit);
    }

    [TestMethod]
    public void RawMaterialsSearchAndFiltersUpdateVisibleList()
    {
        var model = new RawMaterialsViewModel();

        Assert.HasCount(4, model.VisibleMaterials);
        model.SearchText = "mdf";
        Assert.HasCount(1, model.VisibleMaterials);
        Assert.AreEqual("لوح MDF سماكة 18 مم", model.VisibleMaterials.Single().Name);

        model.SearchText = "زان";
        Assert.AreEqual("خشب زان مجفف", model.VisibleMaterials.Single().Name);
        model.SearchText = "اخشاب";
        Assert.HasCount(2, model.VisibleMaterials);

        model.SearchText = string.Empty;
        model.SelectedCategory = "أخشاب طبيعية";
        model.SelectedStatus = RawMaterialsViewModel.ArchivedStatus;
        Assert.HasCount(1, model.VisibleMaterials);
        Assert.IsTrue(model.VisibleMaterials.Single().IsArchived);

        model.SelectedStatus = RawMaterialsViewModel.ActiveStatus;
        Assert.HasCount(1, model.VisibleMaterials);
        Assert.AreEqual("خشب زان مجفف", model.VisibleMaterials.Single().Name);
    }

    [TestMethod]
    public void RawMaterialsActionsRemainNonDestructiveAndEphemeral()
    {
        var model = new RawMaterialsViewModel();
        var originalCount = model.VisibleMaterials.Count;
        var board = model.VisibleMaterials.Single(item => item.Name == "لوح MDF سماكة 18 مم");
        Assert.AreEqual("ر.ي 25,000", board.CostLabel);

        model.Archive(board);
        Assert.HasCount(originalCount, model.VisibleMaterials);
        Assert.IsTrue(board.IsArchived);

        var timber = model.VisibleMaterials.Single(item => item.Name == "خشب زان مجفف");
        var duplicate = model.Duplicate(timber);
        Assert.IsTrue(model.IsEditorOpen);
        Assert.IsTrue(duplicate.Name.EndsWith("نسخة", StringComparison.Ordinal));
        Assert.HasCount(originalCount + 1, model.VisibleMaterials);

        model.CancelEditor();
        model.BeginCreate();
        model.EditorName = "قماش صنعاء Fabric";
        model.EditorCategory = "أقمشة ومفروشات";
        model.EditorUnit = "متر";
        model.EditorCost = 4_200m;
        Assert.IsTrue(model.SaveEditor());
        Assert.HasCount(originalCount + 2, model.VisibleMaterials);

        model.BeginCreate();
        model.EditorName = "خامة بلا مرجع";
        model.EditorCategory = "تصنيف مؤرشف";
        model.EditorUnit = "متر";
        Assert.IsFalse(model.SaveEditor());
        model.EditorCategory = "أقمشة ومفروشات";
        model.EditorUnit = string.Empty;
        Assert.IsFalse(model.SaveEditor());
    }

    [TestMethod]
    public void RawMaterialReferencesCanBeManagedInline()
    {
        var model = new RawMaterialsViewModel();

        model.BeginAddCategory();
        model.ReferenceName = "إكسسوارات";
        Assert.IsTrue(model.SaveReferenceEditor());
        Assert.AreEqual("إكسسوارات", model.EditorCategory);
        Assert.IsTrue(model.ActiveCategories.Any(item => item.Name == "إكسسوارات"));

        model.BeginAddUnit();
        model.ReferenceName = "متر مربع";
        model.ReferenceShortName = "m²";
        Assert.IsTrue(model.SaveReferenceEditor());
        Assert.AreEqual("متر مربع", model.EditorUnit);
        Assert.AreEqual("لوح", model.ActiveUnits.First().DisplayLabel);
        var squareMeter = model.ActiveUnits.Single(item => item.Name == "متر مربع");
        Assert.AreEqual("متر مربع — m²", squareMeter.DisplayLabel);

        model.BeginManageUnits();
        model.BeginEditReference(squareMeter);
        model.ReferenceShortName = "م²";
        Assert.IsTrue(model.SaveReferenceEditor());
        Assert.AreEqual("متر مربع — م²", squareMeter.DisplayLabel);
        Assert.IsTrue(model.IsReferenceManagerOpen);

        model.ArchiveReference(squareMeter);
        Assert.IsTrue(squareMeter.IsArchived);
        Assert.DoesNotContain(squareMeter, model.ActiveUnits);
        Assert.IsFalse(model.IsReferenceEditorOpen);

        foreach (var unit in model.ActiveUnits.Skip(1).ToList())
        {
            model.ArchiveReference(unit);
        }

        var finalUnit = model.ActiveUnits.Single();
        model.ArchiveReference(finalUnit);
        Assert.IsFalse(finalUnit.IsArchived);
        Assert.HasCount(1, model.ActiveUnits);
    }

    [TestMethod]
    public void RawMaterialCostsIgnoreTheAmbientCulture()
    {
        var originalCulture = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("ar-YE");
            var model = new RawMaterialsViewModel();
            var board = model.VisibleMaterials.Single(item => item.Name == "لوح MDF سماكة 18 مم");

            Assert.AreEqual("25,000", board.CostAmountLabel);
        }
        finally
        {
            CultureInfo.CurrentCulture = originalCulture;
        }
    }
}
