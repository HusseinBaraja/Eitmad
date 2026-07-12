namespace Eitmad.Platform.Windows.ProcessSupervision;

internal interface ISupervisionClock
{
    DateTimeOffset UtcNow { get; }
    Task DelayAsync(TimeSpan delay, CancellationToken cancellationToken);
}

internal interface IEngineProcessLauncher
{
    IEngineProcess Launch(EngineLaunchRequest request, long generation, IProcessGroup group);
}

internal interface IEngineProcess : IAsyncDisposable
{
    int ProcessId { get; }
    nint NativeHandle { get; }
    TextReader StandardOutput { get; }
    TextReader StandardError { get; }
    TextWriter StandardInput { get; }
    Task<int> WaitForExitAsync(CancellationToken cancellationToken);
    void Kill();
}

internal interface IProcessGroupFactory
{
    IProcessGroup Create();
}

internal interface IProcessGroup : IDisposable
{
    void Assign(IEngineProcess process);
    void Terminate();
}

internal sealed class SystemSupervisionClock : ISupervisionClock
{
    public DateTimeOffset UtcNow => DateTimeOffset.UtcNow;
    public Task DelayAsync(TimeSpan delay, CancellationToken cancellationToken) => Task.Delay(delay, cancellationToken);
}
