using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.IO;
using System.Reflection;
using System.Windows.Threading;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class AccountSwitchRenderedTests
{
    [TestMethod]
    [DataRow("ipc", false)]
    [DataRow("io", false)]
    [DataRow("disposed", false)]
    [DataRow("ipc", true)]
    [DataRow("io", true)]
    [DataRow("disposed", true)]
    public void FailedActivationDoesNotSkipSubscriptionCleanup(string failure, bool sessionEnded)
    {
        var engine = new FakeEngine
        {
            QueryHandler = _ => new QueryResponseEnvelope
            {
                Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Failed, Payload = new QueryResult() },
            },
        };
        engine.Connect();
        var sessions = new SwitchSessions();
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.SignIn(window, "manager");
            Finish(window.AccountActivation);
            Assert.IsTrue(engine.SubscriptionCount > 0);
            Exception error = failure switch
            {
                "ipc" => new EngineIpcException(EngineIpcFailureKind.ConnectionLost, "Synthetic activation failure."),
                "io" => new IOException("Synthetic activation failure."),
                _ => new ObjectDisposedException("Synthetic adapter"),
            };
            typeof(MainWindow).GetProperty(nameof(window.AccountActivation), BindingFlags.Instance | BindingFlags.NonPublic)!
                .SetValue(window, Task.FromException(error));
            var cleared = new TaskCompletionSource();
            window.AccountSessionCleared += (_, _) => cleared.TrySetResult();
            if (sessionEnded) sessions.End();
            else
            {
                MainWindow.SwitchAccountCommand.Execute(null, window);
                window.ConfirmAccountSwitchButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Finish(window.AccountTransition);
            }
            Finish(cleared.Task);
            Assert.AreEqual(0, engine.SubscriptionCount);
            Assert.AreEqual(sessionEnded ? 0 : 1, sessions.SignOutCount);
            Assert.AreEqual(Visibility.Visible, window.SignInSurface.Visibility);
            Assert.IsTrue(window.SignInSurface.IsEnabled);
        }, showSignIn: true, engine: engine, sessions: sessions);
    }

    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(10));
        if (!task.IsCompleted)
        {
            var frame = new DispatcherFrame();
            var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default);
            Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }

    private sealed class SwitchSessions : IDesktopSessionController
    {
        public event EventHandler<SessionEndedEventArgs>? SessionEnded;
        public int SignOutCount { get; private set; }
        public void End() => SessionEnded?.Invoke(this, new(SessionEndReason.Expired));
        public Task StartAsync(CancellationToken cancellationToken = default) => Task.CompletedTask;
        public Task StopAsync(CancellationToken cancellationToken = default) => Task.CompletedTask;
        public Task<AuthenticatedSurface> SignInAsync(string username, string password, CancellationToken cancellationToken = default) =>
            Task.FromResult(AuthenticatedSurface.Manager);
        public Task SignOutAsync(CancellationToken cancellationToken = default) { SignOutCount++; return Task.CompletedTask; }
        public ValueTask DisposeAsync() => ValueTask.CompletedTask;
    }

    [TestMethod]
    [DataRow(1920, 1080, "manager")]
    [DataRow(1338, 753, "manager")]
    [DataRow(720, 560, "manager")]
    [DataRow(1920, 1080, "receptionist")]
    [DataRow(1338, 753, "receptionist")]
    [DataRow(720, 560, "receptionist")]
    public void ToolbarOpensTheDialogAndCancelKeepsTheAccount(int width, int height, string account)
    {
        WpfTestHost.Run(width, height, window =>
        {
            // Native tooltip placement needs an anchor inside the available desktop.
            window.Left = 0;
            window.Top = 0;
            if (width == 1920) window.WindowState = WindowState.Maximized;
            WpfTestHost.SignIn(window, account);
            WpfTestHost.CompleteLayout(window);
            var toolbar = account == "manager" ? window.ManagerTitleBar : window.ReceptionistSurface.ReceptionistTitleBar;
            var accountButton = toolbar.SwitchAccountButton;
            var label = (ContentPresenter)accountButton.Template.FindName("ActionText", accountButton);
            Assert.AreEqual(width != 720, label.IsVisible);
            AssertToolbarFits(toolbar);
            var dpi = VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"Account toolbar {account}, requested {width} x {height}: actual window {window.ActualWidth} x {window.ActualHeight} DIP, scaling {dpi.DpiScaleX * 100}%, display {SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX} x {SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY} pixels.");
            WpfTestHost.Capture(window, $"ui-09-toolbar-{account}-{width}");
            var tooltip = (ToolTip)accountButton.ToolTip;
            tooltip.PlacementTarget = accountButton;
            tooltip.IsOpen = true;
            var frame = new DispatcherFrame();
            var timer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(350) };
            timer.Tick += (_, _) => { timer.Stop(); frame.Continue = false; };
            timer.Start();
            Dispatcher.PushFrame(frame);
            WpfTestHost.CompleteLayout(tooltip);
            var tooltipContent = (FrameworkElement)tooltip.Content;
            Assert.IsTrue(tooltipContent.ActualWidth >= tooltipContent.DesiredSize.Width);
            Assert.IsTrue(tooltipContent.ActualHeight >= tooltipContent.DesiredSize.Height);
            WpfTestHost.Capture(tooltip, $"ui-09-tooltip-{account}-{width}");
            tooltip.IsOpen = false;
            accountButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var dialog = window.AccountSwitchDialog;
            var cancel = window.CancelAccountSwitchButton;
            Assert.IsTrue(dialog.IsOpen);
            Assert.IsTrue(dialog.IsKeyboardFocusWithin);
            Assert.IsTrue(cancel.IsKeyboardFocused);
            Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(cancel)));
            Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(window.ConfirmAccountSwitchButton)));
            Assert.IsTrue(dialog.DialogWidth <= window.ActualWidth);
            foreach (var button in new[] { cancel, window.ConfirmAccountSwitchButton })
            {
                var bounds = button.TransformToAncestor(window).TransformBounds(new Rect(button.RenderSize));
                Assert.IsTrue(bounds.Left >= 0 && bounds.Top >= 0 && bounds.Right <= window.ActualWidth && bounds.Bottom <= window.ActualHeight);
            }
            WpfTestHost.Capture(window, $"ui-09-dialog-{account}-{width}");
            dialog.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(window), 0, Key.Escape) { RoutedEvent = Keyboard.KeyDownEvent });
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(dialog.IsOpen);
            Assert.AreEqual(Visibility.Collapsed, window.SignInSurface.Visibility);
        }, showSignIn: true);
    }

    private static void AssertToolbarFits(ShellTitleBar toolbar)
    {
        var button = toolbar.SwitchAccountButton;
        foreach (var content in WpfTestHost.Descendants<FrameworkElement>(button)
            .Where(element => element.IsVisible && element is TextBlock or System.Windows.Shapes.Path))
        {
            var bounds = content.TransformToAncestor(button).TransformBounds(new Rect(content.RenderSize));
            Assert.IsTrue(bounds.Left >= 0 && bounds.Right <= button.ActualWidth);
            Assert.IsTrue(bounds.Top >= 0 && bounds.Bottom <= button.ActualHeight);
        }
        var accountBounds = button.TransformToAncestor(toolbar).TransformBounds(new Rect(button.RenderSize));
        Assert.IsTrue(accountBounds.Left >= 0 && accountBounds.Right <= toolbar.ActualWidth);
        foreach (var neighbor in new FrameworkElement[] { toolbar.PrimaryActionButton, toolbar.TitleText, toolbar.SearchBox })
        {
            var bounds = neighbor.TransformToAncestor(toolbar).TransformBounds(new Rect(neighbor.RenderSize));
            Assert.IsFalse(accountBounds.IntersectsWith(bounds));
        }
    }

    [TestMethod]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void AccountActionFitsWithLargerTextAndSystemColors(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            var toolbar = window.ManagerTitleBar;
            toolbar.SwitchAccountButton.FontSize = 18;
            ControlOptions.SetHighContrast(toolbar.SwitchAccountButton, true);
            WpfTestHost.CompleteLayout(window);
            AssertToolbarFits(toolbar);
            WpfTestHost.Capture(window, $"ui-09-system-colors-larger-text-{width}");
        });
    }
}
