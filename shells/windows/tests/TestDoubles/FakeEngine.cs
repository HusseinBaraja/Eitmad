using System.Globalization;
using System.Text;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
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
    private DesktopSessionState? desktopSession;

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
    public bool ThrowQueries { get; set; }
    public long ConfigurationRevision { get; set; } = 1;
    public Func<Query, Task>? QueryBarrier { get; set; }
    public List<DesktopAccountSummary> DesktopAccounts { get; } = [];
    public List<Customer> Customers { get; } = [];
    public ScopeRef CustomerBranch { get; } = new() { Kind = "branch", Id = Guid.NewGuid() };
    public Action<Subscription, FakeSubscription>? SubscribeHook { get; set; }
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
    public int SignOutCount { get; private set; }
    public long? SessionExpiry { get; set; }
    public Dictionary<string, (string Password, DesktopAccountRole Role)> Accounts { get; } = [];
    public IReadOnlySet<string> SupportedCapabilities { get; init; } = new HashSet<string>
    {
        ProtocolIds.Capabilities.EitmadCapabilityConfigV1,
        ProtocolIds.Capabilities.EitmadCapabilitySyncV1,
        ProtocolIds.Capabilities.EitmadCapabilityUpdateV1,
        ProtocolIds.Capabilities.EitmadCapabilityReferenceMarkerV1,
        ProtocolIds.Capabilities.EitmadCapabilityDesktopAccountManagementV1,
        ProtocolIds.Capabilities.EitmadCapabilityCustomerV1,
    };

    public bool SupportsCapability(string capability) => SupportedCapabilities.Contains(capability);

    public Task<DesktopSessionState> SignInAsync(string username, string password, CancellationToken cancellationToken = default)
    {
        if (!Accounts.TryGetValue(username, out var account) || account.Password != password)
            return Task.FromException<DesktopSessionState>(new EngineIpcException(
                EngineIpcFailureKind.AuthenticationRejected, "Synthetic rejection."));
        desktopSession = new DesktopSessionState
        {
            Authorization = new AuthorizationContext
            {
                SessionId = Guid.NewGuid(),
                TenantId = Guid.NewGuid(),
                Scope = new ScopeRef { Kind = "organization", Id = Guid.NewGuid() },
                Identity = new AuthenticatedIdentity
                {
                    PrincipalId = Guid.NewGuid(),
                    PrincipalKind = PrincipalKind.User,
                },
            },
            AccountRole = account.Role,
            ExpiresAt = SessionExpiry ?? DateTimeOffset.UtcNow.AddHours(8).ToUnixTimeMilliseconds(),
        };
        CurrentPermission = account.Role switch
        {
            DesktopAccountRole.Manager => ProtocolIds.Permissions.EitmadPermissionCatalogDraftWriteV1,
            DesktopAccountRole.Receptionist => ProtocolIds.Permissions.EitmadPermissionQuotationDraftWriteV1,
            _ => null,
        };
        return Task.FromResult(desktopSession);
    }

    public Task<DesktopSessionState?> GetSessionStateAsync(CancellationToken cancellationToken = default) =>
        Task.FromResult(desktopSession);

    public Task SignOutAsync(CancellationToken cancellationToken = default)
    {
        desktopSession = null;
        CurrentPermission = null;
        SignOutCount++;
        return Task.CompletedTask;
    }

    private string? CurrentPermission { get; set; }

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

    public async Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default)
    {
        var revision = ConfigurationRevision;
        Interlocked.Increment(ref queryCount);
        lock (queriedKinds)
        {
            queriedKinds.Add(query.Kind);
        }

        if (QueryBarrier is { } barrier) await barrier(query);
        if (ThrowQueries)
            throw new EngineIpcException(EngineIpcFailureKind.ConnectionLost, "Synthetic IPC failure.");

        if (FailConfigurationQuery && query.Kind == Query.ConfigGetKind)
        {
            return new QueryResponseEnvelope
            {
                RequestId = Guid.NewGuid(),
                CorrelationId = Guid.NewGuid(),
                Outcome = new QueryOutcome
                {
                    Status = CommandOutcomeStatus.Failed,
                    Payload = new QueryResult { Code = "CONFIG_UNAVAILABLE" },
                },
            };
        }

        if (query.AsCustomerGet() is { } getCustomer &&
            Customers.All(customer => customer.Id != getCustomer.CustomerId || !IsAuthorizedCustomer(customer)))
        {
            return new QueryResponseEnvelope
            {
                RequestId = Guid.NewGuid(),
                CorrelationId = Guid.NewGuid(),
                Outcome = new QueryOutcome
                {
                    Status = CommandOutcomeStatus.Failed,
                    Payload = new QueryResult { Code = ProtocolIds.ErrorCodes.EitmadErrorCustomerNotFoundV1 },
                },
            };
        }
        var result = query.Kind switch
        {
            Query.PermissionsGetEffectiveKind => QueryResult.ForEffectivePermissions(new EffectivePermissions
            {
                PolicyVersion = 1,
                Permissions = CurrentPermission is null
                    ? []
                    : [new EffectivePermission { Permission = CurrentPermission, Decision = PermissionDecision.Granted }],
            }),
            Query.ConfigGetKind => QueryResult.ForConfiguration(Configuration(revision)),
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
            Query.DesktopAccountListKind => QueryResult.ForDesktopAccounts(new DesktopAccountPage
            {
                Accounts = DesktopAccounts.ToArray(),
            }),
            Query.CustomerGetKind => QueryResult.ForCustomer(Customers.Single(customer =>
                customer.Id == query.AsCustomerGet()!.CustomerId && IsAuthorizedCustomer(customer))),
            Query.CustomerSearchKind => QueryResult.ForCustomers(new CustomerPage
            {
                Items = Customers.Where(customer => IsAuthorizedCustomer(customer)
                    && CustomerMatchesTerm(customer, query.AsCustomerSearch()!.Term))
                    .Take((int)query.AsCustomerSearch()!.Limit).ToArray(),
            }),
            _ => throw new InvalidOperationException("Unexpected fake query."),
        };
        return new QueryResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = result },
        };
    }

    private static string NormalizeCustomerName(string value)
    {
        var result = new StringBuilder(value.Length);
        var pendingSpace = false;
        foreach (var character in value.Normalize(NormalizationForm.FormD))
        {
            var category = CharUnicodeInfo.GetUnicodeCategory(character);
            if (category is UnicodeCategory.NonSpacingMark or UnicodeCategory.SpacingCombiningMark
                or UnicodeCategory.EnclosingMark || character is '\u0640' or '\u200c' or '\u200d') continue;
            if (char.IsWhiteSpace(character))
            {
                pendingSpace = result.Length > 0;
                continue;
            }
            if (pendingSpace) result.Append(' ');
            pendingSpace = false;
            result.Append(char.ToLowerInvariant(character switch
            {
                '\u0622' or '\u0623' or '\u0625' or '\u0671' => '\u0627',
                '\u0649' or '\u06cc' => '\u064a',
                '\u0629' => '\u0647',
                '\u06a9' => '\u0643',
                >= '\u0660' and <= '\u0669' => (char)('0' + character - '\u0660'),
                >= '\u06f0' and <= '\u06f9' => (char)('0' + character - '\u06f0'),
                _ => character,
            }));
        }
        return result.ToString();
    }

    private bool IsAuthorizedCustomer(Customer customer) =>
        customer.Scope.Kind == CustomerBranch.Kind && customer.Scope.Id == CustomerBranch.Id;

    private static bool CustomerMatchesTerm(Customer customer, string term)
    {
        if (NormalizeCustomerName(customer.Name).Contains(NormalizeCustomerName(term), StringComparison.OrdinalIgnoreCase))
            return true;
        var normalizedPhone = NormalizeCustomerPhone(term);
        return normalizedPhone is not null
            && NormalizeCustomerPhone(customer.Phone)?.Contains(normalizedPhone, StringComparison.Ordinal) == true;
    }

    private static string? NormalizeCustomerPhone(string value)
    {
        var digits = new StringBuilder(value.Length);
        for (var index = 0; index < value.Length; index++)
        {
            var character = value[index];
            if (character is >= '0' and <= '9' || character == '+' && index == 0)
                digits.Append(character);
            else if (character is >= '\u0660' and <= '\u0669')
                digits.Append((char)('0' + character - '\u0660'));
            else if (character is >= '\u06f0' and <= '\u06f9')
                digits.Append((char)('0' + character - '\u06f0'));
            else if (character is not (' ' or '-' or '(' or ')')) return null;
        }
        return digits.Length > 0 && digits.ToString() != "+" ? digits.ToString() : null;
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

    public Func<Command, CommandResponseEnvelope>? CommandHandler { get; set; }
    public Func<Command, Task>? CommandBarrier { get; set; }
    public Command? LastCommand { get; private set; }

    public async Task<CommandResponseEnvelope> SubmitCommandAsync(
        Command command,
        Guid idempotencyKey,
        CancellationToken cancellationToken = default)
    {
        LastCommand = command;
        if (CommandBarrier is not null) await CommandBarrier(command);
        return CommandHandler?.Invoke(command) ?? ApplyCommand(command);
    }

    private CommandResponseEnvelope ApplyCommand(Command command)
    {
        Customer? changedCustomer = null;
        if (command.AsDesktopAccountCreate() is { } create)
        {
            DesktopAccounts.Add(new DesktopAccountSummary
            {
                AccountId = Guid.NewGuid(),
                UserId = Guid.NewGuid(),
                DisplayName = create.DisplayName,
                Username = create.Username,
                Role = create.Role,
                Active = true,
                Revision = 1,
            });
        }
        else if (command.AsDesktopAccountUpdate() is { } update)
        {
            var index = DesktopAccounts.FindIndex(account => account.AccountId == update.AccountId);
            if (index >= 0)
            {
                var current = DesktopAccounts[index];
                current.DisplayName = update.DisplayName;
                current.Role = update.Role;
                current.Revision++;
            }
        }
        else if (command.AsDesktopAccountDeactivate() is { } deactivate)
        {
            var account = DesktopAccounts.Single(item => item.AccountId == deactivate.AccountId);
            account.Active = false;
            account.Revision++;
        }
        else if (command.AsCustomerCreate() is { } createCustomer)
        {
            changedCustomer = new Customer
            {
                Id = Guid.NewGuid(),
                Scope = CustomerBranch,
                Name = createCustomer.Name,
                Phone = createCustomer.Phone,
                Address = createCustomer.Address,
                Notes = createCustomer.Notes,
                Status = CustomerStatus.Active,
                Revision = 1,
                SyncState = ErSyncState.Pending,
                UpdatedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
            };
            Customers.Add(changedCustomer);
        }
        else if (command.AsCustomerUpdate() is { } updateCustomer)
        {
            var index = Customers.FindIndex(customer => customer.Id == updateCustomer.CustomerId
                && IsAuthorizedCustomer(customer));
            if (index < 0) return FailedCustomer(ProtocolIds.ErrorCodes.EitmadErrorCustomerNotFoundV1);
            var current = Customers[index];
            if (current.Revision != updateCustomer.ExpectedRevision)
                return FailedCustomer(ProtocolIds.ErrorCodes.EitmadErrorCustomerRevisionConflictV1);
            changedCustomer = new Customer
            {
                Id = current.Id,
                Scope = current.Scope,
                Name = updateCustomer.Name,
                Phone = updateCustomer.Phone,
                Address = updateCustomer.Address,
                Notes = updateCustomer.Notes,
                Status = current.Status,
                Revision = current.Revision + 1,
                SyncState = ErSyncState.Pending,
                UpdatedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
            };
            Customers[index] = changedCustomer;
        }
        return new CommandResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new CommandOutcome
            {
                Status = CommandOutcomeStatus.Succeeded,
                Payload = changedCustomer is null
                    ? new CommandResult()
                    : new CommandResult
                    {
                        Kind = changedCustomer.Revision == 1 ? PurpleKind.CustomerCreated : PurpleKind.CustomerUpdated,
                        Payload = new PayloadClass { Customer = changedCustomer, PotentialDuplicateIds = [] },
                    },
            },
        };
    }

    private static CommandResponseEnvelope FailedCustomer(string code) => new()
    {
        RequestId = Guid.NewGuid(),
        CorrelationId = Guid.NewGuid(),
        Outcome = new CommandOutcome
        {
            Status = CommandOutcomeStatus.Failed,
            Payload = new CommandResult { Code = code },
        },
    };

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
                        SyncState = ErSyncState.Pending,
                        UpdatedAt = 1_800_000_000_001,
                    },
                },
            },
        });

    public Task<IEngineSubscription> SubscribeAsync(
        Subscription subscription,
        CancellationToken cancellationToken = default)
    {
        FakeSubscription? item = null;
        item = new FakeSubscription(() =>
        {
            lock (stateLock)
            {
                if (subscriptions.TryGetValue(subscription.Kind, out var current) && ReferenceEquals(current, item))
                    subscriptions.Remove(subscription.Kind);
            }
        });
        lock (stateLock)
        {
            subscriptions.Add(subscription.Kind, item);
        }

        SubscribeHook?.Invoke(subscription, item);

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

    private static ConfigSnapshot Configuration(long revision) => new()
    {
        Revision = revision,
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
