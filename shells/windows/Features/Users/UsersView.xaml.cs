using System.Windows;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Users;

public partial class UsersView : UserControl
{
    public UsersView() { InitializeComponent(); DataContext = ViewModel; }
    public UsersViewModel ViewModel { get; } = new();
    private System.Windows.Controls.Control? editorReturnTarget;
    private void OpenEditor(object sender, UserPreview? user = null)
    {
        editorReturnTarget = sender as System.Windows.Controls.Control;
        ViewModel.BeginEdit(user);
        Dispatcher.BeginInvoke(UserNameInput.Focus, DispatcherPriority.Input);
    }
    private void RestoreListFocus() => Dispatcher.BeginInvoke(() =>
    {
        if (editorReturnTarget is { IsVisible: true } target) target.Focus();
        else UsersSearchBox.Focus();
        editorReturnTarget = null;
    }, DispatcherPriority.Input);
    private void AddClick(object sender, RoutedEventArgs e) => OpenEditor(sender);
    private void EditClick(object sender, RoutedEventArgs e)
    { if (sender is Button { DataContext: UserPreview user }) OpenEditor(sender, user); }
    private void DeactivateClick(object sender, RoutedEventArgs e)
    { if (sender is Button { DataContext: UserPreview user }) ViewModel.BeginDeactivation(user); }
    private void ApplyClick(object sender, RoutedEventArgs e)
    { if (!ViewModel.ApplyPreview()) UserNameInput.Focus(); else RestoreListFocus(); }
    private void CancelEditClick(object sender, RoutedEventArgs e) { ViewModel.CancelEditor(); RestoreListFocus(); }
    private void CancelDeactivateClick(object sender, RoutedEventArgs e) => ViewModel.CancelDeactivation();
    private void ConfirmDeactivateClick(object sender, RoutedEventArgs e) => ViewModel.DeactivatePreview();
}
