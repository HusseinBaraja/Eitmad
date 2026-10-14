using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Home;
using Eitmad.WindowsShell.Tests.Home;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class HomeSearchRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void SubmittedHomeSearchRendersAndClearsForBothRoles(int width, int height)
    {
        var engine = HomeStateTests.Engine(HomeStateTests.Data());
        var queries = new List<string>();
        engine.QueryBarrier = query => { queries.Add(query.AsHomeRead()!.Term); return Task.CompletedTask; };
        try
        {
            WpfTestHost.Run(width, height, window => {
                var model = (HomeViewModel)window.ManagerHome.DataContext;
                Finish(model.ActivateAsync());
                if (width == 1920) window.WindowState = WindowState.Maximized;
                WpfTestHost.CompleteLayout(window);
                var dpi = VisualTreeHelper.GetDpi(window);
                Console.WriteLine($"UI-13: requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX * 100}%.");
                foreach (var role in new[] { "manager", "reception" })
                {
                    var manager = role == "manager";
                    window.ResponsiveRoot.Visibility = manager ? Visibility.Visible : Visibility.Collapsed;
                    window.ReceptionistSurface.Visibility = manager ? Visibility.Collapsed : Visibility.Visible;
                    WpfTestHost.CompleteLayout(window);
                    var home = manager ? window.ManagerHome : WpfTestHost.FindByName<HomeView>(window.ReceptionistSurface, "ReceptionHome");
                    var titleBar = manager ? window.ManagerTitleBar : WpfTestHost.FindByName<ShellTitleBar>(window.ReceptionistSurface, "ReceptionistTitleBar");
                    titleBar.SearchBox.Text = "عميل";
                    titleBar.SearchBox.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(titleBar.SearchBox), 0, Key.Enter) { RoutedEvent = Keyboard.KeyDownEvent });
                    Finish(model.LastLoad); WpfTestHost.CompleteLayout(window);
                    var context = WpfTestHost.FindByName<StackPanel>(home, "SearchContext");
                    var term = titleBar.SearchBox;
                    var clear = WpfTestHost.FindByName<Button>(home, "ClearSearchButton");
                    Assert.AreEqual("عميل", queries[^1]); Assert.AreEqual(model.SearchTerm, term.Text);
                    Assert.IsTrue(context.IsVisible);
                    var refresh = WpfTestHost.Descendants<Button>(home).Single(b => Equals(b.Content, "إعادة تحميل البيانات"));
                    refresh.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(model.LastLoad);
                    Assert.AreEqual("عميل", queries[^1]); Assert.AreEqual("عميل", term.Text);
                    WpfTestHost.Ancestor<ScrollViewer>(home).ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                    Assert.IsTrue(clear.Focus()); Assert.IsTrue(clear.IsKeyboardFocusWithin);
                    var bounds = context.TransformToAncestor(window).TransformBounds(new Rect(context.RenderSize));
                    Assert.IsTrue(bounds.Top >= 0 && bounds.Bottom <= window.ActualHeight && bounds.Left >= 0 && bounds.Right <= window.ActualWidth, $"Search context is clipped: {bounds}.");
                    WpfTestHost.Capture(window, $"ui13-home-search-{role}-{width}");
                    term.Text = "عميل QT-2026-00001";
                    term.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(term), 0, Key.Enter) { RoutedEvent = Keyboard.KeyDownEvent });
                    Finish(model.LastLoad); WpfTestHost.CompleteLayout(window);
                    Assert.AreEqual("عميل QT-2026-00001", term.Text);
                    Assert.AreEqual(model.SearchTerm, term.Text);
                    if (width == 720)
                    {
                        WpfTestHost.Capture(window, $"ui13-home-search-mixed-{role}-{width}");
                        var termSize = term.FontSize; var clearSize = clear.FontSize;
                        term.FontSize = 18; clear.FontSize = 18;
                        WpfTestHost.CompleteLayout(window); context.BringIntoView(); WpfTestHost.CompleteLayout(window);
                        var largerBounds = context.TransformToAncestor(window).TransformBounds(new Rect(context.RenderSize));
                        Assert.IsTrue(largerBounds.Top >= 0 && largerBounds.Bottom <= window.ActualHeight);
                        Assert.IsTrue(clear.ActualWidth <= context.ActualWidth);
                        WpfTestHost.Capture(window, $"ui13-home-search-larger-text-{role}-{width}");
                        term.FontSize = termSize; clear.FontSize = clearSize;
                    }
                    clear.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(model.LastLoad); WpfTestHost.CompleteLayout(window);
                    Assert.AreEqual("", queries[^1]); Assert.AreEqual("", titleBar.SearchBox.Text);
                    Assert.AreEqual("آخر نشاط", model.ActivityTitle); Assert.AreEqual(Visibility.Collapsed, context.Visibility);
                    Assert.IsTrue(titleBar.SearchBox.IsKeyboardFocusWithin);
                    if (width == 1338) WpfTestHost.Capture(window, $"ui13-home-search-cleared-{role}-{width}");
                }
            }, engine: engine);
        }
        finally { Finish(engine.DisposeAsync().AsTask()); }
    }
    private static void Finish(Task task) { while (!task.IsCompleted) WpfTestHost.PumpDispatcher(); task.GetAwaiter().GetResult(); }
}
