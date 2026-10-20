using System.Globalization;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Features.Customers;

public sealed class CustomerPreview(Customer customer, IReadOnlyList<CustomerHistoryItem> quotations,
    IReadOnlyList<CustomerHistoryItem> orders, bool readOnly = false) : ObservableObject
{
    public Customer Customer { get; private set; } = customer;
    public Guid Id => Customer.Id;
    public string Name => Customer.Name;
    public string Phone => Customer.Phone;
    public string Address => string.IsNullOrWhiteSpace(Customer.Address) ? "غير مضاف" : Customer.Address;
    public string Notes => string.IsNullOrWhiteSpace(Customer.Notes) ? "لا توجد ملاحظات" : Customer.Notes;
    public string SyncStateLabel => Customer.SyncState switch
    {
        SyncState.Pending => "بانتظار المزامنة",
        SyncState.Confirmed => "مؤكد من الخادم",
        SyncState.Rejected => "رُفضت المزامنة",
        SyncState.Conflicted => "تعارض يحتاج مراجعة",
        _ => "حالة المزامنة غير متاحة",
    };
    public bool ReadOnly => readOnly;
    public string BranchId => Customer.Scope.Id.ToString();
    public bool CanEdit => !readOnly && Customer.SyncState is not (SyncState.Rejected or SyncState.Conflicted);
    public bool HasSyncConflict => Customer.SyncState == SyncState.Conflicted;
    public IReadOnlyList<CustomerHistoryItem> Quotations { get; } = quotations;
    public IReadOnlyList<CustomerHistoryItem> Orders { get; } = orders;
    public CustomerHistoryPage? HistoryPage { get; internal set; }
    public bool HasMoreHistory => HistoryPage?.QuotationsNext is not null || HistoryPage?.OrdersNext is not null;
    public string HistoryNotice => HasMoreHistory ? "السجل جزئي. تابع إلى صفحة السجل التالية."
        : HistoryPage is { ServerAvailable: false } ? "آخر سجل محفوظ؛ الاتصال بالخادم غير متاح." : "";

    public void Observe(Customer value)
    {
        if (value.Id != Customer.Id) throw new ArgumentException("Customer identity cannot change.", nameof(value));
        Customer = value;
        Raise(nameof(Name));
        Raise(nameof(Phone));
        Raise(nameof(Address));
        Raise(nameof(Notes));
        Raise(nameof(SyncStateLabel));
        Raise(nameof(CanEdit));
        Raise(nameof(HasSyncConflict));
    }
}

public static class CustomerHistoryProjection
{
    public static CustomerPreview FromPage(Customer customer, CustomerHistoryPage page, bool readOnly)
    {
        var result = Create(customer,
        page.Quotations.Select(q => Features.Quotations.QuotationsViewModel.Project(new QuotationDraft {
            Scope = q.Scope, Snapshot = q.Quotation, UpdatedAt = q.ChangedAt, SyncState = SyncState.Confirmed,
        }, lifecycle: q)), page.Orders.Select(Features.Orders.OrdersViewModel.Project), readOnly);
        result.HistoryPage = page;
        return result;
    }

    public static CustomerPreview Create(
        Customer customer,
        IEnumerable<Features.Quotations.QuotationListItem> quotations,
        IEnumerable<Features.Orders.OrderListItem> orders, bool readOnly = false)
    {
        var customerQuotations = quotations
            .Where(row => row.CustomerId == customer.Id && (row.Lifecycle?.Scope ?? row.Draft?.Scope) is { } scope && scope.Id == customer.Scope.Id)
            .OrderByDescending(row => row.Date)
            .Select(row => new CustomerHistoryItem(row.Number, row.Date,
                string.Join("، ", row.Items.Select(item => item.FurnitureName)), row.StatusLabel, row.FinalTotalLabel))
            .ToArray();
        var customerOrders = orders
            .Where(row => row.CustomerId == customer.Id && row.Record?.Scope.Id == customer.Scope.Id)
            .OrderByDescending(row => row.Date)
            .Select(row => new CustomerHistoryItem(row.Number, row.Date,
                string.Join("، ", row.Items.Select(item => item.Product)), row.StatusLabel, row.FinalTotalLabel))
            .ToArray();
        return new CustomerPreview(customer, customerQuotations, customerOrders, readOnly);
    }

}

public sealed record CustomerHistoryItem(string Number, DateOnly Date, string Items, string Status, string Total)
{
    public string DateLabel => Date.ToString("yyyy/MM/dd", CultureInfo.InvariantCulture);
}
