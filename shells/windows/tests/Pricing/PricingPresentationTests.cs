using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Pricing;

namespace Eitmad.WindowsShell.Tests.Pricing;

[TestClass]
public sealed class PricingPresentationTests
{
    [TestMethod]
    public void SyncRepairNoticeIsManagerOnlyAndClearsWhenSessionEnds()
    {
        var model = new PricingViewModel();
        var page = Data();
        page.CatalogSyncIssues = Enumerable.Range(1, 4).Select(index => new CatalogSyncIssue
            { Kind = "product-category", Id = Guid.NewGuid(), Revision = 1, Name = $"فئة اختبار {index}", Conflicted = index == 1 }).ToArray();
        model.ApplyDurableData(page);
        Assert.IsTrue(model.HasSyncIssues);
        CollectionAssert.AreEqual(page.CatalogSyncIssues, model.SyncIssues.ToArray());
        Assert.HasCount(2, model.VisiblePrices);
        model.ClearSession();
        Assert.IsFalse(model.HasSyncIssues);
        Assert.IsEmpty(model.SyncIssues);
        page.CanManage = false;
        model.ApplyDurableData(page);
        Assert.IsFalse(model.HasSyncIssues);
        Assert.IsEmpty(model.SyncIssues);
    }

    [TestMethod]
    public void UnconfirmedPublicationRetainsProposalAndRetryStateAcrossServerChanges()
    {
        var model = new PricingViewModel();
        var page = Data();
        model.ApplyDurableData(page);
        model.BeginEdit(model.VisiblePrices[0]);
        var proposal = model.SaveInput()!;
        var offlineState = model.EditorPublicationState;
        model.IsBusy = true;
        model.SavePending = true;
        model.Fail(PricingClient.ArabicMessage(PricingFailure.Unconfirmed));
        model.IsBusy = false;
        var retryState = model.EditorPublicationState;
        var retryLabel = model.SaveActionLabel;
        Assert.AreNotEqual(offlineState, retryState);
        Assert.IsFalse(model.CanEdit);
        Assert.IsTrue(model.CanSave);
        page.ServerAvailable = true;
        model.ApplyDurableData(page);
        Assert.AreEqual(retryState, model.EditorPublicationState);
        Assert.AreEqual(retryLabel, model.SaveActionLabel);
        Assert.AreEqual(proposal.SellingPriceYer, model.SaveInput()!.SellingPriceYer);
        Assert.AreEqual(proposal.ExpectedRevision, model.SaveInput()!.ExpectedRevision);
        Assert.IsTrue(model.HasEditorError);
        model.Saved();
        Assert.IsFalse(model.SavePending);
        Assert.IsFalse(model.IsEditorOpen);
        Assert.AreNotEqual(retryLabel, model.SaveActionLabel);
    }

    [TestMethod]
    public void StalePublishedPriceKeepsConfirmedAmountAndRequiresPublication()
    {
        var page = Data();
        page.Items[0].PublicationRequired = true;
        page.Items[1].PublicationRequired = true;
        page.Items[1].Published = null!;
        var model = new PricingViewModel();
        model.ApplyDurableData(page);
        var stale = model.VisiblePrices[0];
        var unpublished = model.VisiblePrices[1];
        Assert.IsTrue(stale.NeedsPublication);
        Assert.IsTrue(stale.IsActive);
        Assert.AreEqual(200_000L, stale.SellingPrice);
        Assert.IsFalse(unpublished.IsActive);
        Assert.IsNull(unpublished.SellingPrice);
        Assert.AreNotEqual(stale.StatusLabel, unpublished.StatusLabel);
        var staleLabel = stale.StatusLabel;
        page.Items[0].PublicationRequired = false;
        model.ApplyDurableData(page);
        Assert.IsFalse(model.VisiblePrices[0].NeedsPublication);
        Assert.AreNotEqual(staleLabel, model.VisiblePrices[0].StatusLabel);
    }

