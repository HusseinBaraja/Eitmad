using System.Windows;
using System.Windows.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class ReceptionistHomeView : UserControl
{
    public ReceptionistHomeView() => InitializeComponent();

    public event EventHandler? AccountSwitchRequested;

    private void SwitchAccountClick(object sender, RoutedEventArgs eventArgs) =>
        AccountSwitchRequested?.Invoke(this, EventArgs.Empty);

    private void ActionClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is not Button { Tag: string action })
        {
            return;
        }

        ReceptionistNotice.Message = action == "عرض سعر جديد"
            ? "واجهة إنشاء عرض السعر ستُربط في المرحلة التالية"
            : $"تم اختيار {action} في وضع المعاينة";
        ReceptionistNotice.RestartDuration();
    }

    private void DismissNotice(object sender, RoutedEventArgs eventArgs) => ReceptionistNotice.Message = string.Empty;
}
