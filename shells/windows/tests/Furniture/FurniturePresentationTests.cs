using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Furniture;

[TestClass]
public sealed class FurniturePresentationTests
{
    [TestMethod]
    public void MissingPartRevisionKeepsEditAndDuplicateClosed()
    {
        var fixture = new FurnitureFixtures();
        fixture.Seed();
        var model = new FurnitureViewModel();
        model.ApplyDurableData(fixture.Snapshot() with { Parts = [], Compositions = [] });
        var row = model.VisibleFurniture.Single();

        Assert.IsFalse(model.BeginEdit(row));
        model.DuplicateFurniture(row);
        Assert.IsFalse(model.IsEditorOpen);
        Assert.AreNotEqual("", model.DataStateText);
        Assert.HasCount(0, model.SelectedParts);
        Assert.HasCount(1, model.VisibleFurniture);
    }

    [TestMethod]
    public async Task SearchLoadsOnlyRowsAndEditorsReuseImmutableReferences()
    {
        var f = new FurnitureFixtures(); var saved = f.Seed();
        // The definition references an older revision than the current picker Part.
        saved.Parts[0].Reference = new CompositionReference { PartId = f.Parts.Parts[0].Id, Scope = f.Parts.Scope, Revision = 7, SchemaVersion = 1 };
        var historical = System.Text.Json.JsonSerializer.Deserialize<Part>(System.Text.Json.JsonSerializer.Serialize(f.Parts.Parts[0]))!;
        historical.Revision = 7; historical.Composition = saved.Parts[0].Reference;
        var engine = f.Engine(); var handler = engine.QueryHandler!; var queries = new List<string>();
        engine.QueryHandler = query => {
            queries.Add(query.Kind);
            return query.Kind == Query.PartCompositionGetKind
                ? Parts.PartFixtures.Success(QueryResult.ForPartComposition(historical)) : handler(query);
        };
        await using var client = new FurnitureClient(engine);
        await client.ActivateAsync();
        Assert.IsTrue((await client.LoadAsync("")).Succeeded);
        CollectionAssert.AreEqual(new[] { Query.FurnitureCategoryListKind, Query.FurnitureListKind }, queries);
        queries.Clear();
        Assert.IsTrue((await client.LoadAsync("خزانة", reloadCategories: false)).Succeeded);
        CollectionAssert.AreEqual(new[] { Query.FurnitureListKind }, queries);
        queries.Clear();
        Assert.IsTrue((await client.LoadEditorAsync(saved)).Succeeded);
        CollectionAssert.AreEqual(new[] { Query.PartListKind, Query.PartCategoryListKind, Query.PartCompositionGetKind }, queries);
        queries.Clear();
        Assert.IsTrue((await client.LoadEditorAsync(saved)).Succeeded);
        Assert.HasCount(0, queries);
        engine.SignalResync(Subscription.PartChangedSubscribeKind);
        var refreshed = await client.LoadEditorAsync(saved);
        Assert.IsTrue(refreshed.Succeeded);
        CollectionAssert.AreEqual(new[] { Query.PartListKind, Query.PartCategoryListKind }, queries);
        Assert.AreEqual(7L, refreshed.Value!.Compositions.Single(p => p.Revision == 7).Revision);
    }

