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
        page.CatalogSyncIssues = [new CatalogSyncIssue { Kind = "product-category", Id = Guid.NewGuid(), Revision = 1, Name = "فئة اختبار" }];
        model.ApplyDurableData(page);
        Assert.IsTrue(model.HasSyncIssues);
        Assert.HasCount(2, model.VisiblePrices);
        model.ClearSession();
        Assert.IsFalse(model.HasSyncIssues);
        page.CanManage = false;
        model.ApplyDurableData(page);
        Assert.IsFalse(model.HasSyncIssues);
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
        model.ApplyReview(new PriceReview { CostYer = 160_000, MarginYer = 60_000 });
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
        model.BeginEdit(model.VisiblePrices[0]); model.ApplyReview(new PriceReview { CostYer = 210_000, MarginYer = -10_000, BelowCost = true });
        Assert.IsNull(model.SaveInput()); model.ConfirmBelowCost = true; Assert.IsTrue(model.SaveInput()!.ConfirmBelowCost);
        model.CancelEditor(); model.BeginEdit(model.VisiblePrices[1]);
        Assert.AreEqual("—", model.EditorMargin); Assert.IsFalse(model.ConfirmBelowCost);
        model.ApplyReview(new PriceReview { CostYer = 285_000, MarginYer = -85_000, BelowCost = true });
        Assert.AreEqual("-85,000 ر.ي", model.EditorMargin);
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
