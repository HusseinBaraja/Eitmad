using System.Collections.ObjectModel;
using System.Globalization;
using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Home;

public sealed record HomeRow(HomeItem Item)
{
    public string Number => Item.Number ?? "غير مرقم";
    public string Title => Item.Title;
    public string State => Item.State;
    public string Kind => Item.Destination switch { HomeDestination.Quotation => "عرض سعر", HomeDestination.Order => "طلب", HomeDestination.Customer => "عميل", _ => "صنف" };
    public string Date => Item.ChangedAt == 0 ? "" : DateTimeOffset.FromUnixTimeMilliseconds(Item.ChangedAt).ToOffset(TimeSpan.FromHours(3)).ToString("yyyy/MM/dd HH:mm", CultureInfo.InvariantCulture);
    public string OpenName => $"فتح {Kind} {Item.Number} {Title}";
}

/// <summary>Projects the authorized Rust home snapshot and discards obsolete session reads.</summary>
public sealed class HomeViewModel : ObservableObject, IAsyncDisposable
{
    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed[] feeds;
    private CancellationTokenSource? load;
    private SynchronizationContext? context;
    private bool active;
    private bool includeCustomerSearch = true;
    private long generation;
    private long sessionGeneration;
    private HomeSnapshot? snapshot;
    private string term = "", notice = "بيانات الرئيسية غير متاحة.";
    public HomeViewModel(IEngineShellBridge engine)
    {
        this.engine = engine;
        feeds = [
            Feed(ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1, Subscription.ForQuotationChangedSubscribe(new())),
            Feed(ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1, Subscription.ForQuotationDraftChangedSubscribe(new())),
            Feed(ProtocolIds.Capabilities.EitmadCapabilityQuotationApprovalV1, Subscription.ForQuotationApprovalChangedSubscribe(new())),
            Feed(ProtocolIds.Capabilities.EitmadCapabilityOrdersV1, Subscription.ForOrderChangedSubscribe(new())),
            Feed(ProtocolIds.Capabilities.EitmadCapabilityCustomerV1, Subscription.ForCustomerChangedSubscribe(new())),
            Feed(ProtocolIds.Capabilities.EitmadCapabilitySalesCatalogV1, Subscription.ForPricingChangedSubscribe(new())),
        ];
    }
    private EngineChangeFeed Feed(string capability, Subscription subscription) => new(engine, capability, subscription,
        _ => { if (active) Refresh(); }, Invalidate, refreshOnStart: false, notifyUnavailable: true);
    public ObservableCollection<HomeRow> Activity { get; } = [];
    public ObservableCollection<HomeRow> ReadyOrders { get; } = [];
    public string Notice { get => notice; private set => Set(ref notice, value); }
    public string SearchTerm => term;
    public bool HasSearchTerm => term.Length > 0;
    public string ActivityTitle => term.Length == 0 ? "آخر نشاط" : "نتائج البحث";
    public bool IsEmpty => snapshot is not null && Activity.Count == 0 && ActivitySections().All(s => Confirmed(s.Section));
    public bool HasNoReadyOrders => ReadyOrders.Count == 0 && Confirmed(snapshot?.Orders);
    public string ActivityMessage => IsEmpty ? "لا توجد سجلات ضمن النتائج المتاحة" : "";
    public string ReadyOrdersMessage => HasNoReadyOrders ? "لا توجد طلبات جاهزة ضمن النتائج المتاحة" : "";
    public string OpenCount => Count(snapshot?.Quotations);
    public string OrderCount => Count(snapshot?.Orders);
    public string ApprovalCount => Count(snapshot?.Approvals);
    public string ReadyCount => Count(snapshot?.Orders, true);
    public Task LastLoad { get; private set; } = Task.CompletedTask;
    public event EventHandler? Invalidated;
    private static string Count(HomeSection? section, bool secondary = false) => section?.Availability != HomeAvailability.Available ? "—"
        : (section.Complete ? "" : "≥ ") + (secondary ? section.SecondaryCount : section.Count).ToString("N0", CultureInfo.InvariantCulture);
    private static bool Confirmed(HomeSection? section) => section is { Availability: HomeAvailability.Available, Complete: true, ServerAvailable: true };
    private IEnumerable<(string Name, HomeSection Section)> ActivitySections()
    {
        if (snapshot is null) yield break;
        yield return ("عروض الأسعار", snapshot.Quotations);
        yield return ("الطلبات", snapshot.Orders);
        if (term.Length == 0) yield break;
        if (includeCustomerSearch) yield return ("العملاء", snapshot.Customers);
        yield return ("الكتالوج", snapshot.Catalog);
    }
    private static string SectionNotice(string name, HomeSection section)
    {
        if (section.Availability == HomeAvailability.Denied) return name + ": ليس لديك صلاحية.";
        if (section.Availability != HomeAvailability.Available) return name + ": غير متاح.";
        var messages = new List<string>();
        if (!section.Complete) messages.Add("نتائج جزئية");
        if (!section.ServerAvailable) messages.Add("غير متصل؛ آخر بيانات محفوظة");
        return messages.Count == 0 ? "" : name + ": " + string.Join("، ", messages) + ".";
    }

