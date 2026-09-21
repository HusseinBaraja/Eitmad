using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Features.Operations;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Authentication;

[TestClass]
public sealed class DesktopSessionControllerTests
{
    [TestMethod]
    public async Task ManagerAndReceptionistSwitchWithoutRetainingAccountState()
    {
        var engine = new FakeEngine();
        engine.Accounts.Add("manager", ("manager-password", DesktopAccountRole.Manager));
        engine.Accounts.Add("reception", ("reception-password", DesktopAccountRole.Receptionist));
        var viewModel = new OperationsViewModel();
        var operations = new OperationsCoordinator(engine, viewModel, new ImmediateDispatcher());
        await using var sessions = new DesktopSessionController(engine, operations);
        await sessions.StartAsync();
        engine.Connect();

        var manager = await sessions.SignInAsync("manager", "manager-password");
        Assert.AreEqual(AuthenticatedSurface.Manager, manager);
        viewModel.ObserveNotification(new Notification
        {
            NotificationId = Guid.NewGuid(),
            MessageId = ProtocolIds.MessageIds.EitmadNotificationSyncCompleteV1,
            Parameters = [],
            Severity = NotificationSeverity.Information,
        }, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        Assert.AreEqual(1, viewModel.Activity.Count);

        await sessions.SignOutAsync();
        Assert.AreEqual(0, viewModel.Activity.Count);
        Assert.AreEqual(0, viewModel.Configuration.Count);
        Assert.AreEqual(0, viewModel.ReferenceMarkers.Count);
        Assert.AreEqual(0, engine.SubscriptionCount);

        var receptionist = await sessions.SignInAsync("reception", "reception-password");
        Assert.AreEqual(AuthenticatedSurface.Receptionist, receptionist);
        Assert.AreEqual(0, viewModel.Activity.Count);
        Assert.AreEqual(1, engine.SignOutCount);
    }

    [TestMethod]
    public async Task RejectedCredentialsNeverActivateAccountData()
    {
        var engine = new FakeEngine();
        engine.Accounts.Add("manager", ("correct-password", DesktopAccountRole.Manager));
        var viewModel = new OperationsViewModel();
        var operations = new OperationsCoordinator(engine, viewModel, new ImmediateDispatcher());
        await using var sessions = new DesktopSessionController(engine, operations);
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
        Assert.AreEqual(0, viewModel.Activity.Count);
    }

    [TestMethod]
    public async Task ExpiryClearsStateAndSignsOut()
    {
        var engine = new FakeEngine
        {
            SessionExpiry = DateTimeOffset.UtcNow.AddMilliseconds(150).ToUnixTimeMilliseconds(),
        };
        engine.Accounts.Add("manager", ("correct-password", DesktopAccountRole.Manager));
        var viewModel = new OperationsViewModel();
        var operations = new OperationsCoordinator(engine, viewModel, new ImmediateDispatcher());
        await using var sessions = new DesktopSessionController(engine, operations);
        var ended = new TaskCompletionSource<SessionEndReason>(TaskCreationOptions.RunContinuationsAsynchronously);
        sessions.SessionEnded += (_, eventArgs) => ended.TrySetResult(eventArgs.Reason);
        await sessions.StartAsync();
        engine.Connect();
        engine.QueryBarrier = query =>
        {
            if (query.Kind == Query.ConfigGetKind)
            {
                viewModel.ObserveNotification(new Notification
                {
                    NotificationId = Guid.NewGuid(),
                    MessageId = ProtocolIds.MessageIds.EitmadNotificationSyncCompleteV1,
                    Parameters = [],
                    Severity = NotificationSeverity.Information,
                }, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
                Assert.AreEqual(1, viewModel.Activity.Count);
            }
            return Task.CompletedTask;
        };
        await sessions.SignInAsync("manager", "correct-password");

        Assert.AreEqual(SessionEndReason.Expired, await ended.Task.WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.AreEqual(0, viewModel.Activity.Count);
        Assert.AreEqual(0, engine.SubscriptionCount);
        Assert.AreEqual(1, engine.SignOutCount);
    }
}
