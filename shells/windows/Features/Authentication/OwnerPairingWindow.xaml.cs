using System.Windows;
using System.Windows.Threading;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;
namespace Eitmad.WindowsShell.Features.Authentication;
public partial class OwnerPairingWindow : Window
{
    private readonly IEngineShellBridge engine;
    private readonly DispatcherTimer expiry = new() { Interval = TimeSpan.FromSeconds(1) };
    private long expiresAt;
    private bool closed;
    public event EventHandler? StaffRequested;
    public OwnerPairingWindow(IEngineShellBridge engine)
    {
        this.engine = engine; InitializeComponent();
        expiry.Tick += (_, _) => UpdateExpiry();
        engine.StateChanged += EngineStateChanged;
        Closed += (_, _) => { closed = true; engine.StateChanged -= EngineStateChanged; ClearCode(); };
        Loaded += (_, _) => OpenCodeButton.Focus();
    }
    private void ClearCode() { expiry.Stop(); CodeText.Text = ""; ExpiryText.Text = ""; expiresAt = 0; }
    private void EngineStateChanged(EngineSupervisionSnapshot snapshot)
    {
        if (snapshot.IpcHealth == EngineIpcHealthState.Connected) return;
        Dispatcher.BeginInvoke(() => { if (!closed) { ClearCode(); PairingNotice.Text = "انقطع الاتصال. تحقق من اتصال الخادم ثم اطلب رمزًا جديدًا."; } });
    }
    private async void OpenClick(object sender, RoutedEventArgs e)
    {
        OpenCodeButton.IsEnabled = false;
        PairingNotice.Text = "جارٍ التحقق من صلاحية المالك…";
        try
        {
            var result = await engine.OnboardingAsync(new DesktopOnboardingAction { Kind = DesktopOnboardingActionKind.OpenPairing });
            if (closed) { result.Code = ""; return; }
            expiresAt = result.ExpiresAt ?? 0; CodeText.Text = result.Code; result.Code = "";
            PairingNotice.Text = "استخدم الرمز مرة واحدة. يرفض الخادم الرمز المنتهي أو المستخدم.";
            expiry.Start(); UpdateExpiry();
        }
        catch (Exception error) when (error is EngineIpcException or NotSupportedException or System.IO.IOException)
        {
            if (!closed) PairingNotice.Text = expiresAt > DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()
                ? "تعذر إنشاء رمز جديد. الرمز الحالي ما زال صالحًا؛ استخدمه أو انتظر انتهاء صلاحيته."
                : "تعذر إنشاء الرمز. يلزم حساب مالك المنشأة واتصال بالخادم. تحقق من الاتصال ثم أعد المحاولة.";
        }
        finally { if (!closed) OpenCodeButton.IsEnabled = true; }
    }
    private void UpdateExpiry()
    {
        var seconds = Math.Max(0, (expiresAt - DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()) / 1000);
        ExpiryText.Text = $"ينتهي الرمز خلال {seconds / 60:0}:{seconds % 60:00}";
        if (seconds == 0) { ClearCode(); CodeText.Text = "انتهت صلاحية الرمز"; ExpiryText.Text = "اطلب رمزًا جديدًا عند جاهزية الجهاز."; PairingNotice.Text = ""; }
    }
    private void StaffClick(object sender, RoutedEventArgs e) { StaffRequested?.Invoke(this, EventArgs.Empty); Close(); }
}
