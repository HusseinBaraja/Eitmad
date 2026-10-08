using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Tests.Quotations;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Orders;
[TestClass]
public sealed class OrderAuthorityTests
{
    internal sealed class Fixture
    {
        internal OrderRecord Order { get; }
        internal FakeEngine Engine { get; }
        internal OrderPage Page { get; }
        internal Fixture(bool manager = false)
        {
            var quote = new QuotationLifecycleTests.Fixture().Record;
            quote.State = QuotationState.Accepted; quote.Number = "QT-2026-00001"; quote.PermittedActions = [];
            Order = new() { Id = Guid.NewGuid(), Scope = quote.Scope, Source = quote, Number = "OR-2026-00001", Revision = 1, State = OrderState.Ready,
                CreatedAt = 1791417600000, ChangedAt = 1791417600000, Work = [], PermittedActions = manager ? [OrderPermittedAction.Cancel, OrderPermittedAction.EditFulfillment] : [OrderPermittedAction.Deliver] };
            Page = new() { Items = [Order], Pending = [], ServerAvailable = true };
            Engine = new() { SupportedCapabilities = new HashSet<string> { ProtocolIds.Capabilities.EitmadCapabilityOrdersV1 } };
            Engine.QueryHandler = _ => SalesCatalogAuthorityTests.Response(QueryResult.ForOrders(QuotationDraftTests.Copy(Page)));
        }
    }
    [TestMethod]
    public async Task UncertainDeliveryPreservesConfirmedStateAndRestartRetriesExactIntent()
    {
        var fixture = new Fixture(); await using var engine = fixture.Engine; await using var client = new OrderClient(engine);
        var model = new OrdersViewModel(true); model.Attach(client); await model.ActivateAsync(); model.OpenOrder(model.VisibleOrders.Single());
        var snapshot = JsonSerializer.Serialize(fixture.Order.Source); Guid key = default; string? intent = null;
        engine.CommandHandler = command => {
            var delivery = command.AsOrderDeliver()!; key = engine.LastIdempotencyKey; intent = JsonSerializer.Serialize(delivery);
            var action = new Dictionary<string, object> { ["kind"] = "deliver", ["payload"] = JsonSerializer.SerializeToElement(new { orderId = delivery.OrderId, expectedRevision = delivery.ExpectedRevision, recipient = delivery.Recipient, method = "inPerson", note = delivery.Note }) };
            fixture.Page.Pending = [new() { Request = new() { Scope = fixture.Order.Scope, IdempotencyKey = key, Action = action } }];
            throw new IOException("Synthetic lost reply");
        };
        model.Recipient = "مستلم تجريبي"; model.DeliveryNote = "تغليف"; model.DeliverOrder(); await model.LastAction;
        Assert.AreEqual(OrderState.Ready, model.SelectedOrder!.Record!.State); Assert.IsTrue(model.CanRetry); Assert.IsTrue(model.HasPending);
        await model.DeactivateAsync();
        model = new OrdersViewModel(true); model.Attach(client); await model.ActivateAsync();
        engine.CommandHandler = command => {
            Assert.AreEqual(key, engine.LastIdempotencyKey); Assert.AreEqual(intent, JsonSerializer.Serialize(command.AsOrderDeliver()));
            fixture.Order.State = OrderState.Delivered; fixture.Order.Revision = 2; fixture.Order.PermittedActions = []; fixture.Page.Pending = [];
            return new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = CommandResult.ForOrder(QuotationDraftTests.Copy(fixture.Order)) } };
        };
        model.Retry(); await model.LastAction; Assert.IsFalse(model.CanRetry); Assert.IsFalse(model.HasPending);
        Assert.AreEqual(OrderState.Delivered, model.VisibleOrders.Single().Record!.State); Assert.AreEqual(snapshot, JsonSerializer.Serialize(fixture.Order.Source));
        await model.DeactivateAsync();
    }
    [TestMethod]
    public async Task RustPermissionsOfflineStateAndDeniedRefreshControlTheShell()
    {
        var fixture = new Fixture(true); await using var engine = fixture.Engine; await using var client = new OrderClient(engine);
        var model = new OrdersViewModel(); model.Attach(client); Assert.HasCount(0, model.VisibleOrders); await model.ActivateAsync(); model.OpenOrder(model.VisibleOrders.Single());
        Assert.IsTrue(model.SelectedOrder!.CanCancel); Assert.IsTrue(model.SelectedOrder.CanEditFulfillment); Assert.IsFalse(model.SelectedOrder.CanDeliver);
        fixture.Page.ServerAvailable = false; fixture.Order.PermittedActions = [];
        await model.ActivateAsync(); model.OpenOrder(model.VisibleOrders.Single());
        Assert.IsTrue(model.ListState.Contains("غير متصل")); Assert.IsFalse(model.SelectedOrder!.CanCancel);
        fixture.Page.Pending = [new() { Request = new() { Scope = fixture.Order.Scope, IdempotencyKey = Guid.NewGuid(), Action = [] } }];
        await model.ActivateAsync(); Assert.IsTrue(model.CanRetry);
        engine.QueryHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 } } };
        await model.ActivateAsync(); Assert.HasCount(0, model.VisibleOrders); Assert.IsFalse(model.IsDetailVisible);
        Assert.IsFalse(model.CanRetry); Assert.IsFalse(model.HasPending);
        await model.DeactivateAsync();
    }
    [TestMethod]
    [DataRow(OrderPermittedAction.StartWork)]
    [DataRow(OrderPermittedAction.CompleteWork)]
    public async Task MissingWorkItemDoesNotSendACommand(OrderPermittedAction action)
    {
        var fixture = new Fixture(true); await using var engine = fixture.Engine; await using var client = new OrderClient(engine);
        fixture.Order.PermittedActions = [action];
        var model = new OrdersViewModel(); model.Attach(client); await model.ActivateAsync(); model.OpenOrder(model.VisibleOrders.Single());
        if (action == OrderPermittedAction.StartWork) model.StartWork(); else model.CompleteWork();
        await model.LastAction;
        Assert.IsNull(engine.LastCommand); Assert.IsFalse(model.CanRetry); Assert.IsTrue(model.ActionsAvailable);
        Assert.IsFalse(string.IsNullOrWhiteSpace(model.ActionNotice));
        await model.DeactivateAsync();
    }
    [TestMethod]
    public async Task SignOutFencesLateOrderReplies()
    {
        var fixture = new Fixture(); await using var engine = fixture.Engine; await using var client = new OrderClient(engine);
        var barrier = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.QueryBarrier = _ => barrier.Task; var model = new OrdersViewModel(true); model.Attach(client);
        var load = model.ActivateAsync(); model.ClearOrders(); barrier.SetResult(); await load;
        Assert.HasCount(0, model.VisibleOrders); Assert.IsFalse(model.IsDetailVisible); Assert.IsFalse(model.HasPending);
        await model.DeactivateAsync();
    }
}
