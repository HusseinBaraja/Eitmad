using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.WindowsShell.Features.Operations;

namespace Eitmad.WindowsShell.Tests.Operations;

[TestClass]
public sealed class OperationsViewModelTests
{
    [TestMethod]
    public void StateMappingCoversOperationalContracts()
    {
        var model = new OperationsViewModel();
        model.ObserveSync(new SyncStatus
        {
            Kind = SyncStatusKind.Syncing,
            Payload = new SyncStatusPayload { Completed = 40, Total = 100 },
        });
        model.ObserveUpdate(new UpdateState
        {
            Kind = UpdateStateKind.Downloading,
            Payload = new UpdateStatePayload { Version = "1.4.0", ProgressBps = 6250 },
        });
        model.ObserveJob(new BackgroundJobStatus
        {
            JobId = Guid.NewGuid(),
            JobKind = "export",
            State = BackgroundJobState.Running,
            CompletedUnits = 3,
            TotalUnits = 12,
            Scope = TestData.Scope(),
        }, 10);
        model.ObserveNotification(new Notification
        {
            NotificationId = Guid.NewGuid(),
            Scope = TestData.Scope(),
            Severity = NotificationSeverity.Success,
            MessageId = ProtocolIds.MessageIds.EitmadNotificationSyncCompleteV1,
            Parameters = [],
        }, 11);

        Assert.AreEqual("تجري الآن", model.SyncCard.Value);
        Assert.AreEqual(0.4d, model.SyncCard.Progress);
        Assert.AreEqual("تنزيل", model.UpdateCard.Value);
        Assert.AreEqual(0.625d, model.UpdateCard.Progress);
        Assert.AreEqual("تصدير البيانات", model.Jobs.Single().Title);
        Assert.AreEqual("اكتملت المزامنة", model.Activity.Single().Title);
    }

    [TestMethod]
    public void ReferenceMarkerMapsArabicAndSyncState()
    {
        var model = new OperationsViewModel();
        model.ObserveReferenceMarkers(new ReferenceMarkerPage
        {
            Items =
            [
                new ReferenceMarker
                {
                    Id = Guid.Parse("8b8ab1ab-731b-46f5-926a-3b5b2f8f6310"),
                    Label = "مرجع REF-١٢",
                    Revision = 4,
                    Scope = TestData.Scope(),
                    SyncState = ReferenceMarkerSyncState.Pending,
                    UpdatedAt = 1_800_000_000_000,
                },
            ],
        });

        Assert.AreEqual("مرجع REF-١٢", model.ReferenceMarkers.Single().Label);
        Assert.AreEqual("بانتظار المزامنة", model.ReferenceMarkers.Single().SyncState);
        Assert.AreEqual("اللقطة محدّثة", model.ReferenceMarkerStatus);
    }

    [TestMethod]
    public void StaleSnapshotsCannotReplaceNewerState()
    {
        var model = new OperationsViewModel();
        model.ObserveConfiguration(TestData.Configuration(7, "ar-YE"), 200);
        model.ObserveConfiguration(TestData.Configuration(8, "en-US"), 200);
        model.ObserveConfiguration(TestData.Configuration(7, "ar-YE"), 300);
        model.ObserveConfiguration(TestData.Configuration(9, "ar-YE"), 100);

        Assert.AreEqual(8L, model.ConfigRevision);
        Assert.AreEqual("en-US", model.SelectedLocale);

        model.ObserveSync(new SyncStatus { Kind = SyncStatusKind.Current, Payload = new SyncStatusPayload() }, 200);
        model.ObserveSync(new SyncStatus { Kind = SyncStatusKind.Failed, Payload = new SyncStatusPayload { Reason = "old" } }, 100);
        Assert.AreEqual("محدّث", model.SyncCard.Value);
    }

    [TestMethod]
    public async Task CommandFaultsAreOwned()
    {
        var handled = new TaskCompletionSource<Exception>(TaskCreationOptions.RunContinuationsAsynchronously);
        var expected = new InvalidOperationException("Synthetic command failure.");
        var command = new AsyncCommand(
            () => Task.FromException(expected),
            onError: error => handled.TrySetResult(error));

        command.Execute(null);

        Assert.AreSame(expected, await handled.Task.WaitAsync(TimeSpan.FromSeconds(3)));
        await TestData.Eventually(() => command.CanExecute(null));
    }

    [TestMethod]
    public void EngineFailureMapsToRecoveryUx()
    {
        var model = new OperationsViewModel();
        model.ObserveSupervision(new EngineSupervisionSnapshot(
            EngineSupervisionState.RestartExhausted,
            4,
            3,
            EngineIpcHealthState.Unavailable,
            null,
            null,
            new EngineExitOutcome(24, false, false)));

        Assert.IsTrue(model.RestartExhausted);
        Assert.IsTrue(model.ShowConnectionBanner);
        Assert.AreEqual("Danger", model.ConnectionTone);
    }
}
