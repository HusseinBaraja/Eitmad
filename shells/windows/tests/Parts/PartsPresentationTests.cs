using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Parts;
using Eitmad.WindowsShell.Features.RawMaterials;

namespace Eitmad.WindowsShell.Tests.Parts;

[TestClass]
public sealed class PartsPresentationTests
{
    [TestMethod]
    public async Task UnknownCategoryAndPartSavesKeepIndependentRetryState()
    {
        var fixture = new PartFixtures();
        await using var engine = fixture.Engine();
        await using var client = new PartClient(engine);
        var model = new PartsViewModel();
        model.ApplyDurableData(fixture.Snapshot());
        model.BeginEdit(model.VisibleParts.Single());
        var part = model.SaveInput();
        var category = new SavePartCategory { Name = "فئة تجريبية" };
        var accepted = engine.CommandHandler;
        engine.CommandHandler = _ => throw new System.IO.IOException("synthetic lost response");

        Assert.AreEqual(MaterialFailureKind.Unavailable, await client.SaveAsync(category));
        var categoryKey = engine.LastIdempotencyKey;
        Assert.AreEqual(MaterialFailureKind.Unavailable, await client.SaveAsync(part));
        var partKey = engine.LastIdempotencyKey;
        Assert.AreNotEqual(categoryKey, partKey);

        Assert.AreEqual(MaterialFailureKind.Conflict, await client.SaveAsync(new SavePartCategory { Name = "فئة أخرى" }));
        Assert.AreEqual(partKey, engine.LastIdempotencyKey);
        part.Name += " تعديل";
        Assert.AreEqual(MaterialFailureKind.Conflict, await client.SaveAsync(part));
        part.Name = model.EditorName;

        engine.CommandHandler = accepted;
        Assert.AreEqual(MaterialFailureKind.None, await client.SaveAsync(part));
        Assert.AreEqual(partKey, engine.LastIdempotencyKey);
        Assert.AreEqual(MaterialFailureKind.None, await client.SaveAsync(category));
        Assert.AreEqual(categoryKey, engine.LastIdempotencyKey);
        Assert.AreEqual(MaterialFailureKind.None, await client.SaveAsync(new SavePartCategory { Name = "فئة أخرى" }));
        Assert.AreNotEqual(categoryKey, engine.LastIdempotencyKey);
    }

    [TestMethod]
    public void CostReferenceRefreshRetainsArchivedUsageUnit()
    {
        var fixture = new PartFixtures();
        fixture.Units[0].Archived = true;
        fixture.Units[0].Revision = 2;
        var model = new PartsViewModel();
        model.ApplyDurableData(fixture.Snapshot());
        model.BeginEdit(model.VisibleParts.Single());
        var usage = model.SelectedMaterials[0];
        model.RefreshCostReferences();

        Assert.AreEqual(fixture.Units[0].Id, usage.SelectedUnit!.Record.Id);
        Assert.HasCount(1, usage.Units);
        Assert.AreEqual(fixture.Units[0].Id, model.SaveInput().Usages[0].UnitId);
        Assert.AreEqual(2, model.SaveInput().Usages[0].UnitRevision);
        Assert.AreEqual("1.2", model.SaveInput().Usages[0].Quantity);
    }

    [TestMethod]
    public void CostReferenceRefreshUsesLatestMatchingUnit()
    {
        var fixture = new PartFixtures();
        var model = new PartsViewModel();
        model.ApplyDurableData(fixture.Snapshot());
        model.BeginEdit(model.VisibleParts.Single());
        var previous = fixture.Units[0];
        var latest = new MaterialUnit
        {
            Id = previous.Id, Scope = previous.Scope, Name = "وحدة محدثة", Symbol = previous.Symbol,
            Dimension = previous.Dimension, Numerator = previous.Numerator, Denominator = previous.Denominator, Revision = 2,
        };
        fixture.Units[0] = latest;
        model.ApplyDurableData(fixture.Snapshot());
        model.RefreshCostReferences();

        Assert.AreSame(latest, model.SelectedMaterials[0].SelectedUnit!.Record);
        Assert.HasCount(1, model.SelectedMaterials[0].Units);
        Assert.AreEqual(2, model.SaveInput().Usages[0].UnitRevision);
    }

