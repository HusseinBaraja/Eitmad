using System.Threading.Channels;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.WindowsShell.Features.Operations;

var tests = new ShellScenarios();
tests.StateMappingCoversOperationalContracts();
tests.EventOrderingRejectsDuplicatesAndStaleSequences();
tests.StaleSnapshotsCannotReplaceNewerState();
await tests.ReconnectionRefreshesWithoutDuplicateSubscriptions();
await tests.ResyncRefreshesRustSnapshots();
tests.EngineFailureMapsToRecoveryUx();
await tests.ShutdownStopsEngineCleanly();
tests.RtlLayoutIncludesMixedDirectionFixtures();
tests.ShellOwnershipRulesAreEnforced();
Console.WriteLine("Windows shell scenarios passed.");

internal sealed class ShellScenarios
{
    private static readonly string RepositoryRoot = FindRepositoryRoot();

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
            Scope = Scope(),
        }, 10);
        model.ObserveNotification(new Notification
        {
            NotificationId = Guid.NewGuid(),
            Scope = Scope(),
            Severity = NotificationSeverity.Success,
            MessageId = "eitmad.notification.sync-complete.v1",
            Parameters = [],
        }, 11);

        Assert.Equal("تجري الآن", model.SyncCard.Value, "sync state mapping");
        Assert.Equal(0.4d, model.SyncCard.Progress, "sync progress mapping");
        Assert.Equal("تنزيل", model.UpdateCard.Value, "update state mapping");
        Assert.Equal(0.625d, model.UpdateCard.Progress, "update progress mapping");
        Assert.Equal("تصدير البيانات", model.Jobs.Single().Title, "job mapping");
        Assert.Equal("اكتملت المزامنة", model.Activity.Single().Title, "notification mapping");
    }

    public void EventOrderingRejectsDuplicatesAndStaleSequences()
    {
        var gate = new EventOrderGate();
        var subscription = Guid.NewGuid();
        Assert.True(gate.TryAccept("sync", Event(subscription, 3)), "first event accepted");
        Assert.False(gate.TryAccept("sync", Event(subscription, 3)), "duplicate event rejected");
        Assert.False(gate.TryAccept("sync", Event(subscription, 2)), "older sequence rejected");
        Assert.True(gate.TryAccept("sync", Event(subscription, 4)), "new sequence accepted");
        Assert.True(gate.TryAccept("sync", Event(Guid.NewGuid(), 1)), "replacement subscription sequence accepted");
    }

    public void StaleSnapshotsCannotReplaceNewerState()
    {
        var model = new OperationsViewModel();
        model.ObserveConfiguration(Configuration(7, "ar-YE"), 200);
        model.ObserveConfiguration(Configuration(6, "en-US"), 300);
        model.ObserveConfiguration(Configuration(8, "en-US"), 100);
        Assert.Equal(7L, model.ConfigRevision, "stale configuration revision rejected");
        Assert.Equal("ar-YE", model.SelectedLocale, "stale locale rejected");

        model.ObserveSync(new SyncStatus { Kind = SyncStatusKind.Current, Payload = new SyncStatusPayload() }, 200);
        model.ObserveSync(new SyncStatus { Kind = SyncStatusKind.Failed, Payload = new SyncStatusPayload { Reason = "old" } }, 100);
        Assert.Equal("محدّث", model.SyncCard.Value, "stale sync state rejected");
    }

    public async Task ReconnectionRefreshesWithoutDuplicateSubscriptions()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync(Request());
        engine.Connect();
        await Eventually(() => engine.QueryCount == 3 && engine.SubscriptionCount == 6);

        engine.Disconnect();
        engine.Connect();
        await Eventually(() => engine.QueryCount == 6);
        Assert.Equal(6, engine.SubscriptionCount, "reconnect reuses supervised subscriptions");
        Assert.False(model.ShowConnectionBanner, "fresh snapshots clear reconnect banner");
    }

    public async Task ResyncRefreshesRustSnapshots()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync(Request());
        engine.Connect();
        await Eventually(() => engine.QueryCount == 3);
        engine.SignalResync("eitmad.sync.status.subscribe.v1");
        await Eventually(() => engine.QueryCount == 6);
        Assert.False(model.ShowConnectionBanner, "resync completes with current snapshots");
    }

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
        Assert.True(model.RestartExhausted, "restart exhaustion exposed");
        Assert.True(model.ShowConnectionBanner, "failure banner visible");
        Assert.Equal("Danger", model.ConnectionTone, "failure banner tone");
    }

    public async Task ShutdownStopsEngineCleanly()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync(Request());
        await coordinator.StopAsync();
        Assert.Equal(1, engine.StopCount, "shutdown delegates one clean stop");
    }

    public void RtlLayoutIncludesMixedDirectionFixtures()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));
        Assert.Contains("FlowDirection=\"RightToLeft\"", xaml, "root RTL layout");
        Assert.Contains("Language=\"ar-YE\"", xaml, "Arabic language metadata");
        Assert.Contains("FlowDirection=\"LeftToRight\"", xaml, "mixed-direction isolation");
        Assert.Contains("CNC-04", xaml, "Arabic and English workshop fixture");
        Assert.Contains("Windows / Rust", xaml, "mixed product fixture");
    }

    public void ShellOwnershipRulesAreEnforced()
    {
        var shell = Path.Combine(RepositoryRoot, "shells", "windows");
        var sources = Directory.EnumerateFiles(shell, "*.cs", SearchOption.AllDirectories)
            .Where(path => !path.Contains($"{Path.DirectorySeparatorChar}generated{Path.DirectorySeparatorChar}", StringComparison.OrdinalIgnoreCase)
                && !path.Contains($"{Path.DirectorySeparatorChar}tests{Path.DirectorySeparatorChar}", StringComparison.OrdinalIgnoreCase));
        var forbidden = new[]
        {
            "Microsoft.Data." + "Sqlite", "System.Data.SqlClient", "Npgsql", "DbContext",
            "HttpClient", "File.WriteAll", "FileStream(", "ConfigurationManager",
            "ProtectedData", "RegistryKey", "Authorize(", "PermissionDecision.Granted",
        };
        foreach (var source in sources)
        {
            var text = File.ReadAllText(source);
            foreach (var token in forbidden)
            {
                Assert.False(text.Contains(token, StringComparison.Ordinal), $"ownership token {token} in {Path.GetFileName(source)}");
            }
        }

        var coordinator = File.ReadAllText(Path.Combine(shell, "Features", "Operations", "OperationsCoordinator.cs"));
        Assert.Contains("SubmitConfigurationPatchAsync", coordinator, "typed configuration patch boundary");
        Assert.False(coordinator.Contains("SendCommandAsync", StringComparison.Ordinal), "shell cannot submit generic commands");
    }

    private static EngineLaunchRequest Request()
    {
        var scope = Scope();
        return new EngineLaunchRequest(
            "C:\\synthetic\\eitmad-engine-cli.exe",
            developmentIdentity: new DevelopmentIdentityAssertion
            {
                TenantId = scope.Id,
                Identity = new AuthenticatedIdentity { PrincipalId = Guid.NewGuid(), PrincipalKind = PrincipalKind.Service, ServiceId = Guid.NewGuid() },
                Scope = scope,
            });
    }

    private static ScopeRef Scope() => new() { Kind = "organization", Id = Guid.NewGuid() };

    private static ConfigSnapshot Configuration(long revision, string locale) => new()
    {
        Revision = revision,
        SchemaVersion = 1,
        Scope = Scope(),
        Entries =
        [
            new ConfigEntry
            {
                Key = "eitmad.config.locale.primary.v1",
                Sensitivity = ConfigSensitivity.Public,
                RestartRequirement = RestartRequirement.None,
                Value = new ConfigReadValue { Kind = ConfigReadValueKind.Text, Value = locale },
            },
        ],
    };

    private static EventEnvelope Event(Guid subscriptionId, long sequence) => new()
    {
        SubscriptionId = subscriptionId,
        CorrelationId = Guid.NewGuid(),
        Cursor = Guid.NewGuid(),
        Sequence = sequence,
        OccurredAt = sequence,
        Event = [],
    };

    private static async Task Eventually(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(3);
        while (!condition())
        {
            if (DateTime.UtcNow >= deadline) throw new InvalidOperationException("Expected asynchronous shell condition was not reached.");
            await Task.Delay(5);
        }
    }

    private static string FindRepositoryRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, "AGENTS.md"))) directory = directory.Parent;
        return directory?.FullName ?? throw new DirectoryNotFoundException("Repository root was not found.");
    }
}

