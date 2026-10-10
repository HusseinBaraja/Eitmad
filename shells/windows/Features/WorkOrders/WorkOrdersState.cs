using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Features.WorkOrders;

public sealed partial class WorkOrdersViewModel
{
    private OrderClient? client;
    private bool active, busy;
    private long generation, loadVersion;
    private string listState = "بيانات تجريبية للمعاينة فقط", assignment = "", actionNotice = "";
    private DateTime dueDate = DateTime.Today.AddDays(7);
    private Command? retryCommand;
    private Guid retryKey;
    private IReadOnlyList<OrderPending> pending = [];
    public bool IsLive => client is not null;
    public bool IsPreview => !IsLive;
    public bool HasPending => retryCommand is not null || pending.Count > 0;
    public string PendingNotice => pending.Any(p => p.RejectedCode is null) ? "توجد عملية بانتظار تأكيد الخادم. أعد المحاولة لتأكيد نتيجتها." : pending.Count > 0 ? "رفض الخادم عملية سابقة. راجع الحالة والبيانات قبل إرسال عملية جديدة." : "";
    public string ListState { get => listState; private set => Set(ref listState, value); }
    public string ActionNotice { get => actionNotice; private set => Set(ref actionNotice, value); }
    public string Assignment { get => assignment; set => Set(ref assignment, value); }
    public DateTime DueDate { get => dueDate; set => Set(ref dueDate, value); }
    public bool ActionsAvailable => !busy;
    public bool CanRetry => !busy && (retryCommand is not null || pending.Any(p => p.RejectedCode is null));
    public Task LastLoad { get; private set; } = Task.CompletedTask;
    public Task LastAction { get; private set; } = Task.CompletedTask;
    public void Attach(OrderClient source)
    {
        client = source; workOrders.Clear(); CloseWorkOrder(); RefreshVisibleWorkOrders();
        source.Changed += (_, _) => { if (active) LastLoad = LoadAsync(); };
        source.Invalidated += (_, _) => ClearWorkOrders();
        Raise(nameof(IsLive)); Raise(nameof(IsPreview));
    }
    public async Task ActivateAsync()
    {
        if (client is null) return;
        active = true; await client.ActivateAsync(); LastLoad = LoadAsync(); await LastLoad;
    }
    public void ClearWorkOrders()
    {
        if (client is null) return;
        active = false; ++generation; ++loadVersion; busy = false; retryCommand = null; pending = [];
        CloseWorkOrder(); workOrders.Clear(); RefreshVisibleWorkOrders(); Assignment = ""; ActionNotice = "";
        ListState = "أوامر العمل غير متاحة."; RaiseActions();
    }
    private async Task LoadAsync()
    {
        if (client is null || !active) return;
        var version = ++loadVersion; var session = generation; Guid? cursor = null;
        var items = new List<WorkOrderListItem>(); var available = true;
        IReadOnlyList<OrderPending> requests = [];
        ListState = "جارٍ تحميل أوامر العمل...";
        do {
            var result = await client.WorkOrdersAsync(cursor);
            if (!active || session != generation || version != loadVersion) return;
            if (!result.Succeeded) {
                CloseWorkOrder(); workOrders.Clear(); RefreshVisibleWorkOrders();
                if (result.Failure == DraftFailure.Denied) { pending = []; retryCommand = null; RaiseActions(); }
                ListState = result.Failure == DraftFailure.Denied ? "ليس لديك صلاحية لعرض أوامر العمل." : "أوامر العمل غير متاحة. أعد التحميل عند الاتصال.";
                return;
            }
            available &= result.Value!.ServerAvailable; requests = result.Value.Pending ?? [];
            items.AddRange(result.Value.Items.Select(Project)); cursor = result.Value.Next;
        } while (cursor is not null);
        var selected = SelectedWorkOrder?.Id;
        workOrders.Clear(); workOrders.AddRange(items);
        SelectedWorkOrder = items.FirstOrDefault(i => i.Id == selected);
        pending = requests; RaiseActions(); RefreshVisibleWorkOrders();
        ListState = available ? items.Count == 0 ? "لا توجد أوامر عمل." : "أوامر العمل المؤكدة" : "غير متصل — آخر حالة مؤكدة. التغيير يحتاج تأكيد الخادم.";
    }
    private async Task OpenProductionAsync(Guid orderId)
    {
        var session = generation;
        await LoadAsync();
        if (active && session == generation && workOrders.FirstOrDefault(w => w.Record?.OrderId == orderId) is { } work) OpenWorkOrder(work);
    }
    private async Task AdvanceAsync()
    {
        if (client is null || busy || SelectedWorkOrder?.Record is not { } value || !SelectedWorkOrder.CanAdvance) return;
        if (CanRetry) { ActionNotice = "أكد نتيجة العملية السابقة قبل إرسال عملية جديدة."; return; }
        var input = new TransitionOrderWork { OrderId = value.OrderId, WorkId = value.Id, ExpectedRevision = value.Revision,
            Assignment = Assignment, DueAt = new DateTimeOffset(DateTime.SpecifyKind(DueDate.Date, DateTimeKind.Unspecified).AddTicks(TimeSpan.TicksPerDay - TimeSpan.TicksPerMillisecond), TimeSpan.FromHours(3)).ToUnixTimeMilliseconds() };
        retryCommand = value.CanStart ? Command.ForOrderWorkStart(input) : Command.ForOrderWorkComplete(input);
        retryKey = Guid.NewGuid(); await SendAsync();
    }
    public void Retry() => LastAction = RetryAsync();
    private async Task RetryAsync()
    {
        if (retryCommand is null && pending.FirstOrDefault(p => p.RejectedCode is null) is { } intent) {
            retryCommand = OrderClient.RetryCommand(intent.Request.Action); retryKey = intent.Request.IdempotencyKey;
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
            ActionNotice = result.Succeeded ? "أكد الخادم العملية." : OrderClient.Message(result.Failure);
            if (result.Succeeded || result.Failure != DraftFailure.Unavailable) retryCommand = null;
            LastLoad = LoadAsync(); await LastLoad;
        }
        finally { if (session == generation) { busy = false; RaiseActions(); } }
    }
    private void RaiseActions() { Raise(nameof(ActionsAvailable)); Raise(nameof(CanRetry)); Raise(nameof(HasPending)); Raise(nameof(PendingNotice)); }
    internal static WorkOrderListItem Project(WorkOrderRecord value) => new(value.Id, value.Number, value.OrderNumber, value.Customer,
        value.Assignment ?? "غير مسند", value.DueAt is { } at ? DateOnly.FromDateTime(DateTimeOffset.FromUnixTimeMilliseconds(at).ToOffset(TimeSpan.FromHours(3)).DateTime) : null,
        value.State switch { WorkState.InProgress => WorkOrderStatus.InProgress, WorkState.Completed => WorkOrderStatus.Completed, WorkState.Cancelled => WorkOrderStatus.Cancelled, _ => WorkOrderStatus.New },
        value.Furniture.Select(l => new WorkOrderFurnitureItem(l.Name, l.VariantName, Features.Reception.SalesCatalogViewModel.DimensionsLabel(l.Dimensions), l.ColorName ?? "—", l.HandleName ?? "—", (int)l.Quantity, FurnitureIllustration.Wardrobe)).ToArray(),
        value.Furniture.SelectMany(l => l.Parts).Select(p => new WorkOrderPart(p.Name, p.Quantity)).ToArray(), value.Note ?? "—") { Record = value };
}