    [TestMethod]
    public void EditorPreservesExactInputAndDisplaysOnlyRustCost()
    {
        var fixture = new PartFixtures(); var model = new PartsViewModel(); model.ApplyDurableData(fixture.Snapshot());
        model.BeginCreate(); model.EditorName = "جانب خزانة"; Assert.IsTrue(model.MoveToMaterials());
        model.AddMaterial(model.FilteredMaterials.First()); model.SelectedMaterials[0].Quantity = "1.200000";
        model.AddMaterial(model.FilteredMaterials.First()); model.SelectedMaterials[1].Quantity = "3";
        Assert.AreEqual("—",model.TotalPartCostLabel);
        model.ApplyCost(fixture.Cost,review:true);
        Assert.AreEqual("9,450",model.TotalPartCostLabel);
        var input = model.SaveInput(); Assert.AreEqual("1.200000",input.Usages[0].Quantity);
        Assert.AreEqual(fixture.Category.Id,input.CategoryId); Assert.IsNull(input.Id);
        model.MoveToPreviousStep(); model.SelectedMaterials[0].Quantity = "0.0000001";
        Assert.AreEqual("—",model.TotalPartCostLabel);
        Assert.AreEqual("0.0000001",model.SaveInput().Usages[0].Quantity);
    }

    [TestMethod]
    public void BackgroundRefreshKeepsUnsavedFieldsAndExpectedPartRevision()
    {
        var fixture = new PartFixtures(); var model = new PartsViewModel(); model.ApplyDurableData(fixture.Snapshot());
        model.BeginEdit(model.VisibleParts.Single()); model.EditorName = "تعديل لم يحفظ";
        fixture.Category.Name = "فئة جديدة"; fixture.Category.Revision = 2;
        var old = fixture.Parts[0]; fixture.Parts[0] = new Part { Id = old.Id, Scope = old.Scope, Name = "تعديل آخر", CategoryId = old.CategoryId, Revision = 2, Description = old.Description, Cost = old.Cost, Composition = old.Composition };
        model.ApplyDurableData(fixture.Snapshot());
        Assert.AreEqual("تعديل لم يحفظ",model.EditorName);
        Assert.AreEqual(1,model.SaveInput().ExpectedRevision);
        Assert.AreEqual(fixture.Category.Id,model.EditorCategory!.Id);
        model.Fail(PartClient.ArabicMessage(MaterialFailureKind.Conflict));
        Assert.IsTrue(model.IsEditorOpen); StringAssert.Contains(model.EditorError,"لم تُحفظ تعديلاتك");
        model.CancelEditor(); model.BeginEdit(model.VisibleParts.Single());
        Assert.AreEqual(2,model.SaveInput().ExpectedRevision);
    }

    [TestMethod]
    public async Task TypedClientReusesRetryKeyAfterUnknownOutcomeAndMapsConflict()
    {
        var fixture = new PartFixtures(); await using var engine = fixture.Engine(); await using var client = new PartClient(engine);
        var loaded = await client.LoadAsync(""); Assert.IsTrue(loaded.Succeeded);
        var model = new PartsViewModel(); model.ApplyDurableData(loaded.Value!); model.BeginArchive(model.VisibleParts.Single());
        var input = model.SaveInput();
        Assert.IsTrue(input.Archived); Assert.AreEqual(3,model.CurrentStep);
        engine.CommandHandler = _ => throw new System.IO.IOException("synthetic lost response");
        Assert.AreEqual(MaterialFailureKind.Unavailable,await client.SaveAsync(input)); var key = engine.LastIdempotencyKey;
        engine.CommandHandler = _ => new CommandResponseEnvelope { RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Failed, Payload = new CommandResult { Code = ProtocolIds.ErrorCodes.EitmadErrorPartRevisionConflictV1 } } };
        Assert.AreEqual(MaterialFailureKind.Conflict,await client.SaveAsync(input)); Assert.AreEqual(key,engine.LastIdempotencyKey);
        Assert.AreEqual(input.Id,engine.LastCommand!.AsPartSave()!.Id);
        Assert.IsTrue(engine.LastCommand.AsPartSave()!.Archived);
    }

    [TestMethod]
    public async Task MaterialPickerUsesRustMatchesWithoutRewritingSearchText()
    {
        var fixture = new PartFixtures(); await using var engine = fixture.Engine(); await using var client = new PartClient(engine);
        var original = engine.QueryHandler;
        string? sentTerm = null;
        engine.QueryHandler = query =>
        {
            if (query.AsMaterialList() is { } list)
            {
                sentTerm = list.Term;
                return PartFixtures.Success(QueryResult.ForMaterials(new MaterialPage { Items = [fixture.Materials[0]] }));
            }
            return original!(query);
        };
        var model = new PartsViewModel(); model.ApplyDurableData(fixture.Snapshot());
        model.MaterialSearchText = "ام دي اف";
        Assert.HasCount(0,model.FilteredMaterials);
        var result = await client.SearchMaterialsAsync(model.MaterialSearchText);
        Assert.IsTrue(result.Succeeded); Assert.AreEqual("ام دي اف",sentTerm);
        model.ApplyMaterialSearchResults(result.Value!);
        Assert.AreEqual(fixture.Materials[0].Id,model.FilteredMaterials.Single().Id);
    }
}
