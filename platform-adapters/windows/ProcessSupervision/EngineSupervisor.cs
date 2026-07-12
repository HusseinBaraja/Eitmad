using System.ComponentModel;
using System.Text.Json;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.ProcessSupervision;

public sealed class EngineSupervisor : IAsyncDisposable
{
    private readonly object gate = new();
    private readonly IEngineProcessLauncher launcher;
    private readonly IProcessGroupFactory processGroupFactory;
    private readonly ISupervisionClock clock;
    private readonly RestartPolicy policy;
    private readonly Queue<DateTimeOffset> restartHistory = new();
    private CancellationTokenSource? sessionCancellation;
    private IProcessGroup? processGroup;
    private IEngineProcess? currentProcess;
    private EngineLaunchRequest? launchRequest;
    private Guid? currentInstanceId;
    private long generation;
    private bool stopRequested;
    private bool disposed;
    private bool forcedCurrentExit;
    private Task? currentMonitor;
    private EngineSupervisionSnapshot snapshot = EngineSupervisionSnapshot.Initial;

    public EngineSupervisor(RestartPolicy? policy = null)
    {
        if (!OperatingSystem.IsWindows())
        {
            throw new PlatformNotSupportedException("Windows engine supervision requires Windows Job Objects.");
        }

        launcher = new WindowsEngineProcessLauncher();
        processGroupFactory = new WindowsProcessGroupFactory();
        clock = new SystemSupervisionClock();
        this.policy = policy ?? RestartPolicy.Default;
    }

    internal EngineSupervisor(
        IEngineProcessLauncher launcher,
        IProcessGroupFactory processGroupFactory,
        ISupervisionClock clock,
        RestartPolicy? policy = null)
    {
        this.launcher = launcher;
        this.processGroupFactory = processGroupFactory;
        this.clock = clock;
        this.policy = policy ?? RestartPolicy.Default;
    }

    public event Action<EngineSupervisionSnapshot>? StateChanged;

    public EngineSupervisionSnapshot Snapshot
    {
        get
        {
            lock (gate)
            {
                return snapshot;
            }
        }
    }

    public Task StartAsync(EngineLaunchRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        cancellationToken.ThrowIfCancellationRequested();

        IProcessGroup? previousGroup;
        IProcessGroup? failedGroup = null;
        lock (gate)
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            if (snapshot.State is EngineSupervisionState.Starting
                or EngineSupervisionState.Running
                or EngineSupervisionState.RestartDelay
                or EngineSupervisionState.Stopping)
            {
                return Task.CompletedTask;
            }

            previousGroup = processGroup;
            sessionCancellation?.Cancel();
            sessionCancellation?.Dispose();
            sessionCancellation = new CancellationTokenSource();
            processGroup = processGroupFactory.Create();
            launchRequest = request;
            restartHistory.Clear();
            stopRequested = false;
            forcedCurrentExit = false;
            snapshot = EngineSupervisionSnapshot.Initial with { Generation = generation };
            try
            {
                LaunchLocked();
            }
            catch
            {
                failedGroup = processGroup;
                processGroup = null;
                snapshot = snapshot with { State = EngineSupervisionState.Faulted };
            }
        }

