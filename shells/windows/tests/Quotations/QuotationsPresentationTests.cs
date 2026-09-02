using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Tests.Quotations;

[TestClass]
public sealed class QuotationsPresentationTests
{
    [TestMethod]
    public void SearchStatusAndDateFiltersComposeAcrossManagerRows()
    {
        var viewModel = new QuotationsViewModel();

        viewModel.SearchText = "المها";
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
        var viewModel = new QuotationsViewModel();
        var pendingApproval = viewModel.VisibleQuotations[0];

        viewModel.OpenQuotation(pendingApproval);

        Assert.IsTrue(viewModel.IsDetailVisible);
        Assert.AreEqual(480_000m, pendingApproval.Subtotal);
        Assert.AreEqual(72_000m, pendingApproval.Discount);
        Assert.AreEqual(408_000m, pendingApproval.FinalTotal);
        Assert.IsTrue(pendingApproval.HasPendingDiscountApproval);

        viewModel.ApproveDiscount();

        Assert.AreEqual(DiscountApprovalDecision.Approved, pendingApproval.ApprovalDecision);
        Assert.IsFalse(pendingApproval.HasPendingDiscountApproval);
        StringAssert.Contains(pendingApproval.ApprovalDecisionLabel, "الموافقة");

        var rejectionPreview = new QuotationsViewModel();
        rejectionPreview.OpenQuotation(rejectionPreview.VisibleQuotations[0]);
        rejectionPreview.RejectDiscount();
        Assert.AreEqual(DiscountApprovalDecision.Rejected, rejectionPreview.SelectedQuotation!.ApprovalDecision);
        StringAssert.Contains(rejectionPreview.SelectedQuotation.ApprovalDecisionLabel, "رفض");

        viewModel.CloseQuotation();
        var readOnlyQuotation = viewModel.VisibleQuotations[1];
        viewModel.OpenQuotation(readOnlyQuotation);
        viewModel.RejectDiscount();

        Assert.IsFalse(readOnlyQuotation.RequiresDiscountApproval);
        Assert.AreEqual(DiscountApprovalDecision.None, readOnlyQuotation.ApprovalDecision);
    }
}
