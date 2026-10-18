using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Customers;

[TestClass]
public sealed class CustomerDirectoryTests
{
    internal static Customer Contact(Guid branch, string name = "عميل تجريبي") => new() {
        Id = Guid.NewGuid(), Scope = new() { Kind = "branch", Id = branch }, Name = name,
        Phone = "+967 777 123 456", Revision = 1, SyncState = SyncState.Confirmed,
    };
    internal static QueryResponseEnvelope Result(QueryResult value) => new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = value } };

    [TestMethod]
    public async Task DirectoryPagesAcrossBranchesReplacesRowsAndClearsLateResultsAfterSignOut()
    {
        var first = Contact(Guid.NewGuid()); var second = Contact(Guid.NewGuid());
        var cursor = new CustomerCursor { CustomerId = first.Id, BranchId = first.Scope.Id };
        await using var engine = new FakeEngine(); await using var client = new CustomerClient(engine);
        engine.QueryHandler = q => Result(QueryResult.ForCustomers(new() { Items = q.AsCustomerSearch()!.After is null ? [first] : [second], Next = q.AsCustomerSearch()!.After is null ? cursor : null! }));
        var model = new CustomerDirectoryViewModel(client, engine);
        await model.ActivateAsync();
        Assert.AreEqual(first, model.Rows.Single()); Assert.IsTrue(model.HasNext);
        model.Next(); await model.LastLoad;
        Assert.AreEqual(second, model.Rows.Single()); Assert.IsFalse(model.HasNext);
        var blocked = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.QueryBarrier = _ => blocked.Task;
        model.Term = "اعتماد"; model.Search(); var late = model.LastLoad;
        await model.DeactivateAsync(); blocked.SetResult(); await late;
        Assert.HasCount(0, model.Rows); Assert.IsNull(model.Selected); Assert.AreEqual("", model.Term);
        Assert.IsFalse(model.HasNext); Assert.IsFalse(model.HasMoreHistory);
    }

    [TestMethod]
    public async Task DetailUsesExactBranchAndRefreshFailureClearsProtectedData()
    {
        var customer = Contact(Guid.NewGuid());
        await using var engine = new FakeEngine(); await using var client = new CustomerClient(engine);
        engine.QueryHandler = q => q.Kind switch {
            Query.CustomerSearchKind => Result(QueryResult.ForCustomers(new() { Items = [customer] })),
            Query.CustomerGetKind => q.AsCustomerGet()!.BranchId == customer.Scope.Id ? Result(QueryResult.ForCustomer(customer)) : throw new InvalidOperationException("Wrong branch"),
            Query.CustomerHistoryKind => Result(QueryResult.ForCustomerHistory(new() { Quotations = [], Orders = [], ServerAvailable = true })),
            _ => throw new InvalidOperationException(),
        };
        var model = new CustomerDirectoryViewModel(client, engine); await model.ActivateAsync();
        model.Open(customer); await model.LastLoad;
        Assert.IsTrue(model.HasDetail); Assert.IsFalse(model.Selected!.CanEdit);
        Assert.AreEqual(customer.Scope.Id.ToString(), model.Selected.BranchId);
        customer.Revision = 2; customer.Name = "اسم محدث";
        await client.ActivateAsync();
        engine.Publish(Subscription.CustomerChangedSubscribeKind, new EventEnvelope {
            Event = new Dictionary<string, object> { ["kind"] = Event.CustomerChangedEventKind,
                ["payload"] = new CustomerChangeNotice { CustomerId = customer.Id, Scope = customer.Scope, Revision = 2, ChangeId = Guid.NewGuid(), ChangedAt = 1 } },
        });
        var deadline = DateTime.UtcNow.AddSeconds(3);
        while (model.Selected?.Customer.Revision != 2 && DateTime.UtcNow < deadline) await Task.Delay(5);
        await model.LastLoad;
        Assert.AreEqual("اسم محدث", model.Selected!.Name); Assert.AreEqual(2, model.Selected.Customer.Revision);
        engine.QueryHandler = _ => throw new System.IO.IOException("Synthetic failure");
        model.Refresh(); await model.LastLoad;
        Assert.IsNull(model.Selected); Assert.HasCount(0, model.Rows);
        await model.DeactivateAsync();
    }
}
