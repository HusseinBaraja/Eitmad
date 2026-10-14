using System.Windows.Documents;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Quotations;
using static Eitmad.WindowsShell.Controls.CustomerDocumentLayout;

namespace Eitmad.WindowsShell.Features.Orders;

// A customer-only print projection of synthetic fixtures, never a screenshot of the shell.
public static class OrderCustomerDocument
{
    public static FlowDocument CreateSaved(Eitmad.Contracts.CustomerDocument saved, bool source = false) =>
        Features.Reception.QuotationCustomerDocument.CreateSaved(saved, source ? "عرض السعر الأصلي" : "الطلب");

    public static FlowDocument Create(OrderListItem order) => Build(order, "الطلب", true);

    public static FlowDocument CreateQuotation(QuotationListItem quotation) => Build(new(
        quotation.Id, quotation.Number, quotation.Customer, quotation.Date, OrderStatus.New,
        quotation.Discount, quotation.Items.Select(line => new OrderLineItem(line.FurnitureName,
            line.Variant, line.Dimensions, line.Color, line.Handle, line.Quantity, line.UnitPrice,
            line.IsFurniture, line.ThumbnailKind, line.Image)).ToArray(), quotation.Phone),
        "عرض السعر الأصلي", false);

    private static FlowDocument Build(OrderListItem order, string title, bool showStatus)
    {
        if (order.Record is not null) throw new InvalidOperationException("A saved Rust document is required.");
        var document = CustomerDocumentLayout.Create(title, order.Number);
        AddIdentity(document, title, order.Number, true, "بيانات تجريبية للمعاينة فقط");
        var terms = new List<Paragraph> { Pair("التاريخ", order.DateLabel, true) };
        if (showStatus) terms.Add(Pair("الحالة", order.IsReady ? "الطلب جاهز" : order.StatusLabel));
        AddDetails(document, [Pair("العميل", order.Customer)], terms);
        var table = Items();
        foreach (var line in order.Items) {
            var row = new TableRow(); row.Cells.Add(Cell(line.Product));
            var options = Cell(line.Variant);
            if (line.IsFurniture) {
                if (line.Color.Length > 0) options.Blocks.Add(Text(line.Color));
                if (line.Handle.Length > 0) options.Blocks.Add(Text(line.Handle));
                if (line.Dimensions.Length > 0) options.Blocks.Add(Text(line.Dimensions, true));
            }
            row.Cells.Add(options); row.Cells.Add(Cell(line.QuantityLabel, true));
            row.Cells.Add(Cell(line.SellingPriceLabel, true)); row.Cells.Add(Cell(line.Total.ToString("N0", System.Globalization.CultureInfo.InvariantCulture) + " ر.ي", true));
            AddItem(table, row);
        }
        document.Blocks.Add(table);
        AddTotals(document, ("المجموع الفرعي", order.SubtotalLabel), ("الخصم", order.DiscountLabel), ("الإجمالي النهائي", order.FinalTotalLabel));
        return document;
    }
}
