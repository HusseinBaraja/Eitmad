using System.Globalization;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Features.Customers;

/// <summary>Transient customer presentation. History stays attached when contact details change.</summary>
public sealed class CustomerPreview(PreviewCustomer contact, IReadOnlyList<CustomerHistoryItem> quotations,
    IReadOnlyList<CustomerHistoryItem> orders) : ObservableObject
{
    public Guid Id { get; } = Guid.NewGuid();
    public PreviewCustomer Contact { get; private set; } = contact;
    public string Name => Contact.Name;
    public string Phone => Contact.Phone;
    public string Address => string.IsNullOrWhiteSpace(Contact.Address) ? "غير مضاف" : Contact.Address;
    public string Notes => string.IsNullOrWhiteSpace(Contact.Notes) ? "لا توجد ملاحظات" : Contact.Notes;
    public IReadOnlyList<CustomerHistoryItem> Quotations { get; private set; } = quotations;
    public IReadOnlyList<CustomerHistoryItem> Orders { get; } = orders;

    public void ApplyPreview(PreviewCustomer value)
    {
        Contact = value;
        Raise(nameof(Name));
        Raise(nameof(Phone));
        Raise(nameof(Address));
        Raise(nameof(Notes));
    }

    public void IncludeQuotation(PreviewCustomer value, CustomerHistoryItem quotation)
    {
        ApplyPreview(value);
        Quotations = [quotation, .. Quotations.Where(item => item.Number != quotation.Number)];
        Raise(nameof(Quotations));
    }
}

/// <summary>Read-only summary of a synthetic quotation or order; no domain calculations.</summary>
public sealed record CustomerHistoryItem(string Number, DateOnly Date, string Items, string Status, string Total)
{
    public string DateLabel => Date.ToString("yyyy/MM/dd", CultureInfo.InvariantCulture);
}
