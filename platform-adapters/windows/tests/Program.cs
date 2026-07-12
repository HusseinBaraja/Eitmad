using System.ComponentModel;
using Eitmad.Platform.Windows.ProcessSupervision;

var tests = new SupervisionScenarios();
await tests.IntentionalStopNeverRestarts();
await tests.UnexpectedDeathRestartsOnce();
await tests.FourthConsecutiveFailureExhaustsRestarts();
await tests.StaleExitCannotReplaceCurrentGeneration();
await tests.CleanShutdownAvoidsForcedTermination();
await tests.ShutdownTimeoutTerminatesProcessGroup();
await tests.TerminationFailureStillCompletesCleanup();

if (args is ["--engine", var enginePath])
{
    await tests.RealEngineStartsAndStopsCleanly(enginePath);
}

Console.WriteLine("Windows process supervision scenarios passed.");

internal sealed class SupervisionScenarios
{
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
        fixture.Group.TerminationException = new Win32Exception(5);
        using var cancellation = new CancellationTokenSource();

        var stop = fixture.Supervisor.StopAsync(cancellation.Token);
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(15));
        cancellation.Cancel();
        await Assert.ThrowsAsync<OperationCanceledException>(() => stop, "cancel stalled shutdown");

        Assert.True(fixture.Launcher.Current.Disposed, "failed termination still disposes process");
        Assert.Equal(EngineSupervisionState.Stopped, fixture.Supervisor.Snapshot.State, "failed termination stop state");
    }

    public async Task RealEngineStartsAndStopsCleanly(string enginePath)
    {
        var runtimeDirectory = Path.Combine(Path.GetTempPath(), $"eitmad-supervision-{Guid.NewGuid():N}");
        Directory.CreateDirectory(runtimeDirectory);
        try
        {
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
            await supervisor.StopAsync();

            Assert.Equal(EngineSupervisionState.Stopped, supervisor.Snapshot.State, "real engine stopped state");
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
        }
        finally
        {
            Directory.Delete(runtimeDirectory, recursive: true);
        }
    }

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
    public List<FakeEngineProcess> Processes { get; } = [];
    public int LaunchCount => Processes.Count;
    public FakeEngineProcess Current => Processes[^1];

    public IEngineProcess Launch(EngineLaunchRequest request, long generation, IProcessGroup group)
    {
        var process = new FakeEngineProcess(1000 + (int)generation);
        Processes.Add(process);
        group.Assign(process);
        return process;
    }
}

internal sealed class FakeEngineProcess(int processId) : IEngineProcess
{
    private readonly TaskCompletionSource<int> exit = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private readonly TrackingWriter input = new();

    public int ProcessId { get; } = processId;
    public nint NativeHandle => 0;
    public TextReader StandardOutput { get; } = new StringReader(string.Empty);
    public TextReader StandardError { get; } = new StringReader(string.Empty);
    public TextWriter StandardInput => input;
    public bool InputClosed => input.IsClosed;
    public bool Disposed { get; private set; }

    public Task<int> WaitForExitAsync(CancellationToken cancellationToken) => exit.Task.WaitAsync(cancellationToken);
    public void Kill() => Exit(137);
    public void Exit(int code) => exit.TrySetResult(code);
    public ValueTask DisposeAsync()
    {
        Disposed = true;
        input.Dispose();
        return ValueTask.CompletedTask;
    }

    private sealed class TrackingWriter : StringWriter
    {
        public bool IsClosed { get; private set; }
        protected override void Dispose(bool disposing)
        {
            IsClosed = true;
            base.Dispose(disposing);
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

    public void Dispose() { }
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
