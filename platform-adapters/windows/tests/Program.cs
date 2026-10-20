using System.ComponentModel;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;

try
{
    if (args.Length != 0 && args is not ["--engine", _])
    {
        throw new ArgumentException("Usage: Eitmad.Platform.Windows.Tests [--engine <path>]");
    }

    await IpcCancellationScenario.DiagnosticsRetainCorrelationAndRedactPayloads();
    await IpcCancellationScenario.DiagnosticQueueNeverWaitsForBlockedStderr();
    var tests = new SupervisionScenarios();
    await tests.UnavailableEngineIsTyped();
    await IpcCancellationScenario.CancelledWritePreservesTheNextFrame();
    await IpcCancellationScenario.BlockedWriteExpiresAndReleasesQueuedRequests();
    await IpcCancellationScenario.ReplyWaitUsesTheRemainingDeadline();
    await IpcCancellationScenario.UnconfirmedCommandsKeepUnknownOutcomeAndIdempotency();
    await IpcCancellationScenario.SessionChangeDoesNotSendQueuedAccountRequests();
    await IpcCancellationScenario.ClosingSubscriptionDiscardsEventsWithoutBreakingTransport();
    await IpcCancellationScenario.ContainmentReleasesSessionRequestsAndTheirLock();
    await IpcCancellationScenario.SubscriptionDeadlineIncludesSessionAdmission();
    await tests.TypedRequestsRequireConnectedEngine();
    await tests.ConfigurationPatchRequiresIdempotency();
    tests.SubscriptionQueueIsBounded();
    tests.SubscriptionAcknowledgementNeverRegresses();
    await tests.SupervisedSubscriptionSurvivesReattach();
    await tests.SupervisedSubscriptionReportsProjectionInvalidation();
    await tests.SupervisedSubscriptionRecoversAfterQueueOverflow();
    await tests.IntentionalStopNeverRestarts();
    await tests.UnexpectedDeathRestartsOnce();
    await tests.FourthConsecutiveFailureExhaustsRestarts();
    await tests.StaleExitCannotReplaceCurrentGeneration();
    await tests.CleanShutdownAvoidsForcedTermination();
    await tests.ShutdownTimeoutTerminatesProcessGroup();
    await IpcCancellationScenario.StalledShutdownStillCleansUpProcess();
    await tests.TerminationFailureStillCompletesCleanup();
    await tests.CallerCancellationStillContainsTheProcess();
    await tests.CleanupFailuresPreserveTheInitiatingFailure();

    if (args is ["--engine", var enginePath])
    {
        await tests.RealEngineStartsAndStopsCleanly(Path.GetFullPath(enginePath));
    }

    Console.WriteLine("Windows process supervision scenarios passed.");
}
catch (Exception error)
{
    if (error is EngineIpcException ipcError)
    {
        Console.Error.WriteLine($"IPC failure: {ipcError.Kind}; code: {ipcError.ContractError?.Code}");
    }
    Console.Error.WriteLine(error);
    Environment.ExitCode = 1;
}

internal sealed class SupervisionScenarios
{
    public async Task UnavailableEngineIsTyped()
    {
        try
        {
            await EngineIpcClient.ConnectAsync(
                $"missing-{Guid.NewGuid():N}",
                DevelopmentPeer(),
                "synthetic-token",
                TimeSpan.FromMilliseconds(20));
        }
        catch (EngineIpcException error)
        {
            Assert.Equal(
                EngineIpcFailureKind.EngineUnavailable,
                error.Kind,
                "unavailable engine failure kind");
            return;
        }

        throw new InvalidOperationException("Expected unavailable engine failure.");
    }

    public async Task TypedRequestsRequireConnectedEngine()
    {
        var fixture = new SupervisorFixture();
        await Assert.ThrowsAsync<EngineIpcException>(
            () => fixture.Supervisor.QueryAsync(Query.ForConfigGet(new GetConfiguration())),
            "typed query requires connected engine");
        await Assert.ThrowsAsync<EngineIpcException>(
            () => fixture.Supervisor.SubmitCommandAsync(
                Command.ForConfigUpdate(new UpdateConfiguration { ExpectedRevision = 1, Changes = [] }),
                Guid.NewGuid()),
            "typed configuration patch requires connected engine");
    }

