using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;

namespace Eitmad.WindowsShell.Features.Operations;

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
    Task StartAsync(EngineLaunchRequest request, CancellationToken cancellationToken = default);
    Task StopAsync(CancellationToken cancellationToken = default);
    Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default);
    Task<CommandResponseEnvelope> SubmitConfigurationPatchAsync(UpdateConfiguration patch, Guid idempotencyKey, CancellationToken cancellationToken = default);
    Task<IEngineSubscription> SubscribeAsync(Subscription subscription, CancellationToken cancellationToken = default);
}

public sealed class WindowsEngineBridge(EngineSupervisor supervisor) : IEngineShellBridge
{
    public event Action<EngineSupervisionSnapshot>? StateChanged
    {
        add => supervisor.StateChanged += value;
        remove => supervisor.StateChanged -= value;
    }

    public EngineSupervisionSnapshot Snapshot => supervisor.Snapshot;

    public Task StartAsync(EngineLaunchRequest request, CancellationToken cancellationToken = default) =>
        supervisor.StartAsync(request, cancellationToken);

    public Task StopAsync(CancellationToken cancellationToken = default) => supervisor.StopAsync(cancellationToken);

    public Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default) =>
        supervisor.QueryAsync(query, cancellationToken: cancellationToken);

    public Task<CommandResponseEnvelope> SubmitConfigurationPatchAsync(
        UpdateConfiguration patch,
        Guid idempotencyKey,
        CancellationToken cancellationToken = default) =>
        supervisor.SubmitConfigurationPatchAsync(patch, idempotencyKey, cancellationToken: cancellationToken);

    public async Task<IEngineSubscription> SubscribeAsync(
        Subscription subscription,
        CancellationToken cancellationToken = default) =>
        new WindowsEngineSubscription(await supervisor.SubscribeAsync(subscription, cancellationToken));

    public ValueTask DisposeAsync() => supervisor.DisposeAsync();

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

public interface IShellDispatcher
{
    void Invoke(Action action);
}
