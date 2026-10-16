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
    public async Task DemoRolesCanReadPricesAndIssueQuotation()
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
        var pricedItems = int.MaxValue;
        foreach (var (name, role) in new[] { ("manager", AuthenticatedSurface.Manager), ("receptionist", AuthenticatedSurface.Receptionist) })
        {
            Assert.AreEqual(role, await sessions.SignInAsync(name,
                secrets.RootElement.GetProperty($"{name}_password").GetString()!));
            var response = await ReadServerConfirmed(bridge, Query.ForSalesCatalogList(new() { Term = "", Limit = 100 }),
                result => result.AsSalesCatalog()!.ServerAvailable);
            var catalog = response.Outcome.Payload.AsSalesCatalog()!;
            pricedItems = Math.Min(pricedItems, catalog.Items.Count(item => item.Price.SellingPriceYer > 0));
            Assert.IsTrue(catalog.Items.Any(item => item.Dimensions is not null && item.Price.SellingPriceYer > 0), "A priced Furniture item is required.");
            Assert.IsTrue(catalog.Items.Any(item => item.Dimensions is null && item.Price.SellingPriceYer > 0), "A priced Product is required.");
            if (name == "receptionist")
                await IssueDemoQuotation(bridge, catalog.Items.First(item => item.Dimensions is null && item.Price.SellingPriceYer > 0));
            await sessions.SignOutAsync();
        }
        await sessions.StopAsync();
        File.WriteAllText(Path.Combine(root!, "demo-workflow-proof.json"), JsonSerializer.Serialize(new
        {
            verified_utc = DateTimeOffset.UtcNow,
            manager_sign_in = true, receptionist_sign_in = true,
            priced_catalog_items = pricedItems, quotation_state = "Issued",
            verification = "Real engine IPC, authenticated TLS and PostgreSQL; no desktop automation",
        }));
    }

    private static async Task IssueDemoQuotation(IEngineShellBridge bridge, CatalogEntry item)
    {
        var created = await SendSmokeCommand(bridge, Command.ForCustomerCreate(new()
        {
            Name = "عميل فحص جاهزية العرض", Phone = "777123456", Address = null!, Notes = null!,
        }), Guid.NewGuid());
        var customer = created.AsCustomerCreated()!.Customer;
        await WaitForCustomerConfirmation(bridge, customer);
        var saved = await SendSmokeCommand(bridge, Command.ForQuotationDraftCreate(new()
        {
            Intent = new()
            {
                Customer = new() { Id = customer.Id, Revision = customer.Revision }, DiscountBasisPoints = 0,
                Lines = [new() { Id = Guid.NewGuid(), Configuration = new()
                {
                    Dimensions = item.Dimensions,
                    Selection = new() { Target = item.Price.Target, PriceRevision = item.Price.Revision, Quantity = 1 },
                } }],
            },
        }), Guid.NewGuid());
        var draft = saved.AsQuotationDraftCreated()!;
        var issued = await SendSmokeCommand(bridge, Command.ForQuotationIssue(new()
        {
            DraftId = draft.Snapshot.Id, ExpectedRevision = 1, ExpectedDraftRevision = draft.Snapshot.Revision,
        }), Guid.NewGuid());
        Assert.AreEqual(QuotationState.Issued, issued.AsQuotation()!.State);
        var document = await bridge.QueryAsync(Query.ForQuotationCustomerDocument(new() { DraftId = draft.Snapshot.Id }));
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, document.Outcome.Status);
        Assert.AreEqual(item.Price.SellingPriceYer, document.Outcome.Payload.AsCustomerDocument()!.TotalYer);
    }
}
