using System.Windows;
using System.Windows.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Controls;

/// <summary>Keyboard-accessible bounded integer stepper for native selection pages.</summary>
public sealed class QuantityStepper : UserControl
{
    public static readonly DependencyProperty ValueProperty = DependencyProperty.Register(nameof(Value), typeof(int), typeof(QuantityStepper),
        new FrameworkPropertyMetadata(1, FrameworkPropertyMetadataOptions.BindsTwoWayByDefault, Changed, Coerce));
    public static readonly DependencyProperty MaximumProperty = DependencyProperty.Register(nameof(Maximum), typeof(int), typeof(QuantityStepper),
        new PropertyMetadata(999, (d, _) => { d.CoerceValue(ValueProperty); ((QuantityStepper)d).Refresh(); }), value => (int)value >= 1);
    private readonly Button decrease = new() { Content = "−", MinWidth = 44, MinHeight = 44 };
    private readonly Button increase = new() { Content = "+", MinWidth = 44, MinHeight = 44 };
    private readonly TextBlock amount = new() { MinWidth = 52, TextAlignment = TextAlignment.Center, VerticalAlignment = VerticalAlignment.Center, FontSize = 20 };
    public QuantityStepper()
    {
        Focusable = false;
        amount.SetResourceReference(TextBlock.ForegroundProperty, System.Windows.SystemColors.WindowTextBrushKey);
        var panel = new StackPanel { Orientation = System.Windows.Controls.Orientation.Horizontal, FlowDirection = System.Windows.FlowDirection.LeftToRight };
        foreach (var button in new[] { decrease, increase }) button.SetResourceReference(StyleProperty, "SecondaryButton");
        System.Windows.Automation.AutomationProperties.SetName(decrease, "تقليل الكمية");
        System.Windows.Automation.AutomationProperties.SetName(increase, "زيادة الكمية");
        decrease.Click += (_, _) => SetCurrentValue(ValueProperty, Value - 1);
        increase.Click += (_, _) => SetCurrentValue(ValueProperty, Value + 1);
        panel.Children.Add(decrease); panel.Children.Add(amount); panel.Children.Add(increase);
        Content = panel; Refresh();
    }
    public int Value { get => (int)GetValue(ValueProperty); set => SetValue(ValueProperty, value); }
    public int Maximum { get => (int)GetValue(MaximumProperty); set => SetValue(MaximumProperty, value); }
    private static object Coerce(DependencyObject d, object value) => Math.Clamp((int)value, 1, ((QuantityStepper)d).Maximum);
    private static void Changed(DependencyObject d, DependencyPropertyChangedEventArgs e) => ((QuantityStepper)d).Refresh();
    private void Refresh() { amount.Text = Value.ToString(System.Globalization.CultureInfo.InvariantCulture); decrease.IsEnabled = Value > 1; increase.IsEnabled = Value < Maximum; }
}


