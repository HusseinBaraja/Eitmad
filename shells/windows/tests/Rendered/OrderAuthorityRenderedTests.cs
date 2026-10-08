using System.Windows;
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
