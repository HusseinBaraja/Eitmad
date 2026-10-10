using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.WorkOrders;
using Eitmad.WindowsShell.Tests.WorkOrders;
namespace Eitmad.WindowsShell.Tests.Rendered;
[TestClass]
public sealed class WorkOrderAuthorityRenderedTests
{
    [TestMethod]
    [DataRow(1920,1080)]
    [DataRow(1338,753)]
    [DataRow(720,560)]
    public void ConfirmedProductionInputsRenderWithKeyboardAccess(int width,int height)
    {
        WpfTestHost.Run(width,height,window=>{
            var fixture = new WorkOrderAuthorityTests.Fixture(); var client = new OrderClient(fixture.Engine);
            try {
                var view = new WorkOrdersView(); view.ViewModel.Attach(client); Finish(view.ViewModel.ActivateAsync()); window.Content = view;
                if(width==1920){window.Left=0;window.WindowState=WindowState.Maximized;}
                WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window,$"work-confirmed-list-{width}");
                view.ViewModel.OpenWorkOrder(view.ViewModel.VisibleWorkOrders.Single()); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window,$"work-confirmed-detail-{width}");
                var assignment=WpfTestHost.FindByAutomationName<TextBox>(view,"مسؤول تنفيذ أمر العمل");assignment.BringIntoView();WpfTestHost.CompleteLayout(window);Assert.IsTrue(assignment.Focus());Assert.IsTrue(assignment.IsKeyboardFocusWithin);
                var due=WpfTestHost.FindByAutomationName<DatePicker>(view,"موعد تسليم أمر العمل");due.BringIntoView();WpfTestHost.CompleteLayout(window);
                var dateInput=WpfTestHost.Descendants<System.Windows.Controls.Primitives.DatePickerTextBox>(due).Single();Assert.IsTrue(dateInput.Focus());Assert.IsTrue(due.IsKeyboardFocusWithin);
                due.IsDropDownOpen=true; WpfTestHost.CompleteLayout(window);Assert.IsTrue(due.IsDropDownOpen);due.IsDropDownOpen=false;
                var advance=WpfTestHost.FindByAutomationName<Button>(view,"تغيير حالة أمر العمل");advance.BringIntoView();WpfTestHost.CompleteLayout(window);Assert.IsTrue(advance.IsEnabled);Assert.IsTrue(advance.Focus());
                Assert.AreEqual(FlowDirection.RightToLeft,view.FlowDirection);
                var dpi=VisualTreeHelper.GetDpi(window); Console.WriteLine($"Production requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scaling {dpi.DpiScaleX*100:0}%");
                WpfTestHost.Capture(window,$"work-confirmed-action-{width}"); view.ViewModel.ClearWorkOrders();
            } finally { Finish(client.DisposeAsync().AsTask()); Finish(fixture.Engine.DisposeAsync().AsTask()); }
        });
    }
    private static void Finish(Task task){task=task.WaitAsync(TimeSpan.FromSeconds(15));if(!task.IsCompleted){var frame=new DispatcherFrame();var dispatcher=Dispatcher.CurrentDispatcher;_=task.ContinueWith(_=>dispatcher.BeginInvoke(new Action(()=>frame.Continue=false)),TaskScheduler.Default);Dispatcher.PushFrame(frame);}task.GetAwaiter().GetResult();}
}
