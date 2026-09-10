using System.Windows;

namespace Eitmad.WindowsShell.Controls;

public class EmptyState : System.Windows.Controls.ContentControl
{
    public static readonly DependencyProperty HeadingProperty = DependencyProperty.Register(nameof(Heading), typeof(string), typeof(EmptyState), new PropertyMetadata(string.Empty));
    public string Heading { get => (string)GetValue(HeadingProperty); set => SetValue(HeadingProperty, value); }

    public static readonly DependencyProperty DescriptionProperty = DependencyProperty.Register(nameof(Description), typeof(string), typeof(EmptyState), new PropertyMetadata(string.Empty));
    public string Description { get => (string)GetValue(DescriptionProperty); set => SetValue(DescriptionProperty, value); }

    public static readonly DependencyProperty IconProperty = DependencyProperty.Register(nameof(Icon), typeof(object), typeof(EmptyState), new PropertyMetadata(null));
    public object Icon { get => (object)GetValue(IconProperty); set => SetValue(IconProperty, value); }

    public static readonly DependencyProperty IsCompactProperty = DependencyProperty.Register(nameof(IsCompact), typeof(bool), typeof(EmptyState), new PropertyMetadata(false));
    public bool IsCompact { get => (bool)GetValue(IsCompactProperty); set => SetValue(IsCompactProperty, value); }

}