    internal static PricePage Data(bool costs = true) => new()
    {
        CanManage = costs, CanReadCosts = costs,
        Items = [Row("خزانة ملابس", "صغير", "غرف النوم", 160_000, 200_000, 40_000), Row("طاولة طعام", "ستة كراسي", "غرف الطعام", 285_000, 360_000, 75_000)],
    };
    internal static PriceItem Row(string name, string variant, string category, long cost, long price, long margin) => new()
    {
        Name = name, VariantName = variant, CategoryName = category, CostYer = cost, MarginYer = margin,
        Target = JsonSerializer.Deserialize<Dictionary<string, object>>(JsonSerializer.Serialize(PriceTarget.ForProduct(new ProductReference
        {
            Scope = new ScopeRef { Kind = "organization", Id = Guid.Parse("00000000-0000-0000-0000-000000000050") },
            ProductId = Guid.NewGuid(), VariantId = Guid.NewGuid(), Revision = 1, SchemaVersion = 1,
        })))!,
        Published = new PriceSummary { Currency = "YER", Revision = 1, SellingPriceYer = price, ConfirmedAt = 1000 },
    };
    [TestMethod]
    public void PriceEditorUsesOnlyReturnedMarginAndRejectsFractionalRials()
    {
        var model = new PricingViewModel(); model.ApplyDurableData(Data()); var row = model.VisiblePrices[0]; model.BeginEdit(row);
        model.EditorSellingPrice = "٢٢٠٬٠٠٠";
        Assert.AreEqual("—", model.EditorMargin);
        Assert.AreEqual(220_000L, model.SaveInput()!.SellingPriceYer);
        model.ApplyReview(220_000, new PriceReview { CostYer = 160_000, MarginYer = 60_000 });
        Assert.AreEqual("60,000 ر.ي", model.EditorMargin);
        Assert.AreEqual(200_000L, row.SellingPrice); // Staging and review cannot mutate a confirmed price.
        model.EditorSellingPrice = "220000.5";
        Assert.IsNull(model.SaveInput()); Assert.IsTrue(model.IsEditorOpen);
    }
    [TestMethod]
    public void SwitchingEqualPricesClearsOldReviewAndBelowCostConfirmation()
    {
        var data = Data(); data.Items[1].Published.SellingPriceYer = 200_000;
        var model = new PricingViewModel(); model.ApplyDurableData(data);
        model.BeginEdit(model.VisiblePrices[0]); model.ApplyReview(200_000, new PriceReview { CostYer = 210_000, MarginYer = -10_000, BelowCost = true });
        Assert.IsNull(model.SaveInput()); model.ConfirmBelowCost = true; Assert.IsTrue(model.SaveInput()!.ConfirmBelowCost);
        model.CancelEditor(); model.BeginEdit(model.VisiblePrices[1]);
        Assert.AreEqual("—", model.EditorMargin); Assert.IsFalse(model.ConfirmBelowCost);
        model.ApplyReview(200_000, new PriceReview { CostYer = 285_000, MarginYer = -85_000, BelowCost = true });
        Assert.AreEqual("-85,000 ر.ي", model.EditorMargin);
    }
    [TestMethod]
    public void ChangedPriceInvalidatesBelowCostReviewAndRejectsLateResults()
    {
        var model = new PricingViewModel(); model.ApplyDurableData(Data()); model.BeginEdit(model.VisiblePrices[0]);
        model.EditorSellingPrice = "150000";
        var belowCost = new PriceReview { CostYer = 160_000, MarginYer = -10_000, BelowCost = true };
        model.ApplyReview(150_000, belowCost);
        Assert.AreEqual("-10,000 ر.ي", model.EditorMargin);
        Assert.IsTrue(model.HasNegativeEditorMargin);
        Assert.IsFalse(model.CanSave);
        model.ConfirmBelowCost = true;
        Assert.IsTrue(model.CanSave);

        model.EditorSellingPrice = "220000.5";
        model.ApplyReview(150_000, belowCost);
        Assert.AreEqual("—", model.EditorMargin);
        Assert.IsFalse(model.HasNegativeEditorMargin);
        Assert.IsFalse(model.ConfirmBelowCost);
        Assert.IsFalse(model.CanSave);
        Assert.IsTrue(model.HasEditorError);
        Assert.IsNull(model.ReviewInput());
        Assert.IsNull(model.SaveInput());

        model.EditorSellingPrice = "220000";
        model.ApplyReview(150_000, belowCost);
        Assert.AreEqual("—", model.EditorMargin);
        Assert.IsFalse(model.HasNegativeEditorMargin);
        Assert.IsFalse(model.HasEditorError);
        model.ApplyReview(220_000, new PriceReview { CostYer = 160_000, MarginYer = 60_000 });
        Assert.AreEqual("60,000 ر.ي", model.EditorMargin);
        Assert.IsTrue(model.CanSave);
        Assert.IsFalse(model.SaveInput()!.ConfirmBelowCost);
    }

    [TestMethod]
    public void ReceptionistProjectionAndSessionClearRemoveInternalFieldsAndEditing()
    {
        var model = new PricingViewModel(); model.ApplyDurableData(Data()); model.BeginEdit(model.VisiblePrices[0]);
        model.ApplyDurableData(Data(false));
        Assert.IsFalse(model.IsEditorOpen); Assert.IsFalse(model.CanManage);
        Assert.IsTrue(model.VisiblePrices.All(row => row.Cost is null && row.Margin is null));
        model.BeginEdit(model.VisiblePrices[0]); Assert.IsFalse(model.IsEditorOpen);
        model.ClearSession(); Assert.HasCount(0, model.VisiblePrices); Assert.AreEqual("—", model.EditorCost);
    }
}
