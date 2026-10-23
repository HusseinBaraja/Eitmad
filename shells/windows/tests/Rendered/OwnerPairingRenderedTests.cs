using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Tests.TestDoubles;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class OwnerPairingRenderedTests
{
    public TestContext TestContext { get; set; } = null!;

    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Refused_replacement_keeps_the_active_code_and_countdown_until_expiry(int width, int height)
    {
        var replacement = new TaskCompletionSource<DesktopOnboardingState>();
        var calls = 0;
        var issued = new DesktopOnboardingState();
        var engine = new FakeEngine { OnboardingHandler = action => {
            Assert.AreEqual(DesktopOnboardingActionKind.OpenPairing, action.Kind);
            calls++;
            if (calls > 1) return replacement.Task;
            issued = new DesktopOnboardingState { Code = "12345678",
                ExpiresAt = DateTimeOffset.UtcNow.AddSeconds(5).ToUnixTimeMilliseconds() };
            return Task.FromResult(issued);
        } };
        engine.Connect();
        WpfTestHost.RunWindow(() => new OwnerPairingWindow(engine) { Width = width, Height = height }, window => {
            if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
            WpfTestHost.CompleteLayout(window);
            var dpi = VisualTreeHelper.GetDpi(window);
            TestContext.WriteLine($"Requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%; primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP.");
            var open = WpfTestHost.FindByName<Button>(window, "OpenCodeButton");
            var code = WpfTestHost.FindByName<TextBlock>(window, "CodeText");
            var expiry = WpfTestHost.FindByName<TextBlock>(window, "ExpiryText");
            var notice = WpfTestHost.FindByName<TextBlock>(window, "PairingNotice");
            open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("12345678", code.Text);
            Assert.AreEqual("", issued.Code);
            var countdown = expiry.Text;
            open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(open.IsEnabled);
            Assert.AreEqual("12345678", code.Text);
            Assert.AreEqual(countdown, expiry.Text);
            replacement.SetException(new EngineIpcException(EngineIpcFailureKind.AuthenticationRejected, "Synthetic active-code refusal"));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(2, calls);
            Assert.AreEqual("12345678", code.Text);
            Assert.IsTrue(notice.Text.Contains("الرمز الحالي ما زال صالحًا", StringComparison.Ordinal));
            Assert.AreEqual(AutomationLiveSetting.Assertive, AutomationProperties.GetLiveSetting(notice));
            Assert.IsTrue(open.IsEnabled); Assert.IsTrue(open.Focus());
            Assert.IsTrue(open.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            WpfTestHost.Capture(window, $"owner-pairing-active-code-refused-{width}");
            PumpUntil(() => expiry.Text != countdown);
            Assert.AreEqual("12345678", code.Text);
            PumpUntil(() => code.Text != "12345678");
            Assert.AreEqual("انتهت صلاحية الرمز", code.Text);
            Assert.AreEqual("", notice.Text);
        });
    }

    [TestMethod]
    [DataRow(false)] [DataRow(true)]
    public void Active_code_is_cleared_on_disconnect_or_close(bool close)
    {
        var engine = new FakeEngine { OnboardingHandler = _ => Task.FromResult(new DesktopOnboardingState {
            Code = "12345678", ExpiresAt = DateTimeOffset.UtcNow.AddMinutes(1).ToUnixTimeMilliseconds(),
        }) };
        engine.Connect();
        WpfTestHost.RunWindow(() => new OwnerPairingWindow(engine), window => {
            var code = WpfTestHost.FindByName<TextBlock>(window, "CodeText");
            var expiry = WpfTestHost.FindByName<TextBlock>(window, "ExpiryText");
            WpfTestHost.FindByName<Button>(window, "OpenCodeButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("12345678", code.Text);
            if (close) window.Close(); else engine.Disconnect();
            WpfTestHost.PumpDispatcher();
            Assert.AreEqual("", code.Text); Assert.AreEqual("", expiry.Text);
        });
    }

    private static void PumpUntil(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow.AddSeconds(8);
        var frame = new DispatcherFrame();
        var timer = new DispatcherTimer(DispatcherPriority.ApplicationIdle) { Interval = TimeSpan.FromMilliseconds(50) };
        timer.Tick += (_, _) => {
            if (condition() || DateTime.UtcNow >= deadline) { timer.Stop(); frame.Continue = false; }
        };
        timer.Start(); Dispatcher.PushFrame(frame);
        Assert.IsTrue(condition(), "The rendered pairing state did not update before the deadline.");
    }
}
