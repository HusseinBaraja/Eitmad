using System.Collections.ObjectModel;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Features.Customers;

public sealed class CustomerDirectoryViewModel : ObservableObject
{
    private readonly CustomerClient client;
    private readonly EngineChangeFeed[] historyFeeds;
    private CancellationTokenSource? pending;
    private long generation;
    private bool active;
    private string term = "", notice = "";
    private CustomerCursor? after, next;
    private CustomerPreview? selected;
    private Customer? openCustomer;
    private CustomerHistoryPage? history;
    public CustomerDirectoryViewModel(CustomerClient client, IEngineShellBridge engine)
    {
        this.client = client;
        client.Changed += CustomerChanged;
        historyFeeds = [
            new(engine, ProtocolIds.Capabilities.EitmadCapabilityOrdersV1, Subscription.ForOrderChangedSubscribe(new()), _ => Refresh(), Refresh, refreshOnStart: false),
            new(engine, ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1, Subscription.ForQuotationChangedSubscribe(new()), _ => Refresh(), Refresh, refreshOnStart: false),
        ];
    }
    public ObservableCollection<Customer> Rows { get; } = [];
    public string Term { get => term; set => Set(ref term, value); }
    public string Notice { get => notice; private set => Set(ref notice, value); }
    public CustomerPreview? Selected { get => selected; private set { Set(ref selected, value); Raise(nameof(HasDetail)); } }
    public bool HasDetail => Selected is not null;
    public bool HasNext => next is not null;
    public bool HasMoreHistory => history?.QuotationsNext is not null || history?.OrdersNext is not null;
    public Task LastLoad { get; private set; } = Task.CompletedTask;
    public async Task ActivateAsync()
    {
        active = true;
        var activation = ++generation;
        foreach (var feed in historyFeeds) {
            await feed.ActivateAsync();
            if (!active || generation != activation) return;
        }
        Search(); await LastLoad;
    }
    public void Search() { after = null; openCustomer = null; Selected = null; LastLoad = LoadPageAsync(); }
    public void Next() { if (next is null) return; after = next; LastLoad = LoadPageAsync(); }
    public void Refresh() { if (!active) return; after = null; LastLoad = openCustomer is { } value ? LoadDetailAsync(value) : LoadPageAsync(); }
    public void Open(Customer customer) { LastLoad = LoadDetailAsync(customer); }
    public void Back() { openCustomer = null; Selected = null; history = null; Raise(nameof(HasMoreHistory)); LastLoad = LoadPageAsync(); }
    public void NextHistory() { if (Selected is { } value && HasMoreHistory) LastLoad = LoadDetailAsync(value.Customer, history); }
    private (long Version, CancellationToken Token) Begin()
    {
        pending?.Cancel(); pending?.Dispose(); pending = new();
        Notice = "جارٍ تحميل بيانات العملاء...";
        return (++generation, pending.Token);
    }
    private bool Current(long version, CancellationToken token) => active && generation == version && !token.IsCancellationRequested;
    private async Task LoadPageAsync()
    {
        if (!active) return;
        var (version, token) = Begin(); Rows.Clear(); next = null; Raise(nameof(HasNext));
        try {
            var result = await client.SearchPageAsync(Term, after, token);
            if (!Current(version, token)) return;
            if (!result.Succeeded) { Notice = CustomerClient.ArabicMessage(result.Failure); return; }
            foreach (var row in result.Value!.Items) Rows.Add(row);
            next = result.Value.Next; Raise(nameof(HasNext));
            Notice = next is not null ? "نتائج جزئية. انتقل إلى الصفحة التالية للمتابعة."
                : Rows.Count == 0 ? "لا توجد نتائج ضمن الفروع المصرح بها." : "بيانات مؤكدة من الخادم.";
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { }
    }
    private async Task LoadDetailAsync(Customer customer, CustomerHistoryPage? previous = null)
    {
        if (!active) return;
        openCustomer = customer;
        var (version, token) = Begin(); Selected = null; Rows.Clear(); history = null; Raise(nameof(HasMoreHistory));
        try {
            var contact = await client.GetAsync(customer.Id, token, customer.Scope.Id);
            if (!Current(version, token)) return;
            if (!contact.Succeeded) { Notice = CustomerClient.ArabicMessage(contact.Failure); return; }
            var records = await client.HistoryAsync(contact.Value!, previous, token);
            if (!Current(version, token)) return;
            // Contact reads stay useful when commercial history is unavailable.
            history = records.Value;
            Selected = records.Succeeded ? CustomerHistoryProjection.FromPage(contact.Value!, history!, true)
                : new CustomerPreview(contact.Value!, [], [], readOnly: true);
            Raise(nameof(HasMoreHistory));
            Notice = !records.Succeeded ? "سجل عروض الأسعار والطلبات غير متاح."
                : HasMoreHistory ? "السجل جزئي. تابع إلى صفحة السجل التالية."
                : history!.ServerAvailable ? "بيانات مؤكدة من الخادم." : "آخر سجل محفوظ؛ الاتصال بالخادم غير متاح.";
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { }
    }
    private void CustomerChanged(object? sender, Guid? id) { if (id is null || id == Guid.Empty || openCustomer is null || openCustomer.Id == id) Refresh(); }
    public void Clear()
    {
        active = false; openCustomer = null; ++generation; pending?.Cancel(); Rows.Clear(); Selected = null;
        history = null; after = next = null; Term = ""; Notice = ""; Raise(nameof(HasNext)); Raise(nameof(HasMoreHistory));
    }
    public Task DeactivateAsync() { Clear(); return Task.WhenAll(historyFeeds.Select(f => f.DeactivateAsync())); }
}
