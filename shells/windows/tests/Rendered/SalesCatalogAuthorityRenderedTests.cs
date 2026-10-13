using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using System.IO;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class SalesCatalogAuthorityRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void SelectionReviewStaysVisibleWhileOptionsScrollInReceptionShell(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            if (width == 1920) window.WindowState = WindowState.Maximized;
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "ProductsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(reception, "CatalogContent");
            var model = (SalesCatalogViewModel)catalog.DataContext;
            var entry = SalesCatalogAuthorityTests.Entry(true);
            entry.Colors = [new() { Id = Guid.NewGuid(), Name = "أبيض", Visual = "#FFFFFF", PriceAdjustmentYer = 0 }];
            entry.Handles = [new() { Id = Guid.NewGuid(), Name = "مقبض H-12", Visual = "BlackMetal", PriceAdjustmentYer = 0 }];
            var engine = new FakeEngine { QueryHandler = q => SalesCatalogAuthorityTests.Handle(q, entry) };
            var client = new SalesCatalogClient(engine);
            model.AttachCatalogClient(client);
            try
            {
                Finish(model.ActivateCatalogAsync());
                model.Select(model.VisibleItems[0]); Finish(model.LastCatalogOperation);
                WpfTestHost.CompleteLayout(window);
                var furniture = WpfTestHost.FindByName<FurnitureSelectionView>(catalog, "SelectionView");
                Assert.IsFalse(furniture.AddButton.IsEnabled);
                model.Selection!.SelectedSize = model.Selection.Sizes[0];
                model.Selection.SelectedColor = model.Selection.Colors[0];
                model.Selection.SelectedHandle = model.Selection.Handles[0];
                model.Selection.WidthCm = "١٣٠٫٥";
                model.Selection.Quantity = 2; Finish(model.LastCatalogOperation);
                WpfTestHost.CompleteLayout(window);
                RecordGeometry(window, furniture, width, height);
                AssertReviewFits(furniture);
                var options = WpfTestHost.FindByName<ScrollViewer>(furniture, "SelectionOptions");
                options.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                AssertReviewFits(furniture);
                options.ScrollToTop(); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, $"ui28-furniture-reception-{width}x{height}");

                model.CloseSelection(); entry = SalesCatalogAuthorityTests.Entry();
                model.ClearFilters(); Finish(model.LastCatalogOperation);
                model.Select(model.VisibleItems[0]); Finish(model.LastCatalogOperation);
                WpfTestHost.CompleteLayout(window);
                var product = WpfTestHost.FindByName<ProductSelectionView>(catalog, "ProductSelectionView");
                Assert.IsFalse(product.AddButton.IsEnabled);
                model.ProductSelection!.SelectedVariant = model.ProductSelection.Variants[0];
                model.ProductSelection.Quantity = 2; Finish(model.LastCatalogOperation);
                WpfTestHost.CompleteLayout(window);
                AssertReviewFits(product);
                WpfTestHost.Capture(window, $"ui28-product-reception-{width}x{height}");
            }
            finally { Finish(model.DeactivateCatalogAsync()); Finish(client.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask()); }
        });
    }

    private static void AssertReviewFits(FrameworkElement selection)
    {
        var options = WpfTestHost.FindByName<ScrollViewer>(selection, "SelectionOptions");
        Assert.IsTrue(options.ViewportHeight > 80, "Option inputs must retain a usable scroll viewport.");
        var names = new[] { "ItemName", "CurrentOptions", "QuantityControl", "LineTotal", "AddButton", "LocalNotice" };
        foreach (var name in names)
        {
            var element = WpfTestHost.FindByName<FrameworkElement>(selection, name);
            var bounds = element.TransformToAncestor(selection).TransformBounds(new Rect(element.RenderSize));
            Assert.IsTrue(element.IsVisible && bounds.Left >= -1 && bounds.Top >= -1
                && bounds.Right <= selection.ActualWidth + 1 && bounds.Bottom <= selection.ActualHeight + 1,
                $"{name} must remain visible without scrolling: {bounds}; selection {selection.ActualWidth}x{selection.ActualHeight}.");
        }
        var quantity = WpfTestHost.FindByName<FrameworkElement>(selection, "QuantityControl");
        var total = WpfTestHost.FindByName<FrameworkElement>(selection, "LineTotal");
        var add = WpfTestHost.FindByName<Button>(selection, "AddButton");
        var totalBounds = total.TransformToAncestor(selection).TransformBounds(new Rect(total.RenderSize));
        foreach (var element in new FrameworkElement[] { quantity, add })
            Assert.IsFalse(totalBounds.IntersectsWith(element.TransformToAncestor(selection).TransformBounds(new Rect(element.RenderSize))),
                "Total, quantity, and Add must not overlap.");
        Assert.IsTrue(add.IsEnabled, "The validated selection must remain available to add locally.");
    }

    private static void RecordGeometry(Window window, FrameworkElement selection, int width, int height)
    {
        var dpi = VisualTreeHelper.GetDpi(window);
        var geometry = $"Requested {width}x{height}; actual window {window.ActualWidth}x{window.ActualHeight} DIP; "
            + $"selection {selection.ActualWidth}x{selection.ActualHeight} DIP; primary display "
            + $"{SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX * 100}%; "
            + "WPF RenderTargetBitmap at 96 DPI, synthetic catalog and Rust responses.";
        Console.WriteLine(geometry);
        if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
        {
            Directory.CreateDirectory(directory);
            File.WriteAllText(Path.Combine(directory, $"ui28-{width}x{height}.txt"), geometry);
        }
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void LiveCatalogAndSelectionsKeepNativeKeyboardPathsAndMixedDirectionValues(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            var engine = new FakeEngine(); var entry = SalesCatalogAuthorityTests.Entry(true);
            entry.Colors = [new() { Id = Guid.NewGuid(), Name = "أبيض", Visual = "#FFFFFF", PriceAdjustmentYer = 0 }];
            entry.Handles = [new() { Id = Guid.NewGuid(), Name = "مقبض H-12", Visual = "BlackMetal", PriceAdjustmentYer = 0 }];
            engine.QueryHandler = q => SalesCatalogAuthorityTests.Handle(q, entry);
            var client = new SalesCatalogClient(engine);
            var model = new SalesCatalogViewModel(new Features.Furniture.FurnitureViewModel(), new Features.Products.ProductsViewModel());
            model.AttachCatalogClient(client);
            var catalog = new SalesCatalogView { DataContext = model }; window.Content = catalog;
            try
            {
                Finish(model.ActivateCatalogAsync()); WpfTestHost.CompleteLayout(window);
                var dpi = VisualTreeHelper.GetDpi(window);
                Console.WriteLine($"Catalog requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight}; scale {dpi.DpiScaleX * 100:0}%");
                Assert.AreEqual(FlowDirection.RightToLeft, catalog.FlowDirection);
                var search = WpfTestHost.FindByName<TextBox>(catalog, "CatalogSearch"); search.Focus(); Assert.IsTrue(search.IsKeyboardFocusWithin);
                Assert.IsTrue(search.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
                var categories = WpfTestHost.FindByAutomationName<ListBox>(catalog, "فئات المنتجات"); categories.SelectedItem = entry.CategoryName;
                Finish(model.LastCatalogOperation); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(entry.CategoryName, model.SelectedCategory); Assert.AreEqual(entry.CategoryName, categories.SelectedItem);
                SalesCatalogRenderedTests.AssertFirstCatalogRowFits(catalog);
                WpfTestHost.Capture(window, $"sales-catalog-live-{width}");
                var select = WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار خزانة تجريبية"); select.Focus(); select.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Finish(model.LastCatalogOperation); WpfTestHost.CompleteLayout(window);
                var detail = WpfTestHost.FindByName<FurnitureSelectionView>(catalog, "SelectionView");
                Assert.IsTrue(WpfTestHost.FindByName<Button>(detail, "BackButton").IsKeyboardFocusWithin);
                var sizes = WpfTestHost.FindByName<ListBox>(detail, "SizesList"); sizes.Focus();
                sizes.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(sizes), 0, Key.Down) { RoutedEvent = Keyboard.KeyDownEvent });
                Assert.IsNotNull(model.Selection!.SelectedSize);
                model.Selection.SelectedColor = model.Selection.Colors[0]; model.Selection.SelectedHandle = model.Selection.Handles[0]; Finish(model.LastCatalogOperation);
                var input = WpfTestHost.FindByAutomationName<TextBox>(detail, "العرض بالسنتيمتر"); input.Focus(); Assert.IsTrue(input.IsKeyboardFocusWithin);
                input.Text = "١٣٠٫٥"; Finish(model.LastCatalogOperation); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(FlowDirection.LeftToRight, input.FlowDirection); Assert.IsTrue(model.Selection.CanAdd);
                Assert.IsTrue(input.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
                var dimensions = WpfTestHost.Descendants<TextBlock>(detail).First(t => t.Text == model.Selection.Sizes[0].DimensionsLabel);
                Assert.AreEqual(FlowDirection.LeftToRight, dimensions.FlowDirection);
                var identifier = WpfTestHost.Descendants<TextBlock>(detail).SelectMany(t => t.Inlines.OfType<System.Windows.Documents.Run>()).First(r => r.Text == "A-12");
                Assert.AreEqual(FlowDirection.LeftToRight, identifier.FlowDirection);
                WpfTestHost.Capture(window, $"sales-furniture-live-{width}");
                var scroll = WpfTestHost.Descendants<ScrollViewer>(detail).First(); scroll.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, $"sales-furniture-total-{width}");
                if (width == 1338)
                {
                    detail.Resources[SystemParameters.HighContrastKey] = true;
                    detail.Resources[SystemColors.WindowBrushKey] = Brushes.Black;
                    detail.Resources[SystemColors.WindowTextBrushKey] = Brushes.White;
                    detail.Resources[SystemColors.ControlBrushKey] = Brushes.Black;
                    detail.Resources[SystemColors.ControlTextBrushKey] = Brushes.White;
                    input.BringIntoView(); WpfTestHost.CompleteLayout(window);
                    WpfTestHost.Capture(window, "sales-furniture-high-contrast");
                    foreach (var key in new[] { SystemParameters.HighContrastKey, SystemColors.WindowBrushKey, SystemColors.WindowTextBrushKey, SystemColors.ControlBrushKey, SystemColors.ControlTextBrushKey }) detail.Resources.Remove(key);
                    scroll.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                }
                var add = WpfTestHost.FindByName<Button>(detail, "AddButton"); add.Focus(); Assert.IsTrue(add.IsKeyboardFocusWithin);
                Assert.AreEqual("إضافة إلى عرض السعر", System.Windows.Automation.AutomationProperties.GetName(add));
                add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(model.LastCatalogOperation); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(model.QuotationLines.Count > 0);
                model.CloseSelection();
                entry = SalesCatalogAuthorityTests.Entry(); model.ClearFilters(); Finish(model.LastCatalogOperation); WpfTestHost.CompleteLayout(window);
                model.Select(model.VisibleItems[0]); Finish(model.LastCatalogOperation); WpfTestHost.CompleteLayout(window);
                var product = WpfTestHost.FindByName<ProductSelectionView>(catalog, "ProductSelectionView");
                var variants = WpfTestHost.FindByName<ListBox>(product, "VariantsList"); variants.Focus();
                variants.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(variants), 0, Key.Down) { RoutedEvent = Keyboard.KeyDownEvent });
                Finish(model.LastCatalogOperation); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(model.ProductSelection!.CanAdd); WpfTestHost.Capture(window, $"sales-product-live-{width}");
            }
            finally { Finish(model.DeactivateCatalogAsync()); Finish(client.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask()); }
        });
    }
    /// <summary>Pumps the STA dispatcher until an asynchronous catalog operation completes, with a bounded test timeout.</summary>
    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(10));
        if (!task.IsCompleted)
        {
            var frame = new DispatcherFrame(); var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default);
            Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }
}
