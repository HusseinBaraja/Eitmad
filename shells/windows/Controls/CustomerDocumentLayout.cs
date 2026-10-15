using System.Windows;
using System.Windows.Documents;
using System.Windows.Media;
using Eitmad.WindowsShell.Features.Reception;
using Brushes = System.Windows.Media.Brushes;
using Brush = System.Windows.Media.Brush;
using Application = System.Windows.Application;
using FlowDirection = System.Windows.FlowDirection;
using FontFamily = System.Windows.Media.FontFamily;

namespace Eitmad.WindowsShell.Controls;

// Shared print presentation only; callers supply the customer projection and its saved values.
internal static class CustomerDocumentLayout
{
    internal const string ItemsTag = "CustomerDocumentItems";
    internal static Brush Accent => Application.Current?.TryFindResource("CopperBrush") as Brush ?? Brushes.Black;
    internal static Brush Line => Application.Current?.TryFindResource("LineBrush") as Brush ?? Brushes.LightGray;
    internal static Brush Tint => Application.Current?.TryFindResource("CopperTintBrush") as Brush ?? Brushes.Gainsboro;

    internal static FlowDocument Create(string title, string? number) => new() {
        FlowDirection = FlowDirection.RightToLeft, Language = System.Windows.Markup.XmlLanguage.GetLanguage("ar-YE"),
        FontFamily = new FontFamily("Segoe UI"), FontSize = 14, Foreground = Brushes.Black, Background = Brushes.White,
        PageWidth = 793.7, PageHeight = 1122.5, PagePadding = new Thickness(48, 100, 48, 48),
        ColumnWidth = double.PositiveInfinity, Tag = (title, number),
    };

    internal static void AddIdentity(FlowDocument document, string title, string number, bool numbered, string? notice = null)
    {
        var table = Columns(348.85, 348.85);
        table.Margin = new Thickness(0, 0, 0, 18);
        var row = new TableRow(); table.RowGroups[0].Rows.Add(row);
        var brand = new TableCell { BorderBrush = Accent, BorderThickness = new Thickness(0, 0, 0, 2), Padding = new Thickness(0, 0, 0, 14) };
        var name = Text("الاعتماد"); name.FontSize = 30; name.FontWeight = FontWeights.Bold; name.Foreground = Accent;
        brand.Blocks.Add(name); row.Cells.Add(brand);
        var identity = new TableCell { BorderBrush = Accent, BorderThickness = new Thickness(0, 0, 0, 2), Padding = new Thickness(0, 0, 0, 14) };
        var heading = Text(title); heading.FontSize = 24; heading.FontWeight = FontWeights.Bold; heading.TextAlignment = TextAlignment.Right;
        identity.Blocks.Add(heading);
        var reference = Text(number, numbered); reference.TextAlignment = numbered ? TextAlignment.Left : TextAlignment.Right; reference.Margin = new Thickness(0, 5, 0, 0);
        identity.Blocks.Add(reference); row.Cells.Add(identity);
        document.Blocks.Add(table);
        if (notice is not null) document.Blocks.Add(new Paragraph(new Run(notice)) {
            FontWeight = FontWeights.Bold, Background = Tint, Padding = new Thickness(10),
            Margin = new Thickness(0, 0, 0, 16), KeepWithNext = true,
        });
    }

    internal static void AddDetails(FlowDocument document, IEnumerable<Paragraph> customer, IEnumerable<Paragraph> terms)
    {
        var table = Columns(348.85, 348.85);
        table.Margin = new Thickness(0, 0, 0, 18);
        var row = new TableRow(); table.RowGroups[0].Rows.Add(row);
        var customerCell = new TableCell { Padding = new Thickness(0, 0, 0, 12) };
        customerCell.Blocks.Add(Heading("بيانات العميل"));
        foreach (var field in customer) customerCell.Blocks.Add(field);
        row.Cells.Add(customerCell);
        var termsCell = new TableCell { Padding = new Thickness(20, 0, 0, 12), BorderBrush = Line, BorderThickness = new Thickness(1, 0, 0, 0) };
        termsCell.Blocks.Add(Heading("بيانات المستند"));
        foreach (var field in terms) termsCell.Blocks.Add(field);
        row.Cells.Add(termsCell); document.Blocks.Add(table);
    }

    internal static Table Items()
    {
        var table = Columns(185, 222.7, 60, 115, 115);
        table.Tag = ItemsTag;
        table.Margin = new Thickness(0, 0, 0, 16);
        var header = new TableRow { FontWeight = FontWeights.Bold, Background = Tint };
        foreach (var label in new[] { "الصنف", "الخيارات المحددة", "الكمية", "سعر الوحدة", "الإجمالي" }) header.Cells.Add(Cell(label));
        table.RowGroups[0].Rows.Add(header);
        return table;
    }

