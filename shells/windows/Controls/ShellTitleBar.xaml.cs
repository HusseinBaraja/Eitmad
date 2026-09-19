using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Button = System.Windows.Controls.Button;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Controls;

public partial class ShellTitleBar : UserControl
{
    public static readonly DependencyProperty TitleProperty = DependencyProperty.Register(nameof(Title), typeof(string), typeof(ShellTitleBar), new PropertyMetadata("لوحة التحكم"));
    public static readonly DependencyProperty SearchPlaceholderProperty = DependencyProperty.Register(nameof(SearchPlaceholder), typeof(string), typeof(ShellTitleBar), new PropertyMetadata("ابحث عن عروض أسعار، عملاء، منتجات، أو أوامر عمل..."));
    public static readonly DependencyProperty PrimaryActionLabelProperty = DependencyProperty.Register(nameof(PrimaryActionLabel), typeof(string), typeof(ShellTitleBar), new PropertyMetadata("عرض سعر جديد"));
    public static readonly DependencyProperty SwitchAccountHintProperty = DependencyProperty.Register(nameof(SwitchAccountHint), typeof(string), typeof(ShellTitleBar), new PropertyMetadata("تبديل الحساب · Alt+K"));

    public ShellTitleBar() => InitializeComponent();

    public string Title { get => (string)GetValue(TitleProperty); set => SetValue(TitleProperty, value); }
    public string SearchPlaceholder { get => (string)GetValue(SearchPlaceholderProperty); set => SetValue(SearchPlaceholderProperty, value); }
    public string PrimaryActionLabel { get => (string)GetValue(PrimaryActionLabelProperty); set => SetValue(PrimaryActionLabelProperty, value); }
    public string SwitchAccountHint { get => (string)GetValue(SwitchAccountHintProperty); set => SetValue(SwitchAccountHintProperty, value); }

    public event EventHandler? AccountSwitchRequested;
    public event EventHandler<ShellActionEventArgs>? ActionRequested;
    public event EventHandler<ShellSearchEventArgs>? SearchSubmitted;

    private void SwitchAccountClick(object sender, RoutedEventArgs e) => AccountSwitchRequested?.Invoke(this, EventArgs.Empty);
    private void PrimaryActionClick(object sender, RoutedEventArgs e) => ActionRequested?.Invoke(this, new ShellActionEventArgs(PrimaryActionLabel, true));
    private void ActionClick(object sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string action }) ActionRequested?.Invoke(this, new ShellActionEventArgs(action, false));
    }
    private void SearchKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key != Key.Enter || string.IsNullOrWhiteSpace(SearchBox.Text)) return;
        SearchSubmitted?.Invoke(this, new ShellSearchEventArgs(SearchBox.Text.Trim()));
        e.Handled = true;
    }
}

public sealed class ShellActionEventArgs(string action, bool isPrimary) : EventArgs
{
    public string Action { get; } = action;
    public bool IsPrimary { get; } = isPrimary;
}

public sealed class ShellSearchEventArgs(string query) : EventArgs
{
    public string Query { get; } = query;
}
