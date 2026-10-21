using System.IO;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Installation;
using Eitmad.WindowsSetup;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class BackupManagementRenderedTests
{
    public TestContext TestContext { get; set; } = null!;
    private sealed class Backend : IWindowsBackupBridge
    {
        public ServerBackupManagement State { get; } = new() {
            Version = 1, IntervalHours = 24, RetentionCount = 7, ActionOutcome = ServerBackupOutcome.Succeeded,
            Destinations = new SetupBackupDestinations { Local = @"C:\SyntheticBackup", External = @"E:\SyntheticBackup" },
            Status = new ServerBackupStatus { Version = 1, DeploymentId = Guid.NewGuid(), LastLocalSuccessAt = 1792526400,
                LastExternalSuccessAt = 1792440000, NextDueAt = 1792612800, ExternalConfigured = true,
                LocalOutcome = ServerBackupOutcome.Succeeded, ExternalOutcome = ServerBackupOutcome.DestinationUnavailable,
                ScheduleOutcome = ServerBackupOutcome.ScheduleFailed, Overdue = true, DiskLossProtected = false },
        };
        public int Calls { get; private set; }
        public SetupBackupDestinations? Submitted { get; private set; }
        public Func<Task<ServerBackupManagement>>? Response { get; set; }
        public Task<ServerBackupManagement> ManageAsync(ServerBackupAction action, SetupBackupDestinations? destinations = null)
        { Calls++; Submitted = destinations; return Response?.Invoke() ?? Task.FromResult(State); }
    }
    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Backup_screen_renders_failures_destinations_and_keyboard_paths(int width, int height)
    {
        var backend = new Backend();
        WpfTestHost.RunWindow(() => new BackupWindow(backend) { Width = width, Height = height }, window => {
            if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; WpfTestHost.CompleteLayout(window); }
            var dpi = VisualTreeHelper.GetDpi(window);
            TestContext.WriteLine($"Before capture: primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%.");
            Assert.AreEqual(FlowDirection.RightToLeft, window.FlowDirection);
            Assert.AreEqual("ar-ye", window.Language.IetfLanguageTag);
            _ = ((BackupWindow)window).ManageAsync(ServerBackupAction.Inspect); WpfTestHost.CompleteLayout(window);
            var refresh = WpfTestHost.FindByName<Button>(window, "RefreshButton"); Assert.IsTrue(refresh.Focus());
            Assert.IsTrue(refresh.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            foreach (var button in WpfTestHost.Descendants<Button>(window).Where(b => b.IsVisible))
                Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(button)));
            WpfTestHost.Capture(window, $"backup-status-{width}");
            WpfTestHost.Descendants<Expander>(window).Single().IsExpanded = true; WpfTestHost.CompleteLayout(window);
            var local = WpfTestHost.FindByName<TextBox>(window, "LocalDestination"); local.BringIntoView(); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(FlowDirection.LeftToRight, local.FlowDirection); Assert.IsTrue(local.Focus());
            Assert.IsTrue(local.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            WpfTestHost.FindByName<Button>(window, "SaveButton").BringIntoView(); WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"backup-destinations-{width}");
            if (width == 720) {
                foreach (var control in WpfTestHost.Descendants<Control>(window)) Eitmad.WindowsShell.Controls.ControlOptions.SetHighContrast(control, true);
                WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window, "backup-system-colors-720");
            }
        });
    }

    [TestMethod]
    public void Capture_failure_retains_last_success_and_rejected_save_uses_reported_destinations()
    {
        var backend = new Backend();
        WpfTestHost.RunWindow(() => new BackupWindow(backend), window => {
            var view = (BackupWindow)window;
            _ = view.ManageAsync(ServerBackupAction.Inspect);
            var last = WpfTestHost.FindByName<TextBlock>(window, "LastCapture").Text;
            var notices = new HashSet<string>();
            foreach (var outcome in new[] { ServerBackupOutcome.InsufficientSpace, ServerBackupOutcome.CaptureFailed,
                ServerBackupOutcome.Interrupted, ServerBackupOutcome.RetentionFailed, ServerBackupOutcome.DestinationUnavailable }) {
                backend.State.ActionOutcome = backend.State.Status.LocalOutcome = outcome;
                _ = view.ManageAsync(ServerBackupAction.Capture); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(last, WpfTestHost.FindByName<TextBlock>(window, "LastCapture").Text);
                Assert.IsTrue(notices.Add(WpfTestHost.FindByName<TextBlock>(window, "Notice").Text));
            }
            var local = (TextBox)window.FindName("LocalDestination"); local.Text = @"D:\RejectedSynthetic";
            backend.State.ActionOutcome = ServerBackupOutcome.DestinationUnavailable;
            _ = view.ManageAsync(ServerBackupAction.SaveDestinations);
            Assert.AreEqual(@"D:\RejectedSynthetic", backend.Submitted!.Local);
            Assert.AreEqual(backend.State.Destinations.Local, local.Text);
            backend.Response = () => Task.FromException<ServerBackupManagement>(new IOException("Synthetic administrator cancellation"));
            _ = view.ManageAsync(ServerBackupAction.Inspect); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<StackPanel>(window, "StatePanel").Visibility);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(window, "CaptureButton").IsEnabled);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(window, "RefreshButton").IsEnabled);
            var calls = backend.Calls; _ = view.ManageAsync(ServerBackupAction.Capture); Assert.AreEqual(calls, backend.Calls);
        });
    }

    [TestMethod]
    public void Busy_request_prevents_duplicate_mutations_and_unconfirmed_success()
    {
        var backend = new Backend();
        WpfTestHost.RunWindow(() => new BackupWindow(backend), window => {
            var view = (BackupWindow)window; _ = view.ManageAsync(ServerBackupAction.Inspect);
            var completion = new TaskCompletionSource<ServerBackupManagement>(); backend.Response = () => completion.Task;
            var pending = view.ManageAsync(ServerBackupAction.Capture);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(window, "CaptureButton").IsEnabled);
            var calls = backend.Calls; _ = view.ManageAsync(ServerBackupAction.Capture); Assert.AreEqual(calls, backend.Calls);
            completion.SetResult(backend.State); WpfTestHost.CompleteLayout(window); Assert.IsTrue(pending.IsCompletedSuccessfully);
        });
    }
}
