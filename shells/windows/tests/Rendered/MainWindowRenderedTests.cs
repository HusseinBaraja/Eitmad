using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.WindowsShell.Layout;

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
            Assert.IsTrue(customerName.Focus());
            Assert.IsTrue(customerName.IsKeyboardFocusWithin);
        });
    }

    [TestMethod]
    public void SelectedNavigationContentStaysReadableDuringHover()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            var homeButton = WpfTestHost.FindByName<Button>(window, "HomeNavButton");
            homeButton.RaiseEvent(new MouseEventArgs(Mouse.PrimaryDevice, 0)
            {
                RoutedEvent = Mouse.MouseEnterEvent,
            });

            foreach (var text in WpfTestHost.Descendants<TextBlock>(homeButton))
            {
                Assert.AreEqual(Colors.White, ((SolidColorBrush)text.Foreground).Color);
            }
        });
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

    [TestMethod]
    public void ResolveModeUsesStableBreakpointBoundaries()
    {
        Assert.AreEqual(ResponsiveLayoutMode.Compact, ResponsiveLayout.ResolveMode(899.99));
        Assert.AreEqual(ResponsiveLayoutMode.Standard, ResponsiveLayout.ResolveMode(900));
        Assert.AreEqual(ResponsiveLayoutMode.Standard, ResponsiveLayout.ResolveMode(1599.99));
        Assert.AreEqual(ResponsiveLayoutMode.Wide, ResponsiveLayout.ResolveMode(1600));
    }
}
