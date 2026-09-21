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

    public Func<string, string, CancellationToken, Task<AuthenticatedSurface>>? AuthenticateAsync { get; set; }

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
        SetBusy(true);
        try
        {
            var surface = await AuthenticateAsync(username, password, signInCancellation.Token);
            SignInError.Visibility = Visibility.Collapsed;
            SignedIn?.Invoke(this, surface);
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
            ShowError("بيانات الدخول غير صحيحة أو انتهت صلاحية الحساب.");
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
        if (reason == SessionEndReason.Expired)
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
    }

    private void SetBusy(bool busy)
    {
        UsernameBox.IsEnabled = !busy;
        PasswordInput.IsEnabled = !busy;
        SignInButton.IsEnabled = !busy;
        SignInButton.Content = busy ? "جارٍ تسجيل الدخول…" : "دخول";
        SignInProgress.Visibility = busy ? Visibility.Visible : Visibility.Collapsed;
    }
}
