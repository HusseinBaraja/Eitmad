using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.RawMaterials;
using Eitmad.WindowsShell.Features.Parts;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Pricing;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Documents;
using Eitmad.WindowsShell.Controls;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class DialogHostRenderedTests
{
    [TestMethod]
    public void WholeShellScrimBlocksNavigationAndRestoresFocus()
    {
        WpfTestHost.Run(1920, 1080, window =>
        {
            var materials = Navigate<RawMaterialsView>(window, "MaterialsNavButton");
            var invoker = WpfTestHost.FindByName<Button>(window, "ProductsNavButton");
            invoker.Focus();
            materials.ViewModel.BeginAddUnit();
            WpfTestHost.CompleteLayout(window);
            var dialog = OpenDialog(window);
            AssertShellScrim(window, dialog, invoker);
            RaiseKey(dialog, Key.Escape);
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(dialog.IsOpen);
            Assert.IsTrue(invoker.IsKeyboardFocused);
            var root = (UIElement)window.Content;
            Assert.IsNull(AdornerLayer.GetAdornerLayer(root).GetAdorners(root));
            invoker.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(window.ProductsSurface.IsVisible, "Navigation resumes after the modal closes.");
        });
    }

    [TestMethod]
    public void WholeShellScrimCoversReceptionNavigationAndStacksOnce()
    {
        WpfTestHost.Run(1920, 1080, window =>
        {
            WpfTestHost.SignIn(window, "receptionist");
            Assert.IsTrue(MainWindow.SwitchAccountCommand.CanExecute(null, window));
            WpfTestHost.FindByName<Button>(window, "QuotationsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var quotations = WpfTestHost.Descendants<Features.Quotations.QuotationsView>(window).Single(view => view.IsVisible);
            var invoker = WpfTestHost.FindByName<Button>(window, "ProductsNavButton");
            invoker.Focus();
            var dialog = WpfTestHost.FindByName<DialogHost>(quotations, "ConversionDialog");
            dialog.IsOpen = true;
            WpfTestHost.CompleteLayout(window);
            AssertShellScrim(window, dialog, invoker);
            Assert.IsFalse(MainWindow.SwitchAccountCommand.CanExecute(null, window), "The shell shortcut cannot open a second action behind the modal.");
            var confirmation = WpfTestHost.FindByName<DialogHost>(window, "AccountSwitchDialog");
            confirmation.IsOpen = true;
            WpfTestHost.CompleteLayout(window);
            AssertShellScrim(window, confirmation, invoker);
            RaiseKey(confirmation, Key.Escape);
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(confirmation.IsOpen);
            AssertShellScrim(window, dialog, invoker);
            dialog.IsOpen = false;
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(invoker.IsKeyboardFocused);
            Assert.IsTrue(MainWindow.SwitchAccountCommand.CanExecute(null, window));
            var root = (UIElement)window.Content;
            Assert.IsNull(AdornerLayer.GetAdornerLayer(root).GetAdorners(root));
        }, showSignIn: true);
    }

    private static void AssertShellScrim(MainWindow window, DialogHost dialog, Button navigation)
    {
        var root = (UIElement)window.Content;
        var scrim = AdornerLayer.GetAdornerLayer(root).GetAdorners(root).Single();
        Assert.AreEqual(root.RenderSize, scrim.RenderSize, "One scrim covers the complete shell.");
        var point = navigation.TranslatePoint(new Point(navigation.ActualWidth / 2, navigation.ActualHeight / 2), window);
        var hit = window.InputHitTest(point);
        Assert.AreSame(scrim, hit, "The scrim receives pointer input over the real sidebar.");
        var pointer = new MouseButtonEventArgs(Mouse.PrimaryDevice, 0, MouseButton.Left) { RoutedEvent = Mouse.PreviewMouseDownEvent };
        scrim.RaiseEvent(pointer);
        Assert.IsTrue(pointer.Handled);
        var key = new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(navigation), 0, Key.Enter) { RoutedEvent = Keyboard.PreviewKeyDownEvent };
        navigation.RaiseEvent(key);
        Assert.IsTrue(key.Handled);
        navigation.Focus();
        WpfTestHost.PumpDispatcher();
        Assert.IsTrue(dialog.IsKeyboardFocusWithin);
        var surface = (Border)dialog.Template.FindName("DialogSurface", dialog);
        var surfacePoint = surface.TranslatePoint(new Point(surface.ActualWidth / 2, surface.ActualHeight / 2), window);
        Assert.AreNotSame(scrim, window.InputHitTest(surfacePoint), "Dialog content remains interactive.");
    }

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
            var dialog = new DialogHost { Title = "تحرير", Content = body, Footer = last, InitialFocusTarget = first, ShowCloseButton = true };
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
            Assert.IsFalse(((Button)dialog.Template.FindName("PART_Close", dialog)).IsVisible);
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
            products.ViewModel.ApplyDurableData(Products.ProductsPresentationTests.Data());
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
            products.ViewModel.FailCategory("تحقق من اسم الفئة.");
            WpfTestHost.CompleteLayout(window);
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
            materials.ViewModel.Fail("تحقق من اسم الوحدة.", reference: true);
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(editor.IsOpen);
            Assert.IsTrue(editor.IsKeyboardFocusWithin);
            RaiseKey(editor, Key.Escape);
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(materials.ViewModel.IsReferenceManagerOpen);
            Assert.IsTrue(OpenDialog(window).IsKeyboardFocusWithin);
            RaiseKey(OpenDialog(window), Key.Escape);

            var parts = Navigate<PartsView>(window, "PartsNavButton");
            parts.ViewModel.ApplyDurableData(new Eitmad.WindowsShell.Tests.Parts.PartFixtures().Snapshot());
            parts.ViewModel.BeginEdit(parts.ViewModel.VisibleParts[0]);
            Assert.IsTrue(parts.ViewModel.MoveToMaterials());
            parts.ViewModel.OpenMaterialPicker();
            CaptureDialog(window, "part-material-picker", width);
            RaiseKey(OpenDialog(window), Key.Escape);

            var furniture = Navigate<FurnitureView>(window, "FurnitureNavButton");
            var furnitureFixture = new Furniture.FurnitureFixtures();
            furnitureFixture.Seed();
            furniture.ViewModel.ApplyDurableData(furnitureFixture.Snapshot());
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
        WpfTestHost.Capture(window, $"dialog-{name}-{width}");
    }
    private static void RaiseKey(UIElement target, Key key) => target.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(target), 0, key) { RoutedEvent = Keyboard.KeyDownEvent });
}