    [TestMethod]
    public void UnsavedCategoryCannotContinueOrSaveAndWhitespaceResolvesSavedCategory()
    {
        var f = new FurnitureFixtures(); var model = f.Model(); model.BeginCreate(); model.EditorName = "خزانة";
        model.EditorCategory = "فئة جديدة";
        Assert.IsFalse(model.MoveToParts()); Assert.AreEqual(1, model.CurrentStep);
        Assert.ThrowsExactly<FurnitureViewModel.UnsavedCategoryException>(() => model.SaveInput(FurnitureState.Draft));
        model.EditorCategory = $" {f.Category.Name} ";
        Assert.IsTrue(model.MoveToParts()); Assert.AreEqual(f.Category.Id, model.SaveInput(FurnitureState.Draft).CategoryId);
    }
    [TestMethod]
    public void ReopenRetainsExactPartReferencesAndEditRevisionAcrossRefresh()
    {
        var f = new FurnitureFixtures(); var saved = f.Seed(); var model = f.Model(); model.BeginEdit(model.VisibleFurniture.Single());
        Assert.AreEqual(saved.Parts[0].Reference.Revision, model.SelectedParts[0].Part.Reference!.Revision);
        var color = model.Colors[0].Id; var handle = model.Handles[0].Id; model.EditorName = "تعديل غير محفوظ";
        saved.Revision = 2; model.ApplyDurableData(f.Snapshot());
        var input = model.SaveInput(FurnitureState.Draft); Assert.AreEqual(1L, input.ExpectedRevision); Assert.AreEqual("تعديل غير محفوظ", input.Name);
        Assert.AreEqual(color, input.Colors[0].Id); Assert.AreEqual(handle, input.Handles[0].Id);
        model.ClearSession(); Assert.IsFalse(model.CanManage); Assert.IsFalse(model.IsEditorOpen); Assert.AreEqual("", model.InternalNotes); Assert.HasCount(0, model.VisibleFurniture);
    }
    [TestMethod]
    public void DuplicateStaysUnsavedWithNewOptionAndVariantIdentities()
    {
        var f = new FurnitureFixtures(); var old = f.Seed(); var model = f.Model(); model.DuplicateFurniture(model.VisibleFurniture.Single());
        var input = model.SaveInput(FurnitureState.Draft); Assert.IsNull(input.Id); Assert.IsNull(input.ExpectedRevision);
        Assert.AreNotEqual(old.Variants[0].Id, input.Variants[0].Id); Assert.AreNotEqual(old.Colors[0].Id, input.Colors[0].Id); Assert.HasCount(1, model.VisibleFurniture);
    }
    [TestMethod]
    public void InputsPreserveExactMillimetresAndRejectFractionalMoneyAndPartCounts()
    {
        var f = new FurnitureFixtures(); f.Seed(); var model = f.Model(); model.BeginEdit(model.VisibleFurniture.Single()); model.Variants[0].Width = 120.1m;
        Assert.AreEqual(1201L, model.SaveInput(FurnitureState.Draft).Variants[0].Dimensions.WidthMm);
        Assert.ThrowsExactly<FormatException>(() => model.Variants[0].SellingPriceInput = "١٢٫٥"); model.Variants[0].SellingPriceInput = "٢٠٠٬٠٠٠"; Assert.AreEqual(200000m, model.Variants[0].SellingPrice);
        model.SelectedParts[0].Quantity = 1.5m; Assert.ThrowsExactly<FormatException>(() => model.SaveInput(FurnitureState.Draft));
    }
    [TestMethod]
    public void CustomizationAndCompatibilityRoundTripWithoutShellCostFormula()
    {
        var f = new FurnitureFixtures(); f.Seed(); var model = f.Model(); model.BeginEdit(model.VisibleFurniture.Single());
        model.BeginEditVariant(model.Variants[0]); model.AllowCustomization = true; model.MinWidth = 100; model.MaxWidth = 160; model.VariantColorChoices[0].Selected = true;
        Assert.IsTrue(model.SaveVariant()); var input = model.SaveInput(FurnitureState.Draft); Assert.AreEqual(1000L, input.Variants[0].Customization.Minimum.WidthMm); Assert.HasCount(1, input.Variants[0].ColorIds);
        model.ApplyReview(f.Review); Assert.AreEqual(18900m, model.CurrentPartsCost); Assert.AreEqual("181,100", model.Variants[0].MarginLabel);
    }
    [TestMethod]
    public void BelowCostAcknowledgementTracksReturnedReviewAndResetsForChangedInput()
    {
        var fixture = new FurnitureFixtures(); fixture.Seed(); var model = fixture.Model();
        model.BeginEdit(model.VisibleFurniture.Single());
        model.ApplyReview(fixture.Review);
        Assert.IsFalse(model.HasBelowCostVariants);
        model.Variants[0].SellingPrice = 15000;
        Assert.IsFalse(model.HasBelowCostVariants, "Unreviewed input must not invent a loss.");
        model.ApplyReview(new FurnitureReview { PartsCostYer = 18900, RowCostsYer = [18900], MarginsYer = [-3900] });
        Assert.AreSame(model.Variants[0], model.BelowCostVariants.Single());
        model.ConfirmBelowCost = true;
        Assert.IsTrue(model.SaveInput(FurnitureState.Active).ConfirmBelowCost);
        model.ApplyReview(new FurnitureReview { PartsCostYer = 18900, RowCostsYer = [18900], MarginsYer = [-3900] });
        Assert.IsTrue(model.ConfirmBelowCost, "An unchanged review retains its acknowledgement.");
        model.SelectedParts[0].Quantity = 3;
        Assert.IsFalse(model.ConfirmBelowCost);
        model.ConfirmBelowCost = true;
        model.ApplyReview(new FurnitureReview { PartsCostYer = 20000, RowCostsYer = [20000], MarginsYer = [-5000] });
        Assert.IsFalse(model.ConfirmBelowCost, "A changed returned loss needs a new acknowledgement.");
        model.ConfirmBelowCost = true;
        model.Variants[0].SellingPrice = 200000;
        Assert.IsFalse(model.ConfirmBelowCost);
        model.ApplyReview(fixture.Review);
        Assert.IsFalse(model.HasBelowCostVariants);
        Assert.IsFalse(model.SaveInput(FurnitureState.Active).ConfirmBelowCost);
        model.DuplicateVariant(model.Variants[0]);
        model.ApplyReview(new FurnitureReview { PartsCostYer = 18900, RowCostsYer = [18900], MarginsYer = [-3900, 181100] });
        Assert.AreSame(model.Variants[0], model.BelowCostVariants.Single());
        model.ApplyReview(new FurnitureReview { PartsCostYer = 18900, RowCostsYer = [18900], MarginsYer = [181100, -3900] });
        Assert.AreSame(model.Variants[1], model.BelowCostVariants.Single(), "Only the size with a returned loss needs acknowledgement.");
        model.RemoveVariant(model.Variants[1]);
        Assert.IsFalse(model.HasBelowCostVariants);
        model.ApplyReview(new FurnitureReview { PartsCostYer = 18900, RowCostsYer = [18900], MarginsYer = [-3900] });
        model.BeginEdit(model.VisibleFurniture.Single());
        Assert.IsFalse(model.HasBelowCostVariants, "Opening another review must not retain the previous loss.");
    }

