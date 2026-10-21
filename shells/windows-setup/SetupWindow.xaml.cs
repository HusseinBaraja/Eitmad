using System.Collections.ObjectModel;
using System.Windows;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Installation;
using Microsoft.Win32;
namespace Eitmad.WindowsSetup;

public partial class SetupWindow : Window
{
    private readonly IWindowsSetupBridge backend;
    private readonly ObservableCollection<string> stages = [];
    private bool busy;
    private bool fresh;
    private WindowsSetupRecovery recovery = WindowsSetupRecovery.InspectManually;
    private WindowsInstallProgress? last;
    public SetupWindow(IWindowsSetupBridge backend)
    {
        this.backend = backend; InitializeComponent(); StageList.ItemsSource = stages;
        Loaded += async (_, _) => await InspectAsync();
        Closing += (_, e) => { if (busy) { e.Cancel = true; SetupNotice.Text = "الإعداد يعمل. انتظر انتهاء المرحلة قبل إغلاق النافذة. إذا انقطع التيار، افتح الإعداد وافحص الحالة المحفوظة."; } else { OwnerPassword.Clear(); RecoveryPassword.Clear(); } };
    }
    private void RoleChanged(object sender, RoutedEventArgs e)
    {
        if (ServerFields is null) return;
        ServerFields.Visibility = ServerChoice.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        if (ClientChoice.IsChecked == true) OwnerPassword.Clear();
        UpdateActions();
    }
    private void UpdateActions()
    {
        InstallButton.IsEnabled = !busy && fresh && (ServerChoice.IsChecked == true || ClientChoice.IsChecked == true);
        InstallButton.Visibility = fresh ? Visibility.Visible : Visibility.Collapsed;
        InspectButton.IsEnabled = !busy; InputPanel.IsEnabled = !busy;
        RetryButton.IsEnabled = !busy && recovery is WindowsSetupRecovery.Retry or WindowsSetupRecovery.OwnerPassword;
        RecoveryPanel.IsEnabled = !busy;
    }
    private async void InspectClick(object sender, RoutedEventArgs e) => await InspectAsync();
    public async Task InspectAsync()
    {
        if (busy) return;
        busy = true; fresh = false; recovery = WindowsSetupRecovery.InspectManually; UpdateActions();
        RecoveryPassword.Clear(); RecoveryPanel.Visibility = Visibility.Collapsed; RetryButton.Visibility = Visibility.Collapsed;
        OpenDesktopButton.Visibility = Visibility.Collapsed;
        try
        {
            var state = await backend.InspectAsync();
            fresh = state is null;
            if (state is null) {
                InputPanel.Visibility = Visibility.Visible; ProgressPanel.Visibility = Visibility.Collapsed; OpenDesktopButton.Visibility = Visibility.Collapsed;
                SetupNotice.Text = "اختر دور هذا الجهاز. تثبيت الخادم يطلب موافقة مسؤول Windows؛ يبقى التطبيق للمستخدم الذي بدأ الإعداد.";
            }
            else
            {
                recovery = state.Recovery;
                last = state.Progress; InputPanel.Visibility = Visibility.Collapsed; ProgressPanel.Visibility = Visibility.Visible;
                stages.Clear(); stages.Add(StageText(last.Stage) + " · " + StatusText(last.Status));
                BusyProgress.Visibility = Visibility.Collapsed;
                if (state.Recovery == WindowsSetupRecovery.Complete) Complete();
                else SetupNotice.Text = state.Interrupted
                    ? "انقطع الإعداد عند: " + StageText(last.Stage) + ". حُفظت الملفات والبيانات. لا تبدأ تثبيتًا جديدًا ولا تحذف المجلدات. افحص حالة خدمات الاعتماد واستكمل الإصلاح وفق دليل استرداد التثبيت."
                    : last.Status == WindowsInstallStatus.Failed ? FailureText(last.Stage)
                    : "الإعداد ما زال يعمل في عملية أخرى. انتظر ثم افحص الحالة. لا تبدأ تثبيتًا آخر.";
                if (recovery is WindowsSetupRecovery.Retry or WindowsSetupRecovery.OwnerPassword)
                {
                    RecoveryPanel.Visibility = Visibility.Visible; RetryButton.Visibility = Visibility.Visible;
                    RecoveryPasswordField.Visibility = recovery == WindowsSetupRecovery.OwnerPassword ? Visibility.Visible : Visibility.Collapsed;
                    StepTitle.Text = "استكمال الإعداد";
                    SetupNotice.Text = FailureText(last.Stage) + " استخدم فحص واستكمال الإعداد من مستخدم Windows الذي بدأ التثبيت وبالحزمة نفسها. إذا تعذر إثبات الحالة، سيتوقف الإعداد دون إعادة إنشاء الورشة.";
                }
                else if (recovery == WindowsSetupRecovery.InspectManually)
                    SetupNotice.Text = "تعذر إثبات حالة التثبيت المحفوظة. احتفظ بكل المجلدات. افحص سجل الإعداد والخدمات وفق دليل استرداد التثبيت؛ لا تغيّر المسارات ولا تنشئ ورشة أخرى.";
            }
        }
        catch (Exception error) when (error is System.IO.IOException or InvalidOperationException or System.ComponentModel.Win32Exception or System.Text.Json.JsonException)
        { SetupNotice.Text = "تعذر قراءة حالة الإعداد. شغّل EitmadSetup.exe من الحزمة الكاملة المعتمدة، ثم أعد فحص الحالة. لا تغيّر أي تثبيت قائم."; }
        finally { busy = false; UpdateActions(); }
    }
    private async void InstallClick(object sender, RoutedEventArgs e) => await InstallAsync();
    private async void RetryClick(object sender, RoutedEventArgs e) => await RetryAsync();
    public async Task RetryAsync()
    {
        if (!RetryButton.IsEnabled) return;
        busy = true; UpdateActions(); stages.Clear();
        BusyProgress.Visibility = Visibility.Visible;
        SetupNotice.Text = "جارٍ فحص التثبيت المحفوظ قبل استكماله…";
        var password = recovery == WindowsSetupRecovery.OwnerPassword ? RecoveryPassword.Password : null;
        RecoveryPassword.Clear();
        try
        {
            await backend.RetryAsync(password, new Progress<WindowsInstallProgress>(p => {
                last = p; stages.Add(StageText(p.Stage) + " · " + StatusText(p.Status));
            }));
            recovery = WindowsSetupRecovery.Complete; RecoveryPanel.Visibility = Visibility.Collapsed;
            RetryButton.Visibility = Visibility.Collapsed; Complete();
        }
        catch (Exception error) when (error is System.IO.IOException or InvalidOperationException or System.ComponentModel.Win32Exception or System.Text.Json.JsonException)
        {
            SetupNotice.Text = FailureText(last?.Stage ?? WindowsInstallStage.Preflight) + " تعذر تأكيد الاستكمال. افحص الحالة المحفوظة. إذا كانت ملفات التجهيز أو الخدمة غير مكتملة، احتفظ بها واتبع دليل الاسترداد.";
            if (last?.Status == WindowsInstallStatus.Blocked) {
                recovery = WindowsSetupRecovery.InspectManually; RecoveryPanel.Visibility = Visibility.Collapsed; RetryButton.Visibility = Visibility.Collapsed;
                SetupNotice.Text = "توقف الاستكمال لأن حالة التثبيت غير مؤكدة. احتفظ بقاعدة البيانات والشهادات وسجل الإعداد. اطلب فحص ملفات التجهيز والخدمات وحساب المالك وفق دليل الاسترداد قبل أي إصلاح.";
            }
        }
        finally { password = null; busy = false; BusyProgress.Visibility = Visibility.Collapsed; UpdateActions(); }
    }
    public async Task InstallAsync()
    {
        if (!InstallButton.IsEnabled) return;
        busy = true; fresh = false; UpdateActions(); stages.Clear();
        InputPanel.Visibility = Visibility.Collapsed; ProgressPanel.Visibility = Visibility.Visible;
        BusyProgress.Visibility = Visibility.Visible; StepTitle.Text = "تثبيت الاعتماد";
        SetupNotice.Text = "اترك النافذة مفتوحة. لن تظهر نتيجة نجاح قبل اكتمال التحقق.";
        var server = ServerChoice.IsChecked == true;
        var workshop = server ? new InitializeWorkshop { Version = 1, WorkshopName = WorkshopName.Text, OwnerUsername = OwnerUsername.Text, OwnerPassword = OwnerPassword.Password } : null;
        OwnerPassword.Clear();
        try
        {
            await backend.InstallAsync(server ? GuidedSetupAction.Server : GuidedSetupAction.Client, workshop,
                server ? new SetupBackupDestinations { Local = LocalDestination.Text, External = string.IsNullOrEmpty(ExternalDestination.Text) ? null! : ExternalDestination.Text } : null,
                new Progress<WindowsInstallProgress>(p => { last = p; stages.Add(StageText(p.Stage) + " · " + StatusText(p.Status)); }));
            Complete();
        }
        catch (Exception error) when (error is System.IO.IOException or InvalidOperationException or System.ComponentModel.Win32Exception or System.Text.Json.JsonException)
        { SetupNotice.Text = FailureText(last?.Stage ?? WindowsInstallStage.Preflight); }
        finally
        {
            if (workshop is not null) workshop.OwnerPassword = "";
            busy = false; BusyProgress.Visibility = Visibility.Collapsed; UpdateActions();
        }
    }
    private void Complete()
    {
        StepTitle.Text = "اكتمل التثبيت";
        SetupNotice.Text = "اكتمل التثبيت بعد التحقق. افتح الاعتماد. على جهاز الموظف، اختر الخادم واربط الجهاز برمز المالك، ثم سجل الدخول بحسابك الشخصي. لإضافة الموظفين، افتح المستخدمون من حساب المالك.";
        OpenDesktopButton.Visibility = Visibility.Visible;
    }
    private async void OpenDesktopClick(object sender, RoutedEventArgs e)
    {
        OpenDesktopButton.IsEnabled = false;
        try { await backend.OpenDesktopAsync(); }
        catch (Exception error) when (error is System.IO.IOException or System.ComponentModel.Win32Exception)
        { SetupNotice.Text = "تعذر فتح التطبيق. افتح الاعتماد من قائمة ابدأ تحت مستخدم Windows الذي بدأ الإعداد."; }
        finally { OpenDesktopButton.IsEnabled = true; }
    }
    private void LocalFolderClick(object sender, RoutedEventArgs e) => PickFolder(LocalDestination);
    private void ExternalFolderClick(object sender, RoutedEventArgs e) => PickFolder(ExternalDestination);
    private void PickFolder(System.Windows.Controls.TextBox target)
    { var picker = new OpenFolderDialog { Title = "اختيار وجهة النسخ الاحتياطي" }; if (picker.ShowDialog(this) == true) target.Text = picker.FolderName; }
    private void ClearExternalClick(object sender, RoutedEventArgs e) => ExternalDestination.Clear();
    public static string StageText(WindowsInstallStage stage) => stage switch {
        WindowsInstallStage.Preflight => "فحص الحزمة والتثبيت السابق", WindowsInstallStage.Elevation => "موافقة مسؤول Windows",
        WindowsInstallStage.PrepareWorkshop => "تجهيز الورشة", WindowsInstallStage.ApplyMigrations => "تجهيز قاعدة البيانات",
        WindowsInstallStage.InitializeOwner => "إنشاء حساب المالك", WindowsInstallStage.StopPreparedDatabase => "حفظ قاعدة البيانات المجهزة",
        WindowsInstallStage.InstallServices => "تثبيت خدمات الخادم", WindowsInstallStage.ConfigureFirewall => "إتاحة الشبكة الخاصة",
        WindowsInstallStage.VerifyHttps => "التحقق الآمن من جاهزية الخادم", WindowsInstallStage.EnrollDesktop => "تجهيز هوية تطبيق هذا المستخدم",
        WindowsInstallStage.ConfigureBackups => "تجهيز النسخ الاحتياطي والتحقق من أول نسخة", WindowsInstallStage.RegisterDesktop => "تثبيت تطبيق الموظف",
        WindowsInstallStage.Complete => "اكتمال التثبيت", _ => "مرحلة غير معروفة" };
    private static string StatusText(WindowsInstallStatus status) => status switch { WindowsInstallStatus.Running => "بدأ التنفيذ", WindowsInstallStatus.Succeeded => "تم التحقق", _ => "لم تكتمل" };
    private static string FailureText(WindowsInstallStage stage) => "لم يكتمل الإعداد عند: " + StageText(stage) + ". " + (stage switch {
        WindowsInstallStage.Preflight => "تحقق من الحزمة المعتمدة والتوقيع والمساحة. إذا وجدت بيانات سابقة، استخدم إصلاح التثبيت. افحص الحالة قبل المحاولة.",
        WindowsInstallStage.Elevation => "وافق على طلب Windows بحساب مسؤول. إذا أغلق الطلب أو انقطع، افحص الحالة قبل أي إجراء آخر.",
        WindowsInstallStage.ConfigureBackups => "تحقق من اتصال قرص النسخ وتشفيره وصلاحيات المجلد والمساحة. قد يكون الخادم مثبتًا بالفعل. استخدم إجراء النسخ الاحتياطي والإصلاح؛ لا تنشئ الورشة مرة ثانية.",
        WindowsInstallStage.VerifyHttps or WindowsInstallStage.ConfigureFirewall => "تحقق من الشبكة الخاصة واسم جهاز الخادم وحالة خدمات الاعتماد. لا تعطّل فحص الشهادة أو جدار الحماية.",
        WindowsInstallStage.RegisterDesktop => "تحقق من مستخدم Windows وتوقيع الحزمة. قد يكون الخادم جاهزًا. أصلح تطبيق هذا المستخدم دون إعادة إنشاء الورشة.",
        WindowsInstallStage.EnrollDesktop => "احتفظ بهوية الجهاز والورشة. افحص اعتماد الجهاز الأول والاتصال بالخادم وفق دليل الاسترداد؛ لا تكرر إنشاء المالك.",
        _ => "احتفظ بقاعدة البيانات والملفات. افحص المرحلة المحفوظة واتبع استرداد التثبيت قبل الإصلاح." });
}
