using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Tests.Rendered;

public sealed partial class WorkflowEnvironmentTests
{
    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public async Task RetainedConnectionStartsWithoutLauncherEnvironment()
    {
        var root = Environment.GetEnvironmentVariable("EITMAD_E2E_ROOT");
        if (string.IsNullOrWhiteSpace(root)) Assert.Inconclusive("Prepare a connected workflow test installation.");
        var engine = Environment.GetEnvironmentVariable("EITMAD_E2E_ENGINE");
        Assert.IsFalse(string.IsNullOrWhiteSpace(engine));
        var directory = Path.Combine(root!, "receptionist");
        Assert.IsTrue(File.Exists(Path.Combine(directory, "connection.json")));
        using var secrets = JsonDocument.Parse(File.ReadAllText(Path.Combine(root!, "secrets.json")));
        var inherited = Environment.GetEnvironmentVariables().Keys.Cast<string>()
            .Where(name => name.StartsWith("EITMAD_", StringComparison.Ordinal))
            .ToDictionary(name => name, Environment.GetEnvironmentVariable);
        foreach (var name in inherited.Keys) Environment.SetEnvironmentVariable(name, null);
        try
        {
            var retained = new Dictionary<string, Dictionary<Guid, string>>();
            for (var start = 0; start < 2; ++start)
            {
                await using var bridge = WindowsEngineBridge.Create(["--engine", engine!, "--runtime-directory", directory]);
                await bridge.StartAsync();
                await WaitForConnection(bridge);
                foreach (var name in new[] { "manager", "receptionist" })
                {
                    var session = await bridge.SignInAsync(name, secrets.RootElement.GetProperty($"{name}_password").GetString()!);
                    Assert.AreEqual(name == "manager" ? DesktopAccountRole.Manager : DesktopAccountRole.Receptionist, session.AccountRole);
                    await ReadServerConfirmed(bridge, Query.ForSalesCatalogList(new() { Term = "", Limit = 100 }),
                        result => result.AsSalesCatalog()!.ServerAvailable);
                    var orders = await ReadAllOrders(bridge);
                    if (retained.TryGetValue(name, out var previous))
                    {
                        CollectionAssert.AreEquivalent(previous.Keys.ToArray(), orders.Keys.ToArray());
                        foreach (var (id, record) in previous) Assert.AreEqual(record, orders[id], "Restart changed a retained order.");
                    }
                    else retained.Add(name, orders);
                    await ReadServerConfirmed(bridge, Query.ForQuotationList(new() { Limit = 100 }),
                        result => result.AsQuotations()!.ServerAvailable);
                    await bridge.SignOutAsync();
                    var denied = await bridge.QueryAsync(Query.ForOrderList(new() { Limit = 1 }));
                    Assert.AreEqual(CommandOutcomeStatus.Failed, denied.Outcome.Status);
                }
                await bridge.StopAsync();
            }
        }
        finally
        {
            foreach (var (name, value) in inherited) Environment.SetEnvironmentVariable(name, value);
        }
    }

    private static async Task<Dictionary<Guid, string>> ReadAllOrders(IEngineShellBridge bridge)
    {
        var records = new Dictionary<Guid, string>();
        Guid? after = null;
        do
        {
            var response = await ReadServerConfirmed(bridge, Query.ForOrderList(new() { After = after, Limit = 100 }),
                result => result.AsOrders()!.ServerAvailable);
            var page = response.Outcome.Payload.AsOrders()!;
            foreach (var order in page.Items) records.Add(order.Id, JsonSerializer.Serialize(order));
            after = page.Next;
        } while (after is not null);
        return records;
    }
}
