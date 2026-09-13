using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
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

    public event EventHandler<PreviewSignedInEventArgs>? SignedIn;

    private void SelectDevelopmentAccountClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is not Button { Tag: string username })
        {
            return;
        }

        UsernameBox.Text = username;
        PasswordInput.Password = username;
        SignInError.Visibility = Visibility.Collapsed;
        PasswordInput.Focus();
    }

    private void CredentialKeyDown(object sender, KeyEventArgs eventArgs)
    {
        if (eventArgs.Key != Key.Enter)
        {
            return;
        }

        TrySignIn();
        eventArgs.Handled = true;
    }

    private void SignInClick(object sender, RoutedEventArgs eventArgs) => TrySignIn();

    private void TrySignIn()
    {
        var username = UsernameBox.Text.Trim();
        var password = PasswordInput.Password;
        var role = (username, password) switch
        {
            ("admin", "admin") => PreviewAccountRole.Manager,
            ("rec", "rec") => PreviewAccountRole.Receptionist,
            _ => (PreviewAccountRole?)null,
        };

        if (role is null)
        {
            SignInError.Visibility = Visibility.Visible;
            PasswordInput.Focus();
            return;
        }

        SignInError.Visibility = Visibility.Collapsed;
        SignedIn?.Invoke(this, new PreviewSignedInEventArgs(role.Value));
    }
}
