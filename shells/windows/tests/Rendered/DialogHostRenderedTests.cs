using System.IO;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.RawMaterials;
using Eitmad.WindowsShell.Features.Parts;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Pricing;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Eitmad.WindowsShell.Controls;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class DialogHostRenderedTests
{
    [TestMethod]
    public void DialogContainsFocusAndReturnsToInvokerWithoutConfirmingOnEnter()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var background = new Button { Content = "فتح" };
            var first = new TextBox();
            var last = new Button { Content = "إلغاء" };
            var body = new StackPanel();
            body.Children.Add(first);
            var dialog = new DialogHost { Title = "تحرير", Content = body, Footer = last, InitialFocusTarget = first };
            var root = new Grid();
            root.Children.Add(background);
            root.Children.Add(dialog);
            window.Content = root;
            WpfTestHost.CompleteLayout(window);
            background.Focus();
            dialog.IsOpen = true;
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(first.IsKeyboardFocused);
            var backgroundClick = new MouseButtonEventArgs(Mouse.PrimaryDevice, 0, MouseButton.Left) { RoutedEvent = Mouse.PreviewMouseDownEvent };
            background.RaiseEvent(backgroundClick);
            Assert.IsTrue(backgroundClick.Handled, "The shell outside the overlay cannot receive pointer actions.");
            background.Focus();
            WpfTestHost.PumpDispatcher();
            Assert.IsTrue(dialog.IsKeyboardFocusWithin);
            for (var index = 0; index < 6; index++)
            {
                ((UIElement)Keyboard.FocusedElement).MoveFocus(new TraversalRequest(FocusNavigationDirection.Next));
                Assert.IsTrue(dialog.IsKeyboardFocusWithin);
            }
            for (var index = 0; index < 6; index++)
            {
                ((UIElement)Keyboard.FocusedElement).MoveFocus(new TraversalRequest(FocusNavigationDirection.Previous));
                Assert.IsTrue(dialog.IsKeyboardFocusWithin);
            }
            var requests = 0;
            dialog.CloseRequested += (_, _) => requests++;
            RaiseKey(dialog, Key.Enter);
            Assert.AreEqual(0, requests);
            RaiseKey(dialog, Key.Escape);
            Assert.AreEqual(1, requests);
            Assert.IsTrue(dialog.IsOpen, "The feature owns state, including validation failure.");
            ((Button)dialog.Template.FindName("PART_Close", dialog)).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.AreEqual(2, requests);
            dialog.IsOpen = false;
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(background.IsKeyboardFocused);
            var restoredClick = new MouseButtonEventArgs(Mouse.PrimaryDevice, 0, MouseButton.Left) { RoutedEvent = Mouse.PreviewMouseDownEvent };
            background.RaiseEvent(restoredClick);
            Assert.IsFalse(restoredClick.Handled);
        });
    }

    [TestMethod]
    public void DropdownEscapeAndManagerTransitionsPreserveDialogOwnership()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var selector = new ComboBox { ItemsSource = new[] { "خشب", "قماش" }, Style = (Style)window.FindResource("SelectInput") };
            var managerInput = new TextBox();
            var manager = new DialogHost { Content = managerInput, InitialFocusTarget = managerInput };
            var editor = new DialogHost { Content = selector, InitialFocusTarget = selector };
            var root = new Grid();
            root.Children.Add(manager);
            root.Children.Add(editor);
            window.Content = root;
            manager.IsOpen = true;
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(managerInput.IsKeyboardFocused);
            manager.IsOpen = false;
            editor.IsOpen = true;
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(editor.IsKeyboardFocusWithin);
            var requests = 0;
            editor.CloseRequested += (_, _) => requests++;
            selector.IsDropDownOpen = true;
            WpfTestHost.CompleteLayout(window);
            RaiseKey(selector, Key.Escape);
            Assert.IsFalse(selector.IsDropDownOpen);
            Assert.AreEqual(0, requests);
            RaiseKey(editor, Key.Escape);
            Assert.AreEqual(1, requests);
            editor.IsOpen = false;
            manager.IsOpen = true;
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(managerInput.IsKeyboardFocused);
        });
    }

    [TestMethod]
    public void LongDialogBodyScrollsWhileTitleAndFooterRemainAvailable()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var body = new StackPanel();
            for (var index = 0; index < 30; index++) body.Children.Add(new TextBox { Text = "بيانات تجريبية", Height = 48 });
            var footer = new Button { Content = "إلغاء" };
            var dialog = new DialogHost { Title = "تحرير بيانات تجريبية", Content = body, Footer = footer, PreferredWidth = 1200, IsOpen = true };
            window.Content = dialog;
            WpfTestHost.CompleteLayout(window);
            var scroll = WpfTestHost.Descendants<ScrollViewer>(dialog).First();
            Assert.IsGreaterThan(0d, scroll.ScrollableHeight);
            var surface = (Border)dialog.Template.FindName("DialogSurface", dialog);
            var bounds = surface.TransformToAncestor(dialog).TransformBounds(new Rect(surface.RenderSize));
            Assert.IsTrue(bounds.Left >= 23 && bounds.Right <= dialog.ActualWidth - 23);
            Assert.IsTrue(bounds.Top >= 23 && bounds.Bottom <= dialog.ActualHeight - 23);
            Assert.IsTrue(footer.IsVisible);
            scroll.ScrollToEnd();
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(footer.IsVisible);
            Assert.IsTrue(((Button)dialog.Template.FindName("PART_Close", dialog)).IsVisible);
        });
    }

    [TestMethod]
    [DataRow(1338)]
    [DataRow(780)]
    public void FeatureDialogsRenderAndManagersRecoverAfterValidationFailure(int width)
    {
        WpfTestHost.Run(width, 753, window =>
        {
            var products = Navigate<ProductsView>(window, "ProductsNavButton");
            products.ViewModel.BeginManageCategories();
            CaptureDialog(window, "product-categories", width);
            var manager = OpenDialog(window);
            var edit = WpfTestHost.Descendants<Button>(manager).First(button => Equals(button.Content, "تعديل"));
            edit.Focus();
            edit.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            CaptureDialog(window, "product-category-editor", width);
            var editor = OpenDialog(window);
            var categoryName = WpfTestHost.FindByName<TextBox>(editor, "CategoryNameBox");
            Assert.IsTrue(categoryName.IsKeyboardFocused);
            categoryName.Text = string.Empty;
            WpfTestHost.Descendants<Button>(editor).Single(button => Equals(button.Content, "حفظ الفئة"))
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(editor.IsOpen);
            Assert.IsFalse(manager.IsOpen);
            Assert.IsFalse(string.IsNullOrEmpty(products.ViewModel.CategoryError));
            Assert.IsTrue(editor.IsKeyboardFocusWithin);
            RaiseKey(editor, Key.Escape);
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(manager.IsOpen);
            Assert.IsFalse(editor.IsOpen);
            Assert.IsTrue(manager.IsKeyboardFocusWithin);
            RaiseKey(manager, Key.Escape);
            products.ViewModel.RequestArchive(products.ViewModel.VisibleProducts.First(item => !item.IsArchived));
            CaptureDialog(window, "product-archive", width);
            RaiseKey(OpenDialog(window), Key.Escape);

            var materials = Navigate<RawMaterialsView>(window, "MaterialsNavButton");
            materials.ViewModel.BeginManageUnits();
            CaptureDialog(window, "material-references", width);
            materials.ViewModel.BeginEditReference(materials.ViewModel.Units[0]);
            CaptureDialog(window, "material-reference-editor", width);
            editor = OpenDialog(window);
            materials.ViewModel.ReferenceName = string.Empty;
            Assert.IsFalse(materials.ViewModel.SaveReferenceEditor());
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(editor.IsOpen);
            Assert.IsTrue(editor.IsKeyboardFocusWithin);
            RaiseKey(editor, Key.Escape);
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(materials.ViewModel.IsReferenceManagerOpen);
            Assert.IsTrue(OpenDialog(window).IsKeyboardFocusWithin);
            RaiseKey(OpenDialog(window), Key.Escape);

            var parts = Navigate<PartsView>(window, "PartsNavButton");
            parts.ViewModel.BeginEdit(parts.ViewModel.VisibleParts[0]);
            Assert.IsTrue(parts.ViewModel.MoveToMaterials());
            parts.ViewModel.OpenMaterialPicker();
            CaptureDialog(window, "part-material-picker", width);
            RaiseKey(OpenDialog(window), Key.Escape);

            var furniture = Navigate<FurnitureView>(window, "FurnitureNavButton");
            furniture.ViewModel.BeginEdit(furniture.ViewModel.VisibleFurniture.First(item => !item.IsArchived));
            Assert.IsTrue(furniture.ViewModel.MoveToParts());
            furniture.ViewModel.OpenPartPicker();
            CaptureDialog(window, "furniture-part-picker", width);
            RaiseKey(OpenDialog(window), Key.Escape);
            Assert.IsTrue(furniture.ViewModel.MoveToVariants());
            furniture.ViewModel.BeginAddVariant();
            CaptureDialog(window, "furniture-variant", width);
            RaiseKey(OpenDialog(window), Key.Escape);
            Assert.IsTrue(furniture.ViewModel.MoveToOptions());
            furniture.ViewModel.BeginAddColor();
            CaptureDialog(window, "furniture-color", width);
            RaiseKey(OpenDialog(window), Key.Escape);
            furniture.ViewModel.BeginAddHandle();
            CaptureDialog(window, "furniture-handle", width);
            RaiseKey(OpenDialog(window), Key.Escape);

            var pricing = Navigate<PricingView>(window, "PricingNavButton");
            pricing.ViewModel.BeginEdit(pricing.ViewModel.VisiblePrices[0]);
            CaptureDialog(window, "pricing-editor", width);
            RaiseKey(OpenDialog(window), Key.Escape);
        });
    }

    private static T Navigate<T>(MainWindow window, string buttonName) where T : FrameworkElement
    {
        WpfTestHost.FindByName<Button>(window, buttonName).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        WpfTestHost.CompleteLayout(window);
        return WpfTestHost.Descendants<T>(window).Single();
    }

    private static DialogHost OpenDialog(MainWindow window) =>
        WpfTestHost.Descendants<DialogHost>(window).Single(dialog => dialog.IsOpen);

    private static void CaptureDialog(MainWindow window, string name, int width)
    {
        WpfTestHost.CompleteLayout(window);
        var dialog = OpenDialog(window);
        Assert.IsTrue(dialog.IsKeyboardFocusWithin, name);
        var surface = (Border)dialog.Template.FindName("DialogSurface", dialog);
        var bounds = surface.TransformToAncestor(dialog).TransformBounds(new Rect(surface.RenderSize));
        Assert.IsTrue(bounds.Left >= 23 && bounds.Right <= dialog.ActualWidth - 23, name);
        Assert.IsTrue(bounds.Top >= 23 && bounds.Bottom <= dialog.ActualHeight - 23, name);
        var directory = Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR");
        if (string.IsNullOrEmpty(directory)) return;
        Directory.CreateDirectory(directory);
        var bitmap = new RenderTargetBitmap((int)Math.Ceiling(window.ActualWidth), (int)Math.Ceiling(window.ActualHeight), 96, 96, PixelFormats.Pbgra32);
        bitmap.Render(window);
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        using var stream = File.Create(Path.Combine(directory, $"dialog-{name}-{width}.png"));
        encoder.Save(stream);
    }
    private static void RaiseKey(UIElement target, Key key) => target.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(target), 0, key) { RoutedEvent = Keyboard.KeyDownEvent });
}


