using System.Windows;

namespace Eitmad.WindowsShell.Controls;

public class StatusBadge : System.Windows.Controls.Control
{
    public static readonly DependencyProperty TextProperty = DependencyProperty.Register(nameof(Text), typeof(string), typeof(StatusBadge), new PropertyMetadata(string.Empty));
    public string Text { get => (string)GetValue(TextProperty); set => SetValue(TextProperty, value); }

    public static readonly DependencyProperty ToneProperty = DependencyProperty.Register(nameof(Tone), typeof(PresentationTone), typeof(StatusBadge), new PropertyMetadata(PresentationTone.Neutral));
    public PresentationTone Tone { get => (PresentationTone)GetValue(ToneProperty); set => SetValue(ToneProperty, value); }

    public static readonly DependencyProperty IconProperty = DependencyProperty.Register(nameof(Icon), typeof(object), typeof(StatusBadge), new PropertyMetadata(null));
    public object Icon { get => (object)GetValue(IconProperty); set => SetValue(IconProperty, value); }

    public static readonly DependencyProperty IsCompactProperty = DependencyProperty.Register(nameof(IsCompact), typeof(bool), typeof(StatusBadge), new PropertyMetadata(false));
    public bool IsCompact { get => (bool)GetValue(IsCompactProperty); set => SetValue(IsCompactProperty, value); }

}
