using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.ProcessSupervision;

public enum EngineSupervisionState
{
    Stopped,
    Starting,
    Running,
    RestartDelay,
    RestartExhausted,
    Stopping,
    Faulted,
}

public sealed record EngineLaunchRequest
{
    public EngineLaunchRequest(string engineExecutablePath, string? runtimeDirectory = null)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(engineExecutablePath);
        EngineExecutablePath = engineExecutablePath;
        RuntimeDirectory = runtimeDirectory;
    }

    public string EngineExecutablePath { get; }
    public string? RuntimeDirectory { get; }
}

public sealed class RestartPolicy
{
    public static RestartPolicy Default { get; } = new(
        3,
        TimeSpan.FromMinutes(1),
        [TimeSpan.FromSeconds(1), TimeSpan.FromSeconds(2), TimeSpan.FromSeconds(4)],
        TimeSpan.FromMinutes(5),
        TimeSpan.FromSeconds(15));

    public RestartPolicy(
        int maximumRestarts,
        TimeSpan restartWindow,
        IReadOnlyList<TimeSpan> restartDelays,
        TimeSpan stableReadyDuration,
        TimeSpan shutdownTimeout)
    {
        ArgumentNullException.ThrowIfNull(restartDelays);
        if (maximumRestarts <= 0 || restartDelays.Count != maximumRestarts)
        {
            throw new ArgumentOutOfRangeException(
                nameof(maximumRestarts),
                "Restart delays must contain exactly one positive delay per allowed restart.");
        }

        if (restartWindow <= TimeSpan.Zero || stableReadyDuration <= TimeSpan.Zero || shutdownTimeout <= TimeSpan.Zero
            || restartDelays.Any(delay => delay <= TimeSpan.Zero))
        {
            throw new ArgumentOutOfRangeException(nameof(restartDelays), "Supervision durations must be positive.");
        }

        MaximumRestarts = maximumRestarts;
        RestartWindow = restartWindow;
        RestartDelays = restartDelays.ToArray();
        StableReadyDuration = stableReadyDuration;
        ShutdownTimeout = shutdownTimeout;
    }

    public int MaximumRestarts { get; }
    public TimeSpan RestartWindow { get; }
    public IReadOnlyList<TimeSpan> RestartDelays { get; }
    public TimeSpan StableReadyDuration { get; }
    public TimeSpan ShutdownTimeout { get; }
}

public sealed record EngineExitOutcome(int ExitCode, bool Intentional, bool Forced);

public sealed record EngineSupervisionSnapshot(
    EngineSupervisionState State,
    long Generation,
    int RestartCount,
    LifecycleSnapshot? LastLifecycle,
    ContractError? LastError,
    EngineExitOutcome? LastExit)
{
    internal static EngineSupervisionSnapshot Initial { get; } = new(
        EngineSupervisionState.Stopped,
        0,
        0,
        null,
        null,
        null);
}
