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
            var result = await Engine.DiscoverServersAsync();
            ServersList.ItemsSource = result.Servers.Select(candidate => new Choice(candidate)).ToArray();
            ConnectionNotice.Text = result.Status is DiscoveryStatus.Complete or DiscoveryStatus.LimitReached
                ? result.Servers.Length == 0 ? "لم يظهر خادم. شغّل جهاز الخادم، وتحقق من اتصال الجهازين بالشبكة الخاصة نفسها، ثم أعد البحث."
                    : "اختر خادم الورشة، ثم أدخل الرمز الظاهر لدى المالك."
                : "لم يكتمل البحث. تحقق من الشبكة الخاصة ثم أعد البحث.";
        }
        catch (Exception error) when (error is EngineIpcException or NotSupportedException or System.IO.IOException)
        { ConnectionNotice.Text = "تعذر البحث. تحقق من تشغيل الاعتماد واتصال الشبكة الخاصة، ثم أعد المحاولة."; }
        finally { SetBusy(false); }
    }
    private void SelectionChanged(object sender, SelectionChangedEventArgs e) => UpdatePairButton();
    private void UpdatePairButton() => PairButton.IsEnabled = !busy && ServersList.SelectedItem is Choice { Candidate.IdentityConflict: false };
    private async void PairClick(object sender, RoutedEventArgs e) => await PairAsync();
    private async void CodeKeyDown(object sender, System.Windows.Input.KeyEventArgs e)
    { if (e.Key == Key.Enter && PairButton.IsEnabled) { e.Handled = true; await PairAsync(); } }
    private async Task PairAsync()
    {
        if (busy || Engine is null || ServersList.SelectedItem is not Choice choice) return;
        var code = PairingCode.Password; PairingCode.Clear();
        SetBusy(true); ConnectionNotice.Text = "جارٍ التحقق من الرمز وهوية الخادم…";
        var action = new DesktopOnboardingAction { Kind = DesktopOnboardingActionKind.Pair,
            Payload = new DesktopOnboardingActionPayload { Candidate = choice.Candidate, Code = code } };
        try
        {
            var result = await Engine.OnboardingAsync(action);
            if (!result.Paired) throw new InvalidOperationException();
            ConnectionNotice.Text = "تم ربط الجهاز. سجل الدخول بحسابك الشخصي.";
            Paired?.Invoke(this, EventArgs.Empty);
        }
        catch (Exception error) when (error is EngineIpcException or NotSupportedException or System.IO.IOException or InvalidOperationException)
        { ConnectionNotice.Text = "لم يتأكد الربط. تحقق من اختيار الخادم وصلاحية الرمز. إذا انقطع الاتصال، أعد المحاولة بالخادم نفسه لاستكمال الطلب المحفوظ. لا تحذف إعدادات الجهاز."; }
        finally { action.Payload.Code = string.Empty; code = string.Empty; SetBusy(false); }
    }
    public void Clear() { PairingCode.Clear(); ServersList.ItemsSource = null; }
    private void SetBusy(bool value)
    { busy = value; DiscoverButton.IsEnabled = !value; ServersList.IsEnabled = !value; PairingCode.IsEnabled = !value; UpdatePairButton(); }
}