    public async Task ConfigurationPatchRequiresIdempotency()
    {
        var fixture = new SupervisorFixture();
        await Assert.ThrowsAsync<ArgumentException>(
            () => fixture.Supervisor.SubmitCommandAsync(
                Command.ForConfigUpdate(new UpdateConfiguration { ExpectedRevision = 1, Changes = [] }),
                Guid.Empty),
            "typed configuration patch requires idempotency");
    }

    public void SubscriptionQueueIsBounded()
    {
        var subscription = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        for (var index = 0; index < EngineSubscription.Capacity; index++)
        {
            Assert.True(
                subscription.TryPublish(EventEnvelope(subscription.SubscriptionId)),
                "subscription queue accepts bounded item");
        }
        Assert.False(
            subscription.TryPublish(EventEnvelope(subscription.SubscriptionId)),
            "subscription queue rejects overflow");
    }

    public void SubscriptionAcknowledgementNeverRegresses()
    {
        var subscription = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        var newer = EventEnvelope(subscription.SubscriptionId, sequence: 2);
        var older = EventEnvelope(subscription.SubscriptionId, sequence: 1);

        subscription.Acknowledge(newer);
        subscription.Acknowledge(older);

        Assert.Equal(newer.Cursor, subscription.ProcessedCursor, "processed cursor remains monotonic");
    }

    public async Task SupervisedSubscriptionSurvivesReattach()
    {
        await using var supervised = new SupervisedEngineSubscription(
            Subscription.ForConfigChangedSubscribe(new ConfigurationChanges()));
        var first = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        supervised.Attach(first, resetCursor: false);
        var firstEvent = EventEnvelope(first.SubscriptionId);
        Assert.True(first.TryPublish(firstEvent), "first attachment publishes");
        var delivered = await ReadOne(supervised);
        supervised.Acknowledge(delivered);
        Assert.Equal(firstEvent.Cursor, supervised.ProcessedCursor, "processed cursor");
        first.Complete(new EngineIpcException(EngineIpcFailureKind.EngineUnavailable, "Synthetic transport loss."));

        var replacement = new EngineSubscription(Guid.NewGuid(), firstEvent.Cursor, resumed: true);
        supervised.Attach(replacement, resetCursor: false);
        var replacementEvent = EventEnvelope(replacement.SubscriptionId);
        Assert.True(replacement.TryPublish(replacementEvent), "replacement attachment publishes");
        Assert.Equal(replacementEvent.Cursor, (await ReadOne(supervised)).Cursor, "replacement event");
    }

