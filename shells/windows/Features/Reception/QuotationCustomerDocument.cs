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
    public static FlowDocument CreateSaved(Eitmad.Contracts.CustomerDocument saved, string title = "عرض سعر")
    {
        var document = new FlowDocument {
            FlowDirection = FlowDirection.RightToLeft, Language = System.Windows.Markup.XmlLanguage.GetLanguage("ar-YE"),
            FontFamily = new FontFamily("Segoe UI"), FontSize = 14, Foreground = Brushes.Black, Background = Brushes.White,
            PageWidth = 793.7, PageHeight = 1122.5, PagePadding = new Thickness(48, 100, 48, 48), ColumnWidth = double.PositiveInfinity,
            Tag = (title, saved.Number),
        };
        document.Blocks.Add(new Paragraph(new Run("الاعتماد — " + title)) { FontSize = 24, FontWeight = FontWeights.Bold, KeepWithNext = true });
        if (saved.IsDraft) document.Blocks.Add(new Paragraph(new Run("مسودة محفوظة — غير صادرة، غير صالحة للطباعة أو التصدير")) { FontWeight = FontWeights.Bold, KeepWithNext = true });
        document.Blocks.Add(Pair("الرقم", saved.Number ?? "غير مرقم", saved.Number is not null));
        document.Blocks.Add(Pair("الإصدار", saved.DocumentRevision.ToString(CultureInfo.InvariantCulture), true));
        document.Blocks.Add(Pair("الحالة", saved.Status));
        document.Blocks.Add(Pair("تاريخ الحفظ", Date(saved.SavedAt), true));
        if (saved.IssuedAt is { } issuedAt) document.Blocks.Add(Pair("تاريخ الإصدار", Date(issuedAt), true));
        if (saved.ValidityDays is { } days) document.Blocks.Add(Pair("مدة الصلاحية بالأيام", days.ToString(CultureInfo.InvariantCulture), true));
        if (saved.ValidUntil is { } until) document.Blocks.Add(Pair("صالح حتى", Date(until), true));
        document.Blocks.Add(Pair("العميل", saved.Customer.Name));
        document.Blocks.Add(Pair("رقم الهاتف", saved.Customer.Phone, true));
        if (!string.IsNullOrEmpty(saved.Customer.Address)) document.Blocks.Add(Pair("العنوان", saved.Customer.Address));
        var table = new Table { CellSpacing = 0, Margin = new Thickness(0, 20, 0, 16) };
        foreach (var width in new[] { 150d, 210d, 55d, 90d, 92d }) table.Columns.Add(new TableColumn { Width = new GridLength(width) });
        var rows = new TableRowGroup(); table.RowGroups.Add(rows);
        var header = new TableRow { FontWeight = FontWeights.Bold, Background = Brushes.Gainsboro };
        foreach (var label in new[] { "الصنف", "الخيارات المحددة", "الكمية", "سعر الوحدة", "الإجمالي" }) header.Cells.Add(Cell(label));
        rows.Rows.Add(header);
        foreach (var line in saved.Lines) {
            var row = new TableRow();
            var item = Cell(line.Name);
            if (!string.IsNullOrEmpty(line.Description)) item.Blocks.Add(TextParagraph(line.Description));
            row.Cells.Add(item);
            var options = Cell(line.VariantName);
            if (!string.IsNullOrEmpty(line.ColorName)) options.Blocks.Add(TextParagraph(line.ColorName));
            if (!string.IsNullOrEmpty(line.HandleName)) options.Blocks.Add(TextParagraph(line.HandleName));
            if (line.Dimensions is { } d) options.Blocks.Add(TextParagraph(SalesCatalogViewModel.DimensionsLabel(d), true));
            row.Cells.Add(options);
            row.Cells.Add(Cell(line.Quantity.ToString(CultureInfo.InvariantCulture), true));
            row.Cells.Add(Cell(Money(line.UnitPriceYer), true)); row.Cells.Add(Cell(Money(line.TotalYer), true)); rows.Rows.Add(row);
        }
        document.Blocks.Add(table);
        var totals = new Section();
        totals.Blocks.Add(Pair("المجموع الفرعي", Money(saved.SubtotalYer), true));
        totals.Blocks.Add(Pair("نسبة الخصم", (saved.DiscountBasisPoints / 100m).ToString("0.00", CultureInfo.InvariantCulture) + "%", true));
        totals.Blocks.Add(Pair("الخصم", Money(saved.DiscountYer), true));
        var total = Pair("الإجمالي النهائي", Money(saved.TotalYer), true); total.FontSize = 20; total.FontWeight = FontWeights.Bold;
        totals.Blocks.Add(total);
        foreach (Paragraph paragraph in totals.Blocks) paragraph.KeepWithNext = paragraph != total;
        document.Blocks.Add(totals);
        return document;
    }
    private static string Date(long value) => DateTimeOffset.FromUnixTimeMilliseconds(value).ToOffset(TimeSpan.FromHours(3)).ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);
    private static Paragraph TextParagraph(string value, bool ltr = false)
    {
        var paragraph = new Paragraph { Margin = new Thickness(0), FlowDirection = ltr ? FlowDirection.LeftToRight : FlowDirection.RightToLeft, TextAlignment = TextAlignment.Right };
        if (ltr) paragraph.Inlines.Add(new Run(value)); else SalesText.Append(paragraph.Inlines, value);
        return paragraph;
    }
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
        if (model.IsLiveQuotation) throw new InvalidOperationException("A saved Rust document is required.");
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
    private static string Money(decimal value) => value.ToString("N0", CultureInfo.InvariantCulture) + " ر.ي";
    private static Paragraph Pair(string label, string value, bool ltr = false)
    {
        var paragraph = new Paragraph { Margin = new Thickness(0, 4, 0, 4) };
        paragraph.Inlines.Add(new Run(label + ":  ") { FontWeight = FontWeights.SemiBold });
        if (ltr) paragraph.Inlines.Add(new Span(new Run(value)) { FlowDirection = FlowDirection.LeftToRight });
        else SalesText.Append(paragraph.Inlines, value);
        return paragraph;
    }
    private static TableCell Cell(string value, bool ltr = false) => new(TextParagraph(value, ltr))
    { Padding = new Thickness(6, 10, 6, 10), BorderBrush = Brushes.LightGray, BorderThickness = new Thickness(0, 0, 0, 1) };
}
