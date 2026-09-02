using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class QuotationsRenderedTests
{
    [TestMethod]
    public void ManagerListAndApprovalDetailRenderAccessibleReviewActions()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "QuotationsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<QuotationsView>(window).Single();
            Assert.AreEqual(Visibility.Visible, view.Visibility);
            Assert.AreEqual("البحث برقم عرض السعر أو العميل", AutomationProperties.GetName(WpfTestHost.FindByName<TextBox>(view, "QuotationSearchBox")));
            Assert.AreEqual(2, WpfTestHost.Descendants<ComboBox>(view).Count());
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "عروض الأسعار"));
            Assert.IsTrue(view.ViewModel.VisibleQuotations.Select(item => item.StatusLabel).Contains("محوّل"));

            WpfTestHost.FindByAutomationName<Button>(view, "فتح عرض السعر")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            WpfTestHost.CompleteLayout(view);

            Assert.IsTrue(view.ViewModel.IsDetailVisible);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "BackToQuotationsButton").IsKeyboardFocusWithin);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Border>(view, "ApprovalSection").Visibility);
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "سعر الوحدة"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "الإجمالي النهائي"));

            WpfTestHost.FindByAutomationName<Button>(view, "الموافقة على خصم عرض السعر")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);

            Assert.IsFalse(view.ViewModel.SelectedQuotation!.HasPendingDiscountApproval);
            StringAssert.Contains(view.ViewModel.SelectedQuotation.ApprovalDecisionLabel, "الموافقة");
        });
    }

    [TestMethod]
    public void DetailWithoutDiscountApprovalHasNoVisibleManagerActions()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            WpfTestHost.FindByName<Button>(window, "QuotationsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<QuotationsView>(window).Single();
            var readOnlyQuotation = view.ViewModel.VisibleQuotations.Single(item => item.Status == QuotationStatus.Active);
            view.ViewModel.OpenQuotation(readOnlyQuotation);
            WpfTestHost.CompleteLayout(view);

            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Border>(view, "ApprovalSection").Visibility);
            Assert.IsFalse(WpfTestHost.Descendants<Button>(view).Any(button =>
                button.IsVisible
                && AutomationProperties.GetName(button) is "الموافقة على خصم عرض السعر" or "رفض خصم عرض السعر"));
        });
    }
}
