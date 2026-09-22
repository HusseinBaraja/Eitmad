using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Customers;

[TestClass]
public sealed class CustomerClientTests
{
    [TestMethod]
    public async Task CreateRestartSearchAndEditUseRustContractsAndRevision()
    {
        await using var engine = new FakeEngine();
        await using var client = new CustomerClient(engine);

        var created = await client.CreateAsync("منزل الصبري", "+٩٦٧ ٧٧٧ ١٢٣ ٤٥٦", "عدن", "اتصل أولاً");
        Assert.IsTrue(created.Succeeded);
        Assert.AreEqual(Command.CustomerCreateKind, engine.LastCommand!.Kind);
        Assert.AreEqual(1, created.Value!.Revision);

        await engine.StopAsync();
        await engine.StartAsync();
        var found = await client.SearchAsync("٧٧٧ ١٢٣");
        Assert.IsTrue(found.Succeeded);
        Assert.IsTrue(engine.WasQueried(Query.CustomerSearchKind));
        Assert.AreEqual(created.Value.Id, found.Value!.Single().Id);

        var updated = await client.UpdateAsync(found.Value!.Single(), "منزل الصبري", "+٩٦٧ ٧٧٧ ١٢٣ ٤٥٦", "عدن، المنصورة", "اتصل أولاً");
        Assert.IsTrue(updated.Succeeded);
        Assert.AreEqual(Command.CustomerUpdateKind, engine.LastCommand!.Kind);
        Assert.AreEqual(2, updated.Value!.Revision);
        Assert.AreEqual("عدن، المنصورة", engine.Customers.Single().Address);
    }

    [TestMethod]
    public async Task StaleUpdateReturnsArabicRevisionConflictWithoutOverwritingCurrentRecord()
    {
        await using var engine = new FakeEngine();
        await using var client = new CustomerClient(engine);
        var stale = (await client.CreateAsync("عميل التعارض", "777000001", "", "")).Value!;
        var current = (await client.UpdateAsync(stale, stale.Name, "777000002", "", "")).Value!;

        var conflict = await client.UpdateAsync(stale, stale.Name, "777000003", "", "");

        Assert.AreEqual(CustomerFailureKind.RevisionConflict, conflict.Failure);
        StringAssert.Contains(CustomerClient.ArabicMessage(conflict.Failure), "لم تُحفظ تعديلاتك");
        Assert.AreEqual(current.Phone, engine.Customers.Single().Phone);
        Assert.AreEqual(current.Revision, engine.Customers.Single().Revision);
    }

    [TestMethod]
    public async Task RustValidationFieldsMapToArabicPresentation()
    {
        await using var engine = new FakeEngine
        {
            CommandHandler = _ => new CommandResponseEnvelope
            {
                RequestId = Guid.NewGuid(),
                CorrelationId = Guid.NewGuid(),
                Outcome = new CommandOutcome
                {
                    Status = CommandOutcomeStatus.Failed,
                    Payload = new CommandResult
                    {
                        Code = ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1,
                        Detail = new ErrorDetail
                        {
                            Kind = DetailKind.Validation,
                            Payload = new DetailPayload { Fields = ["name", "phone"] },
                        },
                    },
                },
            },
        };
        await using var client = new CustomerClient(engine);

        var result = await client.CreateAsync(" ", "invalid", "", "");

        Assert.AreEqual(CustomerFailureKind.Validation, result.Failure);
        CollectionAssert.AreEquivalent(new[] { "name", "phone" }, result.InvalidFields.ToArray());
        StringAssert.Contains(CustomerClient.ArabicMessage(result.Failure), "تحقق من بيانات العميل");
    }

