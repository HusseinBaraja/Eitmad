using FlowDirection = System.Windows.FlowDirection;
using FontFamily = System.Windows.Media.FontFamily;
using Brushes = System.Windows.Media.Brushes;
using System.Globalization;
using System.Windows;
using System.Windows.Documents;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Features.Reception;

public static class QuotationCustomerDocument
{
    // Explicit customer-only projection. Never render the editor or its DataContext for printing.
    public static FlowDocument Create(SalesCatalogViewModel model, DateTime date)
    {
        if (!model.CanPreviewCustomer || !model.CheckRequiredFields())
            throw new InvalidOperationException("Quotation preview is incomplete.");
        return CreateExistingPreview(model, date);
    }

    // Only for the synthetic, already-reviewed quotation projection.
    public static FlowDocument CreateExistingPreview(SalesCatalogViewModel model, DateTime date)
    {
        var document = new FlowDocument
        {
            FlowDirection = FlowDirection.RightToLeft,
            Language = System.Windows.Markup.XmlLanguage.GetLanguage("ar-YE"),
            FontFamily = new FontFamily("Segoe UI"), FontSize = 14,
            Foreground = Brushes.Black, Background = Brushes.White,
            PageWidth = 793.7, PageHeight = 1122.5, PagePadding = new Thickness(48),
            ColumnWidth = double.PositiveInfinity,
        };
        document.Blocks.Add(new Paragraph(new Run("الاعتماد")) { FontSize = 32, FontWeight = FontWeights.Bold, Margin = new Thickness(0, 0, 0, 4) });
        document.Blocks.Add(new Paragraph(new Run("للأثاث والمفروشات · عنوان الشركة التجريبي · 000000000")) { Margin = new Thickness(0, 0, 0, 20) });
        document.Blocks.Add(new Paragraph(new Run("عرض سعر")) { FontSize = 24, FontWeight = FontWeights.Bold });
        document.Blocks.Add(new Paragraph(new Run("نسخة تجريبية — غير محفوظة")) { FontSize = 12 });
        document.Blocks.Add(Pair("رقم عرض السعر", string.IsNullOrWhiteSpace(model.QuotationNumber) ? "لم يُعيّن بعد" : model.QuotationNumber, !string.IsNullOrWhiteSpace(model.QuotationNumber)));
        document.Blocks.Add(Pair("التاريخ", date.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture), true));
        document.Blocks.Add(Pair("العميل", model.CustomerName));
        document.Blocks.Add(Pair("رقم الهاتف", model.Phone, true));
        if (!string.IsNullOrWhiteSpace(model.Address)) document.Blocks.Add(Pair("العنوان", model.Address));
        var table = new Table { CellSpacing = 0, Margin = new Thickness(0, 24, 0, 20) };
        foreach (var width in new[] { 150d, 215d, 55d, 90d, 90d }) table.Columns.Add(new TableColumn { Width = new GridLength(width) });
        var rows = new TableRowGroup(); table.RowGroups.Add(rows);
        var header = new TableRow { FontWeight = FontWeights.Bold, Background = Brushes.Gainsboro };
        foreach (var label in new[] { "الصنف", "الخيارات المحددة", "الكمية", "سعر الوحدة", "الإجمالي" }) header.Cells.Add(Cell(label));
        rows.Rows.Add(header);
        foreach (var line in model.QuotationLines)
        {
            var row = new TableRow();
            row.Cells.Add(Cell(line.Name)); row.Cells.Add(Cell(line.Options.Length == 0 ? "—" : line.Options));
            row.Cells.Add(Cell(line.Quantity.ToString(CultureInfo.InvariantCulture), true));
            row.Cells.Add(Cell(Money(line.UnitPrice), true)); row.Cells.Add(Cell(Money(line.LineTotal), true));
            rows.Rows.Add(row);
        }
        document.Blocks.Add(table);
        document.Blocks.Add(Pair("المجموع الفرعي", Money(model.Subtotal), true));
        document.Blocks.Add(Pair("الخصم", Money(model.Discount), true));
        var total = Pair("الإجمالي النهائي", Money(model.FinalTotal), true);
        total.FontSize = 22; total.FontWeight = FontWeights.Bold;
        total.BorderBrush = Brushes.Black; total.BorderThickness = new Thickness(0, 1, 0, 0); total.Padding = new Thickness(0, 12, 0, 0);
        document.Blocks.Add(total);
        return document;
    }
    private static string Money(decimal value) => value.ToString("N0", CultureInfo.InvariantCulture) + " YER";
    private static Paragraph Pair(string label, string value, bool ltr = false)
    {
        var paragraph = new Paragraph { Margin = new Thickness(0, 4, 0, 4) };
        paragraph.Inlines.Add(new Run(label + ":  ") { FontWeight = FontWeights.SemiBold });
        paragraph.Inlines.Add(new Span(new Run(value)) { FlowDirection = ltr ? FlowDirection.LeftToRight : FlowDirection.RightToLeft });
        return paragraph;
    }
    private static TableCell Cell(string value, bool ltr = false) => new(new Paragraph(new Run(value))
    {
        Margin = new Thickness(0), FlowDirection = ltr ? FlowDirection.LeftToRight : FlowDirection.RightToLeft,
        TextAlignment = TextAlignment.Right,
    }) { Padding = new Thickness(6, 10, 6, 10), BorderBrush = Brushes.LightGray, BorderThickness = new Thickness(0, 0, 0, 1) };
}
