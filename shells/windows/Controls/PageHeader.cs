using System.Windows;
using System.Windows.Input;
using System.Windows.Media;

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

    public static readonly RoutedEvent ButtonClickEvent = System.Windows.Controls.Primitives.ButtonBase.ClickEvent.AddOwner(typeof(PageHeader));
    public event RoutedEventHandler ButtonClick { add => AddHandler(ButtonClickEvent, value); remove => RemoveHandler(ButtonClickEvent, value); }

    public static readonly DependencyProperty ContainsButtonProperty = DependencyProperty.Register(nameof(ContainsButton), typeof(bool), typeof(PageHeader), new PropertyMetadata(false));
    public bool ContainsButton { get => (bool)GetValue(ContainsButtonProperty); set => SetValue(ContainsButtonProperty, value); }

    public static readonly DependencyProperty ButtonTextProperty = DependencyProperty.Register(nameof(ButtonText), typeof(string), typeof(PageHeader), new PropertyMetadata(string.Empty));
    public string ButtonText { get => (string)GetValue(ButtonTextProperty); set => SetValue(ButtonTextProperty, value); }

    public static readonly DependencyProperty ButtonTypeProperty = DependencyProperty.Register(nameof(ButtonType), typeof(HeaderButtonType), typeof(PageHeader), new PropertyMetadata(HeaderButtonType.Primary));
    public HeaderButtonType ButtonType { get => (HeaderButtonType)GetValue(ButtonTypeProperty); set => SetValue(ButtonTypeProperty, value); }

    public static readonly DependencyProperty ButtonIconProperty = DependencyProperty.Register(nameof(ButtonIcon), typeof(Geometry), typeof(PageHeader), new PropertyMetadata(null));
    public Geometry ButtonIcon { get => (Geometry)GetValue(ButtonIconProperty); set => SetValue(ButtonIconProperty, value); }

    public static readonly DependencyProperty ButtonCommandProperty = DependencyProperty.Register(nameof(ButtonCommand), typeof(ICommand), typeof(PageHeader), new PropertyMetadata(null));
    public ICommand ButtonCommand { get => (ICommand)GetValue(ButtonCommandProperty); set => SetValue(ButtonCommandProperty, value); }

    public static readonly DependencyProperty ButtonCommandParameterProperty = DependencyProperty.Register(nameof(ButtonCommandParameter), typeof(object), typeof(PageHeader), new PropertyMetadata(null));
    public object ButtonCommandParameter { get => (object)GetValue(ButtonCommandParameterProperty); set => SetValue(ButtonCommandParameterProperty, value); }

    public static readonly DependencyProperty ButtonHeightProperty = DependencyProperty.Register(nameof(ButtonHeight), typeof(double), typeof(PageHeader), new PropertyMetadata(46d));
    public double ButtonHeight { get => (double)GetValue(ButtonHeightProperty); set => SetValue(ButtonHeightProperty, value); }

    public static readonly DependencyProperty DescriptionFontSizeProperty = DependencyProperty.Register(nameof(DescriptionFontSize), typeof(double), typeof(PageHeader), new PropertyMetadata(11d));
    public double DescriptionFontSize { get => (double)GetValue(DescriptionFontSizeProperty); set => SetValue(DescriptionFontSizeProperty, value); }

    public static readonly DependencyProperty DescriptionMarginProperty = DependencyProperty.Register(nameof(DescriptionMargin), typeof(Thickness), typeof(PageHeader), new PropertyMetadata(new Thickness(0, 4, 0, 0)));
    public Thickness DescriptionMargin { get => (Thickness)GetValue(DescriptionMarginProperty); set => SetValue(DescriptionMarginProperty, value); }

}

public enum HeaderButtonType
{
    Primary,
    Secondary,
    Inline,
}
