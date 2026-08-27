using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;

namespace Eitmad.Platform.Windows.Shell;

public interface IEngineSubscription : IAsyncDisposable
{
    event Action? ResyncRequired;
    IAsyncEnumerable<EventEnvelope> ReadAllAsync(CancellationToken cancellationToken = default);
    void Acknowledge(EventEnvelope delivered);
}

public interface IEngineShellBridge : IAsyncDisposable
{
    event Action<EngineSupervisionSnapshot>? StateChanged;
    EngineSupervisionSnapshot Snapshot { get; }
    Task StartAsync(CancellationToken cancellationToken = default);
    Task StopAsync(CancellationToken cancellationToken = default);
    Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default);
    Task<CommandResponseEnvelope> SubmitConfigurationPatchAsync(UpdateConfiguration patch, Guid idempotencyKey, CancellationToken cancellationToken = default);
    Task<CommandResponseEnvelope> SubmitReferenceMarkerAsync(UpsertReferenceMarker marker, Guid idempotencyKey, CancellationToken cancellationToken = default);
    Task<IEngineSubscription> SubscribeAsync(Subscription subscription, CancellationToken cancellationToken = default);
}

public sealed class WindowsEngineBridge : IEngineShellBridge
{
    private readonly EngineSupervisor supervisor;
    private readonly EngineLaunchRequest launchRequest;

    private WindowsEngineBridge(EngineSupervisor supervisor, EngineLaunchRequest launchRequest)
    {
        this.supervisor = supervisor;
        this.launchRequest = launchRequest;
    }

    public event Action<EngineSupervisionSnapshot>? StateChanged
    {
        add => supervisor.StateChanged += value;
        remove => supervisor.StateChanged -= value;
    }

    public EngineSupervisionSnapshot Snapshot => supervisor.Snapshot;

    public static WindowsEngineBridge Create(IReadOnlyList<string> arguments)
    {
        var runtimeDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Eitmad",
            "engine");
        return new WindowsEngineBridge(
            new EngineSupervisor(),
            new EngineLaunchRequest(ResolveEnginePath(arguments), runtimeDirectory));
    }

    public Task StartAsync(CancellationToken cancellationToken = default) =>
        supervisor.StartAsync(launchRequest, cancellationToken);

    public Task StopAsync(CancellationToken cancellationToken = default) => supervisor.StopAsync(cancellationToken);

    public Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default) =>
        supervisor.QueryAsync(query, cancellationToken: cancellationToken);

    public Task<CommandResponseEnvelope> SubmitConfigurationPatchAsync(
        UpdateConfiguration patch,
        Guid idempotencyKey,
        CancellationToken cancellationToken = default) =>
        supervisor.SubmitConfigurationPatchAsync(patch, idempotencyKey, cancellationToken: cancellationToken);

    public Task<CommandResponseEnvelope> SubmitReferenceMarkerAsync(
        UpsertReferenceMarker marker,
        Guid idempotencyKey,
        CancellationToken cancellationToken = default) =>
        supervisor.SubmitReferenceMarkerAsync(marker, idempotencyKey, cancellationToken: cancellationToken);

    public async Task<IEngineSubscription> SubscribeAsync(
        Subscription subscription,
        CancellationToken cancellationToken = default) =>
        new WindowsEngineSubscription(await supervisor.SubscribeAsync(subscription, cancellationToken));

    public ValueTask DisposeAsync() => supervisor.DisposeAsync();

    private static string ResolveEnginePath(IReadOnlyList<string> arguments)
    {
        var engineArgument = arguments
            .Select((value, index) => (value, index))
            .FirstOrDefault(item => item.value == "--engine");
        if (engineArgument.value is not null && engineArgument.index + 1 < arguments.Count)
        {
            return Path.GetFullPath(arguments[engineArgument.index + 1]);
        }

        return Path.Combine(AppContext.BaseDirectory, "eitmad-engine-cli.exe");
    }

    private sealed class WindowsEngineSubscription(SupervisedEngineSubscription subscription) : IEngineSubscription
    {
        public event Action? ResyncRequired
        {
            add => subscription.ResyncRequired += value;
            remove => subscription.ResyncRequired -= value;
        }

        public IAsyncEnumerable<EventEnvelope> ReadAllAsync(CancellationToken cancellationToken = default) =>
            subscription.ReadAllAsync(cancellationToken);

        public void Acknowledge(EventEnvelope delivered) => subscription.Acknowledge(delivered);

        public ValueTask DisposeAsync() => subscription.DisposeAsync();
    }
}
