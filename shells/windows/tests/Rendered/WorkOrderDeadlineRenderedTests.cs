using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.WorkOrders;
using Eitmad.WindowsShell.Tests.Orders;
using Eitmad.WindowsShell.Tests.WorkOrders;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class WorkOrderDeadlineRenderedTests
{
    [TestMethod]
    [DataRow(true, 1920, 1080)]
    [DataRow(true, 1338, 753)]
    [DataRow(true, 720, 560)]
    [DataRow(false, 1920, 1080)]
    [DataRow(false, 1338, 753)]
    [DataRow(false, 720, 560)]
    public void RejectedStartRetainsInputsAndRendersDeadlineGuidance(bool fromOrders, int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            window.Left = SystemParameters.WorkArea.Left; window.Top = SystemParameters.WorkArea.Top;
            if (width == 1920) window.WindowState = WindowState.Maximized;
            var fixture = new WorkOrderAuthorityTests.Fixture();
            var orderFixture = new OrderAuthorityTests.Fixture(true);
            var engine = fromOrders ? orderFixture.Engine : fixture.Engine;
            orderFixture.Order.State = OrderState.Confirmed;
            orderFixture.Order.PermittedActions = [OrderPermittedAction.StartWork];
            orderFixture.Order.Work = [new() { Id = Guid.NewGuid(), Number = "WO-2026-00001", State = WorkState.Planned, LineIds = [Guid.NewGuid()] }];
            var client = new OrderClient(engine);
            var orders = new OrdersView(); var workOrders = new WorkOrdersView();
            try {
                engine.CommandHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed,
                    Payload = new() { Code = ProtocolIds.ErrorCodes.EitmadErrorOrderInvalidV1 } } };
                var selectedDate = new DateTime(2026, 10, 15);
                string message;
                if (fromOrders) {
                    orders.ViewModel.Attach(client); Finish(orders.ViewModel.ActivateAsync());
                    orders.ViewModel.OpenOrder(orders.ViewModel.VisibleOrders.Single());
                    window.Content = orders;
                    orders.ViewModel.Assignment = "ورشة تجريبية"; orders.ViewModel.DueDate = selectedDate;
                    orders.ViewModel.StartWork(); Finish(orders.ViewModel.LastAction);
                    Assert.AreEqual(selectedDate, orders.ViewModel.DueDate);
                    Assert.AreEqual("ورشة تجريبية", orders.ViewModel.Assignment);
                    Assert.IsTrue(orders.ViewModel.SelectedOrder!.CanStartWork);
                    message = orders.ViewModel.ActionNotice;
                } else {
                    workOrders.ViewModel.Attach(client); Finish(workOrders.ViewModel.ActivateAsync());
                    workOrders.ViewModel.OpenWorkOrder(workOrders.ViewModel.VisibleWorkOrders.Single());
                    window.Content = workOrders;
                    workOrders.ViewModel.Assignment = "ورشة تجريبية"; workOrders.ViewModel.DueDate = selectedDate;
                    workOrders.ViewModel.AdvanceSelectedStatus(); Finish(workOrders.ViewModel.LastAction);
                    Assert.AreEqual(selectedDate, workOrders.ViewModel.DueDate);
                    Assert.AreEqual("ورشة تجريبية", workOrders.ViewModel.Assignment);
                    Assert.IsTrue(workOrders.ViewModel.SelectedWorkOrder!.CanAdvance);
                    message = workOrders.ViewModel.ActionNotice;
                }
                WpfTestHost.CompleteLayout(window);
                var notice = WpfTestHost.Descendants<TextBlock>(window).First(text => text.IsVisible && text.Text == message);
                notice.BringIntoView();
                if (!fromOrders) WpfTestHost.Ancestor<ScrollViewer>(notice).ScrollToEnd();
                WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(notice.ActualHeight > 0);
                var dpi = VisualTreeHelper.GetDpi(window);
                Console.WriteLine($"Entry={(fromOrders ? "Orders" : "WorkOrders")}; requested={width}x{height}; actual={window.ActualWidth}x{window.ActualHeight} DIP; display={SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX}x{SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY} pixels; scale={dpi.DpiScaleX * 100}%; synthetic rejection; notice brought into view.");
                WpfTestHost.Capture(window, $"action-07-{(fromOrders ? "orders" : "work-orders")}-rejected-{width}");
            } finally {
                Finish(orders.ViewModel.DeactivateAsync()); workOrders.ViewModel.ClearWorkOrders();
                Finish(client.DisposeAsync().AsTask()); Finish(fixture.Engine.DisposeAsync().AsTask()); Finish(orderFixture.Engine.DisposeAsync().AsTask());
            }
        });
    }

    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(15));
        if (!task.IsCompleted) {
            var frame = new DispatcherFrame(); var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default);
            Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }
}
