using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Tests.Furniture;
using Eitmad.WindowsShell.Tests.Products;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class EditorViewportRenderedTests
{
    [TestMethod]
    [DataRow(720, 560, false)]
    [DataRow(1338, 753, false)]
    [DataRow(1920, 1080, false)]
    [DataRow(720, 560, true)]
    public void EditorBodyKeepsSpecificationsAndActionsTogether(int width, int height, bool highContrast)
    {
        WpfTestHost.Run(width, height, window =>
        {
            if (width == 1920)
            {
                window.Left = 0;
                window.WindowState = WindowState.Maximized;
                WpfTestHost.CompleteLayout(window);
            }
            var dpi = VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"Display: {SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX}x{SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY} px; scaling {dpi.DpiScaleX * 100}%; requested window {width}x{height} DIP; actual {window.ActualWidth}x{window.ActualHeight} DIP. Captures use a 96-DPI render target, not desktop screenshots.");
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var furniture = WpfTestHost.Descendants<FurnitureView>(window).Single();
            if (highContrast) UseSystemContrast(furniture);
            var fixture = new FurnitureFixtures();
            fixture.Seed();
            furniture.ViewModel.ApplyDurableData(fixture.Snapshot());
            furniture.ViewModel.BeginEdit(furniture.ViewModel.VisibleFurniture.Single());
            furniture.ViewModel.MoveToParts();
            for (var index = 0; index < 10; index++) furniture.ViewModel.SelectedParts.Add(furniture.ViewModel.SelectedParts[0]);
            Measure(window, furniture, "furniture-many-parts", width, height, "إضافة جزء للأثاث", "NextButton", highContrast);
            furniture.ViewModel.MoveToOptions();
            Measure(window, furniture, "furniture-options", width, height, "إضافة لون", "NextButton", highContrast);
            furniture.ViewModel.MoveToReview();
            Measure(window, furniture, "furniture-review", width, height, "DimensionsLabel", "حفظ تعريف الأثاث", highContrast);

            WpfTestHost.FindByName<Button>(window, "ProductsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var products = WpfTestHost.Descendants<ProductsView>(window).Single();
            if (highContrast) UseSystemContrast(products);
            products.ViewModel.ApplyDurableData(ProductsPresentationTests.Data());
            products.ViewModel.BeginEdit(products.ViewModel.VisibleProducts.Single());
            Measure(window, products, "products-editor", width, height, "تكلفة شراء الخيار", null, highContrast);
            for (var index = 0; index < 10; index++) products.ViewModel.AddVariant();
            Measure(window, products, "products-many-options", width, height, "تكلفة شراء الخيار", null, highContrast);
        });
    }

    private static void Measure(MainWindow window, FrameworkElement view, string scene, int width, int height,
        string specificationName, string? actionName, bool highContrast)
    {
        WpfTestHost.CompleteLayout(window);
        var editor = WpfTestHost.FindByName<Grid>(view, "EditorSurface");
        var body = WpfTestHost.FindByName<ScrollViewer>(editor, "EditorBody");
        body.ScrollToTop();
        WpfTestHost.CompleteLayout(window);
        var action = actionName == "NextButton"
            ? WpfTestHost.FindByName<Button>(editor, actionName)
            : WpfTestHost.FindByAutomationName<Button>(editor, actionName ?? "حفظ المنتج");
        var spec = WpfTestHost.Descendants<FrameworkElement>(editor).First(element => element.IsVisible
            && (AutomationProperties.GetName(element) == specificationName
                || element is TextBlock text && text.GetBindingExpression(TextBlock.TextProperty)?.ParentBinding.Path?.Path == specificationName));
        WpfTestHost.Capture(window, $"{scene}-{width}x{height}{(highContrast ? "-contrast" : "")}");
        Assert.IsTrue(IsInside(spec, body), "Important specifications must be inside the initial body viewport.");
        Assert.IsTrue(IsInside(action, editor), "Final actions must stay inside the editor.");
        Assert.IsTrue(WpfTestHost.Descendants<Controls.PageHeader>(editor).All(header => IsInside(header, editor)),
            "The item summary must stay outside the body scroll.");
        Assert.IsLessThanOrEqualTo(1, WpfTestHost.Descendants<ScrollViewer>(editor).Count(scroll => scroll.IsVisible && scroll.ScrollableHeight > 1),
            "Editor tables must use the body scroll.");
        if (scene == "furniture-options")
            Assert.IsTrue(IsInside(WpfTestHost.FindByAutomationName<Button>(editor, "إضافة مقبض"), body), "Both option sections must be visible without switching scroll regions.");
        if (scene == "furniture-review")
        {
            var price = WpfTestHost.Descendants<Controls.AmountDisplay>(editor).First(amount => amount.IsVisible
                && amount.GetBindingExpression(Controls.AmountDisplay.AmountTextProperty)?.ParentBinding.Path?.Path == "SellingPriceLabel");
            Assert.IsTrue(IsInside(price, body), "Reviewed dimensions and price must be visible together.");
        }
    }

    private static void UseSystemContrast(FrameworkElement view)
    {
        view.Resources[SystemParameters.HighContrastKey] = true;
        view.Resources[SystemColors.WindowBrushKey] = Brushes.Black;
        view.Resources[SystemColors.WindowTextBrushKey] = Brushes.White;
        view.Resources[SystemColors.ControlBrushKey] = Brushes.Black;
        view.Resources[SystemColors.ControlTextBrushKey] = Brushes.White;
    }

    private static bool IsInside(FrameworkElement element, FrameworkElement container) => element.IsVisible
        && new Rect(container.RenderSize).Contains(element.TransformToAncestor(container).TransformBounds(new Rect(element.RenderSize)));
}
