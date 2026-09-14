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

    [TestMethod]
    public void FurnitureSelectionRendersOptionsTotalsAndKeepsCatalogFlow()
    {
        WpfTestHost.Run(1338, 1000, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "ProductsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(reception, "CatalogContent");
            var select = WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار خزانة السكينة");
            select.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var detail = WpfTestHost.FindByName<FurnitureSelectionView>(catalog, "SelectionView");
            Assert.IsTrue(detail.IsVisible);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(detail, "BackButton").IsKeyboardFocusWithin);
            var sizes = WpfTestHost.FindByName<ListBox>(detail, "SizesList");
            var colors = WpfTestHost.FindByName<ListBox>(detail, "ColorsList");
            var handles = WpfTestHost.FindByName<ListBox>(detail, "HandlesList");
            var add = WpfTestHost.FindByName<Button>(detail, "AddButton");
            Assert.IsFalse(add.IsEnabled);
            sizes.SelectedIndex = 2; colors.SelectedIndex = 1; handles.SelectedIndex = 1;
            WpfTestHost.CompleteLayout(window);
            var card = (ListBoxItem)sizes.ItemContainerGenerator.ContainerFromIndex(2);
            card.Focus();
            Assert.IsTrue(card.IsKeyboardFocusWithin);
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(card).Any(t => t.Text == "✓" && t.IsVisible));
            Assert.IsFalse(WpfTestHost.Descendants<TextBox>(detail).Any());
            Capture(window, "selection");
            var increase = WpfTestHost.FindByAutomationName<Button>(detail, "زيادة الكمية");
            increase.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("626,000 YER", ((FurnitureSelectionViewModel)detail.DataContext).LineTotalLabel);
            add.BringIntoView();
            WpfTestHost.CompleteLayout(window);
            add.Focus();
            Assert.IsTrue(add.IsKeyboardFocusWithin);
            add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(detail.IsVisible);
            var model = (SalesCatalogViewModel)catalog.DataContext;
            Assert.HasCount(1, model.QuotationLines);
            var title = WpfTestHost.FindByName<Eitmad.WindowsShell.Controls.ShellTitleBar>(reception, "ReceptionistTitleBar");
            Assert.AreEqual(model.QuotationLabel, title.PrimaryActionLabel);
            Capture(window, "selection-total");
            WpfTestHost.FindByAutomationName<Button>(reception, "فتح عرض السعر").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(model.IsReviewingQuotation);
            var continueButton = WpfTestHost.FindByAutomationName<Button>(catalog, "متابعة اختيار الأثاث");
            continueButton.Focus();
            Assert.IsTrue(continueButton.IsKeyboardFocusWithin);
            continueButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(model.IsReviewingQuotation);
            Assert.AreEqual(2, model.Selection!.Quantity);
            window.Width = 1000;
            WpfTestHost.CompleteLayout(window);
            Capture(window, "selection-compact");
            detail.Resources[SystemParameters.HighContrastKey] = true;
            detail.Resources[SystemColors.WindowBrushKey] = Brushes.Black;
            detail.Resources[SystemColors.WindowTextBrushKey] = Brushes.White;
            detail.Resources[SystemColors.ControlBrushKey] = Brushes.Black;
            detail.Resources[SystemColors.ControlTextBrushKey] = Brushes.White;
            WpfTestHost.CompleteLayout(window);
            Capture(window, "selection-system-colors");
            WpfTestHost.FindByName<Button>(detail, "BackButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(detail.IsVisible);
            Assert.IsTrue(select.IsKeyboardFocusWithin);
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