    internal static void AddItem(Table table, TableRow row)
    {
        // Keep each cell's complete paragraph chain on the same page. WPF uses the
        // tallest cell to place the row. These are soft constraints: an exceptional
        // cell taller than the printable page can still flow without clipping text.
        foreach (var cell in row.Cells)
            foreach (var paragraph in cell.Blocks.OfType<Paragraph>()) {
                paragraph.KeepTogether = true;
                paragraph.KeepWithNext = paragraph != cell.Blocks.LastBlock;
            }
        table.RowGroups[0].Rows.Add(row);
    }

    internal static void AddTotals(FlowDocument document, params (string Label, string Value)[] values)
    {
        var table = Columns(347.7, 210, 140);
        table.Margin = new Thickness(0, 0, 0, 8);
        var row = new TableRow(); table.RowGroups[0].Rows.Add(row);
        row.Cells.Add(new TableCell(new Paragraph()) { Padding = new Thickness(0) });
        var labels = new TableCell { Padding = new Thickness(0) };
        var amounts = new TableCell { Padding = new Thickness(0) };
        row.Cells.Add(labels); row.Cells.Add(amounts);
        for (var index = 0; index < values.Length; index++) {
            var final = index == values.Length - 1;
            var label = Text(values[index].Label); var amount = Text(values[index].Value, true);
            foreach (var paragraph in new[] { label, amount }) {
                paragraph.KeepWithNext = !final;
                paragraph.Padding = new Thickness(8, final ? 12 : 5, 8, final ? 12 : 5);
                paragraph.BorderThickness = final ? new Thickness(0, 1, 0, 0) : new Thickness(0);
                paragraph.BorderBrush = Accent;
                if (final) { paragraph.Background = Tint; paragraph.FontSize = 18; paragraph.FontWeight = FontWeights.Bold; }
            }
            labels.Blocks.Add(label); amounts.Blocks.Add(amount);
        }
        document.Blocks.Add(table);
    }

    internal static Paragraph Pair(string label, string value, bool ltr = false)
    {
        var paragraph = new Paragraph { Margin = new Thickness(0, 3, 0, 3) };
        paragraph.Inlines.Add(new Run(label + ":  ") { FontWeight = FontWeights.SemiBold });
        if (ltr) paragraph.Inlines.Add(new Span(new Run(value)) { FlowDirection = FlowDirection.LeftToRight });
        else SalesText.Append(paragraph.Inlines, value);
        return paragraph;
    }

    internal static Paragraph Text(string value, bool ltr = false)
    {
        // The enclosing RTL table mirrors its cells. Arabic aligns to the reading start;
        // explicit LTR amounts align to the physical right edge of their column.
        var paragraph = new Paragraph { Margin = new Thickness(0), FlowDirection = ltr ? FlowDirection.LeftToRight : FlowDirection.RightToLeft, TextAlignment = ltr ? TextAlignment.Right : TextAlignment.Left };
        if (ltr) paragraph.Inlines.Add(new Run(value)); else SalesText.Append(paragraph.Inlines, value);
        return paragraph;
    }

    internal static TableCell Cell(string value, bool ltr = false) => new(Text(value, ltr)) {
        Padding = new Thickness(8, 10, 8, 10), BorderBrush = Line, BorderThickness = new Thickness(0, 0, 0, 1),
    };

    private static Paragraph Heading(string value) => new(new Run(value)) {
        FontSize = 15, FontWeight = FontWeights.Bold, Foreground = Accent, Margin = new Thickness(0, 0, 0, 8), KeepWithNext = true,
    };

    private static Table Columns(params double[] widths)
    {
        var table = new Table { CellSpacing = 0 };
        foreach (var width in widths) table.Columns.Add(new TableColumn { Width = new GridLength(width) });
        table.RowGroups.Add(new TableRowGroup());
        return table;
    }

    internal static void FitTables(FlowDocument document)
    {
        // WPF table columns need absolute widths. Retain their proportions when
        // the selected printer changes the content width; keep text at native size.
        var contentWidth = document.PageWidth - document.PagePadding.Left - document.PagePadding.Right;
        foreach (var table in document.Blocks.OfType<Table>()) {
            var availableWidth = contentWidth - table.Margin.Left - table.Margin.Right;
            var totalWidth = table.Columns.Sum(column => column.Width.Value);
            foreach (var column in table.Columns)
                column.Width = new GridLength(availableWidth * column.Width.Value / totalWidth);
        }
    }
}
