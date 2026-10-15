using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.WindowsShell.Controls;
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
                window.Left=SystemParameters.WorkArea.Left;window.Top=SystemParameters.WorkArea.Top;
                if(width==1920){window.WindowState=WindowState.Maximized;}
                WpfTestHost.CompleteLayout(window);
                var dpi=VisualTreeHelper.GetDpi(window);
                Console.WriteLine($"Production requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; work area {SystemParameters.WorkArea}; scaling {dpi.DpiScaleX*100:0}%");
                WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window,$"work-confirmed-list-{width}");
                view.ViewModel.OpenWorkOrder(view.ViewModel.VisibleWorkOrders.Single()); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window,$"work-confirmed-detail-{width}");
                var assignment=WpfTestHost.FindByAutomationName<TextBox>(view,"مسؤول تنفيذ أمر العمل");assignment.BringIntoView();WpfTestHost.CompleteLayout(window);Assert.IsTrue(assignment.Focus());Assert.IsTrue(assignment.IsKeyboardFocusWithin);
                var due=WpfTestHost.FindByAutomationName<DatePicker>(view,"موعد تسليم أمر العمل");due.BringIntoView();WpfTestHost.CompleteLayout(window);
                var dateInput=WpfTestHost.Descendants<DatePickerTextBox>(due).Single();Assert.IsTrue(dateInput.Focus());Assert.IsTrue(due.IsKeyboardFocusWithin);
                Assert.AreEqual(FlowDirection.LeftToRight,dateInput.FlowDirection);
                Assert.AreEqual(System.Windows.Markup.XmlLanguage.GetLanguage("ar-YE"),due.Language);
                VerifyNativeDateEntry(window,view,due,dateInput,assignment);
                var advance=WpfTestHost.FindByAutomationName<Button>(view,"تغيير حالة أمر العمل");advance.BringIntoView();WpfTestHost.CompleteLayout(window);Assert.IsTrue(advance.IsEnabled);Assert.IsTrue(advance.Focus());
                Assert.AreEqual(FlowDirection.RightToLeft,view.FlowDirection);
                WpfTestHost.Capture(window,$"work-confirmed-action-{width}");
                if(width==720) {
                    ControlOptions.SetHighContrast(due,true);ControlOptions.SetHighContrast(assignment,true);
                    dateInput.Focus();WpfTestHost.CompleteLayout(window);
                    WpfTestHost.Capture(window,"work-date-high-contrast-720");
                }
                view.ViewModel.ClearWorkOrders();
            } finally { Finish(client.DisposeAsync().AsTask()); Finish(fixture.Engine.DisposeAsync().AsTask()); }
        });
    }
    private static void VerifyNativeDateEntry(MainWindow window,WorkOrdersView view,DatePicker due,DatePickerTextBox input,TextBox assignment)
    {
        // Commit through the native editor, then through the native calendar. Both must update the bound date.
        var selected=new DateTime(2026,10,19);
        input.Text=selected.ToString("d",System.Globalization.CultureInfo.GetCultureInfo("ar-YE"));
        input.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next));
        WpfTestHost.CompleteLayout(window);
        Assert.AreEqual(selected,due.SelectedDate);
        Assert.AreEqual(selected,view.ViewModel.DueDate);
        due.IsDropDownOpen=true;WpfTestHost.CompleteLayout(window);
        var popup=(Popup)due.Template.FindName("PART_Popup",due);
        var calendar=WpfTestHost.Descendants<Calendar>(popup.Child).Single();
        Assert.AreEqual(FlowDirection.RightToLeft,calendar.FlowDirection);
        Assert.AreEqual(System.Windows.Markup.XmlLanguage.GetLanguage("ar-YE"),calendar.Language);
        calendar.SelectedDate=selected.AddDays(1);due.IsDropDownOpen=false;
        WpfTestHost.CompleteLayout(window);
        Assert.AreEqual(selected.AddDays(1),view.ViewModel.DueDate);
        view.ViewModel.DueDate=selected;WpfTestHost.CompleteLayout(window);
        Assert.AreEqual(assignment.ActualHeight,due.ActualHeight,1,"The adjacent inputs must have the same rendered height.");
        Assert.IsTrue(input.ActualHeight>=assignment.ActualHeight-4);
        due.IsEnabled=false;
        Assert.IsFalse(input.IsEnabled);
        Assert.IsFalse(((Button)due.Template.FindName("PART_Button",due)).IsEnabled);
        due.IsEnabled=true;
    }

    private static void Finish(Task task){task=task.WaitAsync(TimeSpan.FromSeconds(15));if(!task.IsCompleted){var frame=new DispatcherFrame();var dispatcher=Dispatcher.CurrentDispatcher;_=task.ContinueWith(_=>dispatcher.BeginInvoke(new Action(()=>frame.Continue=false)),TaskScheduler.Default);Dispatcher.PushFrame(frame);}task.GetAwaiter().GetResult();}
}
