using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Authentication;

[TestClass]
public sealed class DesktopSessionControllerTests
{
    [TestMethod]
    public async Task AdmittedStopDoesNotStartAnotherSessionEnd()
    {
        var engine = new FakeEngine();
        engine.Accounts.Add("manager", ("synthetic", DesktopAccountRole.Manager));
        await using var sessions = new DesktopSessionController(engine);
        await sessions.StartAsync(); engine.Connect();
        await sessions.SignInAsync("manager", "synthetic");
        var ended = 0;
        sessions.SessionEnded += (_, _) => ended++;
        using var cancelled = new CancellationTokenSource();
        cancelled.Cancel();
        await Assert.ThrowsAsync<OperationCanceledException>(() => sessions.StopAsync(cancelled.Token));
        Assert.AreEqual(0, engine.StopCount, "A pre-cancelled stop must not enter supervision.");
        Assert.IsNotNull(await engine.GetSessionStateAsync());
        await sessions.StopAsync();
        Assert.AreEqual(1, engine.StopCount);
        Assert.AreEqual(0, engine.SignOutCount, "Stop already owns session containment.");
        Assert.AreEqual(0, ended, "The stopping snapshot must not start another session-end flow.");
    }

    [TestMethod]
    public async Task FailedContainmentReportsUnavailableAndPreventsAnotherAccount()
    {
        var engine = new FakeEngine
        {
            SignOutBarrier = token => Task.Delay(Timeout.Infinite, token),
            StopFailure = new AggregateException(new System.ComponentModel.Win32Exception(5, "Synthetic termination failure."),
                new System.IO.IOException("Synthetic disposal failure.")),
        };
        engine.Accounts.Add("manager", ("synthetic", DesktopAccountRole.Manager));
        await using var sessions = new DesktopSessionController(engine) { SessionEndTimeout = TimeSpan.FromMilliseconds(100) };
        await sessions.StartAsync(); engine.Connect(); await sessions.SignInAsync("manager", "synthetic");
        var ended = new TaskCompletionSource<SessionEndReason>();
        sessions.SessionEnded += (_, args) => ended.TrySetResult(args.Reason);
        engine.Disconnect();
        Assert.AreEqual(SessionEndReason.RecoveryFailed, await ended.Task.WaitAsync(TimeSpan.FromSeconds(2)));
        await Assert.ThrowsAsync<SessionRecoveryException>(() => sessions.SignInAsync("manager", "synthetic"));
        Assert.AreEqual(1, engine.SignInCount, "A stopped snapshot cannot override uncertain containment.");
    }

    [TestMethod]
    public async Task RecoveryWaitsForDelayedAttachmentBeforeTheNextAccountSignsIn()
    {
        var pendingSignOut = new TaskCompletionSource();
        var engine = new FakeEngine { RequireReadyForSignIn = true };
        engine.Accounts.Add("manager", ("synthetic", DesktopAccountRole.Manager));
        engine.Accounts.Add("reception", ("synthetic", DesktopAccountRole.Receptionist));
        await using var sessions = new DesktopSessionController(engine) { SessionEndTimeout = TimeSpan.FromMilliseconds(100) };
        await sessions.StartAsync(); engine.Connect();
        await sessions.SignInAsync("manager", "synthetic");
        engine.SignOutBarrier = _ => pendingSignOut.Task;
        // Containment releases the outstanding old-session request, as real IPC disposal does.
        engine.Stopped = () => pendingSignOut.TrySetException(new Eitmad.Platform.Windows.LocalIpc.EngineIpcException(
            Eitmad.Platform.Windows.LocalIpc.EngineIpcFailureKind.ConnectionLost, "Synthetic containment."));
        var ended = new TaskCompletionSource<SessionEndReason>();
        sessions.SessionEnded += (_, args) => ended.TrySetResult(args.Reason);
        engine.Disconnect();
        Assert.AreEqual(SessionEndReason.ConnectionLost, await ended.Task.WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.AreEqual(1, engine.StopCount);
        Assert.IsTrue(pendingSignOut.Task.IsCompleted);
        engine.SignOutBarrier = null;
        var next = sessions.SignInAsync("reception", "synthetic");
        Assert.IsFalse(next.IsCompleted, "Launch does not imply IPC readiness.");
        Assert.AreEqual(1, engine.SignInCount, "Do not authenticate before attachment.");
        await Task.Delay(50);
        engine.Connect();
        Assert.AreEqual(AuthenticatedSurface.Receptionist, await next.WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.AreEqual(2, engine.SignInCount);
        Assert.IsNotNull(await engine.GetSessionStateAsync());
        await sessions.SignOutAsync(); // The transition lock was released after recovery.
    }

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
