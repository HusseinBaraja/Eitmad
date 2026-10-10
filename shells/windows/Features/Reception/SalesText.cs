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
    public static string GetValue(DependencyObject target) => (string)target.GetValue(ValueProperty);
    public static void SetValue(DependencyObject target, string value) => target.SetValue(ValueProperty, value);
    [GeneratedRegex("([A-Za-z0-9]+(?:[-_./:][A-Za-z0-9]+)*)", RegexOptions.CultureInvariant)]
    private static partial Regex TechnicalRuns();
    private static void Render(DependencyObject target, DependencyPropertyChangedEventArgs args)
    {
        if (target is not TextBlock text) return;
        text.Inlines.Clear();
        var value = args.NewValue as string ?? "";
        Append(text.Inlines, value);
    }
    internal static void Append(InlineCollection inlines, string value)
    {
        var offset = 0;
        foreach (Match match in TechnicalRuns().Matches(value))
        {
            if (match.Index > offset) inlines.Add(new Run(value[offset..match.Index]));
            inlines.Add(new Run(match.Value) { FlowDirection = System.Windows.FlowDirection.LeftToRight });
            offset = match.Index + match.Length;
        }
        if (offset < value.Length) inlines.Add(new Run(value[offset..]));
    }
}
