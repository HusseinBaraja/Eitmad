using System.IO;
using System.Windows;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Installation;

namespace Eitmad.WindowsSetup;

public partial class BackupWindow : Window
{
    private readonly IWindowsBackupBridge backend;
    private bool busy;
    private bool available;
    public BackupWindow(IWindowsBackupBridge backend)
    {
        this.backend = backend; InitializeComponent();
        Closing += (_, e) => { if (busy) { e.Cancel = true; Notice.Text = "الإجراء يعمل. انتظر النتيجة قبل إغلاق النافذة."; } };
    }
    private void Actions()
    {
        RefreshButton.IsEnabled = !busy;
        CaptureButton.IsEnabled = ScheduleButton.IsEnabled = SaveButton.IsEnabled = available && !busy;
        DestinationPanel.IsEnabled = !busy;
        BusyProgress.Visibility = busy ? Visibility.Visible : Visibility.Collapsed;
    }
    public async Task ManageAsync(ServerBackupAction action)
    {
        if (busy || (!available && action != ServerBackupAction.Inspect)) return;
        var destinations = action == ServerBackupAction.SaveDestinations ? new SetupBackupDestinations {
            Local = LocalDestination.Text, External = string.IsNullOrEmpty(ExternalDestination.Text) ? null! : ExternalDestination.Text } : null;
        busy = true; Actions(); Notice.Text = "انتظر موافقة مسؤول Windows ثم اكتمال الفحص. قد يستغرق النسخ وقتًا.";
        try {
            var state = await backend.ManageAsync(action, destinations);
            available = true; StatePanel.Visibility = Visibility.Visible;
            LastCapture.Text = Time(state.Status.LastLocalSuccessAt);
            LocalStatus.Text = Outcome(state.Status.LocalOutcome);
            ExternalStatus.Text = state.Status.ExternalConfigured ? Outcome(state.Status.ExternalOutcome) : "لم تُضبط وجهة خارجية. أضف قرصًا مستقلاً لحماية إضافية.";
            ExternalCapture.Text = state.Status.ExternalConfigured ? Time(state.Status.LastExternalSuccessAt) : "";
            ScheduleStatus.Text = Outcome(state.Status.ScheduleOutcome) + (state.Status.Overdue ? " تأخر موعد الالتقاط؛ شغّل النسخ وافحص الجدولة." : " الموعد التالي لم يتأخر.");
            PolicyText.Text = $"التقاط كل {state.IntervalHours} ساعة. الاحتفاظ بأحدث {state.RetentionCount} أجيال مكتملة لكل وجهة وفق سياسة الخادم.";
            DueText.Text = state.Status.NextDueAt == 0 ? "بانتظار أول التقاط" : Time(state.Status.NextDueAt);
            ProtectionText.Text = state.Status.DiskLossProtected ? "توجد نسخة خارجية حديثة مفحوصة على قرص مستقل. لم تُثبت الاستعادة بعد." : "لا توجد حاليًا نسخة خارجية حديثة ومفحوصة تثبت الحماية من تلف القرص.";
            LocalDestination.Text = state.Destinations.Local; ExternalDestination.Text = state.Destinations.External ?? "";
            Notice.Text = state.ActionOutcome == ServerBackupOutcome.Succeeded
                ? action switch { ServerBackupAction.Capture => "انتهى طلب النسخ. راجع نتيجة كل وجهة أدناه؛ الاستعادة تحتاج اختبارًا مستقلاً.", ServerBackupAction.SaveDestinations => "حُفظت الوجهات بعد فحص الخادم. الأجيال السابقة محفوظة.", ServerBackupAction.RepairSchedule => "انتهى طلب إصلاح الجدولة. راجع حالتها أدناه.", _ => "حالة أبلغ عنها الخادم الآن. حدّثها بعد توصيل القرص أو معالجة العطل." }
                : "لم يكتمل الإجراء. " + Outcome(state.ActionOutcome);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or InvalidOperationException or System.ComponentModel.Win32Exception or System.Text.Json.JsonException)
        {
            available = false; StatePanel.Visibility = Visibility.Collapsed;
            Notice.Text = "تعذر تأكيد الحالة أو لم تكتمل موافقة مسؤول Windows. استخدم الحزمة المعتمدة على جهاز الخادم وتحقق من التثبيت المحفوظ، ثم حدّث الحالة. حساب المالك لا يكفي. لا تحذف النسخ أو بيانات التثبيت.";
        }
        finally { busy = false; Actions(); }
    }
    private static string Time(long? seconds) => seconds is null ? "لم تُسجّل نسخة ناجحة" : DateTimeOffset.FromUnixTimeSeconds(seconds.Value).UtcDateTime.ToString("yyyy-MM-dd HH:mm 'UTC'", System.Globalization.CultureInfo.InvariantCulture);
    private static string Outcome(ServerBackupOutcome outcome) => outcome switch {
        ServerBackupOutcome.Succeeded => "نجح الفحص أو الإجراء.",
        ServerBackupOutcome.NeverRun => "لم يُنفذ بعد. أنشئ نسخة الآن.",
        ServerBackupOutcome.Running => "قيد التنفيذ. انتظر ثم حدّث الحالة.",
        ServerBackupOutcome.DestinationUnavailable => "الوجهة غير متاحة أو غير مقبولة. وصّل القرص وافتح تشفيره وافحص حماية المجلد واستقلال القرص الخارجي.",
        ServerBackupOutcome.InsufficientSpace => "المساحة غير كافية. وفّر مساحة أو غيّر الوجهة؛ لا تحذف الأجيال المحفوظة لتجاوز العطل.",
        ServerBackupOutcome.Interrupted => "انقطع التنفيذ. افحص القرص ثم أعد طلب النسخ.",
        ServerBackupOutcome.RetentionFailed => "نجح الالتقاط وتعذر تطبيق الاحتفاظ. افحص حماية المجلد والأجيال المحجوزة وفق الدليل.",
        ServerBackupOutcome.ScheduleFailed => "الجدولة غير سليمة. افحص المهمة بمسؤول Windows أو اطلب إصلاحها. لا تستبدل مهمة ذات أمر مختلف.",
        _ => "فشل الالتقاط أو الفحص. افحص خدمات الخادم وحماية المجلد والقرص وفق دليل النسخ، ثم أعد المحاولة.",
    };
    private async void RefreshClick(object sender, RoutedEventArgs e) => await ManageAsync(ServerBackupAction.Inspect);
    private async void CaptureClick(object sender, RoutedEventArgs e) => await ManageAsync(ServerBackupAction.Capture);
    private async void SaveClick(object sender, RoutedEventArgs e) => await ManageAsync(ServerBackupAction.SaveDestinations);
    private async void ScheduleClick(object sender, RoutedEventArgs e) => await ManageAsync(ServerBackupAction.RepairSchedule);
}