    public async Task SupervisedSubscriptionRecoversAfterQueueOverflow()
    {
        await using var supervised = new SupervisedEngineSubscription(
            Subscription.ForConfigChangedSubscribe(new ConfigurationChanges()));
        var overflowing = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        supervised.Attach(overflowing, resetCursor: false);
        for (var index = 0; index <= EngineSubscription.Capacity; index++)
        {
            while (!overflowing.TryPublish(EventEnvelope(overflowing.SubscriptionId, index + 1)))
            {
                await Task.Yield();
            }
            await Task.Yield();
        }
        await Task.Delay(20);
        await Assert.ThrowsAsync<EngineIpcException>(
            async () =>
            {
                await foreach (var _ in supervised.ReadAllAsync())
                {
                }
            },
            "overflow completes the supervised queue");

        var replacement = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: true);
        supervised.Attach(replacement, resetCursor: false);
        var replacementEvent = EventEnvelope(replacement.SubscriptionId);
        Assert.True(replacement.TryPublish(replacementEvent), "replacement publishes after overflow");
        Assert.Equal(replacementEvent.Cursor, (await ReadOne(supervised)).Cursor, "replacement event after overflow");
    }

    public async Task SupervisedSubscriptionReportsProjectionInvalidation()
    {
        await using var supervised = new SupervisedEngineSubscription(
            Subscription.ForProductChangedSubscribe(new ProductChanges()));
        var attached = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        supervised.Attach(attached, resetCursor: false);
        attached.Complete(new EngineIpcException(
            EngineIpcFailureKind.SessionChanged, "Synthetic projection invalidation."));

        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(2));
        try
        {
            await foreach (var _ in supervised.ReadAllAsync(timeout.Token)) { }
        }
        catch (EngineIpcException error)
        {
            Assert.Equal(EngineIpcFailureKind.SessionChanged, error.Kind,
                "projection invalidation reaches the supervised consumer");
            return;
        }
        throw new InvalidOperationException("Expected supervised projection invalidation.");
    }

    private static EventEnvelope EventEnvelope(Guid subscriptionId, long sequence = 1) => new()
    {
        SubscriptionId = subscriptionId,
        CorrelationId = Guid.NewGuid(),
        Cursor = Guid.NewGuid(),
        Event = new Dictionary<string, object>
        {
            ["kind"] = Event.ConfigChangedEventKind,
            ["payload"] = new Dictionary<string, object>(),
        },
        OccurredAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
        Sequence = sequence,
    };

    private static async Task<EventEnvelope> ReadOne(SupervisedEngineSubscription subscription)
    {
        await using var enumerator = subscription.ReadAllAsync().GetAsyncEnumerator();
        Assert.True(await enumerator.MoveNextAsync(), "subscription yields event");
        return enumerator.Current;
    }

    public async Task IntentionalStopNeverRestarts()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);
        var process = fixture.Launcher.Current;

        var stop = fixture.Supervisor.StopAsync();
        Assert.True(process.InputClosed, "intentional stop closes engine stdin");
        process.Exit(0);
        await stop;

        await Task.Yield();
        Assert.Equal(1, fixture.Launcher.LaunchCount, "intentional stop must not restart");
        Assert.Equal(EngineSupervisionState.Stopped, fixture.Supervisor.Snapshot.State, "intentional stop state");
    }

    public async Task UnexpectedDeathRestartsOnce()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);

        fixture.Launcher.Current.Exit(9);
        await Eventually(() => fixture.Supervisor.Snapshot.State == EngineSupervisionState.RestartDelay
            && fixture.Clock.HasPendingDelay(TimeSpan.FromSeconds(1)));
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(1));
        await Eventually(() => fixture.Launcher.LaunchCount == 2
            && fixture.Supervisor.Snapshot.Generation == 2
            && fixture.Supervisor.Snapshot.State == EngineSupervisionState.Starting);

        Assert.Equal(2L, fixture.Supervisor.Snapshot.Generation, "replacement generation");
        await fixture.StopCurrent();
    }

    public async Task FourthConsecutiveFailureExhaustsRestarts()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);

        for (var restart = 1; restart <= 3; restart++)
        {
            fixture.Launcher.Current.Exit(20 + restart);
            var expectedDelay = TimeSpan.FromSeconds(1 << (restart - 1));
            await Eventually(() => fixture.Supervisor.Snapshot.State == EngineSupervisionState.RestartDelay
                && fixture.Clock.HasPendingDelay(expectedDelay));
            fixture.Clock.CompleteDelay(expectedDelay);
            var expectedLaunches = restart + 1;
            await Eventually(() => fixture.Launcher.LaunchCount == expectedLaunches
                && fixture.Supervisor.Snapshot.Generation == expectedLaunches
                && fixture.Supervisor.Snapshot.State == EngineSupervisionState.Starting);
        }

        fixture.Launcher.Current.Exit(24);
        await Eventually(() => fixture.Supervisor.Snapshot.State == EngineSupervisionState.RestartExhausted);

        Assert.Equal(4, fixture.Launcher.LaunchCount, "restart exhaustion launch count");
        Assert.Equal(3, fixture.Supervisor.Snapshot.RestartCount, "restart exhaustion counter");
        await fixture.Supervisor.StopAsync();
    }

    public async Task StaleExitCannotReplaceCurrentGeneration()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);
        fixture.Launcher.Current.Exit(7);
        await Eventually(() => fixture.Supervisor.Snapshot.State == EngineSupervisionState.RestartDelay
            && fixture.Clock.HasPendingDelay(TimeSpan.FromSeconds(1)));
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(1));
        await Eventually(() => fixture.Launcher.LaunchCount == 2
            && fixture.Supervisor.Snapshot.Generation == 2
            && fixture.Supervisor.Snapshot.State == EngineSupervisionState.Starting);

        await fixture.Supervisor.ObserveExitAsync(1, 99);

        Assert.Equal(2, fixture.Launcher.LaunchCount, "stale exit launch count");
        Assert.Equal(2L, fixture.Supervisor.Snapshot.Generation, "stale exit generation");
        Assert.Equal(EngineSupervisionState.Starting, fixture.Supervisor.Snapshot.State, "stale exit state");
        await fixture.StopCurrent();
    }

    public async Task CleanShutdownAvoidsForcedTermination()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);
        var process = fixture.Launcher.Current;

        var stop = fixture.Supervisor.StopAsync();
        process.Exit(0);
        await stop;

        Assert.False(fixture.Group.Terminated, "clean shutdown must not terminate job");
        Assert.False(fixture.Supervisor.Snapshot.LastExit?.Forced ?? true, "clean shutdown outcome");
    }

    public async Task ShutdownTimeoutTerminatesProcessGroup()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);

        var stop = fixture.Supervisor.StopAsync();
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(15));
        await stop;

        Assert.True(fixture.Group.Terminated, "shutdown timeout terminates job");
        Assert.True(fixture.Supervisor.Snapshot.LastExit?.Forced ?? false, "forced shutdown outcome");
    }

    public async Task TerminationFailureStillCompletesCleanup()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);
        var failure = new Win32Exception(5);
        fixture.Group.TerminationException = failure;

        var stop = fixture.Supervisor.StopAsync();
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(15));
        try
        {
            await stop.WaitAsync(TimeSpan.FromSeconds(2));
            throw new InvalidOperationException("Expected termination failure.");
        }
        catch (Win32Exception error)
        {
            Assert.True(ReferenceEquals(failure, error), "original termination failure is preserved");
        }

        Assert.True(fixture.Launcher.Current.Disposed, "failed termination still disposes process");
        Assert.True(fixture.Group.Disposed, "failed termination still closes containment");
        Assert.True(fixture.Launcher.Current.Exited, "kill-on-close fallback ends owned process");
        Assert.Equal(EngineSupervisionState.Stopped, fixture.Supervisor.Snapshot.State, "failed termination stop state");
    }

    public async Task CallerCancellationStillContainsTheProcess()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        await Assert.ThrowsAsync<OperationCanceledException>(() => fixture.Supervisor.StopAsync(cancellation.Token), "pre-cancelled stop is not admitted");
        Assert.False(fixture.Launcher.Current.InputClosed, "pre-cancelled stop leaves the session active");

        using var admittedCancellation = new CancellationTokenSource();
        var stop = fixture.Supervisor.StopAsync(admittedCancellation.Token);
        admittedCancellation.Cancel();
        Assert.False(stop.IsCompleted, "cancellation cannot abandon the process-exit wait");
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(15));
        try
        {
            await stop.WaitAsync(TimeSpan.FromSeconds(2));
            throw new InvalidOperationException("Expected caller cancellation.");
        }
        catch (OperationCanceledException error)
        {
            Assert.Equal(admittedCancellation.Token, error.CancellationToken, "stop cancellation retains caller token");
        }
        Assert.True(fixture.Launcher.Current.Disposed && fixture.Launcher.Current.Exited, "cancelled stop releases owned process");
        Assert.True(fixture.Group.Disposed && fixture.Group.Terminated, "cancelled stop retains deadline and containment");
        Assert.Equal(1, fixture.Launcher.LaunchCount, "cancelled intentional stop never restarts");
    }

    public async Task CleanupFailuresPreserveTheInitiatingFailure()
    {
        foreach (var cancel in new[] { false, true })
        {
            var fixture = new SupervisorFixture();
            await fixture.Supervisor.StartAsync(fixture.Request);
            var process = fixture.Launcher.Current;
            var initiating = new IOException("Synthetic stdin close failure.");
            var disposal = new IOException("Synthetic process disposal failure.");
            process.InputCloseException = initiating;
            process.DisposalException = disposal;
            using var cancellation = new CancellationTokenSource();
            var stop = fixture.Supervisor.StopAsync(cancellation.Token);
            Assert.True(fixture.Clock.HasPendingDelay(TimeSpan.FromSeconds(15)), "stdin failure must not skip the exit deadline");
            if (cancel) cancellation.Cancel();
            fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(15));
            try
            {
                await stop.WaitAsync(TimeSpan.FromSeconds(2));
                throw new InvalidOperationException("Expected cleanup failures.");
            }
            catch (Exception error) when (error is AggregateException or OperationCanceledException)
            {
                var failures = error as AggregateException ?? error.InnerException as AggregateException;
                Assert.True(failures is not null, "cleanup errors remain available with cancellation");
                Assert.SequenceEqual(new Exception[] { initiating, disposal }, failures!.InnerExceptions, "initiating failure precedes cleanup failure");
                if (cancel) Assert.Equal(cancellation.Token, ((OperationCanceledException)error).CancellationToken, "cleanup failure retains cancellation token");
            }
            Assert.True(process.Disposed && process.Exited && fixture.Group.Disposed, "all cleanup stages run despite earlier failures");
            Assert.Equal(EngineSupervisionState.Stopped, fixture.Supervisor.Snapshot.State, "cleanup failure reaches stopped");
            Assert.Equal(1, fixture.Launcher.LaunchCount, "failed intentional stop never restarts");
        }
    }

    public async Task RealEngineStartsAndStopsCleanly(string enginePath)
    {
        var runtimeDirectory = Path.Combine(Path.GetTempPath(), $"eitmad-supervision-{Guid.NewGuid():N}");
        Directory.CreateDirectory(runtimeDirectory);
        try
        {
            await SeedDevelopmentAccounts(enginePath, runtimeDirectory);
            await using var supervisor = new EngineSupervisor();
            var request = new EngineLaunchRequest(enginePath, runtimeDirectory);
            var lifecycleStates = new List<Eitmad.Contracts.LifecycleState>();
            supervisor.StateChanged += state =>
            {
                if (state.LastLifecycle is { } lifecycle
                    && (lifecycleStates.Count == 0 || lifecycleStates[^1] != lifecycle.State))
                {
                    lifecycleStates.Add(lifecycle.State);
                }
            };
            await supervisor.StartAsync(request);
            await Eventually(() => supervisor.Snapshot.LastLifecycle?.Ready == true, TimeSpan.FromSeconds(10));
            await Eventually(() => supervisor.IpcConnected, TimeSpan.FromSeconds(10));
            Assert.Equal(EngineIpcHealthState.Connected, supervisor.Snapshot.IpcHealth, "real engine IPC health");
            Assert.Equal(
                ProtocolIds.Version.Minor,
                supervisor.Snapshot.LastLifecycle?.Identity.ProtocolVersion.Minor,
                "real engine protocol version");
            await VerifyDisconnectedAccess(supervisor);
            await supervisor.StopAsync();

            Assert.Equal(EngineSupervisionState.Stopped, supervisor.Snapshot.State, "real engine stopped state");
            Assert.Equal(EngineIpcHealthState.Unavailable, supervisor.Snapshot.IpcHealth, "stopped IPC health");
            Assert.Equal(0, supervisor.Snapshot.LastExit?.ExitCode, "real engine exit code");
            Assert.False(supervisor.Snapshot.LastExit?.Forced ?? true, "real engine graceful exit");
            Assert.SequenceEqual(
                [
                    Eitmad.Contracts.LifecycleState.Starting,
                    Eitmad.Contracts.LifecycleState.Ready,
                    Eitmad.Contracts.LifecycleState.Stopping,
                    Eitmad.Contracts.LifecycleState.Stopped,
                ],
                lifecycleStates,
                "real engine lifecycle sequence");
            await supervisor.StartAsync(request);
            await Eventually(() => supervisor.IpcConnected, TimeSpan.FromSeconds(10));
            await VerifyDisconnectedAccess(supervisor);
            await supervisor.StopAsync();
        }
        finally
        {
            Directory.Delete(runtimeDirectory, recursive: true);
        }
    }

    private static async Task VerifyDisconnectedAccess(EngineSupervisor supervisor)
    {
        try
        {
            await supervisor.SignInAsync("admin", "admin");
            throw new InvalidOperationException("A saved verifier permitted sign-in without server authority.");
        }
        catch (EngineIpcException error)
        {
            Assert.Equal(EngineIpcFailureKind.AuthenticationRejected, error.Kind,
                "saved desktop credentials cannot grant offline access");
        }
        Assert.True(await supervisor.GetSessionStateAsync() is null,
            "failed offline sign-in leaves no active desktop session");
        var response = await supervisor.QueryAsync(Query.ForConfigGet(new GetConfiguration()));
        Assert.Equal(CommandOutcomeStatus.Failed, response.Outcome.Status,
            "protected query fails without server authority");
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorIpcSessionInvalidV1, response.Outcome.Payload.Code,
            "protected query retains the session failure code");
    }

    private static async Task SeedDevelopmentAccounts(string enginePath, string runtimeDirectory)
    {
        var start = new System.Diagnostics.ProcessStartInfo(enginePath)
        {
            CreateNoWindow = true,
            RedirectStandardError = true,
            RedirectStandardOutput = true,
            UseShellExecute = false,
        };
        start.ArgumentList.Add("seed-development-accounts");
        start.ArgumentList.Add("--runtime-directory");
        start.ArgumentList.Add(runtimeDirectory);
        using var process = System.Diagnostics.Process.Start(start)
            ?? throw new InvalidOperationException("Development account seed process did not start.");
        await process.WaitForExitAsync();
        Assert.Equal(0, process.ExitCode, "development account seed exit code");
    }

    private static PeerHello DevelopmentPeer() => new()
    {
        PeerKind = PeerKind.Shell,
        ProductVersion = "0.0.0",
        Protocols = [new SupportedProtocol { Major = 1, MinimumMinor = 0, MaximumMinor = 3 }],
        Capabilities =
        [
            ProtocolIds.Capabilities.EitmadCapabilityLocalIpcV1,
            ProtocolIds.Capabilities.EitmadCapabilityAuthorizationScopesV1,
        ],
        RequiredCapabilities =
        [
            ProtocolIds.Capabilities.EitmadCapabilityLocalIpcV1,
            ProtocolIds.Capabilities.EitmadCapabilityAuthorizationScopesV1,
        ],
        Schemas = [],
    };

    private static async Task Eventually(Func<bool> condition, TimeSpan? timeout = null)
    {
        var deadline = DateTime.UtcNow + (timeout ?? TimeSpan.FromSeconds(2));
        while (!condition())
        {
            if (DateTime.UtcNow >= deadline)
            {
                throw new InvalidOperationException("Expected condition was not reached before timeout.");
            }

            await Task.Delay(5);
        }
    }
}

