using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Authentication;

public partial class ServerSelectionView : UserControl
{
    public ServerSelectionView()
    {
        InitializeComponent();
        Unloaded += (_, _) => Clear();
    }
    public IEngineShellBridge? Engine { get; set; }
    public event EventHandler? Paired;
    private bool busy;
    private bool stateAvailable;
    private Guid? pendingServer;
    private CancellationTokenSource? discoveryCancellation;
    private int generation;
    private sealed record Choice(DiscoveredServer Candidate)
    {
        public string Label => $"{Candidate.Address}:{Candidate.Port}";
        public string Detail => Candidate.IdentityConflict ? "تعارض في هوية الخادم. أبلغ مسؤول الورشة." : "خادم متاح للربط برمز المالك";
        public string AccessibleName => $"خادم الورشة {Label}. {Detail}";
    }
    private async void DiscoverClick(object sender, RoutedEventArgs e) => await DiscoverAsync();
    public async Task DiscoverAsync()
    {
        if (busy || Engine is null) return;
        using var cancellation = new CancellationTokenSource();
        discoveryCancellation = cancellation;
        var currentGeneration = generation;
        SetBusy(true); SetChecking(true); PairingCode.Clear(); ServersList.ItemsSource = null;
        ConnectionNotice.Text = "جارٍ البحث على الشبكات الخاصة…";
        try
        {
            var state = await Engine.OnboardingAsync(new DesktopOnboardingAction { Kind = DesktopOnboardingActionKind.State }, cancellation.Token);
            cancellation.Token.ThrowIfCancellationRequested();
            ApplyState(state);
            if (state.Paired) return;
            var result = await Engine.DiscoverServersAsync(cancellation.Token);
            cancellation.Token.ThrowIfCancellationRequested();
            ServersList.ItemsSource = result.Servers.Select(candidate => new Choice(candidate)).ToArray();
            if (pendingServer is not null)
                ServersList.SelectedItem = ServersList.Items.Cast<Choice>().FirstOrDefault(choice => choice.Candidate.ServerId == pendingServer && !choice.Candidate.IdentityConflict);
            ConnectionNotice.Text = result.Status is DiscoveryStatus.Complete
                ? result.Servers.Length == 0 ? "لم يظهر خادم. شغّل جهاز الخادم، وتحقق من اتصال الجهازين بالشبكة الخاصة نفسها، ثم أعد البحث."
                    : pendingServer is not null ? "يوجد طلب ربط محفوظ. أعد المحاولة مع الخادم نفسه دون إدخال الرمز، أو اترك الطلب لاستخدام رمز جديد." : "اختر خادم الورشة، ثم أدخل الرمز الظاهر لدى المالك."
                : DiscoveryNotice(result.Status);
        }
        catch (OperationCanceledException)
        { if (currentGeneration == generation) InvalidateSelection(DiscoveryStatus.Cancelled); }
        catch (Exception error) when (error is EngineIpcException or NotSupportedException or System.IO.IOException)
        { if (currentGeneration != generation) return; stateAvailable = false; RecoveryPanel.Visibility = Visibility.Visible; ConnectionNotice.Text = "تعذر التحقق من طلب الربط أو البحث. أعد البحث، أو اترك الطلب المحفوظ لاستخدام رمز جديد."; }
        finally
        {
            if (currentGeneration == generation) { discoveryCancellation = null; SetChecking(false); SetBusy(false); DiscoverButton.Focus(); }
        }
    }
    private void SelectionChanged(object sender, SelectionChangedEventArgs e) => UpdatePairButton();
    private void UpdatePairButton() => PairButton.IsEnabled = !busy && stateAvailable && ServersList.SelectedItem is Choice { Candidate.IdentityConflict: false } choice
        && (pendingServer is null || choice.Candidate.ServerId == pendingServer);
    private void ApplyState(DesktopOnboardingState state)
    {
        stateAvailable = !state.Paired;
        pendingServer = state.PendingPairingServer;
        CodeField.Visibility = pendingServer is null ? Visibility.Visible : Visibility.Collapsed;
        RecoveryPanel.Visibility = pendingServer is null ? Visibility.Collapsed : Visibility.Visible;
        PairButton.Content = pendingServer is null ? "ربط هذا الجهاز" : "إعادة محاولة الربط نفسه";
        System.Windows.Automation.AutomationProperties.SetName(PairButton,
            pendingServer is null ? "ربط الجهاز بالخادم المحدد" : "إعادة محاولة طلب الربط المحفوظ مع الخادم نفسه");
        UpdatePairButton();
        if (state.Paired)
        {
            ConnectionNotice.Text = "تم ربط الجهاز. سجل الدخول بحسابك الشخصي.";
            Paired?.Invoke(this, EventArgs.Empty);
        }
    }
    private async void PairClick(object sender, RoutedEventArgs e) => await PairAsync();
    private async void CodeKeyDown(object sender, System.Windows.Input.KeyEventArgs e)
    { if (e.Key == Key.Enter && PairButton.IsEnabled) { e.Handled = true; await PairAsync(); } }
    private async Task PairAsync()
    {
        if (!PairButton.IsEnabled || Engine is null || ServersList.SelectedItem is not Choice choice) return;
        using var cancellation = new CancellationTokenSource();
        discoveryCancellation = cancellation;
        var currentGeneration = generation;
        DesktopOnboardingAction? action = null;
        SetBusy(true); SetChecking(true);
        ConnectionNotice.Text = "جارٍ التحقق من توفر الخادم المحدد قبل إرسال الرمز…";
        try
        {
            var refreshed = await Engine.DiscoverServersAsync(cancellation.Token, choice.Candidate.ServerId);
            cancellation.Token.ThrowIfCancellationRequested();
            if (refreshed.Status != DiscoveryStatus.Complete || refreshed.Servers.Length == 0)
            {
                InvalidateSelection(refreshed.Status);
                return;
            }
            // Rust returns only fresh, non-conflicting offers for the selected identity.
            var candidate = refreshed.Servers[0];
            ServersList.ItemsSource = refreshed.Servers.Select(server => new Choice(server)).ToArray();
            ServersList.SelectedItem = ServersList.Items.Cast<Choice>().First(item => item.Candidate == candidate);
            SetChecking(false); discoveryCancellation = null;
            action = new DesktopOnboardingAction { Kind = DesktopOnboardingActionKind.Pair,
                Payload = new DesktopOnboardingActionPayload { Candidate = candidate,
                    Code = pendingServer is null ? PairingCode.Password : string.Empty } };
            PairingCode.Clear();
            ConnectionNotice.Text = "جارٍ التحقق من الرمز وهوية الخادم…";
            var result = await Engine.OnboardingAsync(action);
            if (currentGeneration != generation) return;
            if (!result.Paired) throw new InvalidOperationException();
            ApplyState(result);
        }
        catch (OperationCanceledException) when (action is null)
        { if (currentGeneration == generation) InvalidateSelection(DiscoveryStatus.Cancelled); }
        catch (Exception error) when (error is EngineIpcException or NotSupportedException or System.IO.IOException or InvalidOperationException or OperationCanceledException)
        {
            if (currentGeneration != generation) return;
            if (action is null) { InvalidateSelection(DiscoveryStatus.Unavailable); return; }
            ConnectionNotice.Text = "لم يتأكد الربط. أعد المحاولة مع الخادم نفسه. إذا انتهت صلاحية الرمز، اترك الطلب واستخدم رمزًا جديدًا من المالك.";
            try
            {
                var state = await Engine.OnboardingAsync(new DesktopOnboardingAction { Kind = DesktopOnboardingActionKind.State });
                if (currentGeneration == generation) ApplyState(state);
            }
            catch (Exception stateError) when (stateError is EngineIpcException or NotSupportedException or System.IO.IOException)
            { if (currentGeneration == generation) { stateAvailable = false; RecoveryPanel.Visibility = Visibility.Visible; } }
        }
        finally
        {
            if (action is not null) action.Payload.Code = string.Empty;
            if (currentGeneration == generation)
            {
                discoveryCancellation = null; SetChecking(false); SetBusy(false);
                if (ServersList.SelectedItem is null) DiscoverButton.Focus();
                else if (stateAvailable && pendingServer is null) PairingCode.Focus();
            }
        }
    }
    private static string DiscoveryNotice(DiscoveryStatus status) => status switch
    {
        DiscoveryStatus.ServerMissing or DiscoveryStatus.Complete => "لم يعد الخادم المحدد متاحًا. شغّل الخادم وتحقق من الشبكة الخاصة، ثم أعد البحث واختره من جديد.",
        DiscoveryStatus.IdentityConflict => "تعارض في هوية الخادم المحدد. توقف الربط. أبلغ مسؤول الورشة، ثم أعد البحث بعد حل التعارض.",
        DiscoveryStatus.NetworkChanged => "تغيرت الشبكة أثناء التحقق. اتصل بالشبكة الخاصة للورشة، ثم أعد البحث واختر الخادم من جديد.",
        DiscoveryStatus.Cancelled => "أُلغي التحقق من الخادم. لم يُرسل رمز الربط. أعد البحث للمتابعة.",
        DiscoveryStatus.Busy => "يوجد بحث جارٍ. انتظر قليلًا، ثم أعد البحث.",
        _ => "تعذر التحقق من الخادم. تحقق من الشبكة الخاصة، ثم أعد البحث. لم يُرسل رمز الربط."
    };
    private void InvalidateSelection(DiscoveryStatus status)
    {
        ServersList.ItemsSource = null; PairingCode.Clear();
        ConnectionNotice.Text = DiscoveryNotice(status);
    }
    private void SetChecking(bool value)
    {
        CancelDiscoveryButton.Visibility = value ? Visibility.Visible : Visibility.Collapsed;
        if (value) CancelDiscoveryButton.Focus();
    }
    private void CancelDiscoveryClick(object sender, RoutedEventArgs e) => discoveryCancellation?.Cancel();
    private void ViewKeyDown(object sender, System.Windows.Input.KeyEventArgs e)
    {
        if (e.Key == Key.Escape && discoveryCancellation is not null)
        { e.Handled = true; discoveryCancellation.Cancel(); }
    }
    private async void DiscardClick(object sender, RoutedEventArgs e)
    {
        if (busy || Engine is null) return;
        SetBusy(true); PairingCode.Clear();
        ConnectionNotice.Text = "جارٍ ترك طلب الربط المحفوظ…";
        try
        {
            var state = await Engine.OnboardingAsync(new DesktopOnboardingAction { Kind = DesktopOnboardingActionKind.DiscardPendingPairing });
            if (state.Paired || state.PendingPairingServer is not null) throw new InvalidOperationException();
            ApplyState(state);
            ConnectionNotice.Text = "تم ترك الطلب المحفوظ. اطلب رمزًا جديدًا من مالك المنشأة. هوية الجهاز وبياناته محفوظة.";
        }
        catch (Exception error) when (error is EngineIpcException or NotSupportedException or System.IO.IOException or InvalidOperationException)
        { ConnectionNotice.Text = "لم يتأكد ترك الطلب. أعد المحاولة أو أعد البحث للتحقق من حالته قبل استخدام رمز جديد."; }
        finally { SetBusy(false); }
        if (stateAvailable && pendingServer is null) PairingCode.Focus();
    }
    public void Clear()
    {
        generation++; discoveryCancellation?.Cancel(); discoveryCancellation = null;
        PairingCode.Clear(); ServersList.ItemsSource = null; stateAvailable = false; pendingServer = null;
        RecoveryPanel.Visibility = Visibility.Collapsed; SetChecking(false); SetBusy(false);
    }
    private void SetBusy(bool value)
    { busy = value; DiscoverButton.IsEnabled = !value; DiscardButton.IsEnabled = !value; ServersList.IsEnabled = !value; PairingCode.IsEnabled = !value; UpdatePairButton(); }
}
