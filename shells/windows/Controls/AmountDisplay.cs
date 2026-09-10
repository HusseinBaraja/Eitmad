using System.Windows;

namespace Eitmad.WindowsShell.Controls;

public class AmountDisplay : System.Windows.Controls.Control
{
    public static readonly DependencyProperty AmountTextProperty = DependencyProperty.Register(nameof(AmountText), typeof(string), typeof(AmountDisplay), new PropertyMetadata(string.Empty));
    public string AmountText { get => (string)GetValue(AmountTextProperty); set => SetValue(AmountTextProperty, value); }

    public static readonly DependencyProperty UnitTextProperty = DependencyProperty.Register(nameof(UnitText), typeof(string), typeof(AmountDisplay), new PropertyMetadata(string.Empty));
    public string UnitText { get => (string)GetValue(UnitTextProperty); set => SetValue(UnitTextProperty, value); }

    public static readonly DependencyProperty UnitPlacementProperty = DependencyProperty.Register(nameof(UnitPlacement), typeof(UnitPlacement), typeof(AmountDisplay), new PropertyMetadata(UnitPlacement.After));
    public UnitPlacement UnitPlacement { get => (UnitPlacement)GetValue(UnitPlacementProperty); set => SetValue(UnitPlacementProperty, value); }

    public static readonly DependencyProperty EmptyTextProperty = DependencyProperty.Register(nameof(EmptyText), typeof(string), typeof(AmountDisplay), new PropertyMetadata("—"));
    public string EmptyText { get => (string)GetValue(EmptyTextProperty); set => SetValue(EmptyTextProperty, value); }

}