internal sealed class SupervisorFixture
{
    public SupervisorFixture()
    {
        Group.Processes = Launcher.Processes;
        Supervisor = new EngineSupervisor(Launcher, new FakeProcessGroupFactory(Group), Clock);
    }

    public FakeClock Clock { get; } = new();
    public FakeProcessLauncher Launcher { get; } = new();
    public FakeProcessGroup Group { get; } = new();
    public EngineSupervisor Supervisor { get; }
    public EngineLaunchRequest Request { get; } = new("C:\\synthetic\\eitmad-engine-cli.exe");

    public async Task StopCurrent()
    {
        var process = Launcher.Current;
        var stop = Supervisor.StopAsync();
        process.Exit(0);
        await stop;
    }
}

internal sealed class FakeClock : ISupervisionClock
{
    private readonly object gate = new();
    private readonly List<PendingDelay> delays = [];

    public DateTimeOffset UtcNow { get; private set; } = new(2026, 7, 12, 0, 0, 0, TimeSpan.Zero);

    public Task DelayAsync(TimeSpan delay, CancellationToken cancellationToken)
    {
        var completion = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var registration = cancellationToken.Register(() => completion.TrySetCanceled(cancellationToken));
        lock (gate)
        {
            delays.Add(new PendingDelay(delay, completion, registration));
        }

        return completion.Task;
    }

