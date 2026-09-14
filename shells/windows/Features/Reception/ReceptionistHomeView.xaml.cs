using System.Windows;
using System.Windows.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class ReceptionistHomeView : UserControl
{
    public ReceptionistHomeView() => InitializeComponent();

    public event EventHandler? AccountSwitchRequested;

    private void SharedAccountSwitchRequested(object? sender, EventArgs eventArgs) =>
        AccountSwitchRequested?.Invoke(this, EventArgs.Empty);

    private void TitleBarActionRequested(object? sender, Controls.ShellActionEventArgs eventArgs) =>
        ShowPreviewFeedback(eventArgs.Action);

    private void TitleBarSearchSubmitted(object? sender, Controls.ShellSearchEventArgs eventArgs) =>
        ShowNotice($"نتائج المعاينة عن: {eventArgs.Query}");

    private void SidebarNavigationRequested(object? sender, Controls.NavigationRequestedEventArgs eventArgs) =>
        ShowNotice($"تم فتح {eventArgs.Destination} في وضع المعاينة");

    private void ActionClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is not Button { Tag: string action })
        {
            return;
        }

        ShowPreviewFeedback(action);
    }

    private void ShowPreviewFeedback(string action) =>
        ShowNotice(action == "عرض سعر جديد"
            ? "واجهة إنشاء عرض السعر ستُربط في المرحلة التالية"
            : $"تم اختيار {action} في وضع المعاينة");

    private void ShowNotice(string message)
    {
        ReceptionistNotice.Message = message;
        ReceptionistNotice.RestartDuration();
    }

    private void DismissNotice(object sender, RoutedEventArgs eventArgs) => ReceptionistNotice.Message = string.Empty;
}
