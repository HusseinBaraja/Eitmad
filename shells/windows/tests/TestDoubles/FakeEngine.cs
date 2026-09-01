using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Tests.TestDoubles;

internal sealed class FakeEngine : IEngineShellBridge
{
    private readonly Dictionary<string, FakeSubscription> subscriptions = [];
    private readonly HashSet<string> queriedKinds = [];
    private readonly object stateLock = new();
    private EngineSupervisionSnapshot snapshot = new(
        EngineSupervisionState.Stopped,
        0,
        0,
        EngineIpcHealthState.Unavailable,
        null,
        null,
        null);
    private int queryCount;
    private int stopCount;

    public event Action<EngineSupervisionSnapshot>? StateChanged;

    public EngineSupervisionSnapshot Snapshot
    {
        get
        {
            lock (stateLock)
            {
                return snapshot;
            }
        }
    }

    public bool FailConfigurationQuery { get; init; }
    public int QueryCount => Volatile.Read(ref queryCount);
    public int SubscriptionCount
    {
        get
        {
            lock (stateLock)
            {
                return subscriptions.Count;
            }
        }
    }

    public int StopCount
    {
        get
        {
            lock (stateLock)
            {
                return stopCount;
            }
        }
    }
    public IReadOnlySet<string> SupportedCapabilities { get; init; } = new HashSet<string>
    {
        ProtocolIds.Capabilities.EitmadCapabilityConfigV1,
        ProtocolIds.Capabilities.EitmadCapabilitySyncV1,
        ProtocolIds.Capabilities.EitmadCapabilityUpdateV1,
        ProtocolIds.Capabilities.EitmadCapabilityReferenceMarkerV1,
    };

    public bool SupportsCapability(string capability) => SupportedCapabilities.Contains(capability);

    public bool WasQueried(string kind)
    {
        lock (queriedKinds)
        {
            return queriedKinds.Contains(kind);
        }
    }

    public Task StartAsync(CancellationToken cancellationToken = default)
    {
        EngineSupervisionSnapshot current;
        lock (stateLock)
        {
            current = snapshot = snapshot with
            {
                State = EngineSupervisionState.Starting,
                Generation = snapshot.Generation + 1,
            };
        }

        StateChanged?.Invoke(current);
        return Task.CompletedTask;
    }

    public Task StopAsync(CancellationToken cancellationToken = default)
    {
        EngineSupervisionSnapshot current;
        lock (stateLock)
        {
            stopCount++;
            current = snapshot = snapshot with
            {
                State = EngineSupervisionState.Stopped,
                IpcHealth = EngineIpcHealthState.Unavailable,
            };
        }

        StateChanged?.Invoke(current);
        return Task.CompletedTask;
    }

    public Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default)
    {
        Interlocked.Increment(ref queryCount);
        lock (queriedKinds)
        {
            queriedKinds.Add(query.Kind);
        }

        if (FailConfigurationQuery && query.Kind == Query.ConfigGetKind)
        {
            return Task.FromResult(new QueryResponseEnvelope
            {
                RequestId = Guid.NewGuid(),
                CorrelationId = Guid.NewGuid(),
                Outcome = new QueryOutcome
                {
                    Status = CommandOutcomeStatus.Failed,
                    Payload = new QueryResult { Code = "CONFIG_UNAVAILABLE" },
                },
            });
        }

        var result = query.Kind switch
        {
            Query.ConfigGetKind => QueryResult.ForConfiguration(Configuration()),
            Query.SyncGetStatusKind => QueryResult.ForSyncStatus(new SyncStatus
            {
                Kind = SyncStatusKind.Current,
                Payload = new SyncStatusPayload(),
            }),
            Query.UpdateGetStateKind => QueryResult.ForUpdateState(new UpdateState
            {
                Kind = UpdateStateKind.Idle,
                Payload = new UpdateStatePayload(),
            }),
            Query.ReferenceMarkerListKind => QueryResult.ForReferenceMarkers(new ReferenceMarkerPage { Items = [] }),
            _ => throw new InvalidOperationException("Unexpected fake query."),
        };
        return Task.FromResult(new QueryResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = result },
        });
    }

    public Task<CommandResponseEnvelope> SubmitConfigurationPatchAsync(
        UpdateConfiguration patch,
        Guid idempotencyKey,
        CancellationToken cancellationToken = default) =>
        Task.FromResult(new CommandResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = new CommandResult() },
        });

    public Task<CommandResponseEnvelope> SubmitReferenceMarkerAsync(
        UpsertReferenceMarker marker,
        Guid idempotencyKey,
        CancellationToken cancellationToken = default) =>
        Task.FromResult(new CommandResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new CommandOutcome
            {
                Status = CommandOutcomeStatus.Succeeded,
                Payload = new CommandResult
                {
                    Kind = PurpleKind.ReferenceMarkerUpserted,
                    Payload = new PayloadClass
                    {
                        Id = marker.MarkerId,
                        Label = marker.Label,
                        Revision = (marker.ExpectedRevision ?? 0) + 1,
                        Scope = new ScopeRef
                        {
                            Kind = "organization",
                            Id = Guid.Parse("2ef36635-1d9d-4bd5-b0e4-fc4a67dfac90"),
                        },
                        SyncState = ReferenceMarkerSyncState.Pending,
                        UpdatedAt = 1_800_000_000_001,
                    },
                },
            },
        });

    public Task<IEngineSubscription> SubscribeAsync(
        Subscription subscription,
        CancellationToken cancellationToken = default)
    {
        var item = new FakeSubscription();
        lock (stateLock)
        {
            subscriptions.Add(subscription.Kind, item);
        }

        return Task.FromResult<IEngineSubscription>(item);
    }

    public void Connect()
    {
        EngineSupervisionSnapshot current;
        lock (stateLock)
        {
            current = snapshot = snapshot with
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
        }

        StateChanged?.Invoke(current);
    }

    public void Disconnect()
    {
        EngineSupervisionSnapshot current;
        lock (stateLock)
        {
            current = snapshot = snapshot with { IpcHealth = EngineIpcHealthState.Connecting };
        }

        StateChanged?.Invoke(current);
    }

    public void SignalResync(string kind)
    {
        FakeSubscription subscription;
        lock (stateLock)
        {
            subscription = subscriptions[kind];
        }

        subscription.SignalResync();
    }

    public void Publish(string kind, EventEnvelope envelope)
    {
        FakeSubscription subscription;
        lock (stateLock)
        {
            subscription = subscriptions[kind];
        }

        subscription.Publish(envelope);
    }

    public async ValueTask DisposeAsync()
    {
        FakeSubscription[] currentSubscriptions;
        lock (stateLock)
        {
            currentSubscriptions = subscriptions.Values.ToArray();
        }

        foreach (var subscription in currentSubscriptions)
        {
            await subscription.DisposeAsync();
        }
    }

    private static ConfigSnapshot Configuration() => new()
    {
        Revision = 1,
        SchemaVersion = 1,
        Scope = new ScopeRef { Kind = "organization", Id = Guid.NewGuid() },
        Entries =
        [
            new ConfigEntry
            {
                Key = ProtocolIds.ConfigKeys.EitmadConfigLocalePrimaryV1,
                Sensitivity = ConfigSensitivity.Public,
                RestartRequirement = RestartRequirement.None,
                Value = new ConfigReadValue { Kind = ConfigReadValueKind.Text, Value = "ar-YE" },
            },
        ],
    };
}
