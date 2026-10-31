using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Windows;
using Eitmad.Platform.Windows.Installation;

namespace Eitmad.WindowsSetup;

/// <summary>Present Windows payload preparation before the Rust-owned guided workflow starts.</summary>
public partial class InstallerWindow : Window
{
    private bool busy;
    public InstallerWindow()
    {
        InitializeComponent();
        Closing += (_, e) => { if (busy) e.Cancel = true; };
    }

    public async Task StartAsync(bool trustPublisher = false)
    {
        if (busy) return;
        busy = true; ClosePreparation.IsEnabled = false; RetryPreparation.Visibility = Visibility.Collapsed;
        PublisherTrustPanel.IsEnabled = false; TrustPublisher.IsEnabled = false;
        PreparationProgress.Visibility = Visibility.Visible;
        InstallerNotice.Text = "جارٍ التحقق من ملف الإعداد وتجهيز المتطلبات. انتظر قليلًا…";
        try
        {
            var assembly = typeof(App).Assembly;
            string Metadata(string name) => assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
                .SingleOrDefault(item => item.Key == name)?.Value ?? throw new IOException("Installer metadata missing.");
            var role = Metadata("InstallerRole");
            if (role is not ("server" or "client")) throw new IOException("Installer role missing.");
            var executable = Environment.ProcessPath ?? throw new IOException("Installer path missing.");
            using var retainedExecutable = await Task.Run(() => AuthenticodeTrust.RetainInstaller(executable));
            using var certificateSource = assembly.GetManifestResourceStream("Eitmad.PublisherCertificate.cer")
                ?? throw new IOException("Publisher certificate missing.");
            if (certificateSource.Length is 0 or > 16384) throw new IOException("Invalid publisher certificate size.");
            var certificateBytes = new byte[certificateSource.Length];
            await certificateSource.ReadExactlyAsync(certificateBytes);
            using var publisher = new PublisherCertificateTrust(certificateBytes, Metadata("PublisherCertificateSha256"));
            await Task.Run(() => publisher.VerifyInstallerSigner(executable));
            if (!await Task.Run(publisher.IsTrustedForMachine))
            {
                ShowPublisherTrust(publisher.Publisher, publisher.Fingerprint);
                if (!trustPublisher || ConfirmedPublisher.IsChecked != true) return;
                InstallerNotice.Text = "وافق على طلبات مسؤول Windows لاعتماد شهادة الناشر. قد يظهر طلبان. تبقى عملية تثبيت التطبيق للمستخدم الذي بدأ الإعداد.";
                await Task.Run(publisher.TrustForMachineAsync);
            }
            PublisherTrustPanel.Visibility = Visibility.Collapsed; TrustPublisher.Visibility = Visibility.Collapsed;
            PreparationProgress.Visibility = Visibility.Visible;
            InstallerNotice.Text = "جارٍ التحقق من ملف الإعداد وتجهيز المتطلبات. انتظر قليلًا…";
            await Task.Run(() => AuthenticodeTrust.Verify(retainedExecutable));
            using var source = assembly.GetManifestResourceStream("Eitmad.InstallerPayload.zip")
                ?? throw new IOException("Installer payload missing.");
            using var payload = await Task.Run(() => EmbeddedInstallerPayload.PrepareAsync(source,
                Metadata("InstallerPayloadSha256")));
            var start = new ProcessStartInfo(Path.Combine(payload.RootDirectory, "setup", "EitmadSetup.exe")) {
                UseShellExecute = false, WorkingDirectory = payload.RootDirectory,
            };
            start.ArgumentList.Add("--role"); start.ArgumentList.Add(role);
            using var child = Process.Start(start) ?? throw new IOException("Guided setup unavailable.");
            Hide(); await child.WaitForExitAsync();
            if (child.ExitCode != 0) throw new IOException("Guided setup stopped.");
            busy = false; Close();
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or InvalidOperationException
            or Win32Exception or System.Security.Cryptography.CryptographicException or System.Security.SecurityException)
        {
            Show(); PreparationProgress.Visibility = Visibility.Collapsed;
            InstallerNotice.Text = "تعذر التحقق من ملف الإعداد أو تجهيز متطلباته. استخدم ملف إعداد الاعتماد المعتمد، وتأكد من وجود مساحة كافية ومن سماح Windows بتشغيله، ثم أعد المحاولة. حُفظت أي ملفات تجهيز سابقة؛ لا تحذف بيانات الورشة.";
            RetryPreparation.Visibility = Visibility.Visible;
        }
        finally
        {
            busy = false; ClosePreparation.IsEnabled = true; PublisherTrustPanel.IsEnabled = true;
            UpdateTrustAction();
        }
    }

    public void ShowPublisherTrust(string publisher, string fingerprint)
    {
        PublisherName.Text = publisher; PublisherFingerprint.Text = fingerprint;
        PublisherTrustPanel.Visibility = Visibility.Visible; TrustPublisher.Visibility = Visibility.Visible;
        PreparationProgress.Visibility = Visibility.Collapsed;
        InstallerNotice.Text = "يستخدم الاعتماد شهادة ناشر ذاتية التوقيع. قارن البصمة مع المصدر الذي سلّمك الملف قبل المتابعة. بموافقة مسؤول Windows، تُضاف الشهادة العامة إلى مخزني الشهادات الجذرية والناشرين الموثوقين لهذا الجهاز، حتى على جهاز الموظف. قد يظهر طلبان. يسمح ذلك بتشغيل البرامج ونصوص الإعداد الموقّعة بهذا المفتاح. الشهادة لا تحتوي على مفتاح خاص.";
        UpdateTrustAction();
    }

    private void UpdateTrustAction() => TrustPublisher.IsEnabled = !busy && ConfirmedPublisher.IsChecked == true;
    private void PublisherConfirmationChanged(object sender, RoutedEventArgs e) { if (TrustPublisher is not null) UpdateTrustAction(); }
    private async void TrustPublisherClick(object sender, RoutedEventArgs e) => await StartAsync(true);
    private async void RetryClick(object sender, RoutedEventArgs e) => await StartAsync();
    private void CloseClick(object sender, RoutedEventArgs e) => Close();
}
