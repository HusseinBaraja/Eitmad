using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Home;
using Eitmad.WindowsShell.Tests.Home;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class HomeSearchRenderedTests
{
    [TestMethod]
    [DataRow(true)]
    [DataRow(false)]
    public void SubmittedSearchAndClearUseTheVisibleRoleToolbar(bool manager)
    {
        var engine = HomeStateTests.Engine(HomeStateTests.Data());
        var queries = new List<string>();
        engine.QueryBarrier = query => { queries.Add(query.AsHomeRead()!.Term); return Task.CompletedTask; };
        try
        {
            WpfTestHost.Run(1338, 753, window => {
                var model = (HomeViewModel)window.ManagerHome.DataContext;
                Finish(model.ActivateAsync(includeCustomerSearch: !manager));
                window.ResponsiveRoot.Visibility = manager ? Visibility.Visible : Visibility.Collapsed;
                window.ReceptionistSurface.Visibility = manager ? Visibility.Collapsed : Visibility.Visible;
                WpfTestHost.CompleteLayout(window);
                var home = manager ? window.ManagerHome : WpfTestHost.FindByName<HomeView>(window.ReceptionistSurface, "ReceptionHome");
                var toolbar = manager ? window.ManagerTitleBar : window.ReceptionistSurface.ReceptionistTitleBar;
                toolbar.SearchBox.Text = "عميل QT-2026-00001";
                toolbar.SearchBox.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(toolbar.SearchBox), 0, Key.Enter) { RoutedEvent = Keyboard.KeyDownEvent });
                Finish(model.LastLoad);
                Assert.AreEqual(toolbar.SearchBox.Text, queries[^1]);
                WpfTestHost.FindByName<Button>(home, "ClearSearchButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Finish(model.LastLoad); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual("", queries[^1]);
                Assert.AreEqual("", toolbar.SearchBox.Text);
                Assert.IsTrue(toolbar.SearchBox.IsKeyboardFocusWithin);
            }, engine: engine);
        }
        finally { Finish(engine.DisposeAsync().AsTask()); }
    }
    private static void Finish(Task task) { while (!task.IsCompleted) WpfTestHost.PumpDispatcher(); task.GetAwaiter().GetResult(); }
}
