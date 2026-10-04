using System.IO;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Pricing;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Pricing;

[TestClass]
public sealed class PricingClientTests
{
    [TestMethod]
    public async Task UnknownPublicationRequiresExactPayloadAndKeyRetry()
    {
        await using var engine = new FakeEngine();
        var count = 0;
        engine.CommandHandler = command =>
        {
            if (++count == 1) throw new IOException("Synthetic lost response");
            var input = command.AsPricingPublish()!;
            return new CommandResponseEnvelope { Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded,
                Payload = CommandResult.ForPricePublished(new PublishedPrice { Target = input.Target, Currency = "YER", Revision = 2, SellingPriceYer = input.SellingPriceYer, Colors = [], Handles = [] }) } };
        };
        await using var client = new PricingClient(engine);
        var model = new PricingViewModel(); model.ApplyDurableData(PricingPresentationTests.Data()); model.BeginEdit(model.VisiblePrices[0]);
        var input = model.SaveInput()!;
        Assert.AreEqual(PricingFailure.Unconfirmed, (await client.PublishAsync(input)).Failure);
        var key = engine.LastIdempotencyKey;
        input.SellingPriceYer = 300_000;
        Assert.AreEqual(PricingFailure.Conflict, (await client.PublishAsync(input)).Failure);
        Assert.AreEqual(1, count);
        input.SellingPriceYer = 200_000;
        Assert.IsTrue((await client.PublishAsync(input)).Succeeded); Assert.AreEqual(key, engine.LastIdempotencyKey);
    }
    [TestMethod]
    public async Task CostPermissionLossBetweenPagesClearsEarlierInternalProjection()
    {
        await using var engine = new FakeEngine(); var count = 0;
        engine.QueryHandler = _ =>
        {
            var data = PricingPresentationTests.Data(++count == 1);
            data.Next = count == 1 ? "synthetic-next" : null!;
            return new QueryResponseEnvelope { Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = QueryResult.ForPrices(data) } };
        };
        await using var client = new PricingClient(engine); var result = await client.LoadAsync("");
        Assert.IsTrue(result.Succeeded); Assert.IsFalse(result.Value!.CanReadCosts);
        Assert.IsTrue(result.Value.Items.All(item => item.CostYer is null && item.MarginYer is null));
    }
}