    public void CompleteDelay(TimeSpan delay)
    {
        PendingDelay pending;
        lock (gate)
        {
            var index = delays.FindIndex(item => item.Duration == delay);
            if (index < 0)
            {
                throw new InvalidOperationException($"No pending delay for {delay}.");
            }

            pending = delays[index];
            delays.RemoveAt(index);
            UtcNow += delay;
        }

        pending.Registration.Dispose();
        pending.Completion.SetResult();
    }

    public bool HasPendingDelay(TimeSpan delay)
    {
        lock (gate)
        {
            return delays.Any(item => item.Duration == delay);
        }
    }

    private sealed record PendingDelay(
        TimeSpan Duration,
        TaskCompletionSource Completion,
        CancellationTokenRegistration Registration);
}

internal sealed class FakeProcessLauncher : IEngineProcessLauncher
{
    public string LifecycleOutput { get; set; } = string.Empty;
    public List<FakeEngineProcess> Processes { get; } = [];
    public int LaunchCount => Processes.Count;
    public FakeEngineProcess Current => Processes[^1];

    public IEngineProcess Launch(EngineLaunchRequest request, long generation, IProcessGroup group)
    {
        var process = new FakeEngineProcess(1000 + (int)generation, LifecycleOutput);
        Processes.Add(process);
        group.Assign(process);
        return process;
    }
}

