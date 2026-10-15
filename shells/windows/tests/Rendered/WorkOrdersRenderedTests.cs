using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Documents;
using Eitmad.WindowsShell.Features.WorkOrders;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class WorkOrdersRenderedTests
{
    [TestMethod]
    public void DeliveryDatesKeepDayMonthYearInArabicReadingOrder()
    {
        WpfTestHost.Run(1920, 1080, window =>
        {
            WpfTestHost.FindByName<Button>(window, "WorkOrdersNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var view = WpfTestHost.Descendants<WorkOrdersView>(window).Single();
            var source = view.ViewModel.VisibleWorkOrders[0];
            view.ViewModel.VisibleWorkOrders.Clear();
            foreach (var date in new DateOnly?[] { new(2026, 10, 21), new(2031, 1, 3), new(2027, 2, 12), null })
            {
                view.ViewModel.VisibleWorkOrders.Add(new WorkOrderListItem(Guid.NewGuid(), source.Number,
                    source.OrderNumber, source.Customer, source.AssignedTo, date, source.Status,
                    source.Furniture, source.Parts, source.Notes));
            }

            WpfTestHost.CompleteLayout(window);
            var labels = WpfTestHost.Descendants<TextBlock>(view).Where(text => text.IsVisible
                && text.DataContext is WorkOrderListItem item && text.Text == item.DueDateLabel).ToArray();
            Assert.HasCount(4, labels);
            foreach (var label in labels) AssertDeliveryDateReadingOrder(label);

            foreach (var item in view.ViewModel.VisibleWorkOrders)
            {
                view.ViewModel.OpenWorkOrder(item);
                WpfTestHost.CompleteLayout(window);
                var label = WpfTestHost.Descendants<TextBlock>(view)
                    .Single(text => text.IsVisible && text.Text == item.DueDateLabel);
                AssertDeliveryDateReadingOrder(label);
                var run = (Run)label.Inlines.FirstInline;
                var rightEdge = Math.Max(label.PointToScreen(new Point()).X,
                    label.PointToScreen(new Point(label.ActualWidth, 0)).X);
                var textRight = Enumerable.Range(0, label.Text.Length + 1)
                    .SelectMany(offset => new[] { LogicalDirection.Forward, LogicalDirection.Backward }
                        .Select(direction => label.PointToScreen(run.ContentStart.GetPositionAtOffset(offset)!
                            .GetCharacterRect(direction).TopLeft).X)).Max();
                Assert.AreEqual(rightEdge, textRight, 2, "The detail date must stay physically right aligned.");
            }
        });
    }

    private static void AssertDeliveryDateReadingOrder(TextBlock label)
    {
        if (label.Text == "لم يحدد") return;

        var run = (Run)label.Inlines.FirstInline;
        // RTL can mirror the TextBlock's local coordinates; compare physical positions.
        double CharacterX(int offset) => label.PointToScreen(run.ContentStart.GetPositionAtOffset(offset)!
            .GetCharacterRect(LogicalDirection.Forward).TopLeft).X;
        var month = label.Text.IndexOf(' ') + 1;
        var year = label.Text.LastIndexOf(' ') + 1;
        Assert.IsTrue(CharacterX(0) > CharacterX(month) && CharacterX(month) > CharacterX(year),
            $"Day, Arabic month and year must read from right to left: {label.Text}");
        for (var index = 0; index < month - 2; index++)
            Assert.IsTrue(CharacterX(index) < CharacterX(index + 1), "Day digits must read left to right.");
        for (var index = year; index < label.Text.Length - 1; index++)
            Assert.IsTrue(CharacterX(index) < CharacterX(index + 1), "Year digits must read left to right.");
    }

    [TestMethod]
    public void ManagerListAndDetailRenderManufacturingInformationWithoutCosting()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "WorkOrdersNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<WorkOrdersView>(window).Single();
            Assert.AreEqual(Visibility.Visible, view.Visibility);
            Assert.AreEqual("البحث في أوامر العمل", AutomationProperties.GetName(WpfTestHost.FindByName<TextBox>(view, "WorkOrderSearchBox")));
            Assert.AreEqual(2, WpfTestHost.Descendants<ComboBox>(view).Count());
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "الأثاث"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "مسند إلى"));
            Assert.IsFalse(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text.Contains("السعر", StringComparison.Ordinal) || text.Text.Contains("الربح", StringComparison.Ordinal)));

            WpfTestHost.FindByAutomationName<Button>(view, "فتح أمر العمل")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            WpfTestHost.CompleteLayout(view);

            Assert.IsTrue(view.ViewModel.IsDetailVisible);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "BackToWorkOrdersButton").IsKeyboardFocusWithin);
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "أمر عمل"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "#024" && text.FlowDirection == FlowDirection.LeftToRight));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "عدد قطع الأثاث: 2"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "خزانة السكينة"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "مكتب العمل الهادئ"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "الأجزاء المطلوبة"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "ملاحظات الطلب"));
            Assert.AreEqual(2, WpfTestHost.Descendants<Canvas>(view).Count(canvas => canvas.IsVisible));

            var statusAction = WpfTestHost.FindByName<Button>(view, "AdvanceStatusButton");
            Assert.AreEqual("تغيير حالة أمر العمل", AutomationProperties.GetName(statusAction));
            statusAction.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(WorkOrderStatus.InProgress, view.ViewModel.SelectedWorkOrder?.Status);
        });
    }

    [TestMethod]
    public void CompactWorkOrderListKeepsWideTableScrollable()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            WpfTestHost.FindByName<Button>(window, "WorkOrdersNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<WorkOrdersView>(window).Single();
            Assert.IsTrue(WpfTestHost.Descendants<ScrollViewer>(view).Any(scroll =>
                scroll.IsVisible && scroll.HorizontalScrollBarVisibility == ScrollBarVisibility.Auto));

            view.ViewModel.OpenWorkOrder(view.ViewModel.VisibleWorkOrders[0]);
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Grid>(view, "DetailSurface").Visibility);
        });
    }
}
