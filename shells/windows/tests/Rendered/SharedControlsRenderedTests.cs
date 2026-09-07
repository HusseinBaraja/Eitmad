using System.IO;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.RawMaterials;
using Eitmad.WindowsShell.Layout;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class SharedControlsRenderedTests
{
    [TestMethod]
    [DataRow(1338d, 753d)]
    [DataRow(780d, 745d)]
    public void SearchBindingsAndFilterFieldsFitEveryPage(double width, double height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            Capture(window, $"dashboard-{width}");
            foreach (var destination in new[] { "Materials", "Parts", "Furniture", "Pricing", "Products", "Quotations", "Orders", "WorkOrders" })
            {
                WpfTestHost.FindByName<Button>(window, destination + "NavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                var panel = WpfTestHost.Descendants<AdaptiveFieldsPanel>(window).Single(element => element.IsVisible);
                var search = WpfTestHost.Descendants<TextBox>(panel).Single(element => element.Name.EndsWith("SearchBox", StringComparison.Ordinal));
                Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(search)));
                Assert.AreEqual(string.Empty, search.Text);
                Assert.IsFalse(string.IsNullOrWhiteSpace(ControlOptions.GetPlaceholder(search)));
                search.Focus();
                search.Text = "خشب";
                WpfTestHost.CompleteLayout(window);
                var binding = search.GetBindingExpression(TextBox.TextProperty)!;
                Assert.AreEqual("خشب", binding.DataItem.GetType().GetProperty("SearchText")!.GetValue(binding.DataItem));
                search.Clear();
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(Visibility.Visible, ((TextBlock)search.Template.FindName("Placeholder", search)).Visibility);
                var children = panel.Children.Cast<FrameworkElement>().Where(child => child.Visibility != Visibility.Collapsed).ToArray();
                var bounds = children.Select(child => child.TransformToAncestor(panel).TransformBounds(new Rect(child.RenderSize))).ToArray();
                for (var index = 0; index < bounds.Length; index++)
                {
                    Assert.IsTrue(bounds[index].Left >= -1 && bounds[index].Right <= panel.ActualWidth + 1, destination);
                    for (var other = index + 1; other < bounds.Length; other++)
                        Assert.IsFalse(bounds[index].IntersectsWith(bounds[other]), destination);
                }
                Capture(window, $"{destination}-{width}");
            }
        });
    }

    [TestMethod]
    public void SelectorFootersKeepTheOwningSelectorAndFeatureActions()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "MaterialsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var materials = WpfTestHost.Descendants<RawMaterialsView>(window).Single();
            WpfTestHost.FindByAutomationName<Button>(materials, "إضافة مادة خام").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var unit = WpfTestHost.FindByAutomationName<ComboBox>(materials, "وحدة المادة الخام");
            unit.IsDropDownOpen = true;
            WpfTestHost.CompleteLayout(window);
            var popup = (Popup)unit.Template.FindName("PART_Popup", unit);
            var add = WpfTestHost.Descendants<Button>(popup.Child).First();
            Assert.AreSame(unit, add.DataContext);
            Assert.AreEqual("unit", add.Tag);
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(add).Any(text => text.Text == "+ إضافة وحدة جديدة"));
            Keyboard.Focus(add);
            WpfTestHost.PumpDispatcher();
            Assert.IsTrue(add.IsKeyboardFocusWithin);
            Capture((FrameworkElement)popup.Child, "unit-popup");
            add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(unit.IsDropDownOpen);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(materials, "ReferenceNameBox").IsKeyboardFocusWithin);

            WpfTestHost.FindByName<Button>(window, "ProductsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var products = WpfTestHost.Descendants<ProductsView>(window).Single();
            WpfTestHost.FindByAutomationName<Button>(products, "إضافة منتج").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var category = WpfTestHost.FindByAutomationName<ComboBox>(products, "فئة المنتج");
            category.IsDropDownOpen = true;
            WpfTestHost.CompleteLayout(window);
            popup = (Popup)category.Template.FindName("PART_Popup", category);
            var items = WpfTestHost.Descendants<ComboBoxItem>(popup.Child).ToArray();
            Assert.IsGreaterThan(0, items.Length);
            items[0].IsSelected = true;
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(category.SelectedValue, products.ViewModel.EditorCategory);
            Capture((FrameworkElement)popup.Child, "category-popup");
            WpfTestHost.FindByAutomationName<Button>(popup.Child, "إضافة فئة جديدة").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(category.IsDropDownOpen);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(products, "CategoryNameBox").IsKeyboardFocusWithin);
        });
    }

    [TestMethod]
    public void OptionalButtonTextAndGrowingInputsPreserveKeyboardAccess()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var panel = new AdaptiveFieldsPanel { MinItemWidth = 220, Margin = new Thickness(24), VerticalAlignment = VerticalAlignment.Top };
            var search = new TextBox { Style = (Style)window.FindResource("SearchInput"), FontSize = 26, Text = "خشب تجريبي" };
            ControlOptions.SetPlaceholder(search, "ابحث باسم المادة أو الفئة");
            AutomationProperties.SetName(search, "البحث في المواد");
            var button = new Button { Style = (Style)window.FindResource("PrimaryButton"), Content = "إضافة مادة", ToolTip = "إضافة مادة خام" };
            AutomationProperties.SetName(button, "إضافة مادة خام");
            ControlOptions.SetIcon(button, (Geometry)window.FindResource("IconPlus"));
            ControlOptions.SetShowText(button, false);
            panel.Children.Add(search);
            panel.Children.Add(button);
            var selector = new ComboBox { Style = (Style)window.FindResource("SelectInput"), IsEditable = true, ItemsSource = new[] { "خشب", "قماش" } };
            panel.Children.Add(selector);
            window.Content = panel;
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(button.Focus());
            Assert.IsTrue(button.IsKeyboardFocused);
            Assert.AreEqual(Visibility.Collapsed, ((ContentPresenter)button.Template.FindName("ActionText", button)).Visibility);
            Assert.AreEqual(Visibility.Visible, ((Border)button.Template.FindName("FocusRing", button)).Visibility);
            Assert.IsGreaterThan(44d, search.ActualHeight);
            var editable = (TextBox)selector.Template.FindName("PART_EditableTextBox", selector);
            Assert.IsTrue(editable.IsVisible);
            editable.Text = "قماش";
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("قماش", selector.Text);
            Assert.AreEqual("قماش", selector.SelectedItem);
            ControlOptions.SetShowText(button, true);
            WpfTestHost.CompleteLayout(window);
            var buttonText = WpfTestHost.Descendants<TextBlock>(button).First(text => text.Text == "إضافة مادة");
            Assert.AreEqual(button.Foreground, buttonText.Foreground);
            ControlOptions.SetHighContrast(search, true);
            ControlOptions.SetHighContrast(button, true);
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(Visibility.Visible, ((ContentPresenter)button.Template.FindName("ActionText", button)).Visibility);
            Assert.AreEqual(SystemColors.WindowBrush, ((Border)search.Template.FindName("InputChrome", search)).Background);
            Capture(window, "large-text-system-colors");
            button.IsEnabled = false;
            search.Focus();
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(button.Focus());
        });
    }

    // Opt-in artifacts for a visual review; routine test runs do not write images.
    private static void Capture(FrameworkElement element, string name)
    {
        var directory = Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR");
        if (string.IsNullOrEmpty(directory)) return;
        Directory.CreateDirectory(directory);
        // Include the presentation root's direction transform when capturing a popup.
        element = PresentationSource.FromVisual(element)?.RootVisual as FrameworkElement ?? element;
        var bitmap = new RenderTargetBitmap((int)Math.Ceiling(element.ActualWidth), (int)Math.Ceiling(element.ActualHeight), 96, 96, PixelFormats.Pbgra32);
        bitmap.Render(element);
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        using var stream = File.Create(Path.Combine(directory, name + ".png"));
        encoder.Save(stream);
    }
}
