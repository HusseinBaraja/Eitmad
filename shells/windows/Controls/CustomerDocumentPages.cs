using System.Globalization;
using System.Windows;
using System.Windows.Documents;
using System.Windows.Media;
using Size = System.Windows.Size;
using Point = System.Windows.Point;
using Brushes = System.Windows.Media.Brushes;
using FlowDirection = System.Windows.FlowDirection;

namespace Eitmad.WindowsShell.Controls;

// One native paginator serves the preview and Windows print/export routes.
public sealed class CustomerDocumentPages : DocumentPaginator, IDocumentPaginatorSource
{
    private readonly FlowDocument document;
    private readonly DocumentPaginator inner;
    private readonly Table? table;
    public CustomerDocumentPages(FlowDocument document)
    {
        this.document = document;
        inner = ((IDocumentPaginatorSource)document).DocumentPaginator;
        table = document.Blocks.OfType<Table>().FirstOrDefault();
        inner.ComputePageCount();
    }
    public DocumentPaginator DocumentPaginator => this;
    public override IDocumentPaginatorSource Source => this;
    public override bool IsPageCountValid => inner.IsPageCountValid;
    public override int PageCount => inner.PageCount;
    public override Size PageSize { get => inner.PageSize; set => inner.PageSize = value; }
    public override void ComputePageCount() => inner.ComputePageCount();
    public override DocumentPage GetPage(int pageNumber)
    {
        var page = inner.GetPage(pageNumber);
        if (page == DocumentPage.Missing) return page;
        var visual = new ContainerVisual();
        if (VisualTreeHelper.GetParent(page.Visual) is ContainerVisual previous) previous.Children.Remove(page.Visual);
        visual.Children.Add(page.Visual);
        var decorations = new DrawingVisual();
        using (var drawing = decorations.RenderOpen()) {
            if (document.Tag is ValueTuple<string, string?> heading) {
                DrawText(drawing, "الاعتماد · " + heading.Item1, new Point(page.Size.Width - 48, 20), 12);
                if (heading.Item2 is { } number) DrawText(drawing, number, new Point(48, 20), 12, FlowDirection.LeftToRight);
            } else DrawText(drawing, "الاعتماد", new Point(page.Size.Width - 48, 20), 12);
            if (pageNumber > 0 && table is not null && inner is DynamicDocumentPaginator dynamicPages && pageNumber <= dynamicPages.GetPageNumber(table.ContentEnd)) {
                var right = page.Size.Width - document.PagePadding.Right;
                var headers = table.RowGroups[0].Rows[0].Cells;
                for (var index = 0; index < table.Columns.Count; index++) {
                    var width = table.Columns[index].Width.Value;
                    drawing.DrawRectangle(Brushes.Gainsboro, null, new Rect(right - width, 60, width, 30));
                    DrawText(drawing, new TextRange(headers[index].ContentStart, headers[index].ContentEnd).Text.Trim(), new Point(right - 6, 66), 12);
                    right -= width;
                }
            }
            DrawText(drawing, "صفحة", new Point(page.Size.Width - 48, page.Size.Height - 32), 12);
            DrawText(drawing, $"{pageNumber + 1} / {PageCount}", new Point(page.Size.Width - 130, page.Size.Height - 32), 12, FlowDirection.LeftToRight);
        }
        visual.Children.Add(decorations);
        return new DocumentPage(visual, page.Size, page.BleedBox, page.ContentBox);
    }
    private static void DrawText(DrawingContext drawing, string value, Point origin, double size, FlowDirection direction = FlowDirection.RightToLeft)
    {
        var text = new FormattedText(value, CultureInfo.InvariantCulture, direction,
            new Typeface("Segoe UI"), size, Brushes.Black, 1);
        drawing.DrawText(text, origin);
    }
}
