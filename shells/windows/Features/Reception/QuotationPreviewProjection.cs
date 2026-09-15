using Eitmad.WindowsShell.Features.Quotations;
using System.Globalization;
using Brushes = System.Windows.Media.Brushes;

namespace Eitmad.WindowsShell.Features.Reception;

// Creates a detached editor for synthetic list fixtures. It never writes back to a quotation.
public static class QuotationPreviewProjection
{
    public static SalesCatalogViewModel Create(QuotationListItem? quotation,
        Features.Furniture.FurnitureViewModel furniture, Features.Products.ProductsViewModel products)
    {
        if (quotation is null) return new SalesCatalogViewModel(furniture, products) { IsReviewingQuotation = true };
        var model = new SalesCatalogViewModel(furniture, products)
        {
            QuotationNumber = quotation.Number, CustomerName = quotation.Customer,
            Phone = quotation.Phone, IsReviewingQuotation = true,
        };
        foreach (var line in quotation.Items)
        {
            var item = new SalesCatalogItem(Guid.NewGuid(), line.FurnitureName, "أثاث", "", line.Variant,
                line.UnitPrice, false, "wardrobe", null);
            var size = new SalesSize(Guid.NewGuid(), line.Variant, "", line.UnitPrice);
            var color = new SalesOption(Guid.NewGuid(), line.Color, 0, Brushes.Transparent);
            var handle = new SalesOption(Guid.NewGuid(), line.Handle, 0, Brushes.Transparent);
            model.QuotationLines.Add(new(new FurnitureSelectionViewModel(item, [size], [color], [handle])
            {
                SelectedSize = size, SelectedColor = color, SelectedHandle = handle, Quantity = line.Quantity,
            }));
        }
        model.DiscountInput = (quotation.Subtotal == 0 ? 0 : quotation.Discount / quotation.Subtotal * 100m)
            .ToString(CultureInfo.InvariantCulture);
        return model;
    }
}
