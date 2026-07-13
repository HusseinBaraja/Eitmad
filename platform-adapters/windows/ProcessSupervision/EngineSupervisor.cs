using System.ComponentModel;
using System.Diagnostics;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;

namespace Eitmad.Platform.Windows.ProcessSupervision;

public sealed class EngineSupervisor : IAsyncDisposable
{
    private readonly object gate = new();
    private readonly IEngineProcessLauncher launcher;
    private readonly IProcessGroupFactory processGroupFactory;
    private readonly ISupervisionClock clock;
    private readonly RestartPolicy policy;
    private readonly Queue<DateTimeOffset> restartHistory = new();
    private readonly Dictionary<Guid, SupervisedEngineSubscription> subscriptions = new();
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
    private EngineIpcClient? currentIpcClient;
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

    public bool IpcConnected
    {
        get
        {
            lock (gate)
            {
                return currentIpcClient is not null;
            }
        }
    }

    public async Task<SupervisedEngineSubscription> SubscribeAsync(
        Subscription contract,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(contract);
        EngineIpcClient client;
        long observedGeneration;
        var subscription = new SupervisedEngineSubscription(contract);
        lock (gate)
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            client = currentIpcClient
                ?? throw new EngineIpcException(
                    EngineIpcFailureKind.EngineUnavailable,
                    "The supervised engine has no active IPC session.");
            observedGeneration = generation;
            subscriptions.Add(subscription.RegistrationId, subscription);
        }