        previousGroup?.Dispose();
        failedGroup?.Dispose();
        PublishSnapshot();
        return Task.CompletedTask;
    }

    public async Task StopAsync(CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        IEngineProcess? process;
        IProcessGroup? group;
        Task? monitor;
        lock (gate)
        {
            if (snapshot.State == EngineSupervisionState.Stopped && currentProcess is null)
            {
                return;
            }

            stopRequested = true;
            sessionCancellation?.Cancel();
            process = currentProcess;
            group = processGroup;
            monitor = currentMonitor;
            snapshot = snapshot with { State = EngineSupervisionState.Stopping };
        }

        PublishSnapshot();

        if (process is null)
        {
            CompleteStop(group);
            return;
        }

        process.StandardInput.Dispose();
        var exitTask = process.WaitForExitAsync(cancellationToken);
        var timeoutTask = clock.DelayAsync(policy.ShutdownTimeout, CancellationToken.None);
        if (await Task.WhenAny(exitTask, timeoutTask).ConfigureAwait(false) == timeoutTask)
        {
            lock (gate)
            {
                if (ReferenceEquals(process, currentProcess))
                {
                    forcedCurrentExit = true;
                }
            }

            try
            {
                group?.Terminate();
            }
            catch (Win32Exception)
            {
                // Cleanup must continue even when Windows cannot terminate the Job Object.
            }
        }

        try
        {
            await exitTask.ConfigureAwait(false);
            if (monitor is not null)
            {
                await monitor.ConfigureAwait(false);
            }
        }
        finally
        {
            await process.DisposeAsync().ConfigureAwait(false);
            CompleteStop(group);
        }
    }

    internal async Task ObserveExitAsync(long observedGeneration, int exitCode)
    {
        TimeSpan? restartDelay = null;
        CancellationToken restartCancellation = default;
        lock (gate)
        {
            if (observedGeneration != generation)
            {
                return;
            }

            var intentional = stopRequested;
            snapshot = snapshot with
            {
                LastExit = new EngineExitOutcome(exitCode, intentional, forcedCurrentExit),
            };
            currentProcess = null;
            currentMonitor = null;

            if (intentional)
            {
                snapshot = snapshot with { State = EngineSupervisionState.Stopped };
            }
            else if (snapshot.LastError?.Retry?.Kind == RetryDispositionKind.Never)
            {
                snapshot = snapshot with { State = EngineSupervisionState.Faulted };
            }
            else
            {
                PruneRestartHistoryLocked();
                if (restartHistory.Count >= policy.MaximumRestarts)
                {
                    snapshot = snapshot with { State = EngineSupervisionState.RestartExhausted };
                }
                else
                {
                    restartHistory.Enqueue(clock.UtcNow);
                    var restartCount = restartHistory.Count;
                    restartDelay = CalculateRestartDelayLocked(restartCount);
                    restartCancellation = sessionCancellation?.Token ?? CancellationToken.None;
                    snapshot = snapshot with
                    {
                        State = EngineSupervisionState.RestartDelay,
                        RestartCount = restartCount,
                    };
                }
            }
        }

        PublishSnapshot();
        if (restartDelay is null)
        {
            return;
        }

        try
        {
            await clock.DelayAsync(restartDelay.Value, restartCancellation).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (restartCancellation.IsCancellationRequested)
        {
            return;
        }

        lock (gate)
        {
            if (observedGeneration != generation
                || stopRequested
                || snapshot.State != EngineSupervisionState.RestartDelay)
            {
                return;
            }

            try
            {
                LaunchLocked();
            }
            catch
            {
                snapshot = snapshot with { State = EngineSupervisionState.Faulted };
            }
        }

        PublishSnapshot();
    }

    public async ValueTask DisposeAsync()
    {
        if (disposed)
        {
            return;
        }

        await StopAsync().ConfigureAwait(false);
        lock (gate)
        {
            disposed = true;
            sessionCancellation?.Dispose();
            sessionCancellation = null;
        }
    }

    private void LaunchLocked()
    {
        var request = launchRequest ?? throw new InvalidOperationException("No engine launch request is active.");
        var group = processGroup ?? throw new InvalidOperationException("No process group is active.");
        generation++;
        currentInstanceId = null;
        forcedCurrentExit = false;
        var process = launcher.Launch(request, generation, group);
        currentProcess = process;
        snapshot = snapshot with
        {
            State = EngineSupervisionState.Starting,
            Generation = generation,
            LastLifecycle = null,
            LastError = null,
        };
        var monitoredGeneration = generation;
        currentMonitor = MonitorProcessAsync(process, monitoredGeneration);
    }

    private async Task MonitorProcessAsync(IEngineProcess process, long monitoredGeneration)
    {
        var stdout = ReadLifecycleOutputAsync(process.StandardOutput, monitoredGeneration);
        var stderr = ReadStructuredErrorsAsync(process.StandardError, monitoredGeneration);
        var exitCode = await process.WaitForExitAsync(CancellationToken.None).ConfigureAwait(false);
        try
        {
            await Task.WhenAll(stdout, stderr).ConfigureAwait(false);
        }
        catch (IOException)
        {
            // Process exit may close a redirected stream while a reader is completing.
        }
        catch (ObjectDisposedException)
        {
            // Forced termination may dispose a redirected stream before EOF is observed.
        }

        await ObserveExitAsync(monitoredGeneration, exitCode).ConfigureAwait(false);
    }

    private async Task ReadLifecycleOutputAsync(TextReader reader, long observedGeneration)
    {
        while (await reader.ReadLineAsync().ConfigureAwait(false) is { } line)
        {
            LifecycleSnapshot? lifecycle;
            try
            {
                lifecycle = JsonSerializer.Deserialize<LifecycleSnapshot>(line, Converter.Settings);
            }
            catch (JsonException)
            {
                continue;
            }

            if (lifecycle?.Identity is not null)
            {
                ObserveLifecycle(observedGeneration, lifecycle);
            }
        }
    }

    private async Task ReadStructuredErrorsAsync(TextReader reader, long observedGeneration)
    {
        while (await reader.ReadLineAsync().ConfigureAwait(false) is { } line)
        {
            ContractError? error;
            try
            {
                error = JsonSerializer.Deserialize<ContractError>(line, Converter.Settings);
            }
            catch (JsonException)
            {
                continue;
            }

            if (error is null)
            {
                continue;
            }

            lock (gate)
            {
                if (observedGeneration == generation)
                {
                    snapshot = snapshot with { LastError = error };
                }
            }

            PublishSnapshot();
        }
    }

    private void ObserveLifecycle(long observedGeneration, LifecycleSnapshot lifecycle)
    {
        var scheduleStableReset = false;
        CancellationToken resetCancellation = default;
        lock (gate)
        {
            if (observedGeneration != generation)
            {
                return;
            }

            if (currentInstanceId is { } instanceId && instanceId != lifecycle.Identity.InstanceId)
            {
                return;
            }

            currentInstanceId ??= lifecycle.Identity.InstanceId;
            var state = lifecycle.State == LifecycleState.Ready
                ? EngineSupervisionState.Running
                : snapshot.State;
            snapshot = snapshot with
            {
                State = state,
                LastLifecycle = lifecycle,
                LastError = lifecycle.Error ?? snapshot.LastError,
            };
            if (lifecycle.State == LifecycleState.Ready)
            {
                scheduleStableReset = true;
                resetCancellation = sessionCancellation?.Token ?? CancellationToken.None;
            }
        }

        PublishSnapshot();
        if (scheduleStableReset)
        {
            _ = ResetRestartBudgetAfterStableReadyAsync(observedGeneration, resetCancellation);
        }
    }

    private async Task ResetRestartBudgetAfterStableReadyAsync(
        long readyGeneration,
        CancellationToken cancellationToken)
    {
        try
        {
            await clock.DelayAsync(policy.StableReadyDuration, cancellationToken).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return;
        }

        lock (gate)
        {
            if (readyGeneration != generation || snapshot.LastLifecycle?.State != LifecycleState.Ready)
            {
                return;
            }

            restartHistory.Clear();
            snapshot = snapshot with { RestartCount = 0 };
        }

        PublishSnapshot();
    }

    private TimeSpan CalculateRestartDelayLocked(int restartCount)
    {
        var policyDelay = policy.RestartDelays[restartCount - 1];
        if (snapshot.LastError?.Retry is { Kind: RetryDispositionKind.SafeAfterDelay, RetryAfterMs: { } milliseconds })
        {
            var contractDelay = TimeSpan.FromMilliseconds(milliseconds);
            return contractDelay > policyDelay ? contractDelay : policyDelay;
        }

        return policyDelay;
    }

    private void PruneRestartHistoryLocked()
    {
        var threshold = clock.UtcNow - policy.RestartWindow;
        while (restartHistory.TryPeek(out var attempt) && attempt < threshold)
        {
            restartHistory.Dequeue();
        }
    }

    private void CompleteStop(IProcessGroup? group)
    {
        lock (gate)
        {
            if (ReferenceEquals(group, processGroup))
            {
                processGroup = null;
            }

            currentProcess = null;
            currentMonitor = null;
            snapshot = snapshot with { State = EngineSupervisionState.Stopped };
        }

        group?.Dispose();
        PublishSnapshot();
    }

    private void PublishSnapshot()
    {
        EngineSupervisionSnapshot current;
        Action<EngineSupervisionSnapshot>? handler;
        lock (gate)
        {
            current = snapshot;
            handler = StateChanged;
        }

        handler?.Invoke(current);
    }
}
