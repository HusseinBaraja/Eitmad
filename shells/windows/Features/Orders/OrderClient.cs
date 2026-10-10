using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Features.Orders;

public sealed class OrderClient : IAsyncDisposable
{
    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed changes;
    public event EventHandler? Changed;
    public event EventHandler? Invalidated;
    public OrderClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        changes = new(engine, ProtocolIds.Capabilities.EitmadCapabilityOrdersV1,
            Subscription.ForOrderChangedSubscribe(new()), _ => Changed?.Invoke(this, EventArgs.Empty),
            () => Invalidated?.Invoke(this, EventArgs.Empty), refreshOnStart: false, notifyUnavailable: true);
    }
    public Task ActivateAsync() => changes.ActivateAsync();
    public Task DeactivateAsync() => changes.DeactivateAsync();
    public Task<DraftResult<WorkOrderPage>> WorkOrdersAsync(Guid? after = null, Guid? orderId = null) => Request(async () => {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityWorkOrdersV1)) return new DraftResult<WorkOrderPage>(null, DraftFailure.Unavailable, []);
        var result = await engine.QueryAsync(Query.ForWorkOrderList(new() { After = after, OrderId = orderId, Limit = 100 }));
        return Result(result.Outcome.Status, result.Outcome.Payload.AsWorkOrders(), result.Outcome.Payload.Code);
    });
    public ValueTask DisposeAsync() => changes.DisposeAsync();
    public Task<DraftResult<CustomerDocument>> DocumentAsync(Guid id, bool source) => Request(async () => {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityCustomerDocumentsV1)) return new DraftResult<CustomerDocument>(null, DraftFailure.Unavailable, []);
        var r = await engine.QueryAsync(source ? Query.ForOrderQuotationDocument(new() { OrderId = id }) : Query.ForOrderCustomerDocument(new() { OrderId = id }));
        return Result(r.Outcome.Status, r.Outcome.Payload.AsCustomerDocument(), r.Outcome.Payload.Code);
    });
    public Task<DraftResult<OrderPage>> ListAsync(Guid? after = null, Guid? orderId = null) => Request(async () => {
        var result = await engine.QueryAsync(orderId is { } id ? Query.ForOrderGet(new() { OrderId = id }) : Query.ForOrderList(new() { After = after, Limit = 100 }));
        return Result(result.Outcome.Status, result.Outcome.Payload.AsOrders(), result.Outcome.Payload.Code);
    });
    public Task<DraftResult<OrderRecord>> SendAsync(Command command, Guid key) => Request(async () => {
        var result = await engine.SubmitCommandAsync(command, key);
        return Result(result.Outcome.Status, result.Outcome.Payload.AsOrder(), result.Outcome.Payload.Code);
    });
    private async Task<DraftResult<T>> Request<T>(Func<Task<DraftResult<T>>> request) where T : class
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityOrdersV1)) return new(null, DraftFailure.Unavailable, []);
        try { return await request(); }
        catch (EngineIpcException e) { return Result<T>(CommandOutcomeStatus.Failed, null, e.ContractError?.Code); }
        catch (Exception e) when (e is IOException or InvalidOperationException or ObjectDisposedException or JsonException) { return new(null, DraftFailure.Unavailable, []); }
    }
    private static DraftResult<T> Result<T>(CommandOutcomeStatus status, T? value, string? code) where T : class =>
        status == CommandOutcomeStatus.Succeeded && value is not null ? new(value, DraftFailure.None, []) : new(null, code switch {
            ProtocolIds.ErrorCodes.EitmadErrorOrderConflictV1 => DraftFailure.Conflict,
            ProtocolIds.ErrorCodes.EitmadErrorOrderInvalidV1 => DraftFailure.Invalid,
            ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => DraftFailure.Denied,
            _ => DraftFailure.Unavailable,
        }, []);
    public static string Message(DraftFailure failure) => failure switch {
        DraftFailure.Conflict => "رفضت العملية: تغير الطلب أو لا تسمح حالته بهذه العملية. حدّث الطلب.",
        DraftFailure.Invalid => "رفضت العملية: تحقق من البيانات المدخلة.",
        DraftFailure.Denied => "ليس لديك صلاحية لهذه العملية.",
        _ => "بانتظار تأكيد الخادم — لم تتغير الحالة المؤكدة. أعد محاولة العملية نفسها.",
    };
    public static Command RetryCommand(Dictionary<string, object> payload) => RetryCommand(JsonSerializer.Deserialize<OrderAction>(JsonSerializer.Serialize(payload))!);
    private static Command RetryCommand(OrderAction action) => action.Kind switch {
        OrderAction.ConvertKind => Command.ForOrderConvert(action.AsConvert()!),
        OrderAction.CancelKind => Command.ForOrderCancel(action.AsCancel()!),
        OrderAction.EditFulfillmentKind => Command.ForOrderFulfillment(action.AsEditFulfillment()!),
        OrderAction.DeliverKind => Command.ForOrderDeliver(action.AsDeliver()!),
        OrderAction.StartWorkKind => Command.ForOrderWorkStart(action.AsStartWork()!),
        OrderAction.CompleteWorkKind => Command.ForOrderWorkComplete(action.AsCompleteWork()!),
        _ => throw new InvalidOperationException("Unsupported order intent."),
    };
}