internal sealed class ImmediateDispatcher : IShellDispatcher
{
    public void Invoke(Action action) => action();
}

internal sealed class FakeEngine : IEngineShellBridge
{
    private readonly Dictionary<string, FakeSubscription> subscriptions = [];
    private EngineSupervisionSnapshot snapshot = new(EngineSupervisionState.Stopped, 0, 0, EngineIpcHealthState.Unavailable, null, null, null);

    public event Action<EngineSupervisionSnapshot>? StateChanged;
    public EngineSupervisionSnapshot Snapshot => snapshot;
    public int QueryCount { get; private set; }
    public int SubscriptionCount => subscriptions.Count;
    public int StopCount { get; private set; }

    public Task StartAsync(EngineLaunchRequest request, CancellationToken cancellationToken = default)
    {
        snapshot = snapshot with { State = EngineSupervisionState.Starting, Generation = snapshot.Generation + 1 };
        StateChanged?.Invoke(snapshot);
        return Task.CompletedTask;
    }

    public Task StopAsync(CancellationToken cancellationToken = default)
    {
        StopCount++;
        snapshot = snapshot with { State = EngineSupervisionState.Stopped, IpcHealth = EngineIpcHealthState.Unavailable };
        StateChanged?.Invoke(snapshot);
        return Task.CompletedTask;
    }

