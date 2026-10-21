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
    public ServerSelectionView() => InitializeComponent();
    public IEngineShellBridge? Engine { get; set; }
    public event EventHandler? Paired;
    private bool busy;
    private bool stateAvailable;
    private Guid? pendingServer;
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
        SetBusy(true); PairingCode.Clear(); ServersList.ItemsSource = null;
        ConnectionNotice.Text = "جارٍ البحث على الشبكات الخاصة…";
        try
        {
            var state = await Engine.OnboardingAsync(new DesktopOnboardingAction { Kind = DesktopOnboardingActionKind.State });
            ApplyState(state);
            if (state.Paired) return;
            var result = await Engine.DiscoverServersAsync();
            ServersList.ItemsSource = result.Servers.Select(candidate => new Choice(candidate)).ToArray();
            if (pendingServer is not null)
                ServersList.SelectedItem = ServersList.Items.Cast<Choice>().FirstOrDefault(choice => choice.Candidate.ServerId == pendingServer && !choice.Candidate.IdentityConflict);
            ConnectionNotice.Text = result.Status is DiscoveryStatus.Complete or DiscoveryStatus.LimitReached
                ? result.Servers.Length == 0 ? "لم يظهر خادم. شغّل جهاز الخادم، وتحقق من اتصال الجهازين بالشبكة الخاصة نفسها، ثم أعد البحث."
                    : pendingServer is not null ? "يوجد طلب ربط محفوظ. أعد المحاولة مع الخادم نفسه دون إدخال الرمز، أو اترك الطلب لاستخدام رمز جديد." : "اختر خادم الورشة، ثم أدخل الرمز الظاهر لدى المالك."
                : "لم يكتمل البحث. تحقق من الشبكة الخاصة ثم أعد البحث.";
        }
        catch (Exception error) when (error is EngineIpcException or NotSupportedException or System.IO.IOException)
        { stateAvailable = false; RecoveryPanel.Visibility = Visibility.Visible; ConnectionNotice.Text = "تعذر التحقق من طلب الربط أو البحث. أعد البحث، أو اترك الطلب المحفوظ لاستخدام رمز جديد."; }
        finally { SetBusy(false); }
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
        var code = pendingServer is null ? PairingCode.Password : string.Empty; PairingCode.Clear();
        SetBusy(true); ConnectionNotice.Text = "جارٍ التحقق من الرمز وهوية الخادم…";
        var action = new DesktopOnboardingAction { Kind = DesktopOnboardingActionKind.Pair,
            Payload = new DesktopOnboardingActionPayload { Candidate = choice.Candidate, Code = code } };
        try
        {
            var result = await Engine.OnboardingAsync(action);
            if (!result.Paired) throw new InvalidOperationException();
            ApplyState(result);
        }
        catch (Exception error) when (error is EngineIpcException or NotSupportedException or System.IO.IOException or InvalidOperationException)
        {
            ConnectionNotice.Text = "لم يتأكد الربط. أعد المحاولة مع الخادم نفسه. إذا انتهت صلاحية الرمز، اترك الطلب واستخدم رمزًا جديدًا من المالك.";
            try { ApplyState(await Engine.OnboardingAsync(new DesktopOnboardingAction { Kind = DesktopOnboardingActionKind.State })); }
            catch (Exception stateError) when (stateError is EngineIpcException or NotSupportedException or System.IO.IOException)
            { stateAvailable = false; RecoveryPanel.Visibility = Visibility.Visible; }
        }
        finally { action.Payload.Code = string.Empty; code = string.Empty; SetBusy(false); }
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
    public void Clear() { PairingCode.Clear(); ServersList.ItemsSource = null; stateAvailable = false; pendingServer = null; RecoveryPanel.Visibility = Visibility.Collapsed; }
    private void SetBusy(bool value)
    { busy = value; DiscoverButton.IsEnabled = !value; DiscardButton.IsEnabled = !value; ServersList.IsEnabled = !value; PairingCode.IsEnabled = !value; UpdatePairButton(); }
}
