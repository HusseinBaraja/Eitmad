using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using Eitmad.WindowsShell.Features.Pricing;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class PricingRenderedTests
{
    [TestMethod]
    public void PricingListAndFocusedPriceEditorRenderAccessibleInteractions()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PricingNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<PricingView>(window).Single();
            Assert.AreEqual(Visibility.Visible, view.Visibility);
            Assert.AreEqual("البحث عن منتج أو مقاس", AutomationProperties.GetName(WpfTestHost.FindByName<TextBox>(view, "PricingSearchBox")));
            Assert.IsGreaterThan(0, view.ViewModel.VisiblePrices.Count);
            Assert.IsFalse(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text is "المواد الخام" or "الأجزاء"));

            var original = view.ViewModel.VisiblePrices[0];
            WpfTestHost.FindByAutomationName<Button>(view, "تعديل سعر البيع")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();

            var input = WpfTestHost.FindByName<TextBox>(view, "PriceInput");
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.IsTrue(input.IsKeyboardFocusWithin);
            Assert.AreEqual(original.Product, view.ViewModel.EditorProduct);
            Assert.AreEqual(original.Variant, view.ViewModel.EditorVariant);

            input.Text = "غير صالح";
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ سعر البيع")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.IsTrue(input.IsKeyboardFocusWithin);

            input.Text = "220000";
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ سعر البيع")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);

            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(220_000m, original.SellingPrice);
            StringAssert.Contains(view.ViewModel.FeedbackMessage, "المعاينة المحلية فقط");
        });
    }

    [TestMethod]
    public void PricingEditorCancelLeavesThePriceUnchanged()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PricingNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PricingView>(window).Single();
            var original = view.ViewModel.VisiblePrices[0];
            var price = original.SellingPrice;

            WpfTestHost.FindByAutomationName<Button>(view, "تعديل سعر البيع")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            WpfTestHost.FindByName<TextBox>(view, "PriceInput").Text = "999999";
            WpfTestHost.FindByAutomationName<Button>(view, "إلغاء تعديل السعر")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(price, original.SellingPrice);
        });
    }
}