        try
        {
            await AttachSubscriptionAsync(
                client,
                subscription,
                observedGeneration,
                cancellationToken).ConfigureAwait(false);
            return subscription;
        }
        catch
        {
            lock (gate)
            {
                subscriptions.Remove(subscription.RegistrationId);
            }
            await subscription.DisposeAsync().ConfigureAwait(false);
            throw;
        }
    }

    public async Task UnsubscribeAsync(
        SupervisedEngineSubscription subscription,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(subscription);
        EngineIpcClient? client;
        EngineSubscription? attached;
        lock (gate)
        {
            subscriptions.Remove(subscription.RegistrationId);
            client = currentIpcClient;
            attached = subscription.Attached;
        }
        try
        {
            if (client is not null && attached is not null)
            {
                await client.UnsubscribeAsync(attached, cancellationToken: cancellationToken)
                    .ConfigureAwait(false);
            }
        }
        finally
        {
            await subscription.DisposeAsync().ConfigureAwait(false);
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
        EngineIpcClient? ipcClient;
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
            ipcClient = currentIpcClient;
            currentIpcClient = null;
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

        if (ipcClient is not null)
        {
            using var ipcShutdown = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            ipcShutdown.CancelAfter(TimeSpan.FromSeconds(2));
            try
            {
                await ipcClient.RequestShutdownAsync(
                    TimeSpan.FromSeconds(2),
                    ipcShutdown.Token).ConfigureAwait(false);
            }
            catch (EngineIpcException)
            {
                // Inherited stdin remains the graceful abandonment fallback.
            }
            finally
            {
                await ipcClient.DisposeAsync().ConfigureAwait(false);
            }
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
        SupervisedEngineSubscription[] activeSubscriptions;
        lock (gate)
        {
            disposed = true;
            sessionCancellation?.Dispose();
            sessionCancellation = null;
            activeSubscriptions = subscriptions.Values.ToArray();
            subscriptions.Clear();
        }
        foreach (var subscription in activeSubscriptions)
        {
            await subscription.DisposeAsync().ConfigureAwait(false);
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
            StartIpcConnection(observedGeneration);
            _ = ResetRestartBudgetAfterStableReadyAsync(observedGeneration, resetCancellation)
                .ContinueWith(
                    static task => Trace.TraceError(
                        "Stable-ready restart reset failed: {0}",
                        task.Exception?.GetBaseException()),
                    CancellationToken.None,
                    TaskContinuationOptions.OnlyOnFaulted | TaskContinuationOptions.ExecuteSynchronously,
                    TaskScheduler.Default);
        }
    }

    private void StartIpcConnection(long observedGeneration)
    {
        IEngineProcess? process;
        DevelopmentIdentityAssertion? identity;
        lock (gate)
        {
            process = observedGeneration == generation ? currentProcess : null;
            identity = launchRequest?.DevelopmentIdentity;
        }

        if (process is null || identity is null)
        {
            return;
        }

        _ = ConnectIpcAsync(process, identity, observedGeneration).ContinueWith(
            static task => Trace.TraceError(
                "Engine IPC connection failed: {0}",
                task.Exception?.GetBaseException()),
            CancellationToken.None,
            TaskContinuationOptions.OnlyOnFaulted | TaskContinuationOptions.ExecuteSynchronously,
            TaskScheduler.Default);
    }

    private async Task ConnectIpcAsync(
        IEngineProcess process,
        DevelopmentIdentityAssertion identity,
        long observedGeneration)
    {
        var peer = new PeerHello
        {
            PeerKind = PeerKind.Shell,
            ProductVersion = "0.0.0",
            Protocols = [new SupportedProtocol { Major = 1, MinimumMinor = 0, MaximumMinor = 1 }],
            Capabilities =
            [
                ProtocolIds.Capabilities.EitmadCapabilityLocalIpcV1,
                ProtocolIds.Capabilities.EitmadCapabilityLocalIpcSubscriptionsV1,
            ],
            RequiredCapabilities = [ProtocolIds.Capabilities.EitmadCapabilityLocalIpcV1],
            Schemas = [],
        };
        var client = await EngineIpcClient.ConnectAsync(
            process.IpcPipeName,
            peer,
            identity,
            process.DevelopmentBearerToken).ConfigureAwait(false);
        var keep = false;
        SupervisedEngineSubscription[] desired = [];
        lock (gate)
        {
            if (observedGeneration == generation
                && ReferenceEquals(process, currentProcess)
                && !stopRequested)
            {
                currentIpcClient = client;
                desired = subscriptions.Values.ToArray();
                keep = true;
            }
        }

        if (!keep)
        {
            await client.DisposeAsync().ConfigureAwait(false);
            return;
        }

        foreach (var subscription in desired)
        {
            try
            {
                await AttachSubscriptionAsync(
                    client,
                    subscription,
                    observedGeneration,
                    CancellationToken.None).ConfigureAwait(false);
            }
            catch (EngineIpcException error)
            {
                Trace.TraceError("Engine subscription restore failed: {0}", error.Kind);
            }
        }
        _ = ObserveIpcCompletionAsync(client, process, identity, observedGeneration);
    }

    private async Task AttachSubscriptionAsync(
        EngineIpcClient client,
        SupervisedEngineSubscription subscription,
        long observedGeneration,
        CancellationToken cancellationToken)
    {
        var resumeAfter = subscription.ProcessedCursor;
        EngineSubscription attached;
        var resetCursor = false;
        try
        {
            attached = await client.SubscribeAsync(
                CreateSubscriptionEnvelope(client, subscription.Contract, resumeAfter),
                cancellationToken: cancellationToken).ConfigureAwait(false);
        }
        catch (EngineIpcException error) when (error.Kind == EngineIpcFailureKind.ResyncRequired)
        {
            subscription.SignalResyncRequired();
            attached = await client.SubscribeAsync(
                CreateSubscriptionEnvelope(client, subscription.Contract, null),
                cancellationToken: cancellationToken).ConfigureAwait(false);
            resetCursor = true;
        }

        bool stale;
        lock (gate)
        {
            stale = observedGeneration != generation
                || !ReferenceEquals(client, currentIpcClient)
                || !subscriptions.ContainsKey(subscription.RegistrationId);
        }
        if (stale)
        {
            await client.UnsubscribeAsync(attached, cancellationToken: cancellationToken)
                .ConfigureAwait(false);
            return;
        }
        subscription.Attach(attached, resetCursor);
    }

    private static SubscriptionEnvelope CreateSubscriptionEnvelope(
        EngineIpcClient client,
        Subscription contract,
        Guid? resumeAfter) => new()
        {
            ProtocolVersion = new ProtocolVersion
            {
                Major = client.NegotiatedSession.Protocol.Major,
                Minor = client.NegotiatedSession.Protocol.Minor,
            },
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Authorization = client.Authorization,
            Subscription = contract,
            ResumeAfter = resumeAfter,
        };

    private async Task ObserveIpcCompletionAsync(
        EngineIpcClient client,
        IEngineProcess process,
        DevelopmentIdentityAssertion identity,
        long observedGeneration)
    {
        await client.Completion.ConfigureAwait(false);
        var reconnect = false;
        lock (gate)
        {
            if (!ReferenceEquals(client, currentIpcClient))
            {
                return;
            }
            currentIpcClient = null;
            reconnect = observedGeneration == generation
                && !stopRequested
                && snapshot.LastLifecycle?.State == LifecycleState.Ready;
        }
        await client.DisposeAsync().ConfigureAwait(false);
        if (!reconnect)
        {
            return;
        }

        var delays = new[]
        {
            TimeSpan.FromMilliseconds(100),
            TimeSpan.FromMilliseconds(500),
            TimeSpan.FromSeconds(2),
            TimeSpan.FromSeconds(5),
        };
        var attempt = 0;
        while (true)
        {
            var delay = delays[Math.Min(attempt, delays.Length - 1)];
            await Task.Delay(delay).ConfigureAwait(false);
            lock (gate)
            {
                if (observedGeneration != generation || stopRequested || currentIpcClient is not null)
                {
                    return;
                }
            }
            try
            {
                await ConnectIpcAsync(process, identity, observedGeneration).ConfigureAwait(false);
                return;
            }
            catch (EngineIpcException)
            {
                attempt++;
            }
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
