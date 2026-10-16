using System.Collections.ObjectModel;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.WorkOrders;

namespace Eitmad.WindowsShell.Features.Orders;

public sealed partial class OrdersViewModel
{
    private OrderClient? client;
    private bool active, busy;
    private long generation, loadVersion;
    private Task? loadOperation;
    private bool loadRequested;
    private string listState = "", actionNotice = "", fulfillmentNote = "", recipient = "", reason = "", assignment = "", deliveryNote = "";
    private DateTime dueDate = DateTime.Today.AddDays(7);
    private Command? retryCommand;
    private Guid retryKey;
    private Guid? retryOrderId;
    private IReadOnlyList<OrderPending> pending = [];
    public event EventHandler? DocumentsInvalidated;
    public async Task<CustomerDocument?> ReadDocumentAsync(bool source)
    {
        if (client is null || !active || SelectedOrder is not { } row) return null;
        var session = generation;
        ActionNotice = "جارٍ تحميل المستند المحفوظ...";
        var result = await client.DocumentAsync(row.Id, source);
        if (!active || session != generation || SelectedOrder?.Id != row.Id) return null;
        ActionNotice = result.Succeeded ? "" : result.Failure == DraftFailure.Denied ? "ليس لديك صلاحية لعرض المستند أو طباعته." : "المستند المحفوظ غير متاح. أعد المحاولة.";
        if (result.Failure == DraftFailure.Denied) DocumentsInvalidated?.Invoke(this, EventArgs.Empty);
        return result.Value;
    }
    public bool IsLive => client is not null;
    public string ListState { get => listState; private set { Set(ref listState, value); Raise(nameof(ListSubtitle)); } }
    public string ActionNotice { get => actionNotice; private set => Set(ref actionNotice, value); }
    public bool ActionsAvailable => !busy;
    public bool HasUnresolvedOperation => retryCommand is not null || pending.Any(p => p.RejectedCode is null);
    public bool CanRetry => !busy && HasUnresolvedOperation;
    public bool HasPending => retryCommand is not null || pending.Count > 0;
    public string PendingNotice => HasUnresolvedOperation ? "بانتظار تأكيد الخادم. أعد محاولة العملية نفسها." : HasPending ? "رفض الخادم عملية سابقة. راجع الحالة والبيانات قبل إرسال عملية جديدة." : "";
    private IEnumerable<OrderPending> SelectedPending => SelectedOrder is { } row ? pending.Where(p => PendingOrderId(p) == row.Id) : [];
    private bool HasSelectedRetry => retryCommand is not null && SelectedOrder?.Id == retryOrderId;
    public bool HasOrderPending => HasSelectedRetry || SelectedPending.Any();
    public bool HasOrderRetry => HasSelectedRetry || SelectedPending.Any(p => p.RejectedCode is null);
    public bool CanRetryOrder => !busy && HasOrderRetry && (retryCommand is null || HasSelectedRetry);
    public string OrderPendingNotice => HasOrderRetry ? "بانتظار تأكيد الخادم. أعد محاولة العملية نفسها." : HasOrderPending ? "رفض الخادم عملية سابقة. راجع الحالة والبيانات قبل إرسال عملية جديدة." : "";
    private static Guid? PendingOrderId(OrderPending operation)
    {
        var action = JsonSerializer.SerializeToElement(operation.Request.Action);
        return action.TryGetProperty("payload", out var payload) && payload.TryGetProperty("orderId", out var id)
            && id.TryGetGuid(out var orderId) ? orderId : null;
    }
    public string FulfillmentNote { get => fulfillmentNote; set => Set(ref fulfillmentNote, value); }
    public string DeliveryNote { get => deliveryNote; set => Set(ref deliveryNote, value); }
    public string Recipient { get => recipient; set => Set(ref recipient, value); }
    public string CancellationReason { get => reason; set => Set(ref reason, value); }
    public string Assignment { get => assignment; set => Set(ref assignment, value); }
    public DateTime DueDate { get => dueDate; set => Set(ref dueDate, value); }
    public AcceptanceMethod DeliveryMethod { get; set; } = AcceptanceMethod.InPerson;
    public IReadOnlyList<AcceptanceChoice> Methods { get; } = [new("حضورياً", AcceptanceMethod.InPerson), new("هاتفياً", AcceptanceMethod.Phone), new("كتابياً", AcceptanceMethod.Written)];
    public Task LastAction { get; private set; } = Task.CompletedTask;
    public Task LastLoad { get; private set; } = Task.CompletedTask;
    public void Attach(OrderClient source)
    {
        client = source; UsePreviewOrders(new ObservableCollection<OrderListItem>());
        source.Changed += (_, _) => { if (active) RequestLoad(); };
        source.Invalidated += (_, _) => ClearOrders();
        Raise(nameof(IsLive)); Raise(nameof(ListSubtitle)); Raise(nameof(DetailSubtitle));
    }
    public async Task ActivateAsync()
    {
        if (client is null) return;
        var session = generation;
        active = true; await client.ActivateAsync();
        if (!active || session != generation) return;
        await RequestLoad();
    }
    public async Task OpenByIdAsync(Guid id)
    {
        if (client is null) return;
        var session = generation;
        if (!active) await ActivateAsync();
        if (!active || session != generation) return;
        var result = await client.ListAsync(orderId: id);
        if (!active || session != generation) return;
        if (result.Succeeded && result.Value!.Items.FirstOrDefault(o => o.Id == id) is { } record) OpenOrder(Project(record));
        else { CloseOrder(); ActionNotice = result.Succeeded ? "الطلب غير متاح ضمن صلاحيات الحساب." : OrderClient.Message(result.Failure); }
    }
    public void ClearOrders()
    {
        if (client is null) return;
        DocumentsInvalidated?.Invoke(this, EventArgs.Empty);
        active = false; ++generation; ++loadVersion; busy = false; retryCommand = null; pending = [];
        loadRequested = false; loadOperation = null; LastLoad = Task.CompletedTask;
        CloseOrder(); orders.Clear(); RefreshVisibleOrders(); ListState = "الطلبات غير متاحة."; ActionNotice = "";
        Recipient = ""; DeliveryNote = ""; FulfillmentNote = ""; CancellationReason = ""; Assignment = ""; RaiseActions();
    }
    public async Task DeactivateAsync() { ClearOrders(); if (client is not null) await client.DeactivateAsync(); }
    private Task RequestLoad()
    {
        ++loadVersion; loadRequested = true;
        return LastLoad = loadOperation is { IsCompleted: false } ? loadOperation : loadOperation = DrainLoadsAsync(generation);
    }
    // Subscription notices and command confirmation share one read sequence.
    // Only requested refreshes run; an unavailable result does not schedule a retry.
    private async Task DrainLoadsAsync(long session)
    {
        await Task.Yield();
        while (active && generation == session && loadRequested)
        {
            loadRequested = false;
            await LoadAsync(loadVersion);
        }
    }
    private async Task LoadAsync(long version)
    {
        if (client is null || !active) return;
        var session = generation; Guid? cursor = null;
        var items = new List<OrderListItem>(); var available = true; IReadOnlyList<OrderPending> requests = [];
        ListState = "جارٍ تحميل الطلبات...";
        do {
            var result = await client.ListAsync(cursor);
            if (!active || session != generation || version != loadVersion) return;
            if (!result.Succeeded) {
                CloseOrder(); orders.Clear(); RefreshVisibleOrders();
                if (result.Failure == DraftFailure.Denied) { pending = []; retryCommand = null; RaiseActions(); }
                ListState = result.Failure == DraftFailure.Denied ? "ليس لديك صلاحية لعرض الطلبات." : "الطلبات غير متاحة. أعد تحميل البيانات عند الاتصال.";
                return;
            }
            available &= result.Value!.ServerAvailable; requests = result.Value.Pending ?? [];
            items.AddRange(result.Value.Items.Select(Project)); cursor = result.Value.Next;
        } while (cursor is not null);
        var selected = SelectedOrder?.Id;
        orders.Clear(); foreach (var item in items) orders.Add(item);
        SelectedOrder = items.FirstOrDefault(i => i.Id == selected);
        pending = requests; RaiseActions(); RefreshVisibleOrders();
        ListState = available ? "" : "غير متصل — آخر حالة مؤكدة. العمليات تحتاج تأكيد الخادم.";
    }
    public void CancelOrder() => LastAction = ActAsync(OrderPermittedAction.Cancel);
    public void SaveFulfillment() => LastAction = ActAsync(OrderPermittedAction.EditFulfillment);
    public void DeliverOrder() => LastAction = ActAsync(OrderPermittedAction.Deliver);
    public void StartWork() => LastAction = ActAsync(OrderPermittedAction.StartWork);
    public void CompleteWork() => LastAction = ActAsync(OrderPermittedAction.CompleteWork);
    private async Task ActAsync(OrderPermittedAction action)
    {
        if (client is null || busy || SelectedOrder?.Record is not { } value || !value.PermittedActions.Contains(action)) return;
        if (CanRetry) { ActionNotice = "أكد نتيجة العملية السابقة قبل إرسال عملية جديدة."; return; }
        retryCommand = action switch {
            OrderPermittedAction.Cancel => Command.ForOrderCancel(new() { OrderId = value.Id, ExpectedRevision = value.Revision, Reason = CancellationReason }),
            OrderPermittedAction.EditFulfillment => Command.ForOrderFulfillment(new() { OrderId = value.Id, ExpectedRevision = value.Revision, Note = FulfillmentNote }),
            OrderPermittedAction.Deliver => Command.ForOrderDeliver(new() { OrderId = value.Id, ExpectedRevision = value.Revision, Recipient = Recipient, Method = DeliveryMethod, Note = DeliveryNote }),
            _ => WorkCommand(value, action),
        };
        if (retryCommand is null) { ActionNotice = "لا يوجد أمر عمل مناسب. حدّث الطلب."; return; }
        retryOrderId = value.Id; retryKey = Guid.NewGuid(); await SendAsync();
    }
    private Command? WorkCommand(OrderRecord value, OrderPermittedAction action)
    {
        var start = action == OrderPermittedAction.StartWork;
        if (value.Work.FirstOrDefault(w => w.State == (start ? WorkState.Planned : WorkState.InProgress)) is not { } work) return null;
        var input = new TransitionOrderWork { OrderId = value.Id, ExpectedRevision = value.Revision, WorkId = work.Id, Assignment = Assignment, DueAt = WorkOrderDueDate.EndOfAdenDay(DueDate) };
        return start ? Command.ForOrderWorkStart(input) : Command.ForOrderWorkComplete(input);
    }
    public void Retry() => LastAction = RetryAsync(false);
    public void RetrySelectedOrder() => LastAction = RetryAsync(true);
    private async Task RetryAsync(bool selectedOnly)
    {
        if (selectedOnly ? !CanRetryOrder : !CanRetry) return;
        if (retryCommand is null && (selectedOnly ? SelectedPending : pending).FirstOrDefault(p => p.RejectedCode is null) is { } intent) {
            retryCommand = OrderClient.RetryCommand(intent.Request.Action); retryKey = intent.Request.IdempotencyKey; retryOrderId = PendingOrderId(intent);
        }
        await SendAsync();
    }
    private async Task SendAsync()
    {
        if (client is null || retryCommand is null || busy) return;
        var session = generation; busy = true; RaiseActions(); ActionNotice = "جارٍ تأكيد العملية من الخادم...";
        try {
            var result = await client.SendAsync(retryCommand, retryKey);
            if (!active || session != generation) return;
            ActionNotice = result.Succeeded ? "أكد الخادم العملية." : OrderClient.Message(result.Failure, retryCommand);
            if (result.Succeeded || result.Failure != DraftFailure.Unavailable) retryCommand = null;
            await RequestLoad();
        }
        finally { if (session == generation) { busy = false; RaiseActions(); } }
    }
    private void RaiseActions()
    {
        Raise(nameof(ActionsAvailable)); Raise(nameof(CanRetry)); Raise(nameof(HasPending)); Raise(nameof(PendingNotice)); Raise(nameof(HasUnresolvedOperation));
        Raise(nameof(HasOrderPending)); Raise(nameof(HasOrderRetry)); Raise(nameof(CanRetryOrder)); Raise(nameof(OrderPendingNotice));
    }
    internal static OrderListItem Project(OrderRecord value)
    {
        var q = value.Source;
        var source = QuotationsViewModel.Project(new QuotationDraft { Scope = q.Scope, Snapshot = q.Quotation, UpdatedAt = q.ChangedAt, SyncState = SyncState.Confirmed }, lifecycle: q);
        var e = q.Quotation.Evaluation;
        return new(value.Id, value.Number, e.Customer.Name, DateOnly.FromDateTime(DateTimeOffset.FromUnixTimeMilliseconds(value.CreatedAt).ToOffset(TimeSpan.FromHours(3)).DateTime),
            value.State switch { OrderState.InProduction => OrderStatus.InProduction, OrderState.Ready => OrderStatus.Ready, OrderState.Delivered => OrderStatus.Delivered, OrderState.Cancelled => OrderStatus.Cancelled, _ => OrderStatus.New },
            e.Totals.DiscountYer, e.Lines.Select(l => new OrderLineItem(l.Name, l.VariantName, l.Dimensions is { } d ? Features.Reception.SalesCatalogViewModel.DimensionsLabel(d) : "", l.ColorName ?? "—", l.HandleName ?? "—", (int)l.Quantity, l.Price.UnitPriceYer, l.Dimensions is not null) { EvaluatedTotal = l.Price.TotalYer }).ToArray(), e.Customer.Phone, source) { Record = value, CustomerId = e.Customer.Id };
    }
}
public sealed record AcceptanceChoice(string Label, AcceptanceMethod Value);
