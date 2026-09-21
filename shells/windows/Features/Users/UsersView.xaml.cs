using System.Windows;
using System.Windows.Threading;
using Eitmad.Platform.Windows.Shell;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Users;

public partial class UsersView : UserControl
{
    public UsersView()
    {
        InitializeComponent();
        DataContext = ViewModel;
        IsVisibleChanged += async (_, _) =>
        {
            if (IsVisible) await ViewModel.LoadAsync();
        };
    }
    public UsersViewModel ViewModel { get; } = new();
    public void Attach(IEngineShellBridge engine) => ViewModel.Attach(engine);
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
    private async void ApplyClick(object sender, RoutedEventArgs e)
    {
        if (!await ViewModel.ApplyAsync(UserPasswordInput.Password)) UserNameInput.Focus();
        else
        {
            UserPasswordInput.Clear();
            RestoreListFocus();
        }
    }
    private void CancelEditClick(object sender, RoutedEventArgs e) { ViewModel.CancelEditor(); RestoreListFocus(); }
    private void CancelDeactivateClick(object sender, RoutedEventArgs e) => ViewModel.CancelDeactivation();
    private async void ConfirmDeactivateClick(object sender, RoutedEventArgs e)
    {
        if (await ViewModel.DeactivateAsync()) RestoreListFocus();
    }
}
