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
public sealed class ServerSelectionLeaseRenderedTests
{
    public TestContext TestContext { get; set; } = null!;

    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Code_entered_after_lease_expiry_revalidates_before_pairing_and_uses_the_fresh_route(int width, int height)
    {
        var original = Candidate();
        original.ExpiresAt = DateTimeOffset.UtcNow.AddSeconds(-1).ToUnixTimeMilliseconds();
        var fresh = Candidate(original.ServerId);
        fresh.Address = "192.168.40.25";
        var refresh = new TaskCompletionSource<ServerDiscoveryResult>();
        var pairCalls = 0;
        var paired = 0;
        var engine = new FakeEngine {
            RevalidationHandler = (selected, _) => {
                if (selected is null) return Task.FromResult(Found(original));
                Assert.AreEqual(original.ServerId, selected);
                return refresh.Task;
            },
            OnboardingHandler = action => {
                if (action.Kind == DesktopOnboardingActionKind.State)
                    return Task.FromResult(new DesktopOnboardingState());
                Assert.AreEqual(DesktopOnboardingActionKind.Pair, action.Kind);
                Assert.AreSame(fresh, action.Payload.Candidate);
                Assert.AreEqual("12345678", action.Payload.Code);
                pairCalls++;
                return Task.FromResult(new DesktopOnboardingState { Paired = true });
            }
        };
        Run(width, height, engine, (window, view) => {
            view.Paired += (_, _) => paired++;
            var list = WpfTestHost.FindByName<ListBox>(view, "ServersList");
            list.SelectedIndex = 0;
            var code = WpfTestHost.FindByName<PasswordBox>(view, "PairingCode");
            var pair = WpfTestHost.FindByName<Button>(view, "PairButton");
            var cancel = WpfTestHost.FindByName<Button>(view, "CancelDiscoveryButton");
            Assert.IsTrue(code.Focus());
            Assert.IsTrue(code.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            Assert.AreSame(pair, Keyboard.FocusedElement);
            code.Focus(); code.Password = "12345678";
            var enter = RaiseKey(code, Key.Enter, Keyboard.KeyDownEvent);
            Assert.IsTrue(enter.Handled);
            Assert.AreEqual(0, pairCalls); Assert.AreEqual(0, paired);
            Assert.IsFalse(pair.IsEnabled); Assert.IsFalse(list.IsEnabled);
            Assert.AreEqual("12345678", code.Password);
            Assert.IsTrue(cancel.IsVisible); Assert.IsTrue(cancel.IsEnabled);
            Assert.AreSame(cancel, Keyboard.FocusedElement);
            Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(cancel)));
            WpfTestHost.Capture(window, $"server-lease-checking-{width}");
            RaiseKey(code, Key.Enter, Keyboard.KeyDownEvent);
            refresh.SetResult(Found(fresh)); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(1, pairCalls); Assert.AreEqual(1, paired);
            Assert.AreEqual("", code.Password); Assert.IsFalse(pair.IsEnabled);
            Assert.AreEqual(0, list.SelectedIndex);
        });
    }

    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Failed_revalidation_clears_selection_and_code_without_starting_pairing(int width, int height)
    {
        foreach (var status in new[] { DiscoveryStatus.ServerMissing, DiscoveryStatus.IdentityConflict,
            DiscoveryStatus.NetworkChanged, DiscoveryStatus.Cancelled, DiscoveryStatus.Unavailable })
        {
            var candidate = Candidate();
            var pairCalls = 0;
            var engine = new FakeEngine {
                RevalidationHandler = (selected, _) => Task.FromResult(selected is null ? Found(candidate)
                    : new ServerDiscoveryResult { Status = status, Servers = [] }),
                OnboardingHandler = action => {
                    if (action.Kind != DesktopOnboardingActionKind.State) pairCalls++;
                    return Task.FromResult(new DesktopOnboardingState());
                }
            };
            Run(width, height, engine, (window, view) => {
                var list = WpfTestHost.FindByName<ListBox>(view, "ServersList"); list.SelectedIndex = 0;
                var code = WpfTestHost.FindByName<PasswordBox>(view, "PairingCode"); code.Password = "12345678";
                var pair = WpfTestHost.FindByName<Button>(view, "PairButton");
                Invoke(pair, window);
                Assert.AreEqual(0, pairCalls); Assert.AreEqual(-1, list.SelectedIndex);
                Assert.AreEqual("", code.Password); Assert.IsFalse(pair.IsEnabled);
                Assert.AreSame(WpfTestHost.FindByName<Button>(view, "DiscoverButton"), Keyboard.FocusedElement);
                var notice = WpfTestHost.FindByName<TextBlock>(view, "ConnectionNotice");
                Assert.IsTrue(notice.Text.Contains(status switch {
                    DiscoveryStatus.ServerMissing => "لم يعد", DiscoveryStatus.IdentityConflict => "تعارض",
                    DiscoveryStatus.NetworkChanged => "تغيرت الشبكة", DiscoveryStatus.Cancelled => "أُلغي", _ => "تعذر" }, StringComparison.Ordinal));
                Assert.AreEqual(AutomationLiveSetting.Polite, AutomationProperties.GetLiveSetting(notice));
                WpfTestHost.Capture(window, $"server-lease-{status}-{width}");
            });
        }
    }

    [TestMethod]
    [DataRow(false)] [DataRow(true)]
    public void Escape_or_cancel_discards_a_late_offer_and_returns_focus_to_search(bool useEscape)
    {
        var candidate = Candidate();
        var refresh = new TaskCompletionSource<ServerDiscoveryResult>();
        CancellationToken scanToken = default;
        var pairCalls = 0;
        var engine = new FakeEngine {
            RevalidationHandler = (selected, token) => {
                if (selected is null) return Task.FromResult(Found(candidate));
                scanToken = token; return refresh.Task;
            },
            OnboardingHandler = action => {
                if (action.Kind != DesktopOnboardingActionKind.State) pairCalls++;
                return Task.FromResult(new DesktopOnboardingState());
            }
        };
        Run(720, 560, engine, (window, view) => {
            WpfTestHost.FindByName<ListBox>(view, "ServersList").SelectedIndex = 0;
            WpfTestHost.FindByName<PasswordBox>(view, "PairingCode").Password = "12345678";
            Invoke(WpfTestHost.FindByName<Button>(view, "PairButton"), window);
            var cancel = WpfTestHost.FindByName<Button>(view, "CancelDiscoveryButton");
            if (useEscape) Assert.IsTrue(RaiseKey(cancel, Key.Escape, Keyboard.PreviewKeyDownEvent).Handled);
            else Invoke(cancel, window);
            Assert.IsTrue(scanToken.IsCancellationRequested);
            refresh.SetResult(Found(candidate)); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(0, pairCalls);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "PairButton").IsEnabled);
            Assert.AreSame(WpfTestHost.FindByName<Button>(view, "DiscoverButton"), Keyboard.FocusedElement);
        });
    }

    [TestMethod]
    public void Clearing_the_screen_cancels_revalidation_and_ignores_its_late_completion()
    {
        var candidate = Candidate();
        var refresh = new TaskCompletionSource<ServerDiscoveryResult>();
        var calls = 0;
        var engine = new FakeEngine {
            RevalidationHandler = (selected, _) => selected is null ? Task.FromResult(Found(candidate)) : refresh.Task,
            OnboardingHandler = _ => { calls++; return Task.FromResult(new DesktopOnboardingState()); }
        };
        Run(720, 560, engine, (window, view) => {
            WpfTestHost.FindByName<ListBox>(view, "ServersList").SelectedIndex = 0;
            Invoke(WpfTestHost.FindByName<Button>(view, "PairButton"), window);
            view.Clear(); refresh.SetResult(Found(candidate)); WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(1, calls);
            Assert.AreEqual(0, WpfTestHost.FindByName<ListBox>(view, "ServersList").Items.Count);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "PairButton").IsEnabled);
        });
    }

    private void Run(int width, int height, FakeEngine engine, Action<Window, ServerSelectionView> check)
    {
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
            _ = view.DiscoverAsync(); WpfTestHost.CompleteLayout(window);
            check(window, view);
        });
    }

    private static DiscoveredServer Candidate(Guid? id = null) => new() {
        ServerId = id ?? Guid.NewGuid(), Address = "192.168.40.20", Port = 58443,
        ExpiresAt = DateTimeOffset.UtcNow.AddSeconds(15).ToUnixTimeMilliseconds() };
    private static ServerDiscoveryResult Found(DiscoveredServer candidate) => new() { Status = DiscoveryStatus.Complete, Servers = [candidate] };
    private static KeyEventArgs RaiseKey(UIElement target, Key key, RoutedEvent routedEvent)
    {
        var args = new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(target), 0, key) { RoutedEvent = routedEvent };
        target.RaiseEvent(args); WpfTestHost.PumpDispatcher(); return args;
    }
    private static void Invoke(Button button, Window window)
    {
        ((IInvokeProvider)new ButtonAutomationPeer(button).GetPattern(PatternInterface.Invoke)).Invoke();
        WpfTestHost.CompleteLayout(window);
    }
}
