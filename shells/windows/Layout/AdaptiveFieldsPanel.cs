using System.Windows;
using Panel = System.Windows.Controls.Panel;
using Size = System.Windows.Size;

namespace Eitmad.WindowsShell.Layout;

/// <summary>Fits form and filter fields into equal columns; rows grow with their content.</summary>
public sealed class AdaptiveFieldsPanel : Panel
{
    public static readonly DependencyProperty MinItemWidthProperty = DependencyProperty.Register(
        nameof(MinItemWidth), typeof(double), typeof(AdaptiveFieldsPanel),
        new FrameworkPropertyMetadata(200d, FrameworkPropertyMetadataOptions.AffectsMeasure),
        value => value is double number && double.IsFinite(number) && number > 0);
    public static readonly DependencyProperty SpacingProperty = DependencyProperty.Register(
        nameof(Spacing), typeof(double), typeof(AdaptiveFieldsPanel),
        new FrameworkPropertyMetadata(12d, FrameworkPropertyMetadataOptions.AffectsMeasure),
        value => value is double number && double.IsFinite(number) && number >= 0);

    public double MinItemWidth { get => (double)GetValue(MinItemWidthProperty); set => SetValue(MinItemWidthProperty, value); }
    public double Spacing { get => (double)GetValue(SpacingProperty); set => SetValue(SpacingProperty, value); }

    protected override Size MeasureOverride(Size availableSize)
    {
        var children = InternalChildren.Cast<UIElement>().Where(child => child.Visibility != Visibility.Collapsed).ToArray();
        if (children.Length == 0) return new Size();
        var width = double.IsInfinity(availableSize.Width) ? children.Length * (MinItemWidth + Spacing) - Spacing : availableSize.Width;
        var columns = ColumnCount(width, children.Length);
        var itemWidth = Math.Max(0, (width - (columns - 1) * Spacing) / columns);
        foreach (var child in children) child.Measure(new Size(itemWidth, double.PositiveInfinity));
        var height = 0d;
        for (var index = 0; index < children.Length; index += columns)
        {
            if (index > 0) height += Spacing;
            height += children.Skip(index).Take(columns).Max(child => child.DesiredSize.Height);
        }
        return new Size(width, height);
    }

    protected override Size ArrangeOverride(Size finalSize)
    {
        var children = InternalChildren.Cast<UIElement>().Where(child => child.Visibility != Visibility.Collapsed).ToArray();
        if (children.Length == 0) return finalSize;
        var columns = ColumnCount(finalSize.Width, children.Length);
        var itemWidth = Math.Max(0, (finalSize.Width - (columns - 1) * Spacing) / columns);
        var top = 0d;
        for (var index = 0; index < children.Length; index += columns)
        {
            var rowHeight = children.Skip(index).Take(columns).Max(child => child.DesiredSize.Height);
            for (var column = 0; column < columns && index + column < children.Length; column++)
                children[index + column].Arrange(new Rect(column * (itemWidth + Spacing), top, itemWidth, rowHeight));
            top += rowHeight + Spacing;
        }
        return finalSize;
    }

    private int ColumnCount(double width, int count) => Math.Max(1, (int)Math.Min(count, Math.Floor((width + Spacing) / (MinItemWidth + Spacing))));
}
