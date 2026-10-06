using System.Text.RegularExpressions;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;

namespace Eitmad.WindowsShell.Features.Reception;

/// <summary>Isolates technical runs in catalog text without changing the returned or stored value.</summary>
public static partial class SalesText
{
    public static readonly DependencyProperty ValueProperty = DependencyProperty.RegisterAttached(
        "Value", typeof(string), typeof(SalesText), new PropertyMetadata("", Render));
    /// <summary>Reads the original attached text value, preserving its stored character order.</summary>
    public static string GetValue(DependencyObject target) => (string)target.GetValue(ValueProperty);
    /// <summary>Sets the original text value for rendering with isolated technical runs.</summary>
    public static void SetValue(DependencyObject target, string value) => target.SetValue(ValueProperty, value);
    /// <summary>Matches Latin identifiers and numeric technical runs that need left-to-right rendering.</summary>
    [GeneratedRegex("([A-Za-z0-9]+(?:[-_./:][A-Za-z0-9]+)*)", RegexOptions.CultureInvariant)]
    private static partial Regex TechnicalRuns();
    /// <summary>Rebuilds native text inlines with left-to-right technical runs while preserving the original value.</summary>
    private static void Render(DependencyObject target, DependencyPropertyChangedEventArgs args)
    {
        if (target is not TextBlock text) return;
        text.Inlines.Clear();
        var value = args.NewValue as string ?? "";
        var offset = 0;
        foreach (Match match in TechnicalRuns().Matches(value))
        {
            if (match.Index > offset) text.Inlines.Add(new Run(value[offset..match.Index]));
            text.Inlines.Add(new Run(match.Value) { FlowDirection = System.Windows.FlowDirection.LeftToRight });
            offset = match.Index + match.Length;
        }
        if (offset < value.Length) text.Inlines.Add(new Run(value[offset..]));
    }
}
