using System.Windows;
using System.Windows.Media;
using Brush = System.Windows.Media.Brush;

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
    public static readonly DependencyProperty ShowFocusRingProperty = DependencyProperty.RegisterAttached(
        "ShowFocusRing", typeof(bool), typeof(ControlOptions), new PropertyMetadata(true));
    public static readonly DependencyProperty CornerRadiusProperty = DependencyProperty.RegisterAttached(
        "CornerRadius", typeof(CornerRadius), typeof(ControlOptions), new PropertyMetadata(new CornerRadius(8)));
    public static readonly DependencyProperty FooterTemplateProperty = DependencyProperty.RegisterAttached(
        "FooterTemplate", typeof(DataTemplate), typeof(ControlOptions), new PropertyMetadata(null));
    public static readonly DependencyProperty HoverBackgroundProperty = DependencyProperty.RegisterAttached(
        "HoverBackground", typeof(Brush), typeof(ControlOptions), new PropertyMetadata(null));
    public static readonly DependencyProperty HoverBorderBrushProperty = DependencyProperty.RegisterAttached(
        "HoverBorderBrush", typeof(Brush), typeof(ControlOptions), new PropertyMetadata(null));
    public static readonly DependencyProperty HoverOpacityProperty = DependencyProperty.RegisterAttached(
        "HoverOpacity", typeof(double), typeof(ControlOptions), new PropertyMetadata(1d));
    public static readonly DependencyProperty PressedOpacityProperty = DependencyProperty.RegisterAttached(
        "PressedOpacity", typeof(double), typeof(ControlOptions), new PropertyMetadata(1d));

    public static string GetPlaceholder(DependencyObject element) => (string)element.GetValue(PlaceholderProperty);
    public static bool GetHighContrast(DependencyObject element) => (bool)element.GetValue(HighContrastProperty);
    public static void SetHighContrast(DependencyObject element, bool value) => element.SetValue(HighContrastProperty, value);
    public static void SetPlaceholder(DependencyObject element, string value) => element.SetValue(PlaceholderProperty, value);
    public static Geometry? GetIcon(DependencyObject element) => (Geometry?)element.GetValue(IconProperty);
    public static void SetIcon(DependencyObject element, Geometry? value) => element.SetValue(IconProperty, value);
    public static bool GetShowText(DependencyObject element) => (bool)element.GetValue(ShowTextProperty);
    public static void SetShowText(DependencyObject element, bool value) => element.SetValue(ShowTextProperty, value);
    public static bool GetShowFocusRing(DependencyObject element) => (bool)element.GetValue(ShowFocusRingProperty);
    public static void SetShowFocusRing(DependencyObject element, bool value) => element.SetValue(ShowFocusRingProperty, value);
    public static CornerRadius GetCornerRadius(DependencyObject element) => (CornerRadius)element.GetValue(CornerRadiusProperty);
    public static void SetCornerRadius(DependencyObject element, CornerRadius value) => element.SetValue(CornerRadiusProperty, value);
    public static DataTemplate? GetFooterTemplate(DependencyObject element) => (DataTemplate?)element.GetValue(FooterTemplateProperty);
    public static void SetFooterTemplate(DependencyObject element, DataTemplate? value) => element.SetValue(FooterTemplateProperty, value);
    public static Brush? GetHoverBackground(DependencyObject element) => (Brush?)element.GetValue(HoverBackgroundProperty);
    public static void SetHoverBackground(DependencyObject element, Brush? value) => element.SetValue(HoverBackgroundProperty, value);
    public static Brush? GetHoverBorderBrush(DependencyObject element) => (Brush?)element.GetValue(HoverBorderBrushProperty);
    public static void SetHoverBorderBrush(DependencyObject element, Brush? value) => element.SetValue(HoverBorderBrushProperty, value);
    public static double GetHoverOpacity(DependencyObject element) => (double)element.GetValue(HoverOpacityProperty);
    public static void SetHoverOpacity(DependencyObject element, double value) => element.SetValue(HoverOpacityProperty, value);
    public static double GetPressedOpacity(DependencyObject element) => (double)element.GetValue(PressedOpacityProperty);
    public static void SetPressedOpacity(DependencyObject element, double value) => element.SetValue(PressedOpacityProperty, value);
}
