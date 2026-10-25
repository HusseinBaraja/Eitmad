using System.IO;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Installation;
using Eitmad.WindowsSetup;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Tests.TestDoubles;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class OnboardingRenderedTests
{
    public TestContext TestContext { get; set; } = null!;
    private sealed class SetupBackend : IWindowsSetupBridge
    {
        public WindowsInstallationState? State { get; set; }
        public Func<Task<WindowsInstallationState?>>? Inspect { get; set; }
        public Func<IProgress<WindowsInstallProgress>, Task>? Install { get; set; }
        public int Calls { get; private set; }
        public GuidedSetupAction? Action { get; private set; }
        public Task<WindowsInstallationState?> InspectAsync() => Inspect?.Invoke() ?? Task.FromResult(State);
        public Task OpenDesktopAsync() => Task.CompletedTask;
        public string? RetryPassword { get; private set; }
        public Task RetryAsync(string? ownerPassword, IProgress<WindowsInstallProgress> progress)
        { Calls++; RetryPassword = ownerPassword; return Install?.Invoke(progress) ?? Task.FromException(new IOException("Synthetic retry failure")); }
        public Task InstallAsync(GuidedSetupAction action, InitializeWorkshop? workshop, SetupBackupDestinations? backup, IProgress<WindowsInstallProgress> progress)
        { Calls++; Action = action; return Install?.Invoke(progress) ?? Task.FromException(new IOException("Synthetic failure")); }
    }
    private void Record(Window window, int width, int height)
    {
        if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; WpfTestHost.CompleteLayout(window); }
        var dpi = VisualTreeHelper.GetDpi(window);
        TestContext.WriteLine($"Requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%; primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP.");
        Assert.AreEqual(FlowDirection.RightToLeft, window.FlowDirection);
        Assert.AreEqual("ar-ye", window.Language.IetfLanguageTag);
    }
    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Publisher_trust_requires_confirmation_and_keeps_fingerprint_readable(int width, int height)
    {
        WpfTestHost.RunWindow(() => new InstallerWindow { Width = width, Height = height }, window => {
            Record(window, width, height);
            ((InstallerWindow)window).ShowPublisherTrust("CN=Eitmad synthetic publisher", new string('A', 64));
            WpfTestHost.CompleteLayout(window);
            var trust = WpfTestHost.FindByName<Button>(window, "TrustPublisher");
            var confirm = WpfTestHost.FindByName<CheckBox>(window, "ConfirmedPublisher");
            Assert.IsFalse(trust.IsEnabled);
            confirm.BringIntoView(); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(confirm.Focus()); Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(confirm)));
            confirm.IsChecked = true;
            Assert.IsTrue(trust.IsEnabled); Assert.IsTrue(confirm.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            Assert.IsTrue(trust.Focus()); Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(trust)));
            var fingerprint = WpfTestHost.FindByName<TextBox>(window, "PublisherFingerprint");
            Assert.AreEqual(FlowDirection.LeftToRight, fingerprint.FlowDirection);
            fingerprint.BringIntoView(); WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"onboarding-publisher-trust-{width}");
            confirm.IsChecked = false; Assert.IsFalse(trust.IsEnabled);
        });
    }
    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Client_setup_and_payload_preparation_have_native_accessible_actions(int width, int height)
    {
        WpfTestHost.RunWindow(() => new SetupWindow(new SetupBackend(), WindowsSetupRole.Client) { Width = width, Height = height }, window => {
            Record(window, width, height);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<StackPanel>(window, "ServerFields").Visibility);
            var install = WpfTestHost.FindByName<Button>(window, "InstallButton");
            Assert.IsTrue(install.IsEnabled); Assert.IsTrue(install.Focus());
            Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(install)));
            Assert.IsTrue(install.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            WpfTestHost.CompleteLayout(window);
            var position = install.TransformToAncestor(window).Transform(new Point());
            Assert.IsTrue(install.IsVisible && position.Y + install.ActualHeight <= window.ActualHeight);
            WpfTestHost.Capture(window, $"onboarding-client-{width}");
        });
        WpfTestHost.RunWindow(() => new InstallerWindow { Width = width, Height = height }, window => {
            Record(window, width, height);
            var close = WpfTestHost.FindByName<Button>(window, "ClosePreparation");
            Assert.IsTrue(close.Focus());
            Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(close)));
            WpfTestHost.Capture(window, $"onboarding-preparation-{width}");
        });
    }
    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Server_setup_forms_and_interruption_are_accessible(int width, int height)
    {
        var backend = new SetupBackend();
        WpfTestHost.RunWindow(() => new SetupWindow(backend, WindowsSetupRole.Server) { Width = width, Height = height }, window =>
        {
            Record(window, width, height);
            var workshop = WpfTestHost.FindByName<TextBox>(window, "WorkshopName");
            Assert.IsTrue(workshop.Focus());
            Assert.IsTrue(workshop.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            WpfTestHost.FindByName<TextBox>(window, "WorkshopName").Text = "ورشة الاختبار للأثاث";
            WpfTestHost.FindByName<TextBox>(window, "OwnerUsername").Text = "مالك الاختبار";
            foreach (var button in WpfTestHost.Descendants<Button>(window).Where(b => b.IsVisible))
                Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(button)));
            Assert.AreEqual(FlowDirection.LeftToRight, WpfTestHost.FindByName<TextBox>(window, "LocalDestination").FlowDirection);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"onboarding-server-{width}");
            var local = WpfTestHost.FindByName<TextBox>(window, "LocalDestination");
            local.BringIntoView(); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(local.Focus());
            WpfTestHost.Capture(window, $"onboarding-backups-{width}");
            if (width == 720)
            {
                foreach (var control in WpfTestHost.Descendants<Control>(window)) Eitmad.WindowsShell.Controls.ControlOptions.SetHighContrast(control, true);
                WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window, "onboarding-backups-system-colors-720");
            }
            backend.State = new WindowsInstallationState { Version = 1, Interrupted = true, Recovery = WindowsSetupRecovery.OwnerPassword,
                Progress = new WindowsInstallProgress { Version = 1, Sequence = 8, Stage = WindowsInstallStage.EnrollDesktop, Status = WindowsInstallStatus.Running } };
            _ = ((SetupWindow)window).InspectAsync(); WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(window, "InstallButton").IsEnabled);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Button>(window, "OpenDesktopButton").Visibility);
            Assert.AreEqual(0, backend.Calls);
            var retry = WpfTestHost.FindByName<Button>(window, "RetryButton");
            Assert.IsTrue(retry.IsEnabled); Assert.IsTrue(retry.Focus());
            Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(retry)));
            var password = WpfTestHost.FindByName<PasswordBox>(window, "RecoveryPassword");
            password.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(password.Focus());
            Assert.IsTrue(password.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            WpfTestHost.Capture(window, $"onboarding-interrupted-{width}");
        });
    }
    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Recovery_screen_exposes_only_the_verified_action(int width, int height)
    {
        var backend = new SetupBackend { State = new WindowsInstallationState { Version = 1, Interrupted = true,
            Recovery = WindowsSetupRecovery.OwnerPassword,
            Progress = new WindowsInstallProgress { Version = 1, Sequence = 8, Stage = WindowsInstallStage.EnrollDesktop, Status = WindowsInstallStatus.Failed } } };
        WpfTestHost.RunWindow(() => new SetupWindow(backend, WindowsSetupRole.Server) { Width = width, Height = height }, window => {
            Record(window, width, height);
            var retry = WpfTestHost.FindByName<Button>(window, "RetryButton");
            Assert.IsTrue(retry.Focus());
            var password = WpfTestHost.FindByName<PasswordBox>(window, "RecoveryPassword");
            Assert.IsTrue(password.Focus()); Assert.IsTrue(password.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            WpfTestHost.Capture(window, $"setup-recovery-owner-{width}");
            foreach (var action in new[] { WindowsSetupRecovery.Retry, WindowsSetupRecovery.Wait, WindowsSetupRecovery.InspectManually }) {
                backend.State.Recovery = action;
                _ = ((SetupWindow)window).InspectAsync(); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(action == WindowsSetupRecovery.Retry, retry.IsEnabled);
                WpfTestHost.Capture(window, $"setup-recovery-{action}-{width}");
            }
        });
    }
    [TestMethod]
    public void Recovery_uses_backend_action_clears_password_and_requires_confirmed_success()
    {
        var backend = new SetupBackend { State = new WindowsInstallationState { Version = 1, Recovery = WindowsSetupRecovery.OwnerPassword,
            Progress = new WindowsInstallProgress { Version = 1, Sequence = 8, Stage = WindowsInstallStage.ConfigureBackups, Status = WindowsInstallStatus.Failed } } };
        WpfTestHost.RunWindow(() => new SetupWindow(backend, WindowsSetupRole.Server), window => {
            var password = WpfTestHost.FindByName<PasswordBox>(window, "RecoveryPassword"); password.Password = "Synthetic-password-123!";
            _ = ((SetupWindow)window).RetryAsync(); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(1, backend.Calls); Assert.AreEqual("", password.Password);
            Assert.AreEqual("Synthetic-password-123!", backend.RetryPassword);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Button>(window, "OpenDesktopButton").Visibility);
            foreach (var action in new[] { WindowsSetupRecovery.Wait, WindowsSetupRecovery.InspectManually }) {
                backend.State.Recovery = action; _ = ((SetupWindow)window).InspectAsync(); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Button>(window, "RetryButton").Visibility);
                _ = ((SetupWindow)window).RetryAsync(); Assert.AreEqual(1, backend.Calls);
            }
            backend.State.Recovery = WindowsSetupRecovery.Retry; _ = ((SetupWindow)window).InspectAsync(); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<FrameworkElement>(window, "RecoveryPasswordField").Visibility);
            backend.Install = _ => Task.CompletedTask;
            _ = ((SetupWindow)window).RetryAsync(); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(2, backend.Calls); Assert.IsNull(backend.RetryPassword);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Button>(window, "OpenDesktopButton").Visibility);
        });
    }
    [TestMethod]
    public void Backend_failure_never_displays_completion_or_repeats_installation()
    {
        var backend = new SetupBackend { Install = progress => {
            progress.Report(new WindowsInstallProgress { Version = 1, Sequence = 1, Stage = WindowsInstallStage.RegisterDesktop, Status = WindowsInstallStatus.Running });
            return Task.FromException(new IOException("Synthetic registration failure"));
        } };
        WpfTestHost.RunWindow(() => new SetupWindow(backend, WindowsSetupRole.Client), window => {
            _ = ((SetupWindow)window).InstallAsync(); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(1, backend.Calls);
            Assert.AreEqual(GuidedSetupAction.Client, backend.Action);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Button>(window, "OpenDesktopButton").Visibility);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(window, "InstallButton").IsEnabled);
        });
    }
    [TestMethod]
    [DataRow("inspect")] [DataRow("install")] [DataRow("retry")]
    public void Denied_payload_access_shows_setup_failure_and_restores_controls(string operation)
    {
        var inspection = new TaskCompletionSource<WindowsInstallationState?>();
        var mutation = new TaskCompletionSource();
        var backend = new SetupBackend { Install = _ => mutation.Task };
        if (operation == "retry") backend.State = new WindowsInstallationState {
            Version = 1, Recovery = WindowsSetupRecovery.Retry,
            Progress = new WindowsInstallProgress { Version = 1, Sequence = 1,
                Stage = WindowsInstallStage.Preflight, Status = WindowsInstallStatus.Failed },
        };
        WpfTestHost.RunWindow(() => new SetupWindow(backend, WindowsSetupRole.Client), window => {
            var view = (SetupWindow)window;
            backend.Inspect = () => inspection.Task;
            var pending = operation switch {
                "inspect" => view.InspectAsync(), "install" => view.InstallAsync(), _ => view.RetryAsync(),
            };
            var inspect = WpfTestHost.FindByName<Button>(window, "InspectButton");
            Assert.IsFalse(inspect.IsEnabled);
            var waiting = WpfTestHost.FindByName<TextBlock>(window, "SetupNotice").Text;
            var denied = new UnauthorizedAccessException("Synthetic payload access denied");
            if (operation == "inspect") inspection.SetException(denied);
            else mutation.SetException(denied);
            WpfTestHost.CompleteLayout(window);
            pending.GetAwaiter().GetResult();
            Assert.AreNotEqual(waiting, WpfTestHost.FindByName<TextBlock>(window, "SetupNotice").Text);
            Assert.IsTrue(inspect.IsEnabled);
            Assert.IsTrue(WpfTestHost.FindByName<StackPanel>(window, "InputPanel").IsEnabled);
            Assert.AreEqual(operation == "retry", WpfTestHost.FindByName<Button>(window, "RetryButton").IsEnabled);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(window, "InstallButton").IsEnabled);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Button>(window, "OpenDesktopButton").Visibility);
            if (operation != "inspect")
                Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<ProgressBar>(window, "BusyProgress").Visibility);
        });
    }
    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Discovery_pairing_rejection_and_owner_screen_keep_keyboard_paths(int width, int height)
    {
        var engine = new FakeEngine { DiscoveryHandler = () => Task.FromResult(new ServerDiscoveryResult {
            Status = DiscoveryStatus.Complete, Servers = [new DiscoveredServer { ServerId = Guid.NewGuid(), Address = "192.168.40.20", Port = 58443, ExpiresAt = DateTimeOffset.UtcNow.AddMinutes(1).ToUnixTimeMilliseconds() }] }),
            OnboardingHandler = action => action.Kind == DesktopOnboardingActionKind.State
                ? Task.FromResult(new DesktopOnboardingState { Paired = false })
                : Task.FromException<DesktopOnboardingState>(new IOException("Synthetic rejection")) };
        WpfTestHost.RunWindow(() => new Window { Width = width, Height = height, MinWidth = 720, MinHeight = 560,
            FlowDirection = FlowDirection.RightToLeft, Language = System.Windows.Markup.XmlLanguage.GetLanguage("ar-YE"),
            Content = new ScrollViewer { Content = new ServerSelectionView { Engine = engine, Margin = new Thickness(28), MaxWidth = 760 }, VerticalScrollBarVisibility = ScrollBarVisibility.Auto } }, window => {
            Record(window, width, height);
            var view = WpfTestHost.Descendants<ServerSelectionView>(window).Single();
            _ = view.DiscoverAsync(); WpfTestHost.CompleteLayout(window);
            var list = WpfTestHost.FindByName<ListBox>(window, "ServersList"); list.SelectedIndex = 0;
            var item = (ListBoxItem)list.ItemContainerGenerator.ContainerFromIndex(0);
            Assert.IsTrue(AutomationProperties.GetName(item).StartsWith("خادم الورشة", StringComparison.Ordinal));
            Assert.IsTrue(list.Focus()); Assert.IsTrue(list.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            var code = WpfTestHost.FindByName<PasswordBox>(window, "PairingCode");
            Assert.AreEqual(FlowDirection.LeftToRight, code.FlowDirection);
            WpfTestHost.Capture(window, $"onboarding-select-server-{width}");
            code.Password = "00000000";
            WpfTestHost.FindByName<Button>(window, "PairButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("", code.Password);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(window, "PairButton").IsEnabled);
            WpfTestHost.Capture(window, $"onboarding-pairing-rejected-{width}");
        });
        WpfTestHost.RunWindow(() => new OwnerPairingWindow(engine) { Width = width, Height = height }, window => {
            Record(window, width, height);
            var initialCode = WpfTestHost.FindByName<TextBlock>(window, "CodeText").Text;
            var open = WpfTestHost.FindByName<Button>(window, "OpenCodeButton"); Assert.IsTrue(open.Focus());
            open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(initialCode, WpfTestHost.FindByName<TextBlock>(window, "CodeText").Text);
            WpfTestHost.Capture(window, $"onboarding-owner-denied-{width}");
        });
        engine.OnboardingHandler = _ => Task.FromResult(new DesktopOnboardingState { Paired = false });
        WpfTestHost.Run(width, height, window => {
            Record(window, width, height);
            var view = window.SignInSurface; view.Attach(engine);
            _ = view.LoadConnectionAsync(); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<ServerSelectionView>(view, "ServerSelection").Visibility);
            WpfTestHost.Capture(window, $"onboarding-sign-in-{width}");
            var reconnect = WpfTestHost.FindByName<Button>(view, "ReconnectButton");
            reconnect.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(reconnect.Focus());
            Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(reconnect)));
            engine.OnboardingHandler = _ => Task.FromResult(new DesktopOnboardingState { Paired = true });
            _ = view.LoadConnectionAsync(); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<ServerSelectionView>(view, "ServerSelection").Visibility);
            WpfTestHost.Capture(window, $"onboarding-retained-connection-{width}");
        }, showSignIn: true);
        VerifyStaffRoute(engine, width, height);
    }
    private void VerifyStaffRoute(FakeEngine engine, int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            Record(window, width, height);
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var pair = WpfTestHost.FindByAutomationName<Button>(window.UsersSurface, "ربط جهاز جديد بالورشة");
            Assert.IsTrue(pair.IsEnabled); Assert.IsTrue(pair.Focus());
            WpfTestHost.Capture(window, $"onboarding-users-{width}");
            window.Dispatcher.BeginInvoke(() => {
                var pairing = window.OwnedWindows.OfType<OwnerPairingWindow>().Single();
                var staff = WpfTestHost.FindByAutomationName<Button>(pairing, "فتح إدارة حسابات الموظفين");
                Assert.IsTrue(staff.Focus()); staff.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            });
            pair.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window); Assert.IsTrue(window.UsersSurface.IsVisible);
        }, engine: engine);
    }
}
