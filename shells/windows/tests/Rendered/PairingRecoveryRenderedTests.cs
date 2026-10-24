using System.IO;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Automation.Peers;
using System.Windows.Automation.Provider;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Tests.TestDoubles;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class PairingRecoveryRenderedTests
{
    public TestContext TestContext { get; set; } = null!;

    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Saved_exchange_retries_and_requires_confirmed_abandonment_before_a_fresh_code(int width, int height)
    {
        var server = Guid.NewGuid();
        var state = new DesktopOnboardingState { Paired = false, PendingPairingServer = server };
        var abandonment = new TaskCompletionSource<DesktopOnboardingState>();
        var calls = new List<DesktopOnboardingActionKind>();
        var pairCalls = 0;
        var pairedEvents = 0;
        var discardFails = true;
        var engine = new FakeEngine {
            DiscoveryHandler = () => Task.FromResult(new ServerDiscoveryResult { Status = DiscoveryStatus.Complete,
                Servers = [new DiscoveredServer { ServerId = server, Address = "192.168.40.20", Port = 58443 },
                    new DiscoveredServer { ServerId = Guid.NewGuid(), Address = "192.168.40.21", Port = 58443 }] }),
            OnboardingHandler = action => {
                calls.Add(action.Kind);
                if (action.Kind == DesktopOnboardingActionKind.State) return Task.FromResult(state);
                if (action.Kind == DesktopOnboardingActionKind.DiscardPendingPairing)
                    return discardFails ? Task.FromException<DesktopOnboardingState>(new IOException("Synthetic storage failure")) : abandonment.Task;
                Assert.AreEqual(DesktopOnboardingActionKind.Pair, action.Kind);
                pairCalls++;
                if (pairCalls == 1) {
                    Assert.AreEqual("", action.Payload.Code);
                    Assert.AreEqual(server, action.Payload.Candidate.ServerId);
                    return Task.FromException<DesktopOnboardingState>(new IOException("Synthetic lost reply"));
                }
                Assert.AreEqual("12345678", action.Payload.Code);
                return Task.FromResult(new DesktopOnboardingState { Paired = true });
            }
        };
        WpfTestHost.RunWindow(() => new Window {
            Width = width, Height = height, MinWidth = 720, MinHeight = 560,
            FlowDirection = FlowDirection.RightToLeft, Language = System.Windows.Markup.XmlLanguage.GetLanguage("ar-YE"),
            Content = new ScrollViewer { VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                Content = new ServerSelectionView { Engine = engine, Margin = new Thickness(28), MaxWidth = 760 } }
        }, window => {
            if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
            WpfTestHost.CompleteLayout(window);
            var dpi = VisualTreeHelper.GetDpi(window);
            TestContext.WriteLine($"Requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%; primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP.");
            var view = WpfTestHost.Descendants<ServerSelectionView>(window).Single();
            view.Paired += (_, _) => pairedEvents++;
            _ = view.DiscoverAsync(); WpfTestHost.CompleteLayout(window);
            var retry = WpfTestHost.FindByName<Button>(view, "PairButton");
            var discard = WpfTestHost.FindByName<Button>(view, "DiscardButton");
            var code = WpfTestHost.FindByName<PasswordBox>(view, "PairingCode");
            var list = WpfTestHost.FindByName<ListBox>(view, "ServersList");
            Assert.AreEqual(0, list.SelectedIndex);
            list.SelectedIndex = 1; Assert.IsFalse(retry.IsEnabled);
            list.SelectedIndex = 0; Assert.IsTrue(retry.IsEnabled);
            Assert.IsFalse(code.IsVisible);
            Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
            Assert.AreEqual("ar-ye", view.Language.IetfLanguageTag);
            Invoke(retry, window);
            Assert.AreEqual(1, pairCalls); Assert.AreEqual(0, pairedEvents);
            Assert.IsTrue(retry.IsEnabled);
            Assert.IsTrue(retry.Focus());
            Assert.IsTrue(retry.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            Assert.AreSame(discard, Keyboard.FocusedElement);
            Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(discard)));
            discard.BringIntoView(); WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"pairing-recovery-pending-{width}");

            Invoke(discard, window);
            Assert.IsFalse(code.IsVisible); Assert.IsTrue(retry.IsEnabled);
            Assert.AreEqual(0, pairedEvents);
            discardFails = false;
            Invoke(discard, window);
            Assert.IsFalse(retry.IsEnabled); Assert.IsFalse(discard.IsEnabled); Assert.IsFalse(list.IsEnabled);
            state = new DesktopOnboardingState { Paired = false };
            abandonment.SetResult(state); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(code.IsVisible); Assert.IsFalse(discard.IsVisible);
            Assert.AreEqual(FlowDirection.LeftToRight, code.FlowDirection);
            Assert.AreSame(code, Keyboard.FocusedElement);
            WpfTestHost.Capture(window, $"pairing-recovery-fresh-code-{width}");
            code.Password = "12345678";
            Invoke(retry, window);
            Assert.AreEqual(2, pairCalls); Assert.AreEqual(1, pairedEvents);
            Assert.AreEqual("", code.Password);
            CollectionAssert.Contains(calls, DesktopOnboardingActionKind.DiscardPendingPairing);
        });
    }

    private static void Invoke(Button button, Window window)
    {
        var provider = (IInvokeProvider)new ButtonAutomationPeer(button).GetPattern(PatternInterface.Invoke);
        provider.Invoke(); WpfTestHost.CompleteLayout(window);
    }

    [TestMethod]
    public void Lost_native_reply_uses_confirmed_committed_state_to_open_sign_in()
    {
        var paired = false;
        var events = 0;
        var engine = new FakeEngine {
            DiscoveryHandler = () => Task.FromResult(new ServerDiscoveryResult { Status = DiscoveryStatus.Complete,
                Servers = [new DiscoveredServer { ServerId = Guid.NewGuid(), Address = "192.168.40.20", Port = 58443 }] }),
            OnboardingHandler = action => {
                if (action.Kind == DesktopOnboardingActionKind.State)
                    return Task.FromResult(new DesktopOnboardingState { Paired = paired });
                Assert.AreEqual(DesktopOnboardingActionKind.Pair, action.Kind);
                paired = true;
                return Task.FromException<DesktopOnboardingState>(new IOException("Synthetic lost native reply"));
            }
        };
        WpfTestHost.RunWindow(() => new Window { Content = new ServerSelectionView { Engine = engine } }, window => {
            var view = WpfTestHost.Descendants<ServerSelectionView>(window).Single();
            view.Paired += (_, _) => events++;
            _ = view.DiscoverAsync(); WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<ListBox>(view, "ServersList").SelectedIndex = 0;
            Invoke(WpfTestHost.FindByName<Button>(view, "PairButton"), window);
            Assert.AreEqual(1, events);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "PairButton").IsEnabled);
        });
    }
}
