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
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void CancelKeepsTheAccountAndDialogFitsTheWindow(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.SignIn(window, "manager");
            MainWindow.SwitchAccountCommand.Execute(null, window);
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
            var dpi = VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"Switch dialog: window {window.ActualWidth} x {window.ActualHeight}, scaling {dpi.DpiScaleX * 100}%, display {SystemParameters.PrimaryScreenWidth} x {SystemParameters.PrimaryScreenHeight} DIPs.");
            WpfTestHost.Capture(window, $"account-switch-{width}x{height}");
            dialog.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(window), 0, Key.Escape) { RoutedEvent = Keyboard.KeyDownEvent });
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(dialog.IsOpen);
            Assert.AreEqual(Visibility.Collapsed, window.SignInSurface.Visibility);
        }, showSignIn: true);
    }
}
