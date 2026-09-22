using System.Globalization;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Features.Customers;

/// <summary>Projects one Rust-owned customer with the existing synthetic sales-history presentation.</summary>
public sealed class CustomerPreview(Customer customer, IReadOnlyList<CustomerHistoryItem> quotations,
    IReadOnlyList<CustomerHistoryItem> orders) : ObservableObject
{
    public Customer Customer { get; private set; } = customer;
    public Guid Id => Customer.Id;
    public string Name => Customer.Name;
    public string Phone => Customer.Phone;
    public string Address => string.IsNullOrWhiteSpace(Customer.Address) ? "غير مضاف" : Customer.Address;
    public string Notes => string.IsNullOrWhiteSpace(Customer.Notes) ? "لا توجد ملاحظات" : Customer.Notes;
    public IReadOnlyList<CustomerHistoryItem> Quotations { get; } = quotations;
    public IReadOnlyList<CustomerHistoryItem> Orders { get; } = orders;

    public void Observe(Customer value)
    {
        if (value.Id != Customer.Id) throw new ArgumentException("Customer identity cannot change.", nameof(value));
        Customer = value;
        Raise(nameof(Name));
        Raise(nameof(Phone));
        Raise(nameof(Address));
        Raise(nameof(Notes));
    }
}

public static class CustomerHistoryProjection
{
    public static CustomerPreview Create(
        Customer customer,
        IEnumerable<Features.Quotations.QuotationListItem> quotations,
        IEnumerable<Features.Orders.OrderListItem> orders)
    {
        var customerQuotations = quotations
            .Where(row => row.CustomerId == customer.Id || row.CustomerId is null && SameContact(row.Customer, row.Phone, customer))
            .OrderByDescending(row => row.Date)
            .Select(row => new CustomerHistoryItem(row.Number, row.Date,
                string.Join("، ", row.Items.Select(item => item.FurnitureName)), row.StatusLabel, row.FinalTotalLabel))
            .ToArray();
        var customerOrders = orders
            .Where(row => row.CustomerId == customer.Id || row.CustomerId is null && SameContact(row.Customer, row.Phone, customer))
            .OrderByDescending(row => row.Date)
            .Select(row => new CustomerHistoryItem(row.Number, row.Date,
                string.Join("، ", row.Items.Select(item => item.Product)), row.StatusLabel, row.FinalTotalLabel))
            .ToArray();
        return new CustomerPreview(customer, customerQuotations, customerOrders);
    }

    private static bool SameContact(string name, string phone, Customer customer) =>
        string.Equals(name, customer.Name, StringComparison.Ordinal)
        && string.Equals(phone, customer.Phone, StringComparison.Ordinal);
}

/// <summary>Read-only summary of a synthetic quotation or order; no domain calculations.</summary>
public sealed record CustomerHistoryItem(string Number, DateOnly Date, string Items, string Status, string Total)
{
    public string DateLabel => Date.ToString("yyyy/MM/dd", CultureInfo.InvariantCulture);
}