internal sealed class FakeEngineProcess(int processId, string lifecycleOutput = "") : IEngineProcess
{
    private readonly TaskCompletionSource<int> exit = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private readonly TrackingWriter input = new();

    public int ProcessId { get; } = processId;
    public nint NativeHandle => 0;
    public TextReader StandardOutput { get; } = new StringReader(lifecycleOutput);
    public TextReader StandardError { get; } = new StringReader(string.Empty);
    public TextWriter StandardInput => input;
    public string IpcPipeName { get; } = $"fake-{processId}";
    public string IpcBootstrapToken { get; } = "fake-bootstrap-token";
    public bool InputClosed => input.IsClosed;
    public bool Exited => exit.Task.IsCompleted;
    public Exception? InputCloseException { get => input.CloseException; set => input.CloseException = value; }
    public Exception? DisposalException { get; set; }
    public bool Disposed { get; private set; }

    public Task<int> WaitForExitAsync(CancellationToken cancellationToken) => exit.Task.WaitAsync(cancellationToken);
    public void Kill() => Exit(137);
    public void Exit(int code) => exit.TrySetResult(code);
    public ValueTask DisposeAsync()
    {
        Disposed = true;
        if (DisposalException is { } error) throw error;
        input.Dispose();
        return ValueTask.CompletedTask;
    }

