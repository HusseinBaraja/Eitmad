using Eitmad.WindowsShell.Features.WorkOrders;

namespace Eitmad.WindowsShell.Tests.WorkOrders;

[TestClass]
public sealed class WorkOrdersPresentationTests
{
    [TestMethod]
    public void SearchStatusAndDueDateFiltersComposeAcrossProductionRows()
    {
        var viewModel = new WorkOrdersViewModel();

        viewModel.SearchText = "خزانه";
        Assert.HasCount(2, viewModel.VisibleWorkOrders);

        viewModel.SearchText = "هدى";
        viewModel.SelectedStatus = WorkOrdersViewModel.InProgressStatus;
        viewModel.SelectedDueDate = WorkOrdersViewModel.NextSevenDays;

        Assert.HasCount(1, viewModel.VisibleWorkOrders);
        Assert.AreEqual("WO-023", viewModel.VisibleWorkOrders[0].Number);

        viewModel.SearchText = string.Empty;
        viewModel.SelectedStatus = WorkOrdersViewModel.AllStatuses;
        viewModel.SelectedDueDate = WorkOrdersViewModel.Overdue;

        Assert.HasCount(1, viewModel.VisibleWorkOrders);
        Assert.AreEqual("WO-018", viewModel.VisibleWorkOrders[0].Number);
    }

    [TestMethod]
    public void DetailExposesBuildSpecificationAndAdvancesOnlyActiveStatuses()
    {
        var viewModel = new WorkOrdersViewModel();
        var workOrder = viewModel.VisibleWorkOrders.Single(item => item.Number == "WO-024");

        viewModel.OpenWorkOrder(workOrder);

        Assert.AreEqual("#024", workOrder.DetailNumberLabel);
        Assert.AreEqual("خزانة كبيرة", workOrder.Furniture[0].Name);
        Assert.AreEqual("200 × 220 × 60 سم", workOrder.Furniture[0].Dimensions);
        Assert.HasCount(4, workOrder.Parts);
        Assert.IsTrue(viewModel.AdvanceSelectedStatus());
        Assert.AreEqual(WorkOrderStatus.InProgress, workOrder.Status);
        StringAssert.Contains(viewModel.FeedbackMessage, "المعاينة المحلية فقط");
        Assert.IsTrue(viewModel.AdvanceSelectedStatus());
        Assert.AreEqual(WorkOrderStatus.Completed, workOrder.Status);
        Assert.IsFalse(viewModel.AdvanceSelectedStatus());
        Assert.IsFalse(workOrder.CanAdvance);

        CollectionAssert.AreEquivalent(
            new[] { "جديد", "قيد التنفيذ", "مكتمل", "ملغي" },
            new WorkOrdersViewModel().VisibleWorkOrders.Select(item => item.StatusLabel).Distinct().ToArray());
    }
}
