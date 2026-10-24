using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;

namespace Eitmad.WindowsShell.Tests.Customers;

[TestClass]
[DoNotParallelize]
public sealed class CustomerSyncLiveTests
{
    [TestMethod]
    [TestCategory("CustomerSyncEnvironment")]
    public async Task PairedDesktopsConfirmCustomerChangesInBothDirectionsAfterRestart()
    {
        var root = Environment.GetEnvironmentVariable("EITMAD_E2E_ROOT");
        var executable = Environment.GetEnvironmentVariable("EITMAD_E2E_ENGINE");
        if (string.IsNullOrWhiteSpace(root) || string.IsNullOrWhiteSpace(executable))
            Assert.Inconclusive("Set EITMAD_E2E_ROOT and EITMAD_E2E_ENGINE for a running isolated Private-LAN workflow environment with an enrolled Manager desktop.");
        using var settings = JsonDocument.Parse(File.ReadAllText(Path.Combine(root!, "secrets.json")));
        var password = settings.RootElement.GetProperty("manager_password").GetString()!;
        var port = new Uri(settings.RootElement.GetProperty("endpoint").GetString()!).Port;
        var directory = Path.Combine(root!, "customer-sync-client-" + Guid.NewGuid().ToString("N"));
        using var deadline = new CancellationTokenSource(TimeSpan.FromMinutes(2));
        var token = deadline.Token;
        await using var owner = new EngineSupervisor();
        await using var client = new EngineSupervisor();
        await Start(owner, executable!, Path.Combine(root!, "manager"), token);
        var ownerSession = await owner.SignInAsync("manager", password, token);
        Assert.AreEqual(DesktopAccountRole.Manager, ownerSession.AccountRole);
        await Start(client, executable!, directory, token);
        var scan = await client.DiscoverServersAsync(token);
        Assert.AreEqual(DiscoveryStatus.Complete, scan.Status);
        var candidate = scan.Servers.Single(server => server.Port == port);
        Assert.IsFalse(candidate.IdentityConflict);
        var code = await owner.OnboardingAsync(new() { Kind = DesktopOnboardingActionKind.OpenPairing }, token);
        try
        {
            var paired = await client.OnboardingAsync(new() {
                Kind = DesktopOnboardingActionKind.Pair,
                Payload = new() { Candidate = candidate, Code = code.Code },
            }, token);
            Assert.IsTrue(paired.Paired);
        }
        finally { code.Code = ""; }
        if (await client.AuthenticateServerAsync("manager", password, null, token))
        {
            await client.StopAsync();
            await Start(client, executable!, directory, token);
        }
        var clientSession = await client.SignInAsync("manager", password, token);
        var device = clientSession.Authorization.Identity.DeviceId;
        Assert.IsNotNull(device);
        Assert.AreNotEqual(ownerSession.Authorization.Identity.DeviceId, device);

        var created = await owner.SubmitCommandAsync(Command.ForCustomerCreate(new() {
            Name = "عميل تحقق الشبكة", Phone = "777123456", Address = null!, Notes = null!,
        }), Guid.NewGuid(), cancellationToken: token);
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, created.Outcome.Status, created.Outcome.Payload.Code);
        var customer = created.Outcome.Payload.AsCustomerCreated()!.Customer;
        await Confirmed(owner, customer, token);
        var received = await Confirmed(client, customer, token);
        Assert.AreEqual("عميل تحقق الشبكة", received.Name);
        var update = Command.ForCustomerUpdate(new() {
            CustomerId = customer.Id, ExpectedRevision = customer.Revision,
            Name = "عميل تحقق الشبكة المحدث", Phone = customer.Phone, Address = "عدن", Notes = null!,
        });
        var changed = await UpdateAfterDownload(client, update, token);
        customer = changed.Outcome.Payload.AsCustomerUpdated()!.Customer;
        await Confirmed(client, customer, token);
        var returned = await Confirmed(owner, customer, token);
        Assert.AreEqual("عميل تحقق الشبكة المحدث", returned.Name);
        Assert.AreEqual("عدن", returned.Address);
        Assert.AreEqual(2, returned.Revision);

        await client.StopAsync();
        await Start(client, executable!, directory, token);
        Assert.IsTrue((await client.OnboardingAsync(new() { Kind = DesktopOnboardingActionKind.State }, token)).Paired);
        var reopened = await client.SignInAsync("manager", password, token);
        Assert.AreEqual(device, reopened.Authorization.Identity.DeviceId);
        received = await Confirmed(client, customer, token);
        Assert.AreEqual("عميل تحقق الشبكة المحدث", received.Name);
        Assert.AreEqual("عدن", received.Address);
    }

    private static async Task Start(EngineSupervisor engine, string executable, string directory, CancellationToken token)
    {
        await engine.StartAsync(new EngineLaunchRequest(Path.GetFullPath(executable), directory), token);
        while (!engine.IpcConnected) await Task.Delay(50, token);
    }

    private static async Task<Customer> Confirmed(EngineSupervisor engine, Customer expected, CancellationToken token)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(token);
        timeout.CancelAfter(TimeSpan.FromSeconds(30));
        while (true)
        {
            // Manager reads use the server organization directory and need the owning branch.
            var response = await engine.QueryAsync(Query.ForCustomerGet(new() {
                CustomerId = expected.Id, BranchId = expected.Scope.Id,
            }), cancellationToken: timeout.Token);
            if (response.Outcome.Status == CommandOutcomeStatus.Succeeded)
            {
                var customer = response.Outcome.Payload.AsCustomer();
                Assert.IsNotNull(customer);
                if (customer.Revision == expected.Revision && customer.SyncState == SyncState.Confirmed)
                {
                    Assert.AreEqual(expected.Scope.Id, customer.Scope.Id);
                    return customer;
                }
            }
            else
            {
                Assert.AreEqual(ProtocolIds.ErrorCodes.EitmadErrorCustomerNotFoundV1, response.Outcome.Payload.Code,
                    "A rejected read must fail the test instead of being reported as a sync timeout.");
            }
            await Task.Delay(100, timeout.Token);
        }
    }

    private static async Task<CommandResponseEnvelope> UpdateAfterDownload(EngineSupervisor engine, Command update, CancellationToken token)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(token);
        timeout.CancelAfter(TimeSpan.FromSeconds(30));
        var key = Guid.NewGuid();
        while (true)
        {
            // An organization server read can precede this installation's branch download.
            // Keep one frozen mutation and key while waiting only for that missing local row.
            var response = await engine.SubmitCommandAsync(update, key, cancellationToken: timeout.Token);
            if (response.Outcome.Status == CommandOutcomeStatus.Succeeded) return response;
            Assert.AreEqual(ProtocolIds.ErrorCodes.EitmadErrorCustomerNotFoundV1, response.Outcome.Payload.Code,
                "An update rejection other than a pending download must fail immediately.");
            await Task.Delay(100, timeout.Token);
        }
    }
}
