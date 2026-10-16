using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Authentication;

namespace Eitmad.WindowsShell.Tests.Rendered;

public sealed partial class WorkflowEnvironmentTests
{
    // Launch readiness uses real IPC and TLS without driving a desktop or changing catalog prices.
    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public async Task DemoRolesCanReadFamilyPurchase()
    {
        var root = Environment.GetEnvironmentVariable("EITMAD_E2E_ROOT");
        if (string.IsNullOrWhiteSpace(root)) Assert.Inconclusive("Run run-demo.ps1 after Prepare.");
        var enginePath = Environment.GetEnvironmentVariable("EITMAD_E2E_ENGINE");
        Assert.IsFalse(string.IsNullOrWhiteSpace(enginePath));
        using var secrets = JsonDocument.Parse(File.ReadAllText(Path.Combine(root!, "secrets.json")));
        var directory = Path.Combine(root!, "receptionist");
        var credential = File.ReadAllText(Path.Combine(directory, "route.json"));
        foreach (var prefix in new[] { "MEDIA", "PRICING", "QUOTATION", "CUSTOMER" })
            Environment.SetEnvironmentVariable($"EITMAD_{prefix}_CREDENTIAL_ID", credential);
        var bridge = WindowsEngineBridge.Create(["--engine", enginePath!, "--runtime-directory", directory]);
        await using var sessions = new DesktopSessionController(bridge);
        await sessions.StartAsync();
        await WaitForConnection(bridge);
        object? purchaseProof = null;
        var pricedCatalogItems = 0;
        foreach (var (name, role) in new[] { ("manager", AuthenticatedSurface.Manager), ("receptionist", AuthenticatedSurface.Receptionist) })
        {
            Assert.AreEqual(role, await sessions.SignInAsync(name,
                secrets.RootElement.GetProperty($"{name}_password").GetString()!));
            var response = await ReadServerConfirmed(bridge, Query.ForSalesCatalogList(new() { Term = "", Limit = 100 }),
                result => result.AsSalesCatalog()!.ServerAvailable);
            var catalog = response.Outcome.Payload.AsSalesCatalog()!;
            pricedCatalogItems = catalog.Items.Length;
            purchaseProof = await VerifyFamilyPurchase(bridge, catalog.Items, name == "manager");
            await sessions.SignOutAsync();
        }
        await sessions.StopAsync();
        File.WriteAllText(Path.Combine(root!, "demo-workflow-proof.json"), JsonSerializer.Serialize(new
        {
            verified_utc = DateTimeOffset.UtcNow,
            manager_sign_in = true, receptionist_sign_in = true,
            priced_catalog_items = pricedCatalogItems, quotation_state = "Converted", order_state = "Ready",
            purchase = purchaseProof,
            verification = "Real engine IPC, authenticated TLS and PostgreSQL; no desktop automation",
        }));
    }

