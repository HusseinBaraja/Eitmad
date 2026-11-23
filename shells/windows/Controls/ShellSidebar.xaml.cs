using System.Windows;
using System.Windows.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Controls;

public partial class ShellSidebar : UserControl
{
    public static readonly DependencyProperty IsReceptionistProperty = DependencyProperty.Register(nameof(IsReceptionist), typeof(bool), typeof(ShellSidebar), new PropertyMetadata(false));
    private Button selectedButton;

    public ShellSidebar()
    {
        InitializeComponent();
        selectedButton = HomeNavButton;
        ControlOptions.SetIsCurrent(selectedButton, true);
    }

    public bool IsReceptionist { get => (bool)GetValue(IsReceptionistProperty); set => SetValue(IsReceptionistProperty, value); }
    public event EventHandler<NavigationRequestedEventArgs>? NavigationRequested;

    public void SelectHome() => Select(HomeNavButton);
    public void SelectDestination(string destination)
    {
        var button = new[] { HomeNavButton, QuotationsNavButton, OrdersNavButton, CustomersNavButton, MaterialsNavButton, PartsNavButton, FurnitureNavButton, PricingNavButton, ProductsNavButton, WorkOrdersNavButton, UsersNavButton }
            .First(candidate => Equals(candidate.Tag, destination));
        Select(button);
    }

    private void NavigationClick(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string destination } button) return;
        Select(button);
        NavigationRequested?.Invoke(this, new NavigationRequestedEventArgs(destination));
    }

    private void Select(Button button)
    {
        ControlOptions.SetIsCurrent(selectedButton, false);
        selectedButton = button;
        ControlOptions.SetIsCurrent(button, true);
    }
}

public sealed class NavigationRequestedEventArgs(string destination) : EventArgs
{
    public string Destination { get; } = destination;
}
