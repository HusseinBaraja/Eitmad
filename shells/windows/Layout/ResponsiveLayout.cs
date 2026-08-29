using System.Windows;

namespace Eitmad.WindowsShell.Layout;

/// <summary>Names the width bands used by responsive Windows shell pages.</summary>
public enum ResponsiveLayoutMode
{
    Compact,
    Standard,
    Wide,
}

/// <summary>
/// Publishes an inherited responsive mode for a page root.
/// Child controls can use XAML data triggers without owning resize event handlers.
/// </summary>
public static class ResponsiveLayout
{
    public const double StandardMinimumWidth = 900;
    public const double WideMinimumWidth = 1600;

    public static readonly DependencyProperty IsEnabledProperty = DependencyProperty.RegisterAttached(
        "IsEnabled",
        typeof(bool),
        typeof(ResponsiveLayout),
        new PropertyMetadata(false, OnIsEnabledChanged));

    private static readonly DependencyPropertyKey ModePropertyKey = DependencyProperty.RegisterAttachedReadOnly(
        "Mode",
        typeof(ResponsiveLayoutMode),
        typeof(ResponsiveLayout),
        new FrameworkPropertyMetadata(
            ResponsiveLayoutMode.Wide,
            FrameworkPropertyMetadataOptions.Inherits));

    public static readonly DependencyProperty ModeProperty = ModePropertyKey.DependencyProperty;

    public static bool GetIsEnabled(DependencyObject element) =>
        (bool)element.GetValue(IsEnabledProperty);

    public static void SetIsEnabled(DependencyObject element, bool value) =>
        element.SetValue(IsEnabledProperty, value);

    public static ResponsiveLayoutMode GetMode(DependencyObject element) =>
        (ResponsiveLayoutMode)element.GetValue(ModeProperty);

    /// <summary>Maps a device-independent page width to one stable layout mode.</summary>
    public static ResponsiveLayoutMode ResolveMode(double width)
    {
        if (width >= WideMinimumWidth)
        {
            return ResponsiveLayoutMode.Wide;
        }

        return width >= StandardMinimumWidth
            ? ResponsiveLayoutMode.Standard
            : ResponsiveLayoutMode.Compact;
    }

    private static void OnIsEnabledChanged(DependencyObject dependencyObject, DependencyPropertyChangedEventArgs eventArgs)
    {
        if (dependencyObject is not FrameworkElement element)
        {
            return;
        }

        if ((bool)eventArgs.NewValue)
        {
            element.Loaded += OnElementLoaded;
            element.SizeChanged += OnElementSizeChanged;
            UpdateMode(element);
            return;
        }

        element.Loaded -= OnElementLoaded;
        element.SizeChanged -= OnElementSizeChanged;
    }

    private static void OnElementLoaded(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is FrameworkElement element)
        {
            UpdateMode(element);
        }
    }

    private static void OnElementSizeChanged(object sender, SizeChangedEventArgs eventArgs)
    {
        if (sender is FrameworkElement element && eventArgs.WidthChanged)
        {
            UpdateMode(element);
        }
    }

    private static void UpdateMode(FrameworkElement element)
    {
        var mode = ResolveMode(element.ActualWidth);
        if (GetMode(element) != mode)
        {
            element.SetValue(ModePropertyKey, mode);
        }
    }
}
