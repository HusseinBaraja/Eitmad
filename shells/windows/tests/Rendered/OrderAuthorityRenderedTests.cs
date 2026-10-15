using System.Windows;
using System.IO;
using System.Text.Json;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
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
    [DataRow(1920,1080)]
    [DataRow(1338,753)]
    [DataRow(720,560)]
    public void ConfirmedOrderActionsRenderForBothRoles(int width,int height)
    {
        WpfTestHost.Run(width,height,window=>{
            var fixture=new OrderAuthorityTests.Fixture(true);var client=new OrderClient(fixture.Engine);
            try {
                var view=new OrdersView();view.ViewModel.Attach(client);Finish(view.ViewModel.ActivateAsync());view.ViewModel.OpenOrder(view.ViewModel.VisibleOrders.Single());window.Content=view;
                if(width==1920){window.Left=0;window.WindowState=WindowState.Maximized;}
                WpfTestHost.CompleteLayout(window);
                var note=WpfTestHost.FindByAutomationName<TextBox>(view,"ملاحظات تنفيذ الطلب");note.BringIntoView();WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(note.IsVisible);Assert.IsTrue(note.Focus());Assert.IsTrue(note.IsKeyboardFocusWithin);
                var save=WpfTestHost.FindByAutomationName<Button>(view,"حفظ ملاحظات تنفيذ الطلب");Assert.IsTrue(save.IsVisible);Assert.IsTrue(save.Focus());
                Assert.AreEqual(FlowDirection.RightToLeft,view.FlowDirection);WpfTestHost.Capture(window,$"order-confirmed-manager-{width}");
                Finish(view.ViewModel.DeactivateAsync());
                fixture.Order.PermittedActions=[OrderPermittedAction.Deliver];view=new OrdersView();view.ConfigureReceptionist();view.ViewModel.Attach(client);Finish(view.ViewModel.ActivateAsync());view.ViewModel.OpenOrder(view.ViewModel.VisibleOrders.Single());window.Content=view;WpfTestHost.CompleteLayout(window);
                var recipient=WpfTestHost.FindByAutomationName<TextBox>(view,"اسم مستلم الطلب");recipient.BringIntoView();WpfTestHost.CompleteLayout(window);Assert.IsTrue(recipient.Focus());
                var methods=WpfTestHost.FindByAutomationName<ComboBox>(view,"طريقة قبول التسليم");Assert.IsTrue(methods.Focus());methods.IsDropDownOpen=true;WpfTestHost.CompleteLayout(window);Assert.IsTrue(methods.IsDropDownOpen);methods.IsDropDownOpen=false;
                var deliver=WpfTestHost.FindByAutomationName<Button>(view,"تأكيد تسليم الطلب");Assert.IsTrue(deliver.IsVisible);Assert.IsTrue(deliver.Focus());
                Assert.IsFalse(WpfTestHost.FindByAutomationName<Button>(view,"إلغاء الطلب قبل التسليم").IsVisible);
                var dpi=VisualTreeHelper.GetDpi(window);Console.WriteLine($"Order requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scaling {dpi.DpiScaleX*100:0}%");
                WpfTestHost.Capture(window,$"order-ready-reception-{width}");
                Finish(view.ViewModel.DeactivateAsync());
            } finally {Finish(client.DisposeAsync().AsTask());Finish(fixture.Engine.DisposeAsync().AsTask());}
        });
    }
    private static void Finish(Task task){task=task.WaitAsync(TimeSpan.FromSeconds(15));if(!task.IsCompleted){var frame=new DispatcherFrame();var dispatcher=Dispatcher.CurrentDispatcher;_=task.ContinueWith(_=>dispatcher.BeginInvoke(new Action(()=>frame.Continue=false)),TaskScheduler.Default);Dispatcher.PushFrame(frame);}task.GetAwaiter().GetResult();}
}
