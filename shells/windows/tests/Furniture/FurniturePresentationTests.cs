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

    /// <summary>Protects list-only search, lazy picker loading, and reuse of historical compositions after Part resync.</summary>
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

    /// <summary>Keeps an unsaved category on the information step and separates its error from numeric conversion errors.</summary>
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
    /// <summary>Verifies reopen retains exact part references and edit revision across refresh.</summary>
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
    /// <summary>Verifies duplicate stays unsaved with new option and variant identities.</summary>
    [TestMethod]
    public void DuplicateStaysUnsavedWithNewOptionAndVariantIdentities()
    {
        var f = new FurnitureFixtures(); var old = f.Seed(); var model = f.Model(); model.DuplicateFurniture(model.VisibleFurniture.Single());
        var input = model.SaveInput(FurnitureState.Draft); Assert.IsNull(input.Id); Assert.IsNull(input.ExpectedRevision);
        Assert.AreNotEqual(old.Variants[0].Id, input.Variants[0].Id); Assert.AreNotEqual(old.Colors[0].Id, input.Colors[0].Id); Assert.HasCount(1, model.VisibleFurniture);
    }
    /// <summary>Verifies inputs preserve exact millimetres and reject fractional money and part counts.</summary>
    [TestMethod]
    public void InputsPreserveExactMillimetresAndRejectFractionalMoneyAndPartCounts()
    {
        var f = new FurnitureFixtures(); f.Seed(); var model = f.Model(); model.BeginEdit(model.VisibleFurniture.Single()); model.Variants[0].Width = 120.1m;
        Assert.AreEqual(1201L, model.SaveInput(FurnitureState.Draft).Variants[0].Dimensions.WidthMm);
        Assert.ThrowsExactly<FormatException>(() => model.Variants[0].SellingPriceInput = "١٢٫٥"); model.Variants[0].SellingPriceInput = "٢٠٠٬٠٠٠"; Assert.AreEqual(200000m, model.Variants[0].SellingPrice);
        model.SelectedParts[0].Quantity = 1.5m; Assert.ThrowsExactly<FormatException>(() => model.SaveInput(FurnitureState.Draft));
    }
    /// <summary>Verifies customization and compatibility round trip without shell cost formula.</summary>
    [TestMethod]
    public void CustomizationAndCompatibilityRoundTripWithoutShellCostFormula()
    {
        var f = new FurnitureFixtures(); f.Seed(); var model = f.Model(); model.BeginEdit(model.VisibleFurniture.Single());
        model.BeginEditVariant(model.Variants[0]); model.AllowCustomization = true; model.MinWidth = 100; model.MaxWidth = 160; model.VariantColorChoices[0].Selected = true;
        Assert.IsTrue(model.SaveVariant()); var input = model.SaveInput(FurnitureState.Draft); Assert.AreEqual(1000L, input.Variants[0].Customization.Minimum.WidthMm); Assert.HasCount(1, input.Variants[0].ColorIds);
        model.ApplyReview(f.Review); Assert.AreEqual(18900m, model.CurrentPartsCost); Assert.AreEqual("181,100", model.Variants[0].MarginLabel);
    }
    /// <summary>Verifies unknown save retries exact key and refuses different payload.</summary>
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
    /// <summary>Verifies client maps conflict denial and typed subscriptions.</summary>
    [TestMethod]
    public async Task ClientMapsConflictDenialAndTypedSubscriptions()
    {
        var f = new FurnitureFixtures(); f.Seed(); var engine = f.Engine(); await using var client = new FurnitureClient(engine); await client.ActivateAsync(); Assert.AreEqual(2, engine.SubscriptionCount);
        var model = f.Model(); model.BeginEdit(model.VisibleFurniture.Single()); var input = model.SaveInput(FurnitureState.Draft);
        engine.CommandHandler = _ => FurnitureFixtures.Failure(ProtocolIds.ErrorCodes.EitmadErrorFurnitureRevisionConflictV1); Assert.AreEqual(FurnitureFailureKind.Conflict, await client.SaveAsync(input));
        engine.CommandHandler = _ => FurnitureFixtures.Failure(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1); Assert.AreEqual(FurnitureFailureKind.Denied, await client.SaveAsync(input));
    }
}