    public async Task ActivateAsync(bool includeCustomerSearch = true)
    {
        // This selects shell destinations only; Rust still authorizes every source read.
        this.includeCustomerSearch = includeCustomerSearch;
        active = true; context = SynchronizationContext.Current;
        var session = ++sessionGeneration;
        foreach (var feed in feeds) { await feed.ActivateAsync(); if (!active || session != sessionGeneration) return; }
        Refresh(); await LastLoad;
    }
    public void Search(string query)
    {
        var next = query.Trim();
        if (term != next) { term = next; Apply(null); }
        Refresh();
    }
    public void Refresh() { if (active) LastLoad = LoadAsync(); }
    private async Task LoadAsync()
    {
        load?.Cancel(); load?.Dispose(); load = new();
        var token = load.Token; var version = ++generation;
        Notice = "جارٍ تحميل البيانات...";
        Raise(nameof(ActivityMessage)); Raise(nameof(ReadyOrdersMessage));
        try
        {
            if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityHomeV1)) { Apply(null); return; }
            var response = await engine.QueryAsync(Query.ForHomeRead(new() { Term = term }), token);
            if (!active || version != generation || token.IsCancellationRequested) return;
            Apply(response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsHome() : null);
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { }
        catch (Exception e) when (e is EngineIpcException or IOException or ObjectDisposedException or InvalidOperationException or JsonException)
        { if (active && version == generation) Apply(null); }
    }
    private void Apply(HomeSnapshot? value)
    {
        snapshot = value; Activity.Clear(); ReadyOrders.Clear();
        if (value is null) Notice = "بيانات الرئيسية غير متاحة.";
        else
        {
            var sections = new[] { ("عروض الأسعار", value.Quotations), ("الطلبات", value.Orders), ("الموافقات", value.Approvals) };
            if (term.Length > 0) {
                if (includeCustomerSearch) sections = [.. sections, ("العملاء", value.Customers)];
                sections = [.. sections, ("الكتالوج", value.Catalog)];
            }
            var messages = new List<string>();
            foreach (var (name, section) in sections)
            {
                var message = SectionNotice(name, section);
                if (message.Length > 0) messages.Add(message);
            }
            foreach (var item in sections.Where(s => s.Item2.Availability == HomeAvailability.Available)
                .SelectMany(s => s.Item2.Items).OrderByDescending(item => item.ChangedAt).ThenBy(item => item.Id)) Activity.Add(new(item));
            if (value.Orders.Availability == HomeAvailability.Available) foreach (var item in value.ReadyOrders) ReadyOrders.Add(new(item));
            Notice = messages.Count == 0 ? "حتى ٨ سجلات لكل قائمة" : string.Join(" ", messages);
        }
        Raise(nameof(OpenCount)); Raise(nameof(OrderCount)); Raise(nameof(ApprovalCount)); Raise(nameof(ReadyCount));
        Raise(nameof(IsEmpty)); Raise(nameof(HasNoReadyOrders)); Raise(nameof(ActivityTitle));
        Raise(nameof(SearchTerm)); Raise(nameof(HasSearchTerm));
        Raise(nameof(ActivityMessage)); Raise(nameof(ReadyOrdersMessage));
    }
    private void Invalidate()
    {
        var session = sessionGeneration;
        void Clear() { if (!active || session != sessionGeneration) return; ++generation; load?.Cancel(); Apply(null); Invalidated?.Invoke(this, EventArgs.Empty); }
        if (context is null) Clear(); else context.Post(_ => Clear(), null);
    }
    public void Clear()
    {
        active = false; ++sessionGeneration; ++generation; load?.Cancel(); term = ""; Apply(null);
    }
    public async Task DeactivateAsync() { Clear(); foreach (var feed in feeds) await feed.DeactivateAsync(); }
    public async ValueTask DisposeAsync() { Clear(); foreach (var feed in feeds) await feed.DisposeAsync(); load?.Dispose(); }
}
