using System.Text;
using System.Windows;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Installation;
using Microsoft.Win32;

namespace Eitmad.WindowsSetup;

public partial class SetupWindow
{
    private readonly StringBuilder log = new();
    public string DiagnosticLog => log.ToString();

    private void AppendLog(string text)
    {
        if (log.Length > 120_000) log.Remove(0, log.Length - 60_000);
        log.AppendLine($"{DateTimeOffset.UtcNow:O} {text}");
        LogDetails.Text = DiagnosticLog;
    }

    private void RecordProgress(WindowsInstallProgress progress)
    {
        last = progress;
        var prefix = StageText(progress.Stage) + " · ";
        var text = prefix + StatusText(progress.Status);
        if (stages.Count > 0 && stages[^1].StartsWith(prefix, StringComparison.Ordinal)) stages[^1] = text;
        else stages.Add(text);
        AppendLog($"stage={progress.Stage}; status={progress.Status}; sequence={progress.Sequence}; failure={progress.Failure?.ToString() ?? "unspecified"}");
        if (!string.IsNullOrEmpty(progress.Details)) AppendLog(progress.Details);
        ProgressDescription.Text = StageText(progress.Stage);
    }

    // Apply progress before the terminal exception so the failure screen uses the actual last step.
    private sealed class SetupProgress(SetupWindow view) : IProgress<WindowsInstallProgress>
    {
        public void Report(WindowsInstallProgress value) => view.Dispatcher.Invoke(() => view.RecordProgress(value));
    }

    private void ShowFailure(Exception? error = null, bool interrupted = false, bool inspectionFailed = false)
    {
        if (error is not null) AppendLog(error.ToString());
        InputPanel.Visibility = Visibility.Collapsed;
        ProgressPanel.Visibility = FailurePanel.Visibility = StageHistory.Visibility = Visibility.Visible;
        BusyProgress.Visibility = Visibility.Collapsed; StageHistory.IsExpanded = false;
        ProgressTitle.Text = inspectionFailed ? "تعذر فحص الإعداد" : interrupted ? "انقطع الإعداد" : "لم يكتمل الإعداد";
        StepTitle.Text = "يحتاج الإعداد إلى معالجة";
        FailureSummary.Text = inspectionFailed ? "تعذر قراءة حالة التثبيت. لم يتأكد الإعداد من إمكانية المتابعة بأمان."
            : "توقف الإعداد عند: " + StageText(last?.Stage ?? WindowsInstallStage.Preflight) + ". لم يُؤكد اكتمال التثبيت.";
        NextAction.Text = FailureGuidance(last?.Failure) ?? FailureText(last?.Stage ?? WindowsInstallStage.Preflight);
        NextAction.Text += recovery is WindowsSetupRecovery.Retry or WindowsSetupRecovery.OwnerPassword
            ? " بعد معالجة السبب، استخدم فحص واستكمال الإعداد بالحزمة نفسها ومن مستخدم Windows الذي بدأ التثبيت."
            : " بعد معالجة السبب، اضغط فحص حالة الإعداد قبل المتابعة.";
        SetupNotice.Text = inspectionFailed ? "تعذر تأكيد حالة الإعداد. راجع السبب والخطوة التالية أعلاه." : "التثبيت غير مكتمل. راجع السبب والخطوة التالية أعلاه.";
        SetupScroll.ScrollToTop();
    }

    private static string? FailureGuidance(WindowsSetupFailure? failure) => failure switch {
        WindowsSetupFailure.DesktopUser => "افتح ملف إعداد الاعتماد بحساب Windows المعتاد، دون تشغيل كمسؤول. سيطلب الإعداد موافقة المسؤول عند الحاجة.",
        WindowsSetupFailure.DesktopPackage or WindowsSetupFailure.ServerPackage => "تعذر التحقق من حزمة التثبيت. استخدم الحزمة المعتمدة كاملة وتحقق من توقيعها والمساحة المتاحة. لا تغيّر ملفاتها.",
        WindowsSetupFailure.ProtectedPaths => "تعذر تأكيد حماية مجلدات الخادم. اطلب من مسؤول Windows فحص صلاحيات المجلدات. لا تنقل البيانات ولا تعطّل فحص الحماية.",
        WindowsSetupFailure.Prerequisites => "لم تكتمل متطلبات Windows. راجع الخطأ في السجل؛ إذا طلب إعادة تشغيل الجهاز، أعد تشغيله ثم افتح الإعداد وافحص الحالة.",
        WindowsSetupFailure.RetainedState => "التثبيت الموجود لا يطابق الحزمة أو الحالة المتوقعة. استخدم الحزمة نفسها واحتفظ بالبيانات وهوية الورشة للفحص قبل الإصلاح.",
        _ => null
    };

    private void CopyLogClick(object sender, RoutedEventArgs e)
    {
        try { Clipboard.SetText(DiagnosticLog); LogNotice.Text = "نُسخ السجل. يمكنك لصقه وإرساله لفحص المشكلة."; }
        catch (System.Runtime.InteropServices.ExternalException) { LogNotice.Text = "تعذر نسخ السجل لأن الحافظة غير متاحة. أعد المحاولة أو احفظ ملف السجل."; }
    }

    private async void SaveLogClick(object sender, RoutedEventArgs e)
    {
        var picker = new SaveFileDialog { Title = "حفظ سجل الإعداد", FileName = $"Eitmad-Setup-{DateTime.Now:yyyyMMdd-HHmmss}.log", Filter = "ملف سجل (*.log)|*.log|ملف نصي (*.txt)|*.txt", DefaultExt = ".log", AddExtension = true };
        if (picker.ShowDialog(this) == true) await SaveLogAsync(picker.FileName);
    }

    public async Task SaveLogAsync(string path)
    {
        SaveLogButton.IsEnabled = false;
        try { await WindowsSetupLog.SaveAsync(path, DiagnosticLog); LogNotice.Text = "حُفظ ملف السجل. أرسله لفحص المشكلة."; }
        catch (Exception error) when (error is System.IO.IOException or UnauthorizedAccessException) { LogNotice.Text = "تعذر حفظ السجل. اختر مجلدًا يمكنك الكتابة فيه أو استخدم نسخ السجل."; }
        finally { SaveLogButton.IsEnabled = true; }
    }
}
