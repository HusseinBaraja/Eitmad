using System.IO;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Products;

[TestClass]
public sealed class ProductClientTests
{
    [TestMethod]
    public async Task UnknownSaveOutcomeRequiresExactRetryAndKeepsCategoryRetrySeparate()
    {
        await using var engine = new FakeEngine();
        var submissions = 0;
        engine.CommandHandler = _ =>
        {
            if (++submissions == 1) throw new IOException("Synthetic lost response.");
            return new CommandResponseEnvelope
            {
                RequestId = Guid.NewGuid(),
                CorrelationId = Guid.NewGuid(),
                Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = new CommandResult() },
            };
        };
        await using var client = new ProductClient(engine);
        var model = new ProductsViewModel();
        model.ApplyDurableData(ProductsPresentationTests.Data());
        model.BeginEdit(model.VisibleProducts.Single());
        var input = model.SaveInput();
        Assert.AreEqual(ProductFailureKind.Unavailable, await client.SaveAsync(input));
        var originalKey = engine.LastIdempotencyKey;
        model.EditorName = "طلب مختلف";
        Assert.AreEqual(ProductFailureKind.Conflict, await client.SaveAsync(model.SaveInput()));
        Assert.AreEqual(1, submissions);

        Assert.AreEqual(ProductFailureKind.None,
            await client.SaveAsync(new SaveProductCategory { Name = "أخرى" }));
        Assert.AreNotEqual(originalKey, engine.LastIdempotencyKey);
        Assert.AreEqual(ProductFailureKind.None, await client.SaveAsync(input));
        Assert.AreEqual(originalKey, engine.LastIdempotencyKey);
        Assert.AreEqual(3, submissions);
    }

    [TestMethod]
    public async Task PermissionLossBetweenPagesClearsEarlierCostsAndNotes()
    {
        await using var engine = new FakeEngine();
        var data = ProductsPresentationTests.Data();
        var product = data.Products.Single();
        var cursor = Guid.NewGuid();
        engine.QueryHandler = query => Success(query.Kind == Query.ProductCategoryListKind
            ? QueryResult.ForProductCategories(data.Categories)
            : QueryResult.ForProducts(query.AsProductList()!.After is null
                ? new ProductPage { Items = [product], Next = cursor, CanManage = true, CanReadCosts = true }
                : new ProductPage { Items = [], CanManage = false, CanReadCosts = false }));
        await using var client = new ProductClient(engine);
        var loaded = await client.LoadAsync("");
        Assert.IsTrue(loaded.Succeeded);
        Assert.IsFalse(loaded.Value!.CanManage);
        Assert.IsFalse(loaded.Value.CanReadCosts);
        Assert.AreEqual("", loaded.Value.Products.Single().Notes);
        Assert.IsTrue(loaded.Value.Products.Single().Variants.All(v => v.PurchaseCostYer is null));
    }

    [TestMethod]
    public async Task FailedEventStreamResubscribesInSameEngineGeneration()
    {
        await using var engine = new FakeEngine();
        FakeSubscription? first = null;
        var replacements = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var subscriptions = 0;
        engine.SubscribeHook = (_, subscription) =>
        {
            first ??= subscription;
            if (++subscriptions == 2) replacements.SetResult();
        };
        await using var client = new ProductClient(engine);
        await client.ActivateAsync();
        first!.FailRead();
        await replacements.Task.WaitAsync(TimeSpan.FromSeconds(5));
        Assert.AreEqual(1, engine.SubscriptionCount);
    }

    private static QueryResponseEnvelope Success(QueryResult payload) => new()
    {
        RequestId = Guid.NewGuid(),
        CorrelationId = Guid.NewGuid(),
        Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = payload },
    };
}
