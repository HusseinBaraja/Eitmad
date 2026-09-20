using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.WindowsShell.Layout;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class MainWindowRenderedTests
{
    [TestMethod]
    public void RootResolvesArabicRtlAndNativeWindowChrome()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            Assert.AreEqual(FlowDirection.RightToLeft, window.FlowDirection);
            Assert.AreEqual("ar-ye", window.Language.IetfLanguageTag.ToLowerInvariant());
            Assert.AreEqual(WindowStyle.SingleBorderWindow, window.WindowStyle);
            Assert.AreEqual(ResizeMode.CanResize, window.ResizeMode);
        });
    }

    [TestMethod]
    public void AuthenticatedReceptionistOpensReceptionSurfaceAndSwitchReturnsToSignIn()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            var cleared = 0;
            window.AccountSessionCleared += (_, _) => cleared++;
            var signIn = WpfTestHost.FindByName<FrameworkElement>(window, "SignInSurface");
            Assert.AreEqual(Visibility.Visible, signIn.Visibility);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility);

            WpfTestHost.SignIn(window, "receptionist");
            WpfTestHost.CompleteLayout(window);

            Assert.AreEqual(Visibility.Collapsed, signIn.Visibility);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<FrameworkElement>(window, "ReceptionistSurface").Visibility);
            var receptionist = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            var receptionistSidebar = WpfTestHost.Descendants<ShellSidebar>(receptionist).Single();
            Assert.IsTrue(receptionistSidebar.IsVisible);
            Assert.IsTrue(receptionistSidebar.IsReceptionist);
            Assert.AreEqual(1, WpfTestHost.Descendants<ShellTitleBar>(receptionist).Count());

            var shortcut = window.InputBindings.OfType<KeyBinding>().Single(binding => binding.Key == Key.K);
            Assert.AreEqual(ModifierKeys.Alt, shortcut.Modifiers);
            MainWindow.SwitchAccountCommand.Execute(null, window);
            WpfTestHost.CompleteLayout(window);

            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<FrameworkElement>(window, "ReceptionistSurface").Visibility);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility);
            Assert.AreEqual(Visibility.Visible, signIn.Visibility);
            Assert.AreEqual(1, cleared);
        }, showSignIn: true);
    }

    [TestMethod]
    public void ReceptionistActionsStackAtCompactWindowWidth()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            WpfTestHost.SignIn(window, "receptionist");
            WpfTestHost.CompleteLayout(window);

            Assert.AreEqual(1, WpfTestHost.FindByName<UniformGrid>(window, "ReceptionistActionsGrid").Columns);
        }, showSignIn: true);
    }

    [TestMethod]
    public void ManagerAndReceptionistUseMatchingSharedChromeGeometry()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            var managerTitleBar = WpfTestHost.FindByName<ShellTitleBar>(window, "ManagerTitleBar");
            var managerSidebar = WpfTestHost.FindByName<ShellSidebar>(window, "ManagerSidebar");
            var managerTitleHeight = managerTitleBar.ActualHeight;
            var managerSidebarWidth = managerSidebar.ActualWidth;
            WpfTestHost.Capture(window, "shared-chrome-manager");

            var receptionistSurface = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            receptionistSurface.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);

            var receptionistTitleBar = WpfTestHost.FindByName<ShellTitleBar>(receptionistSurface, "ReceptionistTitleBar");
            var receptionistSidebar = WpfTestHost.FindByName<ShellSidebar>(receptionistSurface, "ReceptionistSidebar");
            Assert.AreEqual(managerTitleHeight, receptionistTitleBar.ActualHeight, 0.1);
            Assert.AreEqual(managerSidebarWidth, receptionistSidebar.ActualWidth, 0.1);
            WpfTestHost.Capture(window, "shared-chrome-receptionist");
        });
    }

    [TestMethod]
    public void NavigationAndPreviewActionsUpdateVisibleStateAndFocus()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            var materialsButton = WpfTestHost.FindByName<Button>(window, "MaterialsNavButton");
            materialsButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Grid>(window, "DashboardSurface").Visibility);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<FrameworkElement>(window, "RawMaterialsSurface").Visibility);

            var homeButton = WpfTestHost.FindByName<Button>(window, "HomeNavButton");
            homeButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var newQuote = WpfTestHost.Descendants<Button>(window)
                .First(button => Equals(button.Tag, "عرض سعر جديد"));
            newQuote.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();

            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<FrameworkElement>(window, "InteractionPanel").Visibility);
            var customerName = WpfTestHost.FindByName<TextBox>(window, "CustomerNameBox");
            Assert.IsTrue(customerName.IsKeyboardFocusWithin);
        });
    }

    [TestMethod]
    public void SelectedNavigationUsesAContrastingSurfaceAndContent()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            var homeButton = WpfTestHost.FindByName<Button>(window, "HomeNavButton");
            AssertSelectedNavigationContrast(homeButton);

            var furnitureButton = WpfTestHost.FindByName<Button>(window, "FurnitureNavButton");
            furnitureButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            AssertSelectedNavigationContrast(furnitureButton);
            Assert.AreEqual(Colors.Transparent, ((SolidColorBrush)homeButton.Background).Color);
        });
    }

    private static void AssertSelectedNavigationContrast(Button button)
    {
        var surface = (Border)button.Template.FindName("Surface", button);
        var background = (LinearGradientBrush)surface.Background;
        Assert.IsTrue(background.GradientStops.All(stop => ContrastAgainstWhite(stop.Color) >= 4.5));

        Assert.IsTrue(VisualStateManager.GoToState(button, "MouseOver", false));
        WpfTestHost.PumpDispatcher();
        var hoverShade = (Border)button.Template.FindName("HoverShade", button);
        var hoverColor = ((SolidColorBrush)hoverShade.Background).Color;
        Assert.AreEqual(1d, hoverShade.Opacity);
        Assert.AreNotEqual(0, hoverColor.A);
        foreach (var stop in background.GradientStops)
        {
            var hoveredStop = Composite(stop.Color, hoverColor);
            Assert.AreNotEqual(stop.Color, hoveredStop);
            Assert.IsTrue(ContrastAgainstWhite(hoveredStop) >= 4.5);
        }

        var inkColor = ((SolidColorBrush)button.FindResource("InkBrush")).Color;
        Assert.IsTrue(Contrast(Composite(Colors.White, hoverColor), inkColor) >= 4.5);

        foreach (var text in WpfTestHost.Descendants<TextBlock>(button))
        {
            Assert.AreEqual(Colors.White, ((SolidColorBrush)text.Foreground).Color);
        }

        foreach (var icon in WpfTestHost.Descendants<System.Windows.Shapes.Path>(button))
        {
            Assert.AreEqual(Colors.White, ((SolidColorBrush)icon.Fill).Color);
        }
    }

    private static double ContrastAgainstWhite(Color color)
        => Contrast(color, Colors.White);

    private static double Contrast(Color first, Color second)
    {
        var firstLuminance = Luminance(first);
        var secondLuminance = Luminance(second);
        var lighter = Math.Max(firstLuminance, secondLuminance);
        var darker = Math.Min(firstLuminance, secondLuminance);
        return (lighter + 0.05) / (darker + 0.05);
    }

    private static double Luminance(Color color) =>
        (0.2126 * LinearChannel(color.R))
        + (0.7152 * LinearChannel(color.G))
        + (0.0722 * LinearChannel(color.B));

    private static Color Composite(Color background, Color overlay)
    {
        var alpha = overlay.A / 255d;
        return Color.FromRgb(
            (byte)Math.Round((overlay.R * alpha) + (background.R * (1 - alpha))),
            (byte)Math.Round((overlay.G * alpha) + (background.G * (1 - alpha))),
            (byte)Math.Round((overlay.B * alpha) + (background.B * (1 - alpha))));
    }

    private static double LinearChannel(byte channel)
    {
        var value = channel / 255d;
        return value <= 0.04045 ? value / 12.92 : Math.Pow((value + 0.055) / 1.055, 2.4);
    }

    [TestMethod]
    public void StandardAndCompactSizesResolveResponsiveLayout()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            var root = WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot");
            Assert.AreEqual(ResponsiveLayoutMode.Standard, ResponsiveLayout.GetMode(root));
            Assert.AreEqual(2, WpfTestHost.FindByName<UniformGrid>(window, "MetricsGrid").Columns);
            Assert.AreEqual(3, WpfTestHost.FindByName<UniformGrid>(window, "QuickActionsGrid").Columns);
        });

        WpfTestHost.Run(780, 745, window =>
        {
            var root = WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot");
            Assert.AreEqual(ResponsiveLayoutMode.Compact, ResponsiveLayout.GetMode(root));
            Assert.AreEqual(2, WpfTestHost.FindByName<UniformGrid>(window, "MetricsGrid").Columns);
            Assert.AreEqual(2, WpfTestHost.FindByName<UniformGrid>(window, "QuickActionsGrid").Columns);

            var search = WpfTestHost.FindByName<TextBox>(window, "SearchBox");
            var searchBorder = WpfTestHost.Ancestor<Border>(search);
            Assert.AreEqual(1, Grid.GetRow(searchBorder));
            Assert.AreEqual(4, Grid.GetColumnSpan(searchBorder));
        });
    }
}
