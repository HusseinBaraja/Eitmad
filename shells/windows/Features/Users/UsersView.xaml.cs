using System.Windows;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Users;

public partial class UsersView : UserControl
{
    public UsersView() { InitializeComponent(); DataContext = ViewModel; }
    public UsersViewModel ViewModel { get; } = new();
    private void AddClick(object sender, RoutedEventArgs e) => ViewModel.BeginEdit();
    private void EditClick(object sender, RoutedEventArgs e)
    { if (sender is Button { DataContext: UserPreview user }) ViewModel.BeginEdit(user); }
    private void DeactivateClick(object sender, RoutedEventArgs e)
    { if (sender is Button { DataContext: UserPreview user }) ViewModel.BeginDeactivation(user); }
    private void ApplyClick(object sender, RoutedEventArgs e)
    { if (!ViewModel.ApplyPreview()) UserNameInput.Focus(); }
    private void CancelEditClick(object sender, RoutedEventArgs e) => ViewModel.CancelEditor();
    private void CancelDeactivateClick(object sender, RoutedEventArgs e) => ViewModel.CancelDeactivation();
    private void ConfirmDeactivateClick(object sender, RoutedEventArgs e) => ViewModel.DeactivatePreview();
}
