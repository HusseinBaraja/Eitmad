using FlowDirection = System.Windows.FlowDirection;
using System.Windows;
using System.Windows.Documents;
using Eitmad.WindowsShell.Features.Quotations;
using Brushes = System.Windows.Media.Brushes;

namespace Eitmad.WindowsShell.Features.Orders;

// A customer-only print projection of synthetic fixtures, never a screenshot of the shell.
public static class OrderCustomerDocument
{
    public static FlowDocument Create(OrderListItem order) => Build(order, "الطلب", true);

    public static FlowDocument CreateQuotation(QuotationListItem quotation) => Build(new(
        quotation.Id, quotation.Number, quotation.Customer, quotation.Date, OrderStatus.New,
        quotation.Discount, quotation.Items.Select(line => new OrderLineItem(line.FurnitureName,
            line.Variant, line.Dimensions, line.Color, line.Handle, line.Quantity, line.UnitPrice,
            line.IsFurniture, line.ThumbnailKind, line.Image)).ToArray(), quotation.Phone),
        "عرض السعر الأصلي", false);

    private static FlowDocument Build(OrderListItem order, string title, bool showStatus)
    {
        var document = new FlowDocument
        {
            FlowDirection = FlowDirection.RightToLeft,
            Language = System.Windows.Markup.XmlLanguage.GetLanguage("ar-YE"),
            FontFamily = new System.Windows.Media.FontFamily("Segoe UI"), FontSize = 14,
            Foreground = Brushes.Black, Background = Brushes.White,
            PageWidth = 793.7, PageHeight = 1122.5, PagePadding = new Thickness(48),
            ColumnWidth = double.PositiveInfinity,
        };
        document.Blocks.Add(new Paragraph(new Run(title)) { FontSize = 28, FontWeight = FontWeights.Bold });
        Add("بيانات تجريبية للمعاينة فقط", "");
        Add("الرقم", order.Number, true);
        Add("العميل", order.Customer);
        Add("التاريخ", order.DateLabel, true);
        if (showStatus) Add("الحالة", order.IsReady ? "الطلب جاهز" : order.StatusLabel);
        foreach (var line in order.Items)
        {
            document.Blocks.Add(new Paragraph(new Run(line.Product))
                { FontSize = 19, FontWeight = FontWeights.SemiBold, KeepWithNext = true });
            Add("الخيار", line.Variant);
            if (line.IsFurniture)
            {
                if (line.Dimensions.Length > 0) Add("المقاسات", line.Dimensions, true);
                Add("اللون", line.Color);
                Add("المقبض", line.Handle);
            }
            Add("الكمية", line.QuantityLabel, true);
            Add("سعر البيع", line.SellingPriceLabel, true);
        }
        Add("المجموع الفرعي", order.SubtotalLabel, true);
        Add("الخصم", order.DiscountLabel, true);
        Add("الإجمالي النهائي", order.FinalTotalLabel, true);
        return document;

        void Add(string label, string value, bool ltr = false)
        {
            var paragraph = new Paragraph { Margin = new Thickness(0, 4, 0, 4) };
            paragraph.Inlines.Add(new Run(label + "  ") { FontWeight = FontWeights.SemiBold });
            paragraph.Inlines.Add(new Span(new Run(value))
                { FlowDirection = ltr ? FlowDirection.LeftToRight : FlowDirection.RightToLeft });
            document.Blocks.Add(paragraph);
        }
    }
}
