using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using Brush = System.Windows.Media.Brush;
using Brushes = System.Windows.Media.Brushes;
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
        SetNavigationTone(selectedButton, true);
    }

    public bool IsReceptionist { get => (bool)GetValue(IsReceptionistProperty); set => SetValue(IsReceptionistProperty, value); }
    public event EventHandler<NavigationRequestedEventArgs>? NavigationRequested;

    public void SelectHome() => Select(HomeNavButton);
    public void SelectDestination(string destination)
    {
        var button = new[] { HomeNavButton, QuotationsNavButton, OrdersNavButton, MaterialsNavButton, PartsNavButton, FurnitureNavButton, PricingNavButton, ProductsNavButton, WorkOrdersNavButton, UsersNavButton }
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
        SetNavigationTone(selectedButton, false);
        selectedButton = button;
        SetNavigationTone(button, true);
    }

    private static void SetNavigationTone(Button button, bool selected)
    {
        var content = button.Content as DependencyObject ?? button;
        if (selected)
        {
            button.SetResourceReference(BackgroundProperty, "NavSelectedBrush");
            SetContentTone(content, Brushes.White);
            return;
        }
        button.ClearValue(BackgroundProperty);
        foreach (var text in Descendants<TextBlock>(content)) text.ClearValue(TextBlock.ForegroundProperty);
        foreach (var icon in Descendants<System.Windows.Shapes.Path>(content)) icon.ClearValue(System.Windows.Shapes.Shape.FillProperty);
    }

    private static void SetContentTone(DependencyObject content, Brush tone)
    {
        foreach (var text in Descendants<TextBlock>(content)) text.Foreground = tone;
        foreach (var icon in Descendants<System.Windows.Shapes.Path>(content)) icon.Fill = tone;
    }

    private static IEnumerable<T> Descendants<T>(DependencyObject parent) where T : DependencyObject
    {
        for (var index = 0; index < VisualTreeHelper.GetChildrenCount(parent); index++)
        {
            var child = VisualTreeHelper.GetChild(parent, index);
            if (child is T match) yield return match;
            foreach (var descendant in Descendants<T>(child)) yield return descendant;
        }
    }
}

public sealed class NavigationRequestedEventArgs(string destination) : EventArgs
{
    public string Destination { get; } = destination;
}
