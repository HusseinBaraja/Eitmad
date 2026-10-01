using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Furniture;

[TestClass]
public sealed class FurniturePresentationTests
{
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