    private static async Task<object> VerifyFamilyPurchase(IEngineShellBridge bridge, CatalogEntry[] catalog, bool manager)
    {
        var quotations = (await ReadServerConfirmed(bridge, Query.ForQuotationList(new() { Limit = 100 }),
            result => result.AsQuotations()!.ServerAvailable)).Outcome.Payload.AsQuotations()!.Items;
        var orders = (await ReadServerConfirmed(bridge, Query.ForOrderList(new() { Limit = 100 }),
            result => result.AsOrders()!.ServerAvailable)).Outcome.Payload.AsOrders()!.Items;
        var order = orders.Single();
        var quotation = quotations.Single(value => value.Quotation.Id == order.Source.Quotation.Id);
        Assert.AreEqual(QuotationState.Converted, quotation.State);
        Assert.AreEqual(OrderState.Ready, order.State, "Keep one order ready for the delivery scene.");
        Assert.IsNull(order.Delivery);
        Assert.AreEqual(quotation.Quotation.Id, order.Source.Quotation.Id);
        var evaluation = quotation.Quotation.Evaluation;
        Assert.AreEqual("أسرة سامر السعدي", evaluation.Customer.Name);
        Assert.AreEqual("000000000", evaluation.Customer.Phone);
        Assert.AreEqual(280_000L, evaluation.Totals.TotalYer);
        Assert.AreEqual(0L, evaluation.Totals.DiscountYer);
        Assert.AreEqual(2, evaluation.Lines.Length);
        var purchasedCatalog = catalog.Where(item => evaluation.Lines.Any(line =>
            line.Name == item.Name && line.VariantName == item.VariantName)).ToArray();
        Assert.AreEqual(2, purchasedCatalog.Length);
        foreach (var item in purchasedCatalog)
        {
            var line = evaluation.Lines.Single(value => value.Name == item.Name);
            Assert.AreEqual(item.VariantName, line.VariantName);
            Assert.AreEqual(item.Description, line.Description);
            Assert.AreEqual(JsonSerializer.Serialize(item.Dimensions), JsonSerializer.Serialize(line.Dimensions));
            Assert.AreEqual(1L, line.Quantity);
            Assert.AreEqual(item.Price.SellingPriceYer, line.Price.UnitPriceYer);
            Assert.IsNotNull(item.Image);
            var photo = await bridge.QueryAsync(Query.ForCatalogImageGet(new() { Reference = item.Image, Offset = 0 }));
            Assert.AreEqual(CommandOutcomeStatus.Succeeded, photo.Outcome.Status);
            Assert.IsTrue(Convert.FromBase64String(photo.Outcome.Payload.AsCatalogImage()!.Base64).Length > 0);
        }
        foreach (var query in new[] {
            Query.ForQuotationCustomerDocument(new() { DraftId = quotation.Quotation.Id }),
            Query.ForOrderCustomerDocument(new() { OrderId = order.Id }),
            Query.ForOrderQuotationDocument(new() { OrderId = order.Id }) })
        {
            var response = await bridge.QueryAsync(query);
            Assert.AreEqual(CommandOutcomeStatus.Succeeded, response.Outcome.Status);
            var document = response.Outcome.Payload.AsCustomerDocument()!;
            Assert.IsTrue(document.CanPrint);
            Assert.AreEqual(evaluation.Customer.Id, document.Customer.Id);
            Assert.AreEqual(280_000L, document.TotalYer);
            CollectionAssert.AreEquivalent(purchasedCatalog.Select(item => item.Name).ToArray(), document.Lines.Select(line => line.Name).ToArray());
        }
        Assert.AreEqual(1, order.Work.Length);
        Assert.AreEqual(WorkState.Completed, order.Work[0].State);
        if (manager)
        {
            var work = (await ReadServerConfirmed(bridge, Query.ForWorkOrderList(new() { Limit = 100, OrderId = order.Id }),
                result => result.AsWorkOrders()!.ServerAvailable)).Outcome.Payload.AsWorkOrders()!.Items.Single();
            Assert.AreEqual(order.Id, work.OrderId);
            Assert.AreEqual(evaluation.Customer.Name, work.Customer);
            var furniture = work.Furniture.Single();
            var line = evaluation.Lines.Single(value => value.Dimensions is not null);
            Assert.AreEqual(line.Name, furniture.Name);
            Assert.AreEqual(line.VariantName, furniture.VariantName);
            Assert.AreEqual(JsonSerializer.Serialize(line.Dimensions), JsonSerializer.Serialize(furniture.Dimensions));
            Assert.AreEqual(line.Quantity, furniture.Quantity);
            Assert.IsTrue(furniture.Parts.Length > 0);
        }
        else CollectionAssert.Contains(order.PermittedActions, OrderPermittedAction.Deliver);
        return new { customer = evaluation.Customer.Name, quotation = quotation.Number, order = order.Number,
            work_order = order.Work[0].Number, total_yer = evaluation.Totals.TotalYer,
            catalog = purchasedCatalog.Select(item => new { item.Name, item.VariantName, item.Description,
                price_yer = item.Price.SellingPriceYer, item.Dimensions, image_sha256 = item.Image.Sha256 }) };
    }
}