    [TestMethod]
    public async Task UnknownSaveRetriesExactKeyAndRefusesDifferentPayload()
    {
        var f = new FurnitureFixtures(); f.Seed(); var engine = f.Engine(); var model = f.Model(); model.BeginEdit(model.VisibleFurniture.Single());
        await using var client = new FurnitureClient(engine); var input = model.SaveInput(FurnitureState.Draft); var accepted = engine.CommandHandler;
        engine.CommandHandler = _ => throw new System.IO.IOException("synthetic lost response"); Assert.AreEqual(FurnitureFailureKind.Unavailable, await client.SaveAsync(input)); var key = engine.LastIdempotencyKey;
        model.SaveUnconfirmed(input);
        model.BeginCreate(); model.BeginEdit(model.VisibleFurniture.Single()); model.DuplicateFurniture(model.VisibleFurniture.Single()); model.CancelEditor();
        Assert.IsFalse(model.CanEditFields); Assert.IsTrue(model.IsEditorOpen); Assert.AreSame(input, model.SaveInput(FurnitureState.Active));
        input.Name = "different"; Assert.AreEqual(FurnitureFailureKind.Conflict, await client.SaveAsync(input)); input.Name = "خزانة السكينة";
        engine.CommandHandler = accepted; Assert.AreEqual(FurnitureFailureKind.None, await client.SaveAsync(input)); Assert.AreEqual(key, engine.LastIdempotencyKey);
    }
    [TestMethod]
    public async Task ClientMapsConflictDenialAndTypedSubscriptions()
    {
        var f = new FurnitureFixtures(); f.Seed(); var engine = f.Engine(); await using var client = new FurnitureClient(engine); await client.ActivateAsync(); Assert.AreEqual(2, engine.SubscriptionCount);
        var model = f.Model(); model.BeginEdit(model.VisibleFurniture.Single()); var input = model.SaveInput(FurnitureState.Draft);
        engine.CommandHandler = _ => FurnitureFixtures.Failure(ProtocolIds.ErrorCodes.EitmadErrorFurnitureRevisionConflictV1); Assert.AreEqual(FurnitureFailureKind.Conflict, await client.SaveAsync(input));
        engine.CommandHandler = _ => FurnitureFixtures.Failure(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1); Assert.AreEqual(FurnitureFailureKind.Denied, await client.SaveAsync(input));
    }
}
