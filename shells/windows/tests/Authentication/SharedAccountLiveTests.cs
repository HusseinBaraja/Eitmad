using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Features.Users;

namespace Eitmad.WindowsShell.Tests.Authentication;

[TestClass]
[DoNotParallelize]
public sealed class SharedAccountLiveTests
{
    private const string Password = "synthetic-test-password-123";

    [TestMethod]
    [TestCategory("DesktopEnrollmentEnvironment")]
    public async Task TwoEnginesShareAccountsAndRevokeCachedAccessWithoutRestart()
    {
        var input = Environment.GetEnvironmentVariable("EITMAD_SHARED_ACCOUNT_INPUT");
        if (string.IsNullOrEmpty(input)) Assert.Inconclusive("Run the shared account Rust integration scenario.");
        var connection = JsonSerializer.Deserialize<DesktopEnrollmentSettings>(input!)!;
        var root = Environment.GetEnvironmentVariable("EITMAD_SHARED_ACCOUNT_DIRECTORY")!;
        var first = Bridge(Environment.GetEnvironmentVariable("EITMAD_SHARED_ACCOUNT_FIRST")!);
        var second = Bridge(Environment.GetEnvironmentVariable("EITMAD_SHARED_ACCOUNT_SECOND")!);
        await using var owner = new DesktopSessionController(first);
        await using var target = new DesktopSessionController(second);
        await owner.StartAsync(); await target.StartAsync();
        await owner.ServerSignInAsync("owner", Password, connection);
        await target.ServerSignInAsync("owner", Password, connection);
        var firstState = (await first.GetSessionStateAsync())!;
        var secondState = (await second.GetSessionStateAsync())!;
        Assert.AreNotEqual(firstState.Authorization.Identity.DeviceId, secondState.Authorization.Identity.DeviceId);
        Assert.IsTrue(first.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilitySharedAccountsV1));
        Assert.IsTrue(second.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilitySharedAccountsV1));
        var users = new UsersViewModel(); users.Attach(first);
        Assert.IsTrue(await users.LoadAsync());
        var otherUsers = new UsersViewModel(); otherUsers.Attach(second);
        Assert.IsTrue(await otherUsers.LoadAsync());
        await using var accountFeed = new EngineChangeFeed(second,
            ProtocolIds.Capabilities.EitmadCapabilitySharedAccountsV1,
            Subscription.ForDesktopAccountChangedSubscribe(new DesktopAccountChanges()),
            notice => { _ = otherUsers.LoadAsync(); }, refreshOnStart: false);
        await accountFeed.ActivateAsync();
        users.BeginEdit(); users.EditorName = "مشرف الاختبار"; users.EditorUsername = "shared.operator";
        Assert.IsTrue(await users.ApplyAsync(Password), users.EditorError);
        var account = users.VisibleUsers.Single(u => u.Username == "shared.operator");
        using (var refresh = new CancellationTokenSource(TimeSpan.FromSeconds(15)))
            while (!otherUsers.VisibleUsers.Any(u => u.AccountId == account.AccountId))
                await Task.Delay(50, refresh.Token);
        Assert.AreEqual(account.AccountId, otherUsers.VisibleUsers.Single(u => u.Username == account.Username).AccountId);
        await accountFeed.DeactivateAsync();
        await target.SignOutAsync();
        await target.ServerSignInAsync(account.Username, Password, null);
        var nonOwner = new UsersViewModel(); nonOwner.Attach(second);
        Assert.IsFalse(await nonOwner.LoadAsync(), "Manager alone must not administer shared accounts.");
        var firstGeneration = first.Snapshot.Generation;
        var secondGeneration = second.Snapshot.Generation;
        var author = (await second.GetSessionStateAsync())!.Authorization;
        var retained = await SaveCategory(second, "مرجع محفوظ قبل تغيير الدور");
        Assert.IsNotNull(retained);
        Assert.IsNotNull(await SaveCategory(second, "عمل معلق بصاحبه الأصلي"));
        var cache = await second.QueryAsync(Query.ForMaterialReferenceList(new ListMaterialReferences()));
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, cache.Outcome.Status);
        await using var stream = await second.SubscribeAsync(Subscription.ForMaterialChangedSubscribe(new MaterialChanges()));
        var closed = new TaskCompletionSource<SessionEndReason>(TaskCreationOptions.RunContinuationsAsynchronously);
        target.SessionEnded += (_, e) => closed.TrySetResult(e.Reason);
        File.WriteAllText(Path.Combine(root, "ready-offline"), "ready");
        await WaitForFile(root, "server-offline");
        Assert.AreEqual(SessionEndReason.AccessChanged, await closed.Task.WaitAsync(TimeSpan.FromSeconds(20)),
            "An idle protected view must lose access when the live server check fails.");
        await AssertDenied(second, Query.ForMaterialReferenceList(new ListMaterialReferences()));
        await AssertDeniedCommand(second, Command.ForMaterialCategorySave(new SaveMaterialCategory { Name = "عمل مرفوض دون اتصال" }));
        await AssertStreamClosed(stream);
        await Assert.ThrowsAsync<EngineIpcException>(() => second.SubscribeAsync(Subscription.ForMaterialChangedSubscribe(new MaterialChanges())));
        await Assert.ThrowsAsync<EngineIpcException>(() => target.SignInAsync(account.Username, Password));
        await Assert.ThrowsAsync<EngineIpcException>(() => target.ServerSignInAsync(account.Username, Password, null));
        Assert.IsFalse(await users.LoadAsync(), "Account administration must not use the local account cache while offline.");
        Assert.IsFalse(users.CanManageAccounts);
        await owner.SignOutAsync();
        File.WriteAllText(Path.Combine(root, "pending-created"), author.Identity.PrincipalId.ToString());
        await WaitForFile(root, "server-online");
        await AssertDenied(second, Query.ForMaterialReferenceList(new ListMaterialReferences()));
        await owner.ServerSignInAsync("owner", Password, null);
        Assert.AreEqual(AuthenticatedSurface.Receptionist, await target.ServerSignInAsync(account.Username, Password, null));
        var deniedUsers = new UsersViewModel(); deniedUsers.Attach(second);
        Assert.IsFalse(await deniedUsers.LoadAsync());
        Assert.AreEqual(0, deniedUsers.VisibleUsers.Count);
        await AssertDenied(second, Query.ForMaterialReferenceList(new ListMaterialReferences()));
        var deniedChange = await second.SubmitCommandAsync(Command.ForDesktopAccountUpdate(new UpdateDesktopAccount
        { AccountId = account.AccountId, ExpectedRevision = 2, DisplayName = account.Name, Role = DesktopAccountRole.Manager }), Guid.NewGuid());
        Assert.AreEqual(CommandOutcomeStatus.Failed, deniedChange.Outcome.Status);
        Assert.IsTrue(await users.LoadAsync());
        account = users.VisibleUsers.Single(u => u.AccountId == account.AccountId);
        Assert.AreEqual("موظف الاستقبال", account.Role);
        var reassigned = new TaskCompletionSource<SessionEndReason>(TaskCreationOptions.RunContinuationsAsynchronously);
        target.SessionEnded += (_, e) => reassigned.TrySetResult(e.Reason);
        users.BeginEdit(account); users.EditorRole = "مدير";
        Assert.IsTrue(await users.ApplyAsync(""), users.EditorError);
        Assert.AreEqual(SessionEndReason.AccessChanged, await reassigned.Task.WaitAsync(TimeSpan.FromSeconds(20)));
        Assert.AreEqual(AuthenticatedSurface.Manager, await target.ServerSignInAsync(account.Username, Password, null));
        var currentReferences = await second.QueryAsync(Query.ForMaterialReferenceList(new ListMaterialReferences()));
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, currentReferences.Outcome.Status);
        account = users.VisibleUsers.Single(u => u.AccountId == account.AccountId);
        var deactivated = new TaskCompletionSource<SessionEndReason>(TaskCreationOptions.RunContinuationsAsynchronously);
        target.SessionEnded += (_, e) => deactivated.TrySetResult(e.Reason);
        users.BeginDeactivation(account);
        Assert.IsTrue(await users.DeactivateAsync(), users.EditorError);
        Assert.AreEqual(SessionEndReason.AccessChanged, await deactivated.Task.WaitAsync(TimeSpan.FromSeconds(20)));
        Assert.IsFalse(users.VisibleUsers.Single(u => u.AccountId == account.AccountId).IsActive);
        await Assert.ThrowsAsync<EngineIpcException>(() => target.ServerSignInAsync(account.Username, Password, null));
        await Assert.ThrowsAsync<EngineIpcException>(() => target.SignInAsync(account.Username, Password));
        await owner.SignOutAsync();
        await AssertDenied(first, Query.ForDesktopAccountList(new ListDesktopAccounts()));
        Assert.AreEqual(firstGeneration, first.Snapshot.Generation);
        Assert.AreEqual(secondGeneration, second.Snapshot.Generation);
        Assert.AreEqual(0, first.Snapshot.RestartCount); Assert.AreEqual(0, second.Snapshot.RestartCount);
        await owner.StopAsync(); await target.StopAsync();
    }

    private static IEngineShellBridge Bridge(string directory) => WindowsEngineBridge.Create([
        "--engine", Environment.GetEnvironmentVariable("EITMAD_DESKTOP_ENROLLMENT_ENGINE")!, "--runtime-directory", directory]);

    private static async Task<MaterialCategory?> SaveCategory(IEngineShellBridge bridge, string name)
    {
        var saved = await bridge.SubmitCommandAsync(Command.ForMaterialCategorySave(new SaveMaterialCategory { Name = name }), Guid.NewGuid());
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, saved.Outcome.Status);
        return saved.Outcome.Payload.AsMaterialCategorySaved();
    }

    private static async Task AssertDenied(IEngineShellBridge bridge, Query query)
    {
        try { Assert.AreEqual(CommandOutcomeStatus.Failed, (await bridge.QueryAsync(query)).Outcome.Status); }
        catch (EngineIpcException error) { Assert.IsTrue(error.Kind is EngineIpcFailureKind.AuthenticationRejected or EngineIpcFailureKind.EngineUnavailable
            || error.ContractError?.Code == ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1); }
    }

    private static async Task AssertDeniedCommand(IEngineShellBridge bridge, Command command)
    {
        try { Assert.AreEqual(CommandOutcomeStatus.Failed, (await bridge.SubmitCommandAsync(command, Guid.NewGuid())).Outcome.Status); }
        catch (EngineIpcException error) { Assert.IsTrue(error.Kind is EngineIpcFailureKind.AuthenticationRejected or EngineIpcFailureKind.EngineUnavailable); }
    }

    private static async Task AssertStreamClosed(IEngineSubscription stream)
    {
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        try
        {
            await foreach (var update in stream.ReadAllAsync(timeout.Token))
                Assert.Fail("A revoked stream returned protected data.");
        }
        catch (EngineIpcException) { }
        catch (IOException) { }
    }

    private static async Task WaitForFile(string root, string name)
    {
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(30));
        while (!File.Exists(Path.Combine(root, name))) await Task.Delay(50, timeout.Token);
    }
}
