using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Orders;

namespace Eitmad.WindowsShell.Features.Quotations;

public sealed partial class QuotationsViewModel
{
    private OrderClient? ordersClient;
    public event Action<OrderListItem>? OrderConfirmed;
    public void AttachOrders(OrderClient client) => ordersClient = client;
    public async Task ConvertAsync()
    {
        if (ordersClient is null || lifecycleBusy || SelectedQuotation?.Lifecycle is not { } q || !q.PermittedActions.Contains(QuotationPermittedAction.Convert)) return;
        var session = approvalSession; lifecycleBusy = true; RaiseLifecycle();
        LifecycleNotice = "جارٍ تأكيد تحويل العرض من الخادم...";
        try {
            var result = await ordersClient.SendAsync(Command.ForOrderConvert(new() { DraftId = q.Quotation.Id, ExpectedRevision = q.Revision }), Guid.NewGuid());
            if (!draftsActive || session != approvalSession) return;
            LifecycleNotice = result.Succeeded ? "أكد الخادم الطلب." : OrderClient.Message(result.Failure) + " راجع صفحة الطلبات.";
            LastDraftLoad = LoadDraftsAsync(); await LastDraftLoad;
            if (result.Succeeded && draftsActive && session == approvalSession) OrderConfirmed?.Invoke(OrdersViewModel.Project(result.Value!));
        }
        finally { if (session == approvalSession) { lifecycleBusy = false; RaiseLifecycle(); } }
    }
    public async Task OpenLinkedOrderAsync()
    {
        if (ordersClient is null || SelectedQuotation is not { } q) return;
        var session = approvalSession; Guid? cursor = null;
        do {
            var result = await ordersClient.ListAsync(cursor);
            if (!draftsActive || session != approvalSession) return;
            if (!result.Succeeded) { LifecycleNotice = OrderClient.Message(result.Failure); return; }
            var order = result.Value!.Items.FirstOrDefault(o => o.Source.Quotation.Id == q.Id);
            if (order is not null) { OrderConfirmed?.Invoke(OrdersViewModel.Project(order)); return; }
            cursor = result.Value.Next;
        } while (cursor is not null);
        LifecycleNotice = "الطلب المرتبط غير متاح في النسخة المؤكدة. حدّث البيانات عند الاتصال.";
    }
}