    [TestMethod]
    public async Task SearchIsBoundedAndChangeSubscriptionReportsOnlyCustomerIdentity()
    {
        await using var engine = new FakeEngine();
        SearchCustomers? observedSearch = null;
        engine.QueryBarrier = query =>
        {
            observedSearch = query.AsCustomerSearch();
            return Task.CompletedTask;
        };
        await using var client = new CustomerClient(engine);
        Guid? changed = null;
        client.Changed += (_, customerId) => changed = customerId;
        await client.ActivateAsync();

        await client.SearchAsync("اعتماد");
        Assert.AreEqual(CustomerClient.SearchLimit, observedSearch!.Limit);

        var customerId = Guid.NewGuid();
        engine.Publish(Subscription.CustomerChangedSubscribeKind, new EventEnvelope
        {
            SubscriptionId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Cursor = Guid.NewGuid(),
            Sequence = 1,
            OccurredAt = 1_800_000_000_000,
            Event = new Dictionary<string, object>
            {
                ["kind"] = Event.CustomerChangedEventKind,
                ["payload"] = new CustomerChangeNotice
                {
                    CustomerId = customerId,
                    Scope = new ScopeRef { Kind = "branch", Id = Guid.NewGuid() },
                    Revision = 2,
                    ChangedAt = 1_800_000_000_000,
                    ChangeId = Guid.NewGuid(),
                },
            },
        });
        await EventuallyAsync(() => changed == customerId);
    }

    [TestMethod]
    public async Task SelectedCustomerRefreshesFromSubscriptionWithoutLosingSelection()
    {
        await using var engine = new FakeEngine();
        var original = Customer("عميل تجريبي", "700000001");
        engine.Customers.Add(original);
        await using var client = new CustomerClient(engine);
        await client.ActivateAsync();
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel(), client);
        model.AttachCustomer(PreviewCustomer.FromContract(original));
        engine.Customers[0] = new Customer
        {
            Id = original.Id, Scope = original.Scope, Name = original.Name,
            Phone = "700000002", Address = original.Address, Notes = original.Notes,
            Status = original.Status, Revision = 2, SyncState = original.SyncState,
            UpdatedAt = original.UpdatedAt + 1,
        };
        engine.Publish(Subscription.CustomerChangedSubscribeKind, new EventEnvelope
        {
            SubscriptionId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(),
            Cursor = Guid.NewGuid(), Sequence = 1, OccurredAt = original.UpdatedAt + 1,
            Event = new Dictionary<string, object>
            {
                ["kind"] = Event.CustomerChangedEventKind,
                ["payload"] = new CustomerChangeNotice
                {
                    CustomerId = original.Id, Scope = original.Scope, Revision = 2,
                    ChangedAt = original.UpdatedAt + 1, ChangeId = Guid.NewGuid(),
                },
            },
        });
        await EventuallyAsync(() => model.SelectedCustomer?.Revision == 2);
        Assert.AreEqual("700000002", model.Phone);
        Assert.AreEqual(original.Id, model.SelectedCustomer!.Id);
    }

    [TestMethod]
    public async Task LaterCustomerSearchDiscardsAnObsoleteResponse()
    {
        await using var engine = new FakeEngine();
        engine.Customers.Add(Customer("عميل قديم", "700000001"));
        engine.Customers.Add(Customer("عميل جديد", "700000002"));
        var oldSearch = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var newSearch = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.QueryBarrier = query => query.AsCustomerSearch()?.Term switch
        {
            "قديم" => oldSearch.Task,
            "جديد" => newSearch.Task,
            _ => Task.CompletedTask,
        };
        await using var client = new CustomerClient(engine);
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel(), client);

        model.CustomerName = "قديم";
        model.CustomerName = "جديد";
        newSearch.SetResult();
        await EventuallyAsync(() => model.CustomerMatches.Count == 1);
        Assert.AreEqual("عميل جديد", model.CustomerMatches.Single().Name);

        oldSearch.SetResult();
        await Task.Delay(20);
        Assert.AreEqual("عميل جديد", model.CustomerMatches.Single().Name);
    }

    private static async Task EventuallyAsync(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(3);
        while (!condition())
        {
            if (DateTime.UtcNow >= deadline) Assert.Fail("Expected customer event was not received.");
            await Task.Delay(5);
        }
    }

    private static Customer Customer(string name, string phone) => new()
    {
        Id = Guid.NewGuid(),
        Scope = new ScopeRef { Kind = "branch", Id = Guid.NewGuid() },
        Name = name,
        Phone = phone,
        Address = null!,
        Notes = null!,
        Status = CustomerStatus.Active,
        Revision = 1,
        SyncState = ErSyncState.Pending,
        UpdatedAt = 1_800_000_000_000,
    };
}
