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
    public void FailedContainmentShowsUnavailableInsteadOfLeavingTheAccountHidden()
    {
        var pending = new TaskCompletionSource();
        var sessions = new SwitchSessions
        {
            StopFailure = new AggregateException(new System.ComponentModel.Win32Exception(5, "Synthetic containment failure."),
                new IOException("Synthetic cleanup failure.")),
        };
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.SignIn(window, "manager");
            window.AccountTransitionTimeout = TimeSpan.FromMilliseconds(100);
            typeof(MainWindow).GetProperty(nameof(window.AccountActivation), BindingFlags.Instance | BindingFlags.NonPublic)!
                .SetValue(window, pending.Task);
            try
            {
                var cleared = new TaskCompletionSource<SessionEndReason?>();
                window.AccountSessionCleared += (_, reason) => cleared.TrySetResult(reason);
                MainWindow.SwitchAccountCommand.Execute(null, window);
                window.ConfirmAccountSwitchButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Finish(window.AccountTransition.WaitAsync(TimeSpan.FromSeconds(2)));
                Assert.AreEqual(SessionEndReason.RecoveryFailed, cleared.Task.Result);
                Assert.AreEqual(Visibility.Visible, window.SignInSurface.Visibility);
                Assert.IsFalse(window.SignInSurface.IsEnabled);
                Assert.AreEqual(0, sessions.SignOutCount);
                Assert.IsFalse(MainWindow.SwitchAccountCommand.CanExecute(null, window));
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "issue3-containment-failed");
            }
            finally { pending.TrySetResult(); }
        }, showSignIn: true, sessions: sessions);
    }

    [TestMethod]
    [DataRow(1920, 1080, false)]
    [DataRow(1338, 753, false)]
    [DataRow(720, 560, false)]
    [DataRow(1338, 753, true)]
    public void PendingUnsubscribesShareOneRecoveryBudgetAndRestoreSignIn(int width, int height, bool sessionEnded)
    {
        var release = new TaskCompletionSource();
        var disposing = 0;
        var engine = new FakeEngine
        {
            SupportedCapabilities = new HashSet<string>
            {
                ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1,
                ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1,
                ProtocolIds.Capabilities.EitmadCapabilityQuotationApprovalV1,
                ProtocolIds.Capabilities.EitmadCapabilityOrdersV1,
                ProtocolIds.Capabilities.EitmadCapabilityCustomerV1,
                ProtocolIds.Capabilities.EitmadCapabilitySalesCatalogV1,
            },
            QueryHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() } },
            SubscribeHook = (_, subscription) => subscription.DisposeBarrier = () => { disposing++; return release.Task; },
        };
        engine.Connect();
        var sessions = new SwitchSessions { OnStop = () => release.TrySetResult() };
        WpfTestHost.Run(width, height, window =>
        {
            try
            {
                window.AccountTransitionTimeout = TimeSpan.FromMilliseconds(150);
                WpfTestHost.SignIn(window, "manager"); Finish(window.AccountActivation);
                var count = engine.SubscriptionCount;
                Assert.IsTrue(count >= 10);
                var cleared = new TaskCompletionSource<SessionEndReason?>();
                window.AccountSessionCleared += (_, reason) => cleared.TrySetResult(reason);
                if (sessionEnded) sessions.End();
                else
                {
                    MainWindow.SwitchAccountCommand.Execute(null, window);
                    window.ConfirmAccountSwitchButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                }
                Assert.AreEqual(count, disposing, "All feeds must stop delivery before any unsubscribe can delay cleanup.");
                Assert.IsFalse(window.SignInSurface.IsEnabled);
                Finish(window.AccountTransition.WaitAsync(TimeSpan.FromSeconds(2)));
                Assert.AreEqual(SessionEndReason.ConnectionLost, cleared.Task.Result);
                Assert.AreEqual(1, sessions.StopCount);
                Assert.AreEqual(0, sessions.SignOutCount, "Containment is not a successful sign-out acknowledgement.");
                Assert.AreEqual(0, engine.SubscriptionCount);
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(Visibility.Visible, window.SignInSurface.Visibility);
                Assert.IsTrue(window.SignInSurface.IsEnabled);
                Assert.IsTrue(WpfTestHost.FindByName<TextBox>(window, "UsernameBox").IsKeyboardFocused);
                var username = WpfTestHost.FindByName<TextBox>(window, "UsernameBox");
                Assert.IsTrue(username.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
                Assert.IsTrue(window.SignInSurface.IsKeyboardFocusWithin);
                Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(username)));
                var dpi = VisualTreeHelper.GetDpi(window);
                Console.WriteLine($"Recovery render: {window.ActualWidth}x{window.ActualHeight}; DPI scale {dpi.DpiScaleX}x{dpi.DpiScaleY}; primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIPs.");
                WpfTestHost.Capture(window, $"issue3-recovery-{width}-{sessionEnded}");
            }
            finally { release.TrySetResult(); }
        }, showSignIn: true, engine: engine, sessions: sessions);
    }

    [TestMethod]
    public void LateActivationCompletionCannotEnterTheNextAccount()
    {
        var release = new TaskCompletionSource();
        var pending = true;
        var queries = 0;
        var old = Home.HomeStateTests.Data();
        var engine = new FakeEngine
        {
            SupportedCapabilities = new HashSet<string> { ProtocolIds.Capabilities.EitmadCapabilityHomeV1 },
            QueryBarrier = _ => { queries++; return pending ? release.Task : Task.CompletedTask; },
            QueryHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = QueryResult.ForHome(old) } },
        };
        engine.Connect();
        var sessions = new SwitchSessions();
        WpfTestHost.Run(1338, 753, window =>
        {
            window.AccountTransitionTimeout = TimeSpan.FromMilliseconds(100);
            try
            {
                WpfTestHost.SignIn(window, "manager");
                var activation = window.AccountActivation;
                Assert.IsFalse(activation.IsCompleted);
                MainWindow.SwitchAccountCommand.Execute(null, window);
                window.ConfirmAccountSwitchButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Finish(window.AccountTransition.WaitAsync(TimeSpan.FromSeconds(2)));
                Assert.AreEqual(1, sessions.StopCount);
                pending = false;
                WpfTestHost.SignIn(window, "manager"); Finish(window.AccountActivation);
                var home = (Features.Home.HomeViewModel)window.ManagerHome.DataContext;
                var current = home.Activity.Single().Item.Id;
                old = Home.HomeStateTests.Data();
                release.TrySetResult(); Finish(activation);
                Assert.AreEqual(current, home.Activity.Single().Item.Id, "A late old-account reply cannot replace the new projection.");
                Assert.AreEqual(2, queries, "The old activation must not start any further account work.");
                Assert.AreEqual(Visibility.Collapsed, window.SignInSurface.Visibility);
            }
            finally { release.TrySetResult(); }
        }, showSignIn: true, engine: engine, sessions: sessions);
    }

    [TestMethod]
    [DataRow(false)]
    [DataRow(true)]
    public void PendingActivationCanBeEndedWithoutReleasingThePeer(bool sessionEnded)
    {
        var subscribed = new TaskCompletionSource();
        var release = new TaskCompletionSource();
        var engine = new FakeEngine
        {
            SupportedCapabilities = new HashSet<string> { ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1 },
            SubscribeBarrier = async token => { subscribed.TrySetResult(); await release.Task.WaitAsync(token); },
        };
        engine.Connect();
        var sessions = new SwitchSessions();
        WpfTestHost.Run(1338, 753, window =>
        {
            try
            {
                WpfTestHost.SignIn(window, "manager");
                Finish(subscribed.Task);
                Assert.IsFalse(window.AccountActivation.IsCompleted);
                var cleared = new TaskCompletionSource();
                window.AccountSessionCleared += (_, _) => cleared.TrySetResult();
                if (sessionEnded) sessions.End();
                else
                {
                    MainWindow.SwitchAccountCommand.Execute(null, window);
                    window.ConfirmAccountSwitchButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                }
                Finish(cleared.Task.WaitAsync(TimeSpan.FromSeconds(2)));
                Assert.AreEqual(0, engine.SubscriptionCount);
                Assert.AreEqual(sessionEnded ? 0 : 1, sessions.SignOutCount);
                Assert.AreEqual(Visibility.Visible, window.SignInSurface.Visibility);
                Assert.IsTrue(window.SignInSurface.IsEnabled);
                Assert.AreEqual(0, sessions.StopCount, "Cancellation should preserve a healthy engine.");
            }
            finally { release.TrySetResult(); Finish(window.AccountActivation); }
        }, showSignIn: true, engine: engine, sessions: sessions);
    }

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
        public Task<AuthenticatedSurface> ServerSignInAsync(string username, string password, DesktopEnrollmentSettings? connection, CancellationToken cancellationToken = default) => SignInAsync(username, password, cancellationToken);
        public event EventHandler<SessionEndedEventArgs>? SessionEnded;
        public int SignOutCount { get; private set; }
        public int StopCount { get; private set; }
        public Action? OnStop { get; init; }
        public Exception? StopFailure { get; init; }
        public void End() => SessionEnded?.Invoke(this, new(SessionEndReason.Expired));
        public Task StartAsync(CancellationToken cancellationToken = default) => Task.CompletedTask;
        public Task StopAsync(CancellationToken cancellationToken = default)
        {
            Assert.IsFalse(cancellationToken.IsCancellationRequested, "An expired transition token must not bypass containment.");
            StopCount++; OnStop?.Invoke(); return StopFailure is null ? Task.CompletedTask : Task.FromException(StopFailure);
        }
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
            WpfTestHost.SignIn(window, account);
            var toolbar = account == "manager" ? window.ManagerTitleBar : window.ReceptionistSurface.ReceptionistTitleBar;
            var accountButton = toolbar.SwitchAccountButton;
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

}
