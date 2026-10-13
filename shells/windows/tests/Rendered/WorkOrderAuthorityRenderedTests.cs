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
    public void OrderNotesKeepEmptyStateCompactAndPreserveWrappedAuthorityText(int width,int height)
    {
        WpfTestHost.Run(width,height,window=>{
            var fixture = new WorkOrderAuthorityTests.Fixture(); var client = new OrderClient(fixture.Engine);
            try {
                var view = new WorkOrdersView(); view.ViewModel.Attach(client); Finish(view.ViewModel.ActivateAsync()); window.Content = view;
                if(width==1920) window.WindowState=WindowState.Maximized;
                WpfTestHost.CompleteLayout(window);
                var section = WpfTestHost.FindByName<Grid>(view,"NotesSection");
                var empty = WpfTestHost.FindByName<TextBlock>(view,"EmptyNotesText");
                var text = WpfTestHost.FindByName<TextBlock>(view,"NotesText");
                var dpi = VisualTreeHelper.GetDpi(window);
                var geometry = $"Requested {width}x{height}; actual window {window.ActualWidth}x{window.ActualHeight} DIP; primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX*100:0}%; WPF RenderTargetBitmap at 96 DPI; synthetic authority responses.";
                Console.WriteLine(geometry);
                if(Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory) {
                    System.IO.Directory.CreateDirectory(directory);
                    System.IO.File.WriteAllText(System.IO.Path.Combine(directory,$"ui37-{width}.txt"),geometry);
                }
                foreach(var note in new string?[]{null,""," \t\n "}) {
                    fixture.Work.Note = note!;
                    view.ViewModel.OpenWorkOrder(WorkOrdersViewModel.Project(fixture.Work));
                    WpfTestHost.CompleteLayout(window); section.BringIntoView(); WpfTestHost.CompleteLayout(window);
                    Assert.IsTrue(empty.IsVisible);
                    Assert.IsFalse(text.IsVisible);
                    Assert.IsTrue(section.ActualHeight<=40,"Missing notes must use a compact row before assignment fields.");
                    if(note is null) {
                        WpfTestHost.FindByName<Button>(view,"AdvanceStatusButton").BringIntoView(); WpfTestHost.CompleteLayout(window);
                        WpfTestHost.Capture(window,$"ui37-empty-{width}");
                    }
                }
                const string suppliedNote = "  يرجى مراجعة اتجاه فتح الأبواب مع العميل قبل بدء التصنيع.\nتُحفظ المقاسات المعتمدة كما وردت في الطلب، مع التأكد من اللون والمقابض قبل التجميع. تواصل مع مسؤول التنفيذ عند الحاجة إلى توضيح، ولا تغيّر المواصفات دون الرجوع إلى الطلب المعتمد.  ";
                fixture.Work.Note = suppliedNote;
                view.ViewModel.OpenWorkOrder(WorkOrdersViewModel.Project(fixture.Work));
                WpfTestHost.CompleteLayout(window); section.BringIntoView(); WpfTestHost.CompleteLayout(window);
                Assert.IsFalse(empty.IsVisible);
                Assert.IsTrue(text.IsVisible);
                Assert.AreEqual(suppliedNote,text.Text,"The authority note must retain whitespace and line breaks.");
                Assert.IsTrue(text.ActualHeight>24,"The supplied line break must remain visible.");
                if(width==720) Assert.IsTrue(text.ActualHeight>48,"Long notes must wrap at the minimum window size.");
                WpfTestHost.FindByName<Button>(view,"AdvanceStatusButton").BringIntoView(); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window,$"ui37-notes-{width}");
                if(width==720) {
                    ControlOptions.SetHighContrast(section,true); WpfTestHost.CompleteLayout(window);
                    WpfTestHost.Capture(window,"ui37-notes-high-contrast-720");
                }
                view.ViewModel.ClearWorkOrders();
            } finally { Finish(client.DisposeAsync().AsTask()); Finish(fixture.Engine.DisposeAsync().AsTask()); }
        });
    }

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
                VerifyCalendarAtScreenEdges(window,due,width);
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

    private static void VerifyCalendarAtScreenEdges(MainWindow window,DatePicker due,int width)
    {
        var workArea=SystemParameters.WorkArea;
        foreach(var edge in new[]{"left","right"}) {
            if(window.WindowState!=WindowState.Maximized) {
                window.Left=edge=="left"?workArea.Left:workArea.Right-window.ActualWidth;
                window.Top=Math.Max(workArea.Top,SystemParameters.PrimaryScreenHeight-window.ActualHeight);
            }
            due.BringIntoView();WpfTestHost.CompleteLayout(window);
            ((Button)due.Template.FindName("PART_Button",due)).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);Assert.IsTrue(due.IsDropDownOpen);
            var popup=(Popup)due.Template.FindName("PART_Popup",due);
            var calendar=WpfTestHost.Descendants<Calendar>(popup.Child).Single();
            var root=(FrameworkElement)PresentationSource.FromVisual(calendar).RootVisual;
            var origin=root.PointToScreen(new Point());
            var dpi=VisualTreeHelper.GetDpi(root);
            var bounds=new Rect(origin,new Size(root.ActualWidth*dpi.DpiScaleX,root.ActualHeight*dpi.DpiScaleY));
            // Native WPF popups can overlap the taskbar, but must fit on the display.
            var screenBounds=new Rect(0,0,SystemParameters.PrimaryScreenWidth*dpi.DpiScaleX,SystemParameters.PrimaryScreenHeight*dpi.DpiScaleY);
            Assert.IsTrue(screenBounds.Contains(bounds),$"Calendar {bounds} must remain inside {screenBounds} at the {edge} edge.");
            Console.WriteLine($"Calendar {width}, {edge} edge: {bounds} physical pixels; window {window.Left},{window.Top} DIP.");
            WpfTestHost.Capture(calendar,$"work-date-calendar-{width}-{edge}");
            due.IsDropDownOpen=false;WpfTestHost.CompleteLayout(window);
        }
    }
    private static void Finish(Task task){task=task.WaitAsync(TimeSpan.FromSeconds(15));if(!task.IsCompleted){var frame=new DispatcherFrame();var dispatcher=Dispatcher.CurrentDispatcher;_=task.ContinueWith(_=>dispatcher.BeginInvoke(new Action(()=>frame.Continue=false)),TaskScheduler.Default);Dispatcher.PushFrame(frame);}task.GetAwaiter().GetResult();}
}
