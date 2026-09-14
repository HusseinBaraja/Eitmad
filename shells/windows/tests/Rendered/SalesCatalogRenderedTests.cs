using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class SalesCatalogRenderedTests
{
    [TestMethod]
    public void ReceptionCatalogSupportsNavigationSearchClearAndKeyboardSelection()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "ProductsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(reception, "CatalogContent");
            Assert.IsTrue(catalog.IsVisible);
            Assert.AreEqual(FlowDirection.RightToLeft, catalog.FlowDirection);
            Assert.AreEqual("الكل", WpfTestHost.FindByAutomationName<ListBox>(catalog, "فئات المنتجات").SelectedItem);
            Capture(window, "normal");
            var search = WpfTestHost.FindByName<TextBox>(catalog, "CatalogSearch");
            search.Focus();
            Assert.IsTrue(search.IsKeyboardFocusWithin);
            search.Text = "لا يوجد";
            WpfTestHost.CompleteLayout(window);
            var clear = WpfTestHost.FindByAutomationName<Button>(catalog, "مسح الفلاتر");
            Assert.IsTrue(clear.IsVisible);
            clear.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(search.IsKeyboardFocusWithin);
            Assert.AreEqual(string.Empty, search.Text);
            var categories = WpfTestHost.FindByAutomationName<ListBox>(catalog, "فئات المنتجات");
            categories.SelectedItem = "المراتب";
            WpfTestHost.CompleteLayout(window);
            Assert.HasCount(1, ((SalesCatalogViewModel)catalog.DataContext).VisibleItems);
            Capture(window, "retail");
            var select = WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار مرتبة طبية");
            select.Focus();
            Assert.IsTrue(select.IsKeyboardFocusWithin);
            select.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsFalse(string.IsNullOrEmpty(((SalesCatalogViewModel)catalog.DataContext).SelectionNotice));
            ((SalesCatalogViewModel)catalog.DataContext).ClearFilters();
            window.Width = 1000;
            WpfTestHost.CompleteLayout(window);
            Capture(window, "compact");
            WpfTestHost.FindByName<Button>(reception, "HomeNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(catalog.IsVisible);
            var catalogNav = WpfTestHost.FindByName<Button>(reception, "ProductsNavButton");
            Assert.IsTrue(catalogNav.IsVisible);
            catalogNav.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(catalog.IsVisible);
            catalog.Resources[SystemParameters.HighContrastKey] = true;
            catalog.Resources[SystemColors.WindowBrushKey] = Brushes.Black;
            catalog.Resources[SystemColors.WindowTextBrushKey] = Brushes.White;
            catalog.Resources[SystemColors.ControlBrushKey] = Brushes.Black;
            catalog.Resources[SystemColors.ControlTextBrushKey] = Brushes.White;
            catalog.Resources[SystemColors.GrayTextBrushKey] = Brushes.White;
            WpfTestHost.CompleteLayout(window);
            Capture(window, "system-colors");
        });
    }

    private static void Capture(FrameworkElement element, string size)
    {
        var directory = Environment.GetEnvironmentVariable("EITMAD_CATALOG_CAPTURE_DIR");
        if (string.IsNullOrWhiteSpace(directory)) return;
        Directory.CreateDirectory(directory);
        var bitmap = new RenderTargetBitmap((int)element.ActualWidth, (int)element.ActualHeight, 96, 96, PixelFormats.Pbgra32);
        bitmap.Render(element);
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        using var stream = File.Create(Path.Combine(directory, $"catalog-{size}.png"));
        encoder.Save(stream);
    }
}
