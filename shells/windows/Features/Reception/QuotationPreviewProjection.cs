using Eitmad.WindowsShell.Features.Quotations;
using System.Globalization;
using Brushes = System.Windows.Media.Brushes;

namespace Eitmad.WindowsShell.Features.Reception;

// Projects a synthetic editor. A session may attach a temporary publish callback; no durable writes occur.
public static class QuotationPreviewProjection
{
    public static SalesCatalogViewModel Create(QuotationListItem? quotation,
        Features.Furniture.FurnitureViewModel furniture, Features.Products.ProductsViewModel products)
    {
        if (quotation is null) return new SalesCatalogViewModel(furniture, products);
        var model = new SalesCatalogViewModel(furniture, products)
        {
            QuotationNumber = quotation.Number, CustomerName = quotation.Customer,
            Phone = quotation.Phone, Address = quotation.Address, Notes = quotation.Notes, IsReviewingQuotation = true,
        };
        foreach (var line in quotation.Items)
        {
            var item = new SalesCatalogItem(Guid.NewGuid(), line.FurnitureName, "أثاث", "", line.Variant,
                line.UnitPrice, false, line.ThumbnailKind, line.Image);
            if (!line.IsFurniture)
            {
                var variant = new SalesProductVariant(Guid.NewGuid(), line.Variant, line.UnitPrice);
                model.QuotationLines.Add(new(new ProductSelectionViewModel(item,
                    string.IsNullOrEmpty(line.Variant) ? [] : [variant]) { SelectedVariant = variant, Quantity = line.Quantity }));
                continue;
            }
            var size = new SalesSize(Guid.NewGuid(), line.Variant, line.Dimensions, line.UnitPrice);
            var color = new SalesOption(Guid.NewGuid(), line.Color, 0, Brushes.Transparent);
            var handle = new SalesOption(Guid.NewGuid(), line.Handle, 0, Brushes.Transparent);
            model.QuotationLines.Add(new(new FurnitureSelectionViewModel(item, [size], [color], [handle])
            {
                SelectedSize = size, SelectedColor = color, SelectedHandle = handle, Quantity = line.Quantity,
            }));
        }
        model.DiscountInput = (quotation.Subtotal == 0 ? 0 : quotation.Discount / quotation.Subtotal * 100m)
            .ToString(CultureInfo.InvariantCulture);
        model.PreviewId = quotation.Id;
        model.ObserveApproval(quotation);
        return model;
    }
}
