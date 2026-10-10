using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.WorkOrders;
using Eitmad.WindowsShell.Tests.Orders;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.Quotations;
using Eitmad.WindowsShell.Tests.TestDoubles;
namespace Eitmad.WindowsShell.Tests.WorkOrders;

[TestClass]
public sealed class WorkOrderAuthorityTests
{
    internal sealed class Fixture
    {
        internal OrderRecord Order { get; } = new OrderAuthorityTests.Fixture(true).Order;
        internal WorkOrderRecord Work { get; }
        internal WorkOrderPage Page { get; }
        internal FakeEngine Engine { get; }
        internal Fixture()
        {
            var reference = new FurnitureReference { Scope = Order.Scope, FurnitureId = Guid.NewGuid(), VariantId = Guid.NewGuid(), Revision = 4, SchemaVersion = 1 };
            Work = new() { Id = Guid.NewGuid(), Scope = Order.Scope, OrganizationId = Guid.NewGuid(), OrderId = Order.Id, OrderNumber = Order.Number, Revision = 1, Number = "WO-2026-00001", Customer = "عميل إنتاج تجريبي", State = WorkState.Planned, CanStart = true,
                Furniture = [new() { LineId = Guid.NewGuid(), Reference = reference, Name = "خزانة غرفة النوم", VariantName = "ثلاثة أبواب", Dimensions = new() { WidthMm = 1800, HeightMm = 2200, DepthMm = 600 }, ColorName = "جوزي", HandleName = "نحاسي", Quantity = 2,
                    Parts = [new() { Name = "جانب الخزانة", Quantity = 4, Reference = new() { Scope = Order.Scope, PartId = Guid.NewGuid(), Revision = 3, SchemaVersion = 1 } }] }] };
            Page = new() { ServerAvailable = true, Items = [Work], Pending = [] };
            Engine = new() { SupportedCapabilities = new HashSet<string> { ProtocolIds.Capabilities.EitmadCapabilityOrdersV1, ProtocolIds.Capabilities.EitmadCapabilityWorkOrdersV1 } };
            Engine.QueryHandler = _ => SalesCatalogAuthorityTests.Response(QueryResult.ForWorkOrders(QuotationDraftTests.Copy(Page)));
        }
    }
    [TestMethod]
    public async Task SelectedDueDateIncludesTheWholeDayInYemenTime()
    {
        var fixture = new Fixture(); await using var engine = fixture.Engine; await using var client = new OrderClient(engine);
        var model = new WorkOrdersViewModel(); model.Attach(client); await model.ActivateAsync(); model.OpenWorkOrder(model.VisibleWorkOrders.Single());
        model.Assignment = "ورشة تجريبية"; model.DueDate = new DateTime(2026, 10, 10);
        engine.CommandHandler = command => {
            var dueAt = DateTimeOffset.FromUnixTimeMilliseconds(command.AsOrderWorkStart()!.DueAt!.Value);
            Assert.AreEqual(new DateTimeOffset(2026, 10, 10, 23, 59, 59, 999, TimeSpan.FromHours(3)), dueAt);
            Assert.IsTrue(dueAt > new DateTimeOffset(2026, 10, 10, 16, 30, 0, TimeSpan.FromHours(3)));
            return new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = CommandResult.ForOrder(QuotationDraftTests.Copy(fixture.Order)) } };
        };
        model.AdvanceSelectedStatus(); await model.LastAction;
        Assert.IsNotNull(engine.LastCommand); model.ClearWorkOrders();
    }
    [TestMethod]
    public async Task LostProductionReplyRetainsExactWorkAndRevisionAcrossRestart()
    {
        var fixture = new Fixture(); await using var engine = fixture.Engine; await using var client = new OrderClient(engine);
        var model = new WorkOrdersViewModel(); model.Attach(client); await model.ActivateAsync(); model.OpenWorkOrder(model.VisibleWorkOrders.Single());
        model.Assignment = "ورشة تجريبية"; Guid key = default; string? intent = null;
        engine.CommandHandler = command => {
            var input = command.AsOrderWorkStart()!; key = engine.LastIdempotencyKey; intent = JsonSerializer.Serialize(input);
            Assert.AreEqual(fixture.Work.Id, input.WorkId); Assert.AreEqual(fixture.Work.OrderId, input.OrderId);
            fixture.Page.Pending = [new() { Request = new() { Scope = fixture.Work.Scope, IdempotencyKey = key, Action = new Dictionary<string, object> { ["kind"] = "startWork", ["payload"] = JsonSerializer.SerializeToElement(input) } } }];
            throw new IOException("Synthetic lost reply");
        };
        model.AdvanceSelectedStatus(); await model.LastAction;
        Assert.AreEqual(WorkOrderStatus.New, model.SelectedWorkOrder!.Status); Assert.IsTrue(model.CanRetry);
        model.ClearWorkOrders(); model = new WorkOrdersViewModel(); model.Attach(client); await model.ActivateAsync();
        engine.CommandHandler = command => {
            Assert.AreEqual(key, engine.LastIdempotencyKey); Assert.AreEqual(intent, JsonSerializer.Serialize(command.AsOrderWorkStart()));
            fixture.Work.State = WorkState.InProgress; fixture.Work.Revision = 2; fixture.Work.CanStart = false; fixture.Work.CanComplete = true; fixture.Page.Pending = [];
            return new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = CommandResult.ForOrder(QuotationDraftTests.Copy(fixture.Order)) } };
        };
        model.Retry(); await model.LastAction; Assert.IsFalse(model.CanRetry); Assert.AreEqual(WorkOrderStatus.InProgress, model.VisibleWorkOrders.Single().Status);
        model.ClearWorkOrders();
    }
    [TestMethod]
    public async Task OfflineAndDeniedReadsDoNotPermitLocalProductionChanges()
    {
        var fixture = new Fixture(); await using var engine = fixture.Engine; await using var client = new OrderClient(engine);
        fixture.Page.ServerAvailable = false; fixture.Work.CanStart = false;
        var model = new WorkOrdersViewModel(); model.Attach(client); await model.ActivateAsync(); model.OpenWorkOrder(model.VisibleWorkOrders.Single());
        model.AdvanceSelectedStatus(); await model.LastAction; Assert.IsNull(engine.LastCommand); Assert.IsFalse(model.SelectedWorkOrder!.CanAdvance); Assert.IsTrue(model.ListState.Contains("غير متصل"));
        engine.QueryHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 } } };
        await model.ActivateAsync(); Assert.HasCount(0, model.VisibleWorkOrders); Assert.IsFalse(model.IsDetailVisible); Assert.IsFalse(model.CanRetry);
        model.ClearWorkOrders();
    }
    [TestMethod]
    public async Task SignOutFencesLateManufacturingRead()
    {
        var fixture = new Fixture(); await using var engine = fixture.Engine; await using var client = new OrderClient(engine);
        var barrier = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously); engine.QueryBarrier = _ => barrier.Task;
        var model = new WorkOrdersViewModel(); model.Attach(client); var load = model.ActivateAsync(); model.ClearWorkOrders(); barrier.SetResult(); await load;
        Assert.HasCount(0, model.VisibleWorkOrders); Assert.IsFalse(model.IsDetailVisible);
    }
}
