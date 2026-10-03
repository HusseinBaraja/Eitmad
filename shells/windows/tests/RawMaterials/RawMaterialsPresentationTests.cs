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
    public void FiltersUseAuthorityRowsAndDuplicationOnlyStagesUnsavedFields()
    {
        var model = MaterialFixtures.Model();
        model.SelectedCategory = "أخشاب طبيعية";
        model.SelectedStatus = RawMaterialsViewModel.ArchivedStatus;
        Assert.IsTrue(model.VisibleMaterials.Single().IsArchived);
        model.SelectedStatus = RawMaterialsViewModel.ActiveStatus;
        var original = model.VisibleMaterials.Single();
        model.Duplicate(original);
        Assert.IsTrue(model.IsEditorOpen);
        Assert.AreEqual(original.CategoryId, model.EditorCategoryId);
        Assert.AreEqual(original.UnitId, model.EditorUnitId);
        Assert.AreEqual(original.CurrentCost, model.EditorCost);
        Assert.IsNull(model.EditingMaterial);
        Assert.HasCount(1, model.VisibleMaterials);
    }

    [TestMethod]
    public void RawMaterialCostsIgnoreTheAmbientCulture()
    {
        var originalCulture = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("ar-YE");
            var model = MaterialFixtures.Model();
            var board = model.VisibleMaterials.Single(item => item.Name == "لوح MDF سماكة 18 مم");

            Assert.AreEqual("25,000", board.CostAmountLabel);
        }
        finally
        {
            CultureInfo.CurrentCulture = originalCulture;
        }
    }
}
