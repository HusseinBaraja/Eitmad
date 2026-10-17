using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.WorkOrders;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.Quotations;

namespace Eitmad.WindowsShell.Tests.WorkOrders;

[TestClass]
public sealed class WorkOrderDueDateTests
{
    [TestMethod]
    [DataRow(true, 15, 1792097999999L)]
    [DataRow(false, 15, 1792097999999L)]
    [DataRow(true, 16, 1792184399999L)]
    [DataRow(false, 16, 1792184399999L)]
    public async Task BothEntryPointsSubmitTheSelectedAdenDayAndShowConfirmedOrRejectedState(bool fromOrders, int day, long expectedDueAt)
    {
        var fixture = new WorkOrderAuthorityTests.Fixture();
        await using var engine = fixture.Engine;
        await using var client = new OrderClient(engine);
        fixture.Order.State = OrderState.Confirmed;
        fixture.Order.PermittedActions = [OrderPermittedAction.StartWork];
        fixture.Order.Work = [new() { Id = fixture.Work.Id, Number = fixture.Work.Number, State = WorkState.Planned,
            LineIds = fixture.Work.Furniture.Select(item => item.LineId).ToArray() }];
        var orders = new OrdersViewModel();
        var workOrders = new WorkOrdersViewModel();
        engine.QueryHandler = query => query.Kind == Query.WorkOrderListKind
            ? SalesCatalogAuthorityTests.Response(QueryResult.ForWorkOrders(QuotationDraftTests.Copy(fixture.Page)))
            : SalesCatalogAuthorityTests.Response(QueryResult.ForOrders(new() { Items = [QuotationDraftTests.Copy(fixture.Order)], Pending = [], ServerAvailable = true }));
        orders.Attach(client); workOrders.Attach(client);
        await orders.ActivateAsync(); await workOrders.ActivateAsync();
        orders.OpenOrder(orders.VisibleOrders.Single()); workOrders.OpenWorkOrder(workOrders.VisibleWorkOrders.Single());
        var selectedDate = new DateTime(2026, 10, day, 12, 0, 0, DateTimeKind.Utc);
        orders.DueDate = selectedDate; workOrders.DueDate = selectedDate;
        orders.Assignment = "ورشة تجريبية"; workOrders.Assignment = orders.Assignment;
        var accepted = day >= 16;
        engine.CommandHandler = command => {
            var input = command.AsOrderWorkStart()!;
            Assert.AreEqual(fixture.Order.Id, input.OrderId);
            Assert.AreEqual(fixture.Work.Id, input.WorkId);
            Assert.AreEqual(1L, input.ExpectedRevision);
            Assert.AreEqual("ورشة تجريبية", input.Assignment);
            Assert.AreEqual(expectedDueAt, input.DueAt);
            // Synthetic authority replies test presentation; Rust deadline validation is tested in eitmad-orders.
            if (!accepted) return new() { Outcome = new() { Status = CommandOutcomeStatus.Failed,
                Payload = new() { Code = ProtocolIds.ErrorCodes.EitmadErrorOrderInvalidV1 } } };
            fixture.Work.DueAt = input.DueAt; fixture.Work.Assignment = input.Assignment;
            fixture.Work.State = WorkState.InProgress; fixture.Work.CanStart = false; fixture.Work.CanComplete = true;
            fixture.Order.Work[0].DueAt = input.DueAt; fixture.Order.Work[0].State = WorkState.InProgress;
            fixture.Order.State = OrderState.InProduction;
            return new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded,
                Payload = CommandResult.ForOrder(QuotationDraftTests.Copy(fixture.Order)) } };
        };
        if (fromOrders) { orders.StartWork(); await orders.LastAction; }
        else { workOrders.AdvanceSelectedStatus(); await workOrders.LastAction; }
        Assert.IsNotNull(engine.LastCommand);
        var notice = fromOrders ? orders.ActionNotice : workOrders.ActionNotice;
        Assert.IsFalse(fromOrders ? orders.CanRetry : workOrders.CanRetry);
        if (accepted) {
            StringAssert.Contains(notice, "أكد الخادم");
            await workOrders.ActivateAsync();
            var row = workOrders.VisibleWorkOrders.Single();
            Assert.AreEqual(WorkOrderStatus.InProgress, row.Status);
            Assert.AreEqual(new DateOnly(2026, 10, day), row.DueDate);
            workOrders.OpenWorkOrder(row);
            Assert.AreEqual(selectedDate.Date, workOrders.DueDate);
        } else {
            StringAssert.Contains(notice, "رفض");
            StringAssert.Contains(notice, "موعد");
            Assert.AreEqual(OrderState.Confirmed, fixture.Order.State);
            Assert.AreEqual(WorkOrderStatus.New, workOrders.VisibleWorkOrders.Single().Status);
        }
        await orders.DeactivateAsync(); workOrders.ClearWorkOrders();
    }
}
