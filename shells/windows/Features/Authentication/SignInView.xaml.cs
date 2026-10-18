using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;
using System.IO;
using Eitmad.Platform.Windows.LocalIpc;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Authentication;

public partial class SignInView : UserControl
{
    public SignInView()
    {
        InitializeComponent();
        Loaded += (_, _) => Dispatcher.BeginInvoke(UsernameBox.Focus, DispatcherPriority.Input);
    }

    private CancellationTokenSource? signInCancellation;

    public Func<string, string, Eitmad.Contracts.DesktopEnrollmentSettings?, CancellationToken, Task<AuthenticatedSurface>>? AuthenticateAsync { get; set; }

    public event EventHandler<AuthenticatedSurface>? SignedIn;

    private void CredentialKeyDown(object sender, KeyEventArgs eventArgs)
    {
        if (eventArgs.Key != Key.Enter)
        {
            return;
        }

        _ = TrySignInAsync();
        eventArgs.Handled = true;
    }

    private void SignInClick(object sender, RoutedEventArgs eventArgs) => _ = TrySignInAsync();

    private async Task TrySignInAsync()
    {
        var username = UsernameBox.Text.Trim();
        var password = PasswordInput.Password;
        PasswordInput.Clear();
        if (string.IsNullOrWhiteSpace(username) || string.IsNullOrEmpty(password))
        {
            ShowError("أدخل اسم المستخدم وكلمة المرور.");
            PasswordInput.Focus();
            return;
        }
        if (AuthenticateAsync is null)
        {
            ShowConnectionFailure();
            return;
        }

        signInCancellation?.Cancel();
        signInCancellation?.Dispose();
        signInCancellation = new CancellationTokenSource();
        using var diagnostic = new OperationDiagnostics(DiagnosticPhase.SignIn);
        SetBusy(true);
        try
        {
            Eitmad.Contracts.DesktopEnrollmentSettings? connection = null;
            if (ConnectionSetup.IsExpanded)
            {
                if (!Guid.TryParse(OrganizationId.Text, out var organization) || !Guid.TryParse(BranchId.Text, out var branch))
                {
                    ShowError("أدخل معرّف المنشأة ومعرّف الفرع كما استلمتهما من المسؤول.");
                    return;
                }
                connection = new Eitmad.Contracts.DesktopEnrollmentSettings {
                    Endpoint = ServerEndpoint.Text.Trim(), TrustedCertificate = TrustCertificate.Text.Trim(),
                    TenantCode = TenantCode.Text.Trim(), Organization = organization, Branch = branch,
                };
            }
            var surface = await AuthenticateAsync(username, password, connection, signInCancellation.Token);
            SignInError.Visibility = Visibility.Collapsed;
            SignedIn?.Invoke(this, surface);
            diagnostic.Finish();
        }
        catch (OperationCanceledException) when (signInCancellation.IsCancellationRequested)
        {
        }
        catch (SessionPermissionException)
        {
            ShowError("لا يملك هذا الحساب صلاحية الدخول إلى واجهة العمل.");
        }
        catch (EngineIpcException error) when (error.Kind == EngineIpcFailureKind.AuthenticationRejected)
        {
            ShowError("تعذر التحقق من الحساب أو اتصال المنشأة. راجع كلمة المرور وإعدادات الخادم وشهادة الثقة ثم أعد المحاولة. تبقى بياناتك محفوظة.");
        }
        catch (EngineIpcException)
        {
            ShowConnectionFailure();
        }
        catch (Exception error) when (error is IOException or InvalidOperationException)
        {
            ShowConnectionFailure();
        }
        finally
        {
            password = string.Empty;
            SetBusy(false);
        }
    }

    public void Reset(SessionEndReason? reason = null)
    {
        signInCancellation?.Cancel();
        UsernameBox.Clear();
        PasswordInput.Clear();
        if (reason == SessionEndReason.RecoveryFailed)
            ShowError("تعذر إنهاء الجلسة بأمان. أنهِ الاعتماد من أيقونة البرنامج ثم افتحه من جديد.");
        else if (reason == SessionEndReason.AccessChanged)
            ShowError("تعذر تأكيد الوصول إلى حسابك. تحقق من اتصال الخادم ثم سجل الدخول. تبقى بياناتك محفوظة.");
        else if (reason == SessionEndReason.Expired)
            ShowError("انتهت الجلسة. سجّل الدخول من جديد.");
        else if (reason == SessionEndReason.ConnectionLost)
            ShowConnectionFailure();
        else
            SignInError.Visibility = Visibility.Collapsed;
        Dispatcher.BeginInvoke(UsernameBox.Focus, DispatcherPriority.Input);
    }

    public void ShowConnectionFailure() =>
        ShowError("تعذر الاتصال بمحرك الاعتماد. تحقق من تشغيله ثم أعد المحاولة.");

    private void ShowError(string message)
    {
        SignInError.Text = message;
        SignInError.Visibility = Visibility.Visible;
        Dispatcher.BeginInvoke(() => SignInButton.BringIntoView(), DispatcherPriority.Loaded);
    }

    private void SetBusy(bool busy)
    {
        UsernameBox.IsEnabled = !busy;
        PasswordInput.IsEnabled = !busy;
        SignInButton.IsEnabled = !busy;
        ConnectionSetup.IsEnabled = !busy;
        SignInButton.Content = busy ? "جارٍ تسجيل الدخول…" : "دخول";
        SignInProgress.Visibility = busy ? Visibility.Visible : Visibility.Collapsed;
    }
}
