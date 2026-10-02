using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Authentication;

[TestClass]
public sealed class DesktopSessionControllerTests
{
    [TestMethod]
    public async Task ManagerAndReceptionistUseSeparateEngineSessions()
    {
        var engine = new FakeEngine();
        engine.Accounts.Add("manager", ("manager-password", DesktopAccountRole.Manager));
        engine.Accounts.Add("reception", ("reception-password", DesktopAccountRole.Receptionist));
        await using var sessions = new DesktopSessionController(engine);
        await sessions.StartAsync();
        engine.Connect();

        var manager = await sessions.SignInAsync("manager", "manager-password");
        Assert.AreEqual(AuthenticatedSurface.Manager, manager);

        await sessions.SignOutAsync();
        Assert.AreEqual(0, engine.SubscriptionCount);

        var receptionist = await sessions.SignInAsync("reception", "reception-password");
        Assert.AreEqual(AuthenticatedSurface.Receptionist, receptionist);
        Assert.AreEqual(1, engine.SignOutCount);
    }

    [TestMethod]
    public async Task RejectedCredentialsNeverActivateAccountData()
    {
        var engine = new FakeEngine();
        engine.Accounts.Add("manager", ("correct-password", DesktopAccountRole.Manager));
        await using var sessions = new DesktopSessionController(engine);
        await sessions.StartAsync();
        engine.Connect();

        try
        {
            await sessions.SignInAsync("manager", "wrong-password");
            Assert.Fail("Rejected credentials created a session.");
        }
        catch (Eitmad.Platform.Windows.LocalIpc.EngineIpcException)
        {
        }

        Assert.AreEqual(0, engine.SubscriptionCount);
    }

    [TestMethod]
    public async Task ConnectionLossEndsTheSessionAndEngineLifetimeStillStops()
    {
        var engine = new FakeEngine();
        engine.Accounts.Add("manager", ("correct-password", DesktopAccountRole.Manager));
        await using var sessions = new DesktopSessionController(engine);
        var ended = new TaskCompletionSource<SessionEndReason>(TaskCreationOptions.RunContinuationsAsynchronously);
        sessions.SessionEnded += (_, args) => ended.TrySetResult(args.Reason);
        await sessions.StartAsync();
        engine.Connect();
        await sessions.SignInAsync("manager", "correct-password");

        engine.Disconnect();

        Assert.AreEqual(SessionEndReason.ConnectionLost, await ended.Task.WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.IsNull(await engine.GetSessionStateAsync());
        Assert.AreEqual(1, engine.SignOutCount);
        await sessions.StopAsync();
        Assert.AreEqual(1, engine.StopCount);
    }

    [TestMethod]
    public async Task ExpiryClearsStateAndSignsOut()
    {
        var engine = new FakeEngine
        {
            SessionExpiry = DateTimeOffset.UtcNow.AddMilliseconds(150).ToUnixTimeMilliseconds(),
        };
        engine.Accounts.Add("manager", ("correct-password", DesktopAccountRole.Manager));
        await using var sessions = new DesktopSessionController(engine);
        var ended = new TaskCompletionSource<SessionEndReason>(TaskCreationOptions.RunContinuationsAsynchronously);
        sessions.SessionEnded += (_, eventArgs) => ended.TrySetResult(eventArgs.Reason);
        await sessions.StartAsync();
        engine.Connect();
        await sessions.SignInAsync("manager", "correct-password");

        Assert.AreEqual(SessionEndReason.Expired, await ended.Task.WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.AreEqual(0, engine.SubscriptionCount);
        Assert.AreEqual(1, engine.SignOutCount);
    }
}
