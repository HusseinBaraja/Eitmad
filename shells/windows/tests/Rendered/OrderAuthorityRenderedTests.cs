using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Tests.Orders;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class OrderAuthorityRenderedTests
{
    [TestMethod]
    public void PendingRecoveryFollowsSelectedOrderAndRetainsExactRetry()
    {
        WpfTestHost.Run(1920, 1080, window => {
            window.WindowState = WindowState.Maximized;
            var fixture = new OrderAuthorityTests.Fixture(true);
            var client = new OrderClient(fixture.Engine);
            var view = new OrdersView(); view.ViewModel.Attach(client);
            try
            {
                Finish(view.ViewModel.ActivateAsync()); view.ViewModel.OpenOrder(view.ViewModel.VisibleOrders.Single());
                window.Content = view; WpfTestHost.CompleteLayout(window);
                var recovery = WpfTestHost.FindByName<StackPanel>(view, "OrderRecovery");
                var retry = WpfTestHost.FindByName<Button>(view, "RetrySelectedOrderButton");
                Assert.IsFalse(recovery.IsVisible, "Ordinary orders have no recovery action.");

                var unrelated = Pending(Guid.NewGuid());
                fixture.Page.Pending = [unrelated]; Refresh();
                Assert.IsFalse(recovery.IsVisible, "Another order's operation must not appear here.");
                view.ViewModel.RetrySelectedOrder(); Finish(view.ViewModel.LastAction);
                Assert.IsNull(fixture.Engine.LastCommand);

                var operation = Pending(fixture.Order.Id);
                operation.RejectedCode = ProtocolIds.ErrorCodes.EitmadErrorOrderInvalidV1;
                fixture.Page.Pending = [operation]; Refresh();
                Assert.IsTrue(recovery.IsVisible); Assert.IsFalse(retry.IsVisible);
                Assert.IsTrue(view.ViewModel.OrderPendingNotice.Contains("رفض"));

                operation = Pending(fixture.Order.Id);
                fixture.Page.Pending = [unrelated, operation]; fixture.Page.ServerAvailable = false;
                fixture.Order.PermittedActions = []; Refresh();
                retry.BringIntoView(); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(retry.IsVisible); Assert.IsTrue(retry.IsEnabled); Assert.IsTrue(retry.Focus());
                Assert.IsTrue(retry.IsKeyboardFocusWithin);
                Assert.AreEqual("إعادة محاولة عملية الطلب المعلقة", AutomationProperties.GetName(retry));
                Assert.IsTrue(view.ViewModel.OrderPendingNotice.Contains("بانتظار"));
                CaptureRecovery("order-unresolved-offline");

                var barrier = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
                fixture.Engine.CommandBarrier = _ => barrier.Task;
                fixture.Engine.CommandHandler = command => {
                    Assert.AreEqual(operation.Request.IdempotencyKey, fixture.Engine.LastIdempotencyKey);
                    Assert.AreEqual(fixture.Order.Id, command.AsOrderFulfillment()!.OrderId);
                    Assert.AreEqual("تغليف القطع", command.AsOrderFulfillment()!.Note);
                    throw new IOException("Synthetic unavailable reply");
                };
                retry.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(retry.IsVisible, "An in-flight retry must stay visible."); Assert.IsFalse(retry.IsEnabled);
                barrier.SetResult(); Finish(view.ViewModel.LastAction); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(retry.IsVisible); Assert.IsTrue(retry.IsEnabled);
                Assert.AreEqual(OrderState.Ready, view.ViewModel.SelectedOrder!.Record!.State);
                Assert.IsTrue(view.ViewModel.ActionNotice.Contains("بانتظار"));
                CaptureRecovery("order-retry-unavailable");

                fixture.Engine.CommandBarrier = null;
                fixture.Engine.CommandHandler = command => {
                    Assert.AreEqual(operation.Request.IdempotencyKey, fixture.Engine.LastIdempotencyKey);
                    Assert.AreEqual("تغليف القطع", command.AsOrderFulfillment()!.Note);
                    fixture.Page.Pending = [];
                    return new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = CommandResult.ForOrder(fixture.Order) } };
                };
                view.ViewModel.RetrySelectedOrder(); Finish(view.ViewModel.LastAction); WpfTestHost.CompleteLayout(window);
                Assert.IsFalse(recovery.IsVisible, "A confirmed operation no longer needs recovery.");

                // A lost reply still needs recovery if the following pending query fails.
                fixture.Order.PermittedActions = [OrderPermittedAction.EditFulfillment]; Refresh();
                fixture.Engine.CommandHandler = _ => { fixture.Engine.ThrowQueries = true; fixture.Engine.QueryHandler = null; throw new IOException("Synthetic lost reply"); };
                view.ViewModel.FulfillmentNote = "تغليف القطع"; view.ViewModel.SaveFulfillment(); Finish(view.ViewModel.LastAction);
                Assert.IsTrue(view.ViewModel.HasPending); Assert.IsTrue(view.ViewModel.HasUnresolvedOperation);
                Assert.IsTrue(view.ViewModel.PendingNotice.Contains("بانتظار"));

                void Refresh() { Finish(view.ViewModel.ActivateAsync()); WpfTestHost.CompleteLayout(window); }
                void CaptureRecovery(string name)
                {
                    WpfTestHost.Ancestor<ScrollViewer>(recovery).ScrollToEnd(); WpfTestHost.CompleteLayout(window);
                    WpfTestHost.Capture(window, name);
                }
                OrderPending Pending(Guid id) => new() { Request = new() {
                    Scope = fixture.Order.Scope, IdempotencyKey = Guid.NewGuid(), Action = new() {
                        ["kind"] = "editFulfillment", ["payload"] = JsonSerializer.SerializeToElement(new { orderId = id, expectedRevision = 1, note = "تغليف القطع" }) } } };
            }
            finally { Finish(view.ViewModel.DeactivateAsync()); Finish(client.DisposeAsync().AsTask()); Finish(fixture.Engine.DisposeAsync().AsTask()); }
        });
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ConfirmedOrderActionsRenderForBothRoles(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            var fixture = new OrderAuthorityTests.Fixture(true);
            var client = new OrderClient(fixture.Engine);
            try
            {
                var view = new OrdersView();
                view.ViewModel.Attach(client);
                Finish(view.ViewModel.ActivateAsync());
                view.ViewModel.OpenOrder(view.ViewModel.VisibleOrders.Single());
                window.Content = view;
                if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
                WpfTestHost.CompleteLayout(window);
                RecordGeometry(window, width, height);
                AssertReviewBeforeActions(view, "ManagerOrderItems");
                WpfTestHost.Capture(window, $"order-confirmed-manager-{width}");
                CaptureCompactItems(view, window, "ManagerOrderItems", "manager", width);

                var note = WpfTestHost.FindByAutomationName<TextBox>(view, "ملاحظات تنفيذ الطلب");
                note.BringIntoView(); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(note.IsVisible); Assert.IsTrue(note.Focus()); Assert.IsTrue(note.IsKeyboardFocusWithin);
                var save = WpfTestHost.FindByAutomationName<Button>(view, "حفظ ملاحظات تنفيذ الطلب");
                Assert.IsTrue(save.IsVisible); Assert.IsTrue(save.Focus());
                var fulfillment = WpfTestHost.FindByName<Border>(view, "FulfillmentSection");
                var cancellation = WpfTestHost.FindByName<Border>(view, "CancellationSection");
                AssertPrecedes(view, fulfillment, cancellation);
                Assert.IsFalse(WpfTestHost.Descendants<Button>(fulfillment).Any(button =>
                    AutomationProperties.GetName(button) == "إلغاء الطلب قبل التسليم"));
                var reason = WpfTestHost.FindByAutomationName<TextBox>(cancellation, "سبب إلغاء الطلب");
                Assert.IsTrue(reason.Focus());
                var cancel = WpfTestHost.FindByAutomationName<Button>(cancellation, "إلغاء الطلب قبل التسليم");
                cancel.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(cancel.Focus());
                WpfTestHost.Capture(window, $"order-manager-actions-{width}");
                Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
                Finish(view.ViewModel.DeactivateAsync());

                fixture.Order.PermittedActions = [OrderPermittedAction.Deliver];
                view = new OrdersView(); view.ConfigureReceptionist(); view.ViewModel.Attach(client);
                Finish(view.ViewModel.ActivateAsync()); view.ViewModel.OpenOrder(view.ViewModel.VisibleOrders.Single());
                window.Content = view; WpfTestHost.CompleteLayout(window);
                AssertReviewBeforeActions(view, "ReceptionOrderItems");
                WpfTestHost.Capture(window, $"order-ready-reception-{width}");
                CaptureCompactItems(view, window, "ReceptionOrderItems", "reception", width);
                var recipient = WpfTestHost.FindByAutomationName<TextBox>(view, "اسم مستلم الطلب");
                recipient.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(recipient.Focus());
                var methods = WpfTestHost.FindByAutomationName<ComboBox>(view, "طريقة قبول التسليم");
                Assert.IsTrue(methods.Focus()); methods.IsDropDownOpen = true;
                WpfTestHost.CompleteLayout(window); Assert.IsTrue(methods.IsDropDownOpen); methods.IsDropDownOpen = false;
                var deliver = WpfTestHost.FindByAutomationName<Button>(view, "تأكيد تسليم الطلب");
                deliver.BringIntoView(); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(deliver.IsVisible); Assert.IsTrue(deliver.Focus());
                Assert.IsFalse(WpfTestHost.FindByName<Border>(view, "CancellationSection").IsVisible);
                Assert.IsFalse(WpfTestHost.FindByName<Border>(view, "FulfillmentSection").IsVisible);
                WpfTestHost.Capture(window, $"order-reception-delivery-{width}");
                if (width == 720)
                {
                    foreach (var control in WpfTestHost.Descendants<Control>(view))
                        Eitmad.WindowsShell.Controls.ControlOptions.SetHighContrast(control, true);
                    foreach (var text in WpfTestHost.Descendants<TextBlock>(view)) text.FontSize *= 1.2;
                    deliver.BringIntoView(); WpfTestHost.CompleteLayout(window);
                    WpfTestHost.Capture(window, "order-delivery-larger-text-system-colors-720");
                }
                Finish(view.ViewModel.DeactivateAsync());
            }
            finally { Finish(client.DisposeAsync().AsTask()); Finish(fixture.Engine.DisposeAsync().AsTask()); }
        });
    }

    private static void AssertReviewBeforeActions(OrdersView view, string itemsName)
    {
        var identity = WpfTestHost.FindByName<Border>(view, "OrderIdentity");
        var items = WpfTestHost.FindByName<FrameworkElement>(view, itemsName);
        var production = WpfTestHost.FindByName<StackPanel>(view, "OrderProductionContext");
        var actions = WpfTestHost.FindByName<StackPanel>(view, "OrderActions");
        AssertPrecedes(view, identity, items);
        AssertPrecedes(view, items, production);
        AssertPrecedes(view, production, actions);
        Assert.AreEqual(Math.Min(720, WpfTestHost.Ancestor<StackPanel>(actions).ActualWidth), actions.ActualWidth, 1,
            "The action area must use the available width up to its reading limit.");
    }

    private static void CaptureCompactItems(OrdersView view, Window window, string itemsName, string role, int width)
    {
        if (width != 720) return;
        WpfTestHost.FindByName<FrameworkElement>(view, itemsName).BringIntoView();
        WpfTestHost.CompleteLayout(window);
        WpfTestHost.Capture(window, $"order-{role}-items-720");
    }

    private static void AssertPrecedes(OrdersView view, FrameworkElement first, FrameworkElement next) =>
        Assert.IsTrue(first.TranslatePoint(new Point(0, first.ActualHeight), view).Y <= next.TranslatePoint(new Point(), view).Y,
            "Order review content must precede its action forms, with cancellation separate from routine actions.");

    private static void RecordGeometry(Window window, int width, int height)
    {
        var dpi = VisualTreeHelper.GetDpi(window);
        var geometry = $"Order requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; "
            + $"display {SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX:0}x{SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY:0} px; "
            + $"scale {dpi.DpiScaleX * 100:0}%; WPF RenderTargetBitmap at 96 DPI; synthetic Rust responses.";
        Console.WriteLine(geometry);
        if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
        {
            System.IO.Directory.CreateDirectory(directory);
            System.IO.File.WriteAllText(System.IO.Path.Combine(directory, $"ui35-{width}.txt"), geometry);
        }
    }

    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(15));
        if (!task.IsCompleted)
        {
            var frame = new DispatcherFrame(); var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default);
            Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }
}
