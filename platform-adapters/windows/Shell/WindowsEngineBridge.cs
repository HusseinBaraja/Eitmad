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
    Task<IEngineSubscription> SubscribeAsync(Subscription subscription, CancellationToken cancellationToken = default);
}

public sealed class WindowsEngineBridge : IEngineShellBridge
{
    private static readonly Guid DevelopmentPrincipal = new("a0a8e326-d0ba-4e96-bc91-e486b53da9c2");
    private static readonly Guid DevelopmentService = new("2df1c605-133c-4a3f-b80a-b3333db18198");
    private static readonly Guid DevelopmentScope = new("a1588e27-2f73-4ff4-a316-0eb0ad7145c7");
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
            new EngineLaunchRequest(ResolveEnginePath(arguments), runtimeDirectory, CreateDevelopmentIdentity()));
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

        var environmentPath = Environment.GetEnvironmentVariable("EITMAD_ENGINE_PATH");
        return string.IsNullOrWhiteSpace(environmentPath)
            ? Path.Combine(AppContext.BaseDirectory, "eitmad-engine-cli.exe")
            : Path.GetFullPath(environmentPath);
    }

    private static DevelopmentIdentityAssertion CreateDevelopmentIdentity() => new()
    {
        TenantId = DevelopmentScope,
        Identity = new AuthenticatedIdentity
        {
            PrincipalId = DevelopmentPrincipal,
            PrincipalKind = PrincipalKind.Service,
            ServiceId = DevelopmentService,
        },
        Scope = new ScopeRef { Kind = "organization", Id = DevelopmentScope },
    };

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
