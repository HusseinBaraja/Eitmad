using System.Globalization;
using System.Windows;
using System.Windows.Documents;
using Eitmad.WindowsShell.Controls;
using static Eitmad.WindowsShell.Controls.CustomerDocumentLayout;

namespace Eitmad.WindowsShell.Features.Reception;

public static class QuotationCustomerDocument
{
    public static FlowDocument CreateSaved(Eitmad.Contracts.CustomerDocument saved, string title = "عرض سعر")
    {
        var document = CustomerDocumentLayout.Create(title, saved.Number);
        AddIdentity(document, title, saved.Number ?? "غير مرقم", saved.Number is not null,
            saved.IsDraft ? "مسودة محفوظة — غير صادرة، غير صالحة للطباعة أو التصدير" : null);
        var terms = new List<Paragraph> {
            Pair("الإصدار", saved.DocumentRevision.ToString(CultureInfo.InvariantCulture), true),
            Pair("الحالة", saved.Status), Pair("تاريخ الحفظ", Date(saved.SavedAt), true),
        };
        if (saved.IssuedAt is { } issuedAt) terms.Add(Pair("تاريخ الإصدار", Date(issuedAt), true));
        if (saved.ValidityDays is { } days) terms.Add(Pair("مدة الصلاحية بالأيام", days.ToString(CultureInfo.InvariantCulture), true));
        if (saved.ValidUntil is { } until) terms.Add(Pair("صالح حتى", Date(until), true));
        var customer = new List<Paragraph> { Pair("العميل", saved.Customer.Name), Pair("رقم الهاتف", saved.Customer.Phone, true) };
        if (!string.IsNullOrEmpty(saved.Customer.Address)) customer.Add(Pair("العنوان", saved.Customer.Address));
        AddDetails(document, customer, terms);
        var table = Items();
        foreach (var line in saved.Lines) {
            var row = new TableRow();
            var item = Cell(line.Name);
            item.Blocks.FirstBlock.FontWeight = FontWeights.SemiBold;
            if (!string.IsNullOrEmpty(line.Description)) item.Blocks.Add(Text(line.Description));
            row.Cells.Add(item);
            var options = Cell(line.VariantName);
            if (!string.IsNullOrEmpty(line.ColorName)) options.Blocks.Add(Text(line.ColorName));
            if (!string.IsNullOrEmpty(line.HandleName)) options.Blocks.Add(Text(line.HandleName));
            if (line.Dimensions is { } dimensions) options.Blocks.Add(Text(SalesCatalogViewModel.DimensionsLabel(dimensions), true));
            row.Cells.Add(options);
            row.Cells.Add(Cell(line.Quantity.ToString(CultureInfo.InvariantCulture), true));
            row.Cells.Add(Cell(Money(line.UnitPriceYer), true)); row.Cells.Add(Cell(Money(line.TotalYer), true));
            AddItem(table, row);
        }
        document.Blocks.Add(table);
        AddTotals(document, ("المجموع الفرعي", Money(saved.SubtotalYer)),
            ("نسبة الخصم", (saved.DiscountBasisPoints / 100m).ToString("0.00", CultureInfo.InvariantCulture) + "%"),
            ("الخصم", Money(saved.DiscountYer)), ("الإجمالي النهائي", Money(saved.TotalYer)));
        return document;
    }

    private static string Date(long value) => DateTimeOffset.FromUnixTimeMilliseconds(value).ToOffset(TimeSpan.FromHours(3)).ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);

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
        var numbered = !string.IsNullOrWhiteSpace(model.QuotationNumber);
        var document = CustomerDocumentLayout.Create("عرض سعر", numbered ? model.QuotationNumber : null);
        AddIdentity(document, "عرض سعر", numbered ? model.QuotationNumber : "لم يُعيّن بعد", numbered, "نسخة تجريبية — غير محفوظة");
        var customer = new List<Paragraph> { Pair("العميل", model.CustomerName), Pair("رقم الهاتف", model.Phone, true) };
        if (!string.IsNullOrWhiteSpace(model.Address)) customer.Add(Pair("العنوان", model.Address));
        AddDetails(document, customer, [Pair("التاريخ", date.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture), true)]);
        var table = Items();
        foreach (var line in model.QuotationLines) {
            var row = new TableRow();
            row.Cells.Add(Cell(line.Name)); row.Cells.Add(Cell(line.Options.Length == 0 ? "—" : line.Options));
            row.Cells.Add(Cell(line.Quantity.ToString(CultureInfo.InvariantCulture), true));
            row.Cells.Add(Cell(Money(line.UnitPrice), true)); row.Cells.Add(Cell(Money(line.LineTotal), true));
            AddItem(table, row);
        }
        document.Blocks.Add(table);
        AddTotals(document, ("المجموع الفرعي", Money(model.Subtotal)), ("الخصم", Money(model.Discount)), ("الإجمالي النهائي", Money(model.FinalTotal)));
        return document;
    }

    private static string Money(decimal value) => value.ToString("N0", CultureInfo.InvariantCulture) + " ر.ي";
}
