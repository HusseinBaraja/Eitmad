using Eitmad.WindowsShell.Features.Orders;

namespace Eitmad.WindowsShell.Tests.Orders;

[TestClass]
public sealed class OrdersPresentationTests
{
    [TestMethod]
    public void SearchStatusAndDateFiltersComposeAcrossManagerRows()
    {
        var viewModel = new OrdersViewModel();

        viewModel.SearchText = "المها";
        Assert.HasCount(1, viewModel.VisibleOrders);
        Assert.AreEqual("ORD-2026-0087", viewModel.VisibleOrders[0].Number);

        viewModel.SearchText = string.Empty;
        viewModel.SelectedStatus = OrdersViewModel.InProductionStatus;
        viewModel.SelectedDate = OrdersViewModel.LastSevenDays;

        Assert.HasCount(1, viewModel.VisibleOrders);
        Assert.AreEqual(OrderStatus.InProduction, viewModel.VisibleOrders[0].Status);
    }

    [TestMethod]
    public void DetailCalculatesReviewTotalsAndExposesEveryRequiredStatus()
    {
        var viewModel = new OrdersViewModel();
        var order = viewModel.VisibleOrders[0];

        viewModel.OpenOrder(order);

        Assert.IsTrue(viewModel.IsDetailVisible);
        Assert.AreEqual(515_000m, order.Subtotal);
        Assert.AreEqual(35_000m, order.Discount);
        Assert.AreEqual(480_000m, order.FinalTotal);
        CollectionAssert.AreEquivalent(
            new[] { "جديد", "قيد الإنتاج", "جاهز", "تم التسليم", "ملغي" },
            viewModel.VisibleOrders.Select(item => item.StatusLabel).ToArray());

        viewModel.CloseOrder();
        Assert.IsTrue(viewModel.IsListVisible);
    }
}
