using System.Windows;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Controls;

/// <summary>Presentation options for the shell's native WPF control templates.</summary>
public static class ControlOptions
{
    // Templates bind this to the Windows resource so theme changes update existing controls.
    public static readonly DependencyProperty HighContrastProperty = DependencyProperty.RegisterAttached(
        "HighContrast", typeof(bool), typeof(ControlOptions), new PropertyMetadata(false));
    public static readonly DependencyProperty PlaceholderProperty = DependencyProperty.RegisterAttached(
        "Placeholder", typeof(string), typeof(ControlOptions), new PropertyMetadata(string.Empty));
    public static readonly DependencyProperty IconProperty = DependencyProperty.RegisterAttached(
        "Icon", typeof(Geometry), typeof(ControlOptions), new PropertyMetadata(null));
    public static readonly DependencyProperty ShowTextProperty = DependencyProperty.RegisterAttached(
        "ShowText", typeof(bool), typeof(ControlOptions), new PropertyMetadata(true));
    public static readonly DependencyProperty CornerRadiusProperty = DependencyProperty.RegisterAttached(
        "CornerRadius", typeof(CornerRadius), typeof(ControlOptions), new PropertyMetadata(new CornerRadius(8)));
    public static readonly DependencyProperty FooterTemplateProperty = DependencyProperty.RegisterAttached(
        "FooterTemplate", typeof(DataTemplate), typeof(ControlOptions), new PropertyMetadata(null));

    public static string GetPlaceholder(DependencyObject element) => (string)element.GetValue(PlaceholderProperty);
    public static bool GetHighContrast(DependencyObject element) => (bool)element.GetValue(HighContrastProperty);
    public static void SetHighContrast(DependencyObject element, bool value) => element.SetValue(HighContrastProperty, value);
    public static void SetPlaceholder(DependencyObject element, string value) => element.SetValue(PlaceholderProperty, value);
    public static Geometry? GetIcon(DependencyObject element) => (Geometry?)element.GetValue(IconProperty);
    public static void SetIcon(DependencyObject element, Geometry? value) => element.SetValue(IconProperty, value);
    public static bool GetShowText(DependencyObject element) => (bool)element.GetValue(ShowTextProperty);
    public static void SetShowText(DependencyObject element, bool value) => element.SetValue(ShowTextProperty, value);
    public static CornerRadius GetCornerRadius(DependencyObject element) => (CornerRadius)element.GetValue(CornerRadiusProperty);
    public static void SetCornerRadius(DependencyObject element, CornerRadius value) => element.SetValue(CornerRadiusProperty, value);
    public static DataTemplate? GetFooterTemplate(DependencyObject element) => (DataTemplate?)element.GetValue(FooterTemplateProperty);
    public static void SetFooterTemplate(DependencyObject element, DataTemplate? value) => element.SetValue(FooterTemplateProperty, value);
}
