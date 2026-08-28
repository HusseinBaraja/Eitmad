using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.WindowsShell.Features.Operations;
using Brush = System.Windows.Media.Brush;
using Brushes = System.Windows.Media.Brushes;
using Button = System.Windows.Controls.Button;
using Color = System.Windows.Media.Color;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;

namespace Eitmad.WindowsShell;

public partial class MainWindow : Window
{
    private const string SearchPlaceholder = "ابحث عن عروض أسعار، عملاء، منتجات، أو أوامر عمل...";
    private readonly DispatcherTimer toastTimer;
    private Button selectedNavButton;

    public MainWindow(OperationsViewModel viewModel)
    {
        InitializeComponent();
        DataContext = viewModel;
        selectedNavButton = HomeNavButton;
        foreach (var button in VisualDescendants<Button>(SidebarNavigation))
        {
            button.MouseEnter += NavigationMouseEnter;
            button.MouseLeave += NavigationMouseLeave;
        }
        toastTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(2.5) };
        toastTimer.Tick += (_, _) =>
        {
            toastTimer.Stop();
            InteractionToast.Visibility = Visibility.Collapsed;
        };
    }

    private void NavigationClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is not Button button || button.Tag is not string destination)
        {
            return;
        }

        SetNavigationTone(selectedNavButton, false);
        selectedNavButton = button;
        SetNavigationTone(button, true);
        DashboardTitle.Text = destination == "الرئيسية" ? "لوحة التحكم" : destination;
        ShowToast($"تم فتح {destination} في وضع المعاينة");
    }

    private void NavigationMouseEnter(object sender, System.Windows.Input.MouseEventArgs eventArgs)
    {
        if (sender is Button button && ReferenceEquals(button, selectedNavButton))
        {
            SetNavigationContentTone(button, Brushes.Black);
        }
    }

    private void NavigationMouseLeave(object sender, System.Windows.Input.MouseEventArgs eventArgs)
    {
        if (sender is Button button && ReferenceEquals(button, selectedNavButton))
        {
            SetNavigationContentTone(button, Brushes.White);
        }
    }

    private void PreviewActionClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { Tag: string action })
        {
            ShowToast($"تم اختيار {action}");
        }
    }

    private void OpenPreviewPanelClick(object sender, RoutedEventArgs eventArgs)
    {
        PreviewPanelTitle.Text = sender is Button { Tag: string title } ? title : "عرض سعر جديد";
        InteractionPanel.Visibility = Visibility.Visible;
        CustomerNameBox.Focus();
    }

    private void ClosePreviewPanelClick(object sender, RoutedEventArgs eventArgs) =>
        InteractionPanel.Visibility = Visibility.Collapsed;

    private void PreviewSubmitClick(object sender, RoutedEventArgs eventArgs)
    {
        if (string.IsNullOrWhiteSpace(CustomerNameBox.Text))
        {
            ShowToast("أدخل اسم العميل للمتابعة");
            CustomerNameBox.Focus();
            return;
        }

        InteractionPanel.Visibility = Visibility.Collapsed;
        ShowToast("تم فحص المسودة محلياً؛ الحفظ معطل في وضع المعاينة");
    }

    private void SearchGotFocus(object sender, KeyboardFocusChangedEventArgs eventArgs)
    {
        if (SearchBox.Text == SearchPlaceholder)
        {
            SearchBox.Clear();
            SearchBox.Foreground = (Brush)FindResource("InkBrush");
        }
    }

    private void SearchLostFocus(object sender, KeyboardFocusChangedEventArgs eventArgs)
    {
        if (string.IsNullOrWhiteSpace(SearchBox.Text))
        {
            SearchBox.Text = SearchPlaceholder;
            SearchBox.Foreground = new SolidColorBrush(Color.FromRgb(0x8D, 0x87, 0x81));
        }
    }

    private void SearchKeyDown(object sender, KeyEventArgs eventArgs)
    {
        if (eventArgs.Key != Key.Enter || string.IsNullOrWhiteSpace(SearchBox.Text) || SearchBox.Text == SearchPlaceholder)
        {
            return;
        }

        ShowToast($"نتائج المعاينة عن: {SearchBox.Text.Trim()}");
        eventArgs.Handled = true;
    }

    private void ShowToast(string message)
    {
        InteractionToastText.Text = message;
        InteractionToast.Visibility = Visibility.Visible;
        toastTimer.Stop();
        toastTimer.Start();
    }

    private static void SetNavigationTone(Button button, bool selected)
    {
        button.Background = selected
            ? new LinearGradientBrush(Color.FromRgb(0xB6, 0x76, 0x34), Color.FromRgb(0x7C, 0x41, 0x0C), 35)
            : Brushes.Transparent;
        if (selected)
        {
            SetNavigationContentTone(button, button.IsMouseOver ? Brushes.Black : Brushes.White);
            return;
        }

        foreach (var text in VisualDescendants<TextBlock>(button))
        {
            text.Foreground = new SolidColorBrush(Color.FromRgb(0x20, 0x1A, 0x17));
        }

        foreach (var icon in VisualDescendants<System.Windows.Shapes.Path>(button))
        {
            icon.ClearValue(System.Windows.Shapes.Shape.FillProperty);
        }
    }

    private static void SetNavigationContentTone(Button button, Brush tone)
    {
        foreach (var text in VisualDescendants<TextBlock>(button))
        {
            text.Foreground = tone;
        }

        foreach (var icon in VisualDescendants<System.Windows.Shapes.Path>(button))
        {
            icon.Fill = tone;
        }
    }

    private static IEnumerable<T> VisualDescendants<T>(DependencyObject parent) where T : DependencyObject
    {
        for (var index = 0; index < VisualTreeHelper.GetChildrenCount(parent); index++)
        {
            var child = VisualTreeHelper.GetChild(parent, index);
            if (child is T match)
            {
                yield return match;
            }

            foreach (var descendant in VisualDescendants<T>(child))
            {
                yield return descendant;
            }
        }
    }

}
