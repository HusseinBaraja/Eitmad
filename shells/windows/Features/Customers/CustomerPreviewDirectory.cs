using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Features.Customers;

/// <summary>Session-only contacts projected from the existing synthetic reception fixtures.</summary>
public sealed class CustomerPreviewDirectory
{
    private readonly Dictionary<Guid, CustomerPreview> byOrder = [];
    private readonly Dictionary<Guid, CustomerPreview> byQuotation = [];

    public CustomerPreviewDirectory()
    {
        var orders = new OrdersViewModel(true).VisibleOrders.ToArray();
        var quotations = new QuotationsViewModel(true).VisibleQuotations.ToArray();
        // Fixture names establish the initial association only. Edits retain these ID mappings.
        foreach (var name in orders.Select(row => row.Customer).Concat(quotations.Select(row => row.Customer)).Distinct())
        {
            var customerOrders = orders.Where(row => row.Customer == name).OrderByDescending(row => row.Date).ToArray();
            var customerQuotations = quotations.Where(row => row.Customer == name).OrderByDescending(row => row.Date).ToArray();
            var phone = customerOrders.FirstOrDefault()?.Phone ?? customerQuotations[0].Phone;
            var customer = new CustomerPreview(new(name, phone, "عدن — عنوان تجريبي", ""),
                customerQuotations.Select(row => new CustomerHistoryItem(row.Number, row.Date,
                    string.Join("، ", row.Items.Select(item => item.FurnitureName)), row.StatusLabel, row.FinalTotalLabel)).ToArray(),
                customerOrders.Select(row => new CustomerHistoryItem(row.Number, row.Date,
                    string.Join("، ", row.Items.Select(item => item.Product)), row.StatusLabel, row.FinalTotalLabel)).ToArray());
            foreach (var row in customerOrders) byOrder.Add(row.Id, customer);
            foreach (var row in customerQuotations) byQuotation.Add(row.Id, customer);
        }
    }

    public void IncludeQuotation(QuotationListItem quotation)
    {
        var contact = new PreviewCustomer(quotation.Customer, quotation.Phone, quotation.Address, quotation.Notes);
        var history = new CustomerHistoryItem(quotation.Number, quotation.Date,
            string.Join("، ", quotation.Items.Select(item => item.FurnitureName)),
            quotation.StatusLabel, quotation.FinalTotalLabel);
        var customer = byQuotation.GetValueOrDefault(quotation.Id)
            ?? byOrder.Values.Concat(byQuotation.Values).FirstOrDefault(item => item.Name == quotation.Customer)
            ?? new CustomerPreview(contact, [], []);
        customer.IncludeQuotation(contact, history);
        byQuotation[quotation.Id] = customer;
    }

    public CustomerPreview ForOrder(Guid id) => byOrder[id];
    public CustomerPreview ForQuotation(Guid id) => byQuotation[id];
}
