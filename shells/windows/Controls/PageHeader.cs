using System.Windows;

namespace Eitmad.WindowsShell.Controls;

public class PageHeader : System.Windows.Controls.ContentControl
{
    public static readonly DependencyProperty TitleProperty = DependencyProperty.Register(nameof(Title), typeof(string), typeof(PageHeader), new PropertyMetadata(string.Empty));
    public string Title { get => (string)GetValue(TitleProperty); set => SetValue(TitleProperty, value); }

    public static readonly DependencyProperty SubtitleProperty = DependencyProperty.Register(nameof(Subtitle), typeof(string), typeof(PageHeader), new PropertyMetadata(string.Empty));
    public string Subtitle { get => (string)GetValue(SubtitleProperty); set => SetValue(SubtitleProperty, value); }

    public static readonly DependencyProperty IconProperty = DependencyProperty.Register(nameof(Icon), typeof(object), typeof(PageHeader), new PropertyMetadata(null));
    public object Icon { get => (object)GetValue(IconProperty); set => SetValue(IconProperty, value); }

    public static readonly DependencyProperty BackActionProperty = DependencyProperty.Register(nameof(BackAction), typeof(object), typeof(PageHeader), new PropertyMetadata(null));
    public object BackAction { get => (object)GetValue(BackActionProperty); set => SetValue(BackActionProperty, value); }

}
