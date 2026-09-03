using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using Eitmad.WindowsShell.Features.Products;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class ProductsRenderedTests
{
    [TestMethod]
    public void ProductsNavigationRendersManagerListAndFocusedShortEditor()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "ProductsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<ProductsView>(window).Single();
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Grid>(view, "ListSurface").Visibility);
            var addButton = WpfTestHost.FindByAutomationName<Button>(view, "إضافة منتج");
            Assert.AreEqual("إضافة منتج", AutomationProperties.GetName(addButton));

            addButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();

            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Grid>(view, "EditorSurface").Visibility);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "ProductNameBox").IsKeyboardFocusWithin);
            Assert.AreEqual("حفظ المنتج", AutomationProperties.GetName(
                WpfTestHost.FindByAutomationName<Button>(view, "حفظ المنتج")));
            Assert.IsNotNull(WpfTestHost.FindByAutomationName<RadioButton>(view, "للمنتج خيارات مختلفة نعم"));
            Assert.AreEqual(
                Visibility.Collapsed,
                WpfTestHost.FindByAutomationName<Button>(view, "أرشفة المنتج من صفحة التعديل").Visibility);
        });
    }

    [TestMethod]
    public void ProductRowMenuUsesCompactActionsAndArchiveConfirmation()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "ProductsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<ProductsView>(window).Single();
            var action = WpfTestHost.FindByAutomationName<Button>(view, "إجراءات المنتج");

            action.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();

            Assert.IsNotNull(action.ContextMenu);
            Assert.IsTrue(action.ContextMenu.IsOpen);
            CollectionAssert.AreEquivalent(
                new[] { "تعديل", "تكرار", "أرشفة" },
                action.ContextMenu.Items.OfType<MenuItem>().Select(item => item.Header).Cast<string>().ToArray());

            var product = (ProductListItem)action.DataContext;
            view.ViewModel.RequestArchive(product);
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(view.ViewModel.IsArchiveConfirmationOpen);
            Assert.IsNotNull(WpfTestHost.FindByAutomationName<Button>(view, "تأكيد أرشفة المنتج"));
        });
    }
}