    public Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default)
    {
        QueryCount++;
        var result = query.Kind switch
        {
            Query.ConfigGetKind => QueryResult.ForConfiguration(ShellScenariosConfiguration()),
            Query.SyncGetStatusKind => QueryResult.ForSyncStatus(new SyncStatus { Kind = SyncStatusKind.Current, Payload = new SyncStatusPayload() }),
            Query.UpdateGetStateKind => QueryResult.ForUpdateState(new UpdateState { Kind = UpdateStateKind.Idle, Payload = new UpdateStatePayload() }),
            _ => throw new InvalidOperationException("Unexpected fake query."),
        };
        return Task.FromResult(new QueryResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = result },
        });
    }

    public Task<CommandResponseEnvelope> SubmitConfigurationPatchAsync(UpdateConfiguration patch, Guid idempotencyKey, CancellationToken cancellationToken = default) =>
        Task.FromResult(new CommandResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = new CommandResult() },
        });

    public Task<IEngineSubscription> SubscribeAsync(Subscription subscription, CancellationToken cancellationToken = default)
    {
        var item = new FakeSubscription();
        subscriptions.Add(subscription.Kind, item);
        return Task.FromResult<IEngineSubscription>(item);
    }

    public void Connect()
    {
        snapshot = snapshot with
        {
            State = EngineSupervisionState.Running,
            IpcHealth = EngineIpcHealthState.Connected,
            LastLifecycle = new LifecycleSnapshot
            {
                Live = true,
                Ready = true,
                State = LifecycleState.Ready,
                Health = HealthStatus.Healthy,
                Checks = [],
                ObservedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                Identity = new EngineProcessIdentity
                {
                    InstanceId = Guid.NewGuid(),
                    Mode = EngineMode.SupervisedDesktop,
                    ProcessId = 100,
                    ProductVersion = "0.0.0",
                    ProtocolVersion = new ProtocolVersion { Major = 1, Minor = 3 },
                    StartedAt = 1,
                },
            },
        };
        StateChanged?.Invoke(snapshot);
    }

    public void Disconnect()
    {
        snapshot = snapshot with { IpcHealth = EngineIpcHealthState.Connecting };
        StateChanged?.Invoke(snapshot);
    }

    public void SignalResync(string kind) => subscriptions[kind].SignalResync();

    public ValueTask DisposeAsync()
    {
        foreach (var subscription in subscriptions.Values) subscription.DisposeAsync();
        return ValueTask.CompletedTask;
    }

    private static ConfigSnapshot ShellScenariosConfiguration() => new()
    {
        Revision = 1,
        SchemaVersion = 1,
        Scope = new ScopeRef { Kind = "organization", Id = Guid.NewGuid() },
        Entries =
        [
            new ConfigEntry
            {
                Key = "eitmad.config.locale.primary.v1", Sensitivity = ConfigSensitivity.Public,
                RestartRequirement = RestartRequirement.None,
                Value = new ConfigReadValue { Kind = ConfigReadValueKind.Text, Value = "ar-YE" },
            },
        ],
    };
}

internal sealed class FakeSubscription : IEngineSubscription
{
    private readonly Channel<EventEnvelope> events = Channel.CreateUnbounded<EventEnvelope>();
    public event Action? ResyncRequired;
    public IAsyncEnumerable<EventEnvelope> ReadAllAsync(CancellationToken cancellationToken = default) => events.Reader.ReadAllAsync(cancellationToken);
    public void Acknowledge(EventEnvelope delivered) { }
    public void SignalResync() => ResyncRequired?.Invoke();
    public ValueTask DisposeAsync()
    {
        events.Writer.TryComplete();
        return ValueTask.CompletedTask;
    }
}

internal static class Assert
{
    public static void True(bool value, string message) { if (!value) throw new InvalidOperationException($"Assertion failed: {message}."); }
    public static void False(bool value, string message) => True(!value, message);
    public static void Equal<T>(T expected, T actual, string message)
    {
        if (!EqualityComparer<T>.Default.Equals(expected, actual)) throw new InvalidOperationException($"Assertion failed: {message}. Expected {expected}; actual {actual}.");
    }
    public static void Contains(string expected, string actual, string message) => True(actual.Contains(expected, StringComparison.Ordinal), message);
}
