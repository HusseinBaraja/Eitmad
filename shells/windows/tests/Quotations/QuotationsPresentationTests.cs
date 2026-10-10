using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Tests.Quotations;

[TestClass]
public sealed class QuotationsPresentationTests
{
    [TestMethod]
    public void ReceptionPreviewPreservesSourceAndApprovalBoundary()
    {
        var model = new QuotationsViewModel(true, preview: true);
        var furniture = Eitmad.WindowsShell.Tests.Furniture.FurnitureFixtures.SalesModel();
        var products = new Features.Products.ProductsViewModel();
        foreach (var row in model.VisibleQuotations.Where(row => row.CanEdit))
        {
            var editor = Features.Reception.QuotationPreviewProjection.Create(row, furniture, products);
            Assert.AreEqual(row.Number, editor.QuotationNumber);
            Assert.AreEqual(row.Subtotal, editor.Subtotal);
            Assert.AreEqual(row.Discount, editor.Discount);
            Assert.AreEqual(row.FinalTotal, editor.FinalTotal);
            editor.QuotationLines.Clear();
            Assert.IsTrue(row.Items.Count > 0);
            model.OpenQuotation(row);
            model.ApproveDiscount();
            Assert.AreEqual(DiscountApprovalDecision.None, row.ApprovalDecision);
        }
        var empty = Features.Reception.QuotationPreviewProjection.Create(null, furniture, products);
        Assert.IsTrue(empty.IsQuotationEmpty);
        Assert.AreEqual("", empty.QuotationNumber);
    }

    [TestMethod]
    [DataRow("المها")]
    [DataRow("ٱلـمَهَا")]
    public void SearchStatusAndDateFiltersComposeAcrossManagerRows(string search)
    {
        var viewModel = new QuotationsViewModel(preview: true);

        viewModel.SearchText = search;
        Assert.HasCount(1, viewModel.VisibleQuotations);
        Assert.AreEqual("QT-2026-0142", viewModel.VisibleQuotations[0].Number);

        viewModel.SearchText = string.Empty;
        viewModel.SelectedStatus = QuotationsViewModel.ClosedStatus;
        Assert.HasCount(2, viewModel.VisibleQuotations);

        viewModel.SelectedDate = QuotationsViewModel.LastThirtyDays;
        Assert.HasCount(1, viewModel.VisibleQuotations);
        Assert.AreEqual(QuotationStatus.Cancelled, viewModel.VisibleQuotations[0].Status);
    }

    [TestMethod]
    public void DetailCalculatesTotalsAndLimitsActionsToRequiredDiscountApproval()
    {
        var viewModel = new QuotationsViewModel(preview: true);
        var pendingApproval = viewModel.VisibleQuotations[0];

        viewModel.OpenQuotation(pendingApproval);

        Assert.IsTrue(viewModel.IsDetailVisible);
        Assert.AreEqual(480_000m, pendingApproval.Subtotal);
        Assert.AreEqual(72_000m, pendingApproval.Discount);
        Assert.AreEqual(408_000m, pendingApproval.FinalTotal);
        Assert.IsFalse(pendingApproval.HasPendingDiscountApproval);
        Assert.IsFalse(viewModel.CanDecideApproval);

        viewModel.ApproveDiscount();

        Assert.AreEqual(DiscountApprovalDecision.None, pendingApproval.ApprovalDecision);
        Assert.IsFalse(pendingApproval.HasPendingDiscountApproval);
        Assert.AreEqual("", pendingApproval.ApprovalDecisionLabel);

        var rejectionPreview = new QuotationsViewModel(preview: true);
        rejectionPreview.OpenQuotation(rejectionPreview.VisibleQuotations[0]);
        rejectionPreview.RejectDiscount();
        Assert.AreEqual(DiscountApprovalDecision.None, rejectionPreview.SelectedQuotation!.ApprovalDecision);
        Assert.AreEqual("", rejectionPreview.SelectedQuotation.ApprovalDecisionLabel);

        viewModel.CloseQuotation();
        var readOnlyQuotation = viewModel.VisibleQuotations[1];
        viewModel.OpenQuotation(readOnlyQuotation);
        viewModel.RejectDiscount();

        Assert.IsFalse(readOnlyQuotation.RequiresDiscountApproval);
        Assert.AreEqual(DiscountApprovalDecision.None, readOnlyQuotation.ApprovalDecision);
    }
}