    private sealed class TrackingWriter : StringWriter
    {
        public bool IsClosed { get; private set; }
        public Exception? CloseException { get; set; }
        protected override void Dispose(bool disposing)
        {
            IsClosed = true;
            base.Dispose(disposing);
            if (CloseException is { } error) throw error;
        }
    }
}

internal sealed class FakeProcessGroupFactory(FakeProcessGroup group) : IProcessGroupFactory
{
    public IProcessGroup Create() => group;
}

internal sealed class FakeProcessGroup : IProcessGroup
{
    public List<FakeEngineProcess> Processes { get; set; } = [];
    public bool Terminated { get; private set; }
    public bool Disposed { get; private set; }
    public Exception? TerminationException { get; set; }
    public void Assign(IEngineProcess process) { }
    public void Terminate()
    {
        Terminated = true;
        if (TerminationException is { } exception)
        {
            throw exception;
        }

        foreach (var process in Processes)
        {
            process.Exit(137);
        }
    }

    public void Dispose()
    {
        Disposed = true;
        // Match the production Job Object's kill-on-close containment.
        foreach (var process in Processes) process.Exit(137);
    }
}

internal static class Assert
{
    public static void True(bool value, string message)
    {
        if (!value) throw new InvalidOperationException($"Assertion failed: {message}.");
    }

    public static void False(bool value, string message) => True(!value, message);

    public static void Equal<T>(T expected, T actual, string message)
    {
        if (!EqualityComparer<T>.Default.Equals(expected, actual))
        {
            throw new InvalidOperationException($"Assertion failed: {message}. Expected {expected}; actual {actual}.");
        }
    }

    public static void SequenceEqual<T>(IEnumerable<T> expected, IEnumerable<T> actual, string message)
    {
        if (!expected.SequenceEqual(actual))
        {
            throw new InvalidOperationException($"Assertion failed: {message}.");
        }
    }

    public static async Task ThrowsAsync<TException>(Func<Task> action, string message)
        where TException : Exception
    {
        try
        {
            await action();
        }
        catch (TException)
        {
            return;
        }

        throw new InvalidOperationException($"Assertion failed: {message}. Expected {typeof(TException).Name}.");
    }
}
