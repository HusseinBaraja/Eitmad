using System.Collections.ObjectModel;
using System.Windows;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Installation;
namespace Eitmad.WindowsSetup;

public partial class SetupWindow : Window
{
    private readonly IWindowsSetupBridge backend;
    private readonly WindowsSetupRole role;
    private readonly ObservableCollection<string> stages = [];
    private bool busy;
    private bool fresh;
    private WindowsSetupRecovery recovery = WindowsSetupRecovery.InspectManually;
    private WindowsInstallProgress? last;
    public SetupWindow(IWindowsSetupBridge backend, WindowsSetupRole role)
    {
        this.backend = backend; this.role = role;
        InitializeComponent(); StageList.ItemsSource = stages;
        AppendLog($"Eitmad setup {typeof(App).Assembly.GetName().Version}; role={role}; Windows={Environment.OSVersion.Version}");
        ServerFields.Visibility = role == WindowsSetupRole.Server ? Visibility.Visible : Visibility.Collapsed;
        RoleTitle.Text = role == WindowsSetupRole.Server ? "إعداد خادم الورشة" : "إعداد جهاز الموظف";
        RoleDescription.Text = role == WindowsSetupRole.Server
            ? "ينشئ ورشة جديدة وحساب المالك، ويثبت الخدمات وتطبيق الاعتماد لهذا المستخدم في Windows. أبقِ الخادم مشغّلًا ومتصلًا بالشبكة الخاصة."
            : "يثبت تطبيق الاعتماد. بعد فتحه، اختر خادم الورشة وأدخل رمز الربط من المالك، ثم سجل الدخول بحسابك.";
        Loaded += async (_, _) => await InspectAsync();
        Closing += (_, e) => { if (busy) { e.Cancel = true; SetupNotice.Text = "الإعداد يعمل. انتظر انتهاء المرحلة قبل إغلاق النافذة. إذا انقطع التيار، افتح الإعداد وافحص الحالة المحفوظة."; } else { OwnerPassword.Clear(); RecoveryPassword.Clear(); } };
    }
    private void UpdateActions()
    {
        InstallButton.IsEnabled = !busy && fresh;
        InstallButton.Visibility = fresh ? Visibility.Visible : Visibility.Collapsed;
        InspectButton.IsEnabled = !busy; InputPanel.IsEnabled = !busy;
        RetryButton.IsEnabled = !busy && recovery is WindowsSetupRecovery.Retry or WindowsSetupRecovery.OwnerPassword;
        RecoveryPanel.IsEnabled = !busy;
        CopyLogButton.IsEnabled = SaveLogButton.IsEnabled = !busy;
    }
    private async void InspectClick(object sender, RoutedEventArgs e) => await InspectAsync();
    public async Task InspectAsync()
    {
        if (busy) return;
        busy = true; fresh = false; recovery = WindowsSetupRecovery.InspectManually; UpdateActions();
        RecoveryPassword.Clear(); RecoveryPanel.Visibility = Visibility.Collapsed; RetryButton.Visibility = Visibility.Collapsed;
        OpenDesktopButton.Visibility = Visibility.Collapsed;
        AppendLog("Inspect started");
        try
        {
            var state = await backend.InspectAsync();
            fresh = state is null;
            if (state is null) {
                InputPanel.Visibility = Visibility.Visible; ProgressPanel.Visibility = FailurePanel.Visibility = StageHistory.Visibility = Visibility.Collapsed; OpenDesktopButton.Visibility = Visibility.Collapsed;
                AppendLog("No retained installation journal found"); last = null;
                SetupNotice.Text = role == WindowsSetupRole.Server
                    ? "اتبع خطوات الإعداد. تثبيت الخادم يطلب موافقة مسؤول Windows؛ يبقى التطبيق للمستخدم الذي بدأ الإعداد."
                    : "اضغط تثبيت، ثم اتبع إرشادات ربط الجهاز داخل التطبيق.";
            }
            else
            {
                recovery = state.Recovery;
                last = state.Progress; InputPanel.Visibility = Visibility.Collapsed; ProgressPanel.Visibility = Visibility.Visible;
                stages.Clear(); RecordProgress(last); StageHistory.Visibility = Visibility.Visible;
                AppendLog($"inspection: interrupted={state.Interrupted}; recovery={state.Recovery}");
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
                if (recovery == WindowsSetupRecovery.Wait) {
                    FailurePanel.Visibility = Visibility.Collapsed;
                    ProgressTitle.Text = "الإعداد ما زال يعمل";
                    ProgressDescription.Text = "توجد عملية إعداد أخرى. انتظر انتهاءها، ثم اضغط فحص حالة الإعداد. لا تبدأ تثبيتًا آخر.";
                } else if (recovery != WindowsSetupRecovery.Complete) ShowFailure(interrupted: state.Interrupted);
            }
        }
        catch (Exception error) when (error is System.IO.IOException or UnauthorizedAccessException or InvalidOperationException or System.ComponentModel.Win32Exception or System.Text.Json.JsonException)
        { ShowFailure(error, inspectionFailed: true); }
        finally { busy = false; UpdateActions(); }
    }
    private async void InstallClick(object sender, RoutedEventArgs e) => await InstallAsync();
    private async void RetryClick(object sender, RoutedEventArgs e) => await RetryAsync();
    public async Task RetryAsync()
    {
        if (!RetryButton.IsEnabled) return;
        busy = true; UpdateActions(); stages.Clear();
        FailurePanel.Visibility = Visibility.Collapsed; StageHistory.IsExpanded = true;
        AppendLog("Retry started");
        BusyProgress.Visibility = Visibility.Visible;
        SetupNotice.Text = "جارٍ فحص التثبيت المحفوظ قبل استكماله…";
        var password = recovery == WindowsSetupRecovery.OwnerPassword ? RecoveryPassword.Password : null;
        RecoveryPassword.Clear();
        try
        {
            await backend.RetryAsync(password, new SetupProgress(this));
            recovery = WindowsSetupRecovery.Complete; RecoveryPanel.Visibility = Visibility.Collapsed;
            RetryButton.Visibility = Visibility.Collapsed; Complete();
        }
        catch (Exception error) when (error is System.IO.IOException or UnauthorizedAccessException or InvalidOperationException or System.ComponentModel.Win32Exception or System.Text.Json.JsonException)
        {
            SetupNotice.Text = FailureText(last?.Stage ?? WindowsInstallStage.Preflight) + " تعذر تأكيد الاستكمال. افحص الحالة المحفوظة. إذا كانت ملفات التجهيز أو الخدمة غير مكتملة، احتفظ بها واتبع دليل الاسترداد.";
            if (last?.Status == WindowsInstallStatus.Blocked) {
                recovery = WindowsSetupRecovery.InspectManually; RecoveryPanel.Visibility = Visibility.Collapsed; RetryButton.Visibility = Visibility.Collapsed;
                SetupNotice.Text = "توقف الاستكمال لأن حالة التثبيت غير مؤكدة. احتفظ بقاعدة البيانات والشهادات وسجل الإعداد. اطلب فحص ملفات التجهيز والخدمات وحساب المالك وفق دليل الاسترداد قبل أي إصلاح.";
            }
            ShowFailure(error);
        }
        finally { password = null; busy = false; BusyProgress.Visibility = Visibility.Collapsed; UpdateActions(); }
    }
    public async Task InstallAsync()
    {
        if (!InstallButton.IsEnabled) return;
        busy = true; fresh = false; UpdateActions(); stages.Clear(); last = null;
        FailurePanel.Visibility = Visibility.Collapsed; StageHistory.Visibility = Visibility.Visible; StageHistory.IsExpanded = true;
        ProgressTitle.Text = "جارٍ إعداد الاعتماد"; AppendLog("Install started");
        InputPanel.Visibility = Visibility.Collapsed; ProgressPanel.Visibility = Visibility.Visible;
        BusyProgress.Visibility = Visibility.Visible; StepTitle.Text = "تثبيت الاعتماد";
        SetupNotice.Text = "اترك النافذة مفتوحة. لن تظهر نتيجة نجاح قبل اكتمال التحقق.";
        var server = role == WindowsSetupRole.Server;
        var workshop = server ? new InitializeWorkshop { Version = 1, WorkshopName = WorkshopName.Text, OwnerUsername = OwnerUsername.Text, OwnerPassword = OwnerPassword.Password } : null;
        OwnerPassword.Clear();
        try
        {
            await backend.InstallAsync(server ? GuidedSetupAction.Server : GuidedSetupAction.Client, workshop,
                new SetupProgress(this));
            Complete();
        }
        catch (Exception error) when (error is System.IO.IOException or UnauthorizedAccessException or InvalidOperationException or System.ComponentModel.Win32Exception or System.Text.Json.JsonException)
        { ShowFailure(error); }
        finally
        {
            if (workshop is not null) workshop.OwnerPassword = "";
            busy = false; BusyProgress.Visibility = Visibility.Collapsed; UpdateActions();
        }
    }
    private void Complete()
    {
        FailurePanel.Visibility = RecoveryPanel.Visibility = RetryButton.Visibility = Visibility.Collapsed;
        recovery = WindowsSetupRecovery.Complete;
        StepTitle.Text = ProgressTitle.Text = "اكتمل التثبيت";
        ProgressDescription.Text = "أكمل برنامج التثبيت التحقق من المراحل المطلوبة.";
        AppendLog("Installation confirmed complete");
        SetupNotice.Text = "اكتمل التثبيت بعد التحقق. افتح الاعتماد. على جهاز الموظف، اختر الخادم واربط الجهاز برمز المالك، ثم سجل الدخول بحسابك الشخصي. لإضافة الموظفين، افتح المستخدمون من حساب المالك.";
        OpenDesktopButton.Visibility = Visibility.Visible;
    }
    private async void OpenDesktopClick(object sender, RoutedEventArgs e) => await OpenDesktopAsync();
    public async Task OpenDesktopAsync()
    {
        if (!OpenDesktopButton.IsEnabled || OpenDesktopButton.Visibility != Visibility.Visible) return;
        OpenDesktopButton.IsEnabled = false;
        AppendLog("Desktop activation started");
        try
        {
            await backend.OpenDesktopAsync();
            AppendLog("Windows confirmed desktop activation");
            FailurePanel.Visibility = Visibility.Collapsed;
            SetupNotice.Text = "أكد Windows تشغيل التطبيق. يمكنك إغلاق نافذة الإعداد.";
        }
        catch (Exception error) when (error is System.IO.IOException or System.ComponentModel.Win32Exception)
        {
            AppendLog(error.ToString());
            FailurePanel.Visibility = Visibility.Visible;
            FailureSummary.Text = "اكتمل التثبيت، لكن تعذر فتح التطبيق.";
            NextAction.Text = "افتح الاعتماد من قائمة ابدأ تحت مستخدم Windows الذي بدأ الإعداد. إذا تعذر فتحه، احفظ السجل وأرسله لفحص السبب.";
            SetupNotice.Text = "";
        }
        finally { OpenDesktopButton.IsEnabled = true; }
    }
    public static string StageText(WindowsInstallStage stage) => stage switch {
        WindowsInstallStage.Preflight => "فحص الحزمة والتثبيت السابق", WindowsInstallStage.Elevation => "موافقة مسؤول Windows",
        WindowsInstallStage.PrepareWorkshop => "تجهيز الورشة", WindowsInstallStage.ApplyMigrations => "تجهيز قاعدة البيانات",
        WindowsInstallStage.InitializeOwner => "إنشاء حساب المالك", WindowsInstallStage.StopPreparedDatabase => "حفظ قاعدة البيانات المجهزة",
        WindowsInstallStage.InstallServices => "تثبيت خدمات الخادم", WindowsInstallStage.ConfigureFirewall => "إتاحة الشبكة الخاصة",
        WindowsInstallStage.VerifyHttps => "التحقق الآمن من جاهزية الخادم", WindowsInstallStage.EnrollDesktop => "تجهيز هوية تطبيق هذا المستخدم",
        WindowsInstallStage.RegisterDesktop => "تثبيت تطبيق الموظف",
        WindowsInstallStage.Complete => "اكتمال التثبيت", _ => "مرحلة غير معروفة" };
    private static string StatusText(WindowsInstallStatus status) => status switch { WindowsInstallStatus.Running => "بدأ التنفيذ", WindowsInstallStatus.Succeeded => "تم التحقق", WindowsInstallStatus.Blocked => "توقف لحماية البيانات", _ => "لم تكتمل" };
    private static string FailureText(WindowsInstallStage stage) => "لم يكتمل الإعداد عند: " + StageText(stage) + ". " + (stage switch {
        WindowsInstallStage.Preflight => "تحقق من الحزمة المعتمدة والتوقيع والمساحة. إذا وجدت بيانات سابقة، استخدم إصلاح التثبيت. افحص الحالة قبل المحاولة.",
        WindowsInstallStage.Elevation => "وافق على طلب Windows بحساب مسؤول. إذا أغلق الطلب أو انقطع، افحص الحالة قبل أي إجراء آخر.",
        WindowsInstallStage.VerifyHttps or WindowsInstallStage.ConfigureFirewall => "تحقق من الشبكة الخاصة واسم جهاز الخادم وحالة خدمات الاعتماد. لا تعطّل فحص الشهادة أو جدار الحماية.",
        WindowsInstallStage.RegisterDesktop => "تحقق من مستخدم Windows وتوقيع الحزمة. قد يكون الخادم جاهزًا. أصلح تطبيق هذا المستخدم دون إعادة إنشاء الورشة.",
        WindowsInstallStage.EnrollDesktop => "احتفظ بهوية الجهاز والورشة. افحص اعتماد الجهاز الأول والاتصال بالخادم وفق دليل الاسترداد؛ لا تكرر إنشاء المالك.",
        _ => "احتفظ بقاعدة البيانات والملفات. افحص المرحلة المحفوظة واتبع استرداد التثبيت قبل الإصلاح." });
}
