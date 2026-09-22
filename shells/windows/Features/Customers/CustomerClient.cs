using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Customers;

public enum CustomerFailureKind
{
    None,
    Validation,
    RevisionConflict,
    Denied,
    NotFound,
    Unavailable,
}

public sealed record CustomerResult<T>(T? Value, CustomerFailureKind Failure, IReadOnlySet<string> InvalidFields)
    where T : class
{
    public bool Succeeded => Failure == CustomerFailureKind.None && Value is not null;
    public static CustomerResult<T> Success(T value) => new(value, CustomerFailureKind.None, new HashSet<string>());
    public static CustomerResult<T> Failed(CustomerFailureKind failure, IReadOnlySet<string>? fields = null) =>
        new(null, failure, fields ?? new HashSet<string>());
}

/// <summary>Thin typed adapter over Rust-owned customer commands, queries, and change events.</summary>
public sealed class CustomerClient : IAsyncDisposable
{
    public const long SearchLimit = 20;
    private readonly IEngineShellBridge engine;
    private readonly SemaphoreSlim subscriptionGate = new(1, 1);
    private readonly object stateLock = new();
    private CancellationTokenSource? subscriptionCancellation;
    private IEngineSubscription? subscription;
    private SynchronizationContext? synchronizationContext;
    private long subscribedGeneration = -1;
    private bool active;
    private bool disposed;

    public CustomerClient(IEngineShellBridge engine) => this.engine = engine;

    public event EventHandler<Guid?>? Changed;

    public async Task<CustomerResult<IReadOnlyList<Customer>>> SearchAsync(
        string term,
        CancellationToken cancellationToken = default)
    {
        try
        {
            var response = await engine.QueryAsync(Query.ForCustomerSearch(new SearchCustomers
            {
                Term = term.Trim(),
                Limit = SearchLimit,
            }), cancellationToken);
            return response.Outcome.Status == CommandOutcomeStatus.Succeeded
                && response.Outcome.Payload.AsCustomers() is { } page
                ? CustomerResult<IReadOnlyList<Customer>>.Success(page.Items)
                : CustomerResult<IReadOnlyList<Customer>>.Failed(MapFailure(response.Outcome.Payload.Code), ValidationFields(response.Outcome.Payload.Detail));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (EngineIpcException error)
        {
            return CustomerResult<IReadOnlyList<Customer>>.Failed(
                MapFailure(error.ContractError?.Code), ValidationFields(error.ContractError?.Detail));
        }
        catch (Exception error) when (error is IOException or InvalidOperationException)
        {
            return CustomerResult<IReadOnlyList<Customer>>.Failed(CustomerFailureKind.Unavailable);
        }
    }

    public async Task<CustomerResult<Customer>> GetAsync(Guid customerId, CancellationToken cancellationToken = default)
    {
        try
        {
            var response = await engine.QueryAsync(
                Query.ForCustomerGet(new GetCustomer { CustomerId = customerId }), cancellationToken);
            return response.Outcome.Status == CommandOutcomeStatus.Succeeded
                && response.Outcome.Payload.AsCustomer() is { } customer
                ? CustomerResult<Customer>.Success(customer)
                : CustomerResult<Customer>.Failed(MapFailure(response.Outcome.Payload.Code), ValidationFields(response.Outcome.Payload.Detail));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (EngineIpcException error)
        {
            return CustomerResult<Customer>.Failed(
                MapFailure(error.ContractError?.Code), ValidationFields(error.ContractError?.Detail));
        }
        catch (Exception error) when (error is IOException or InvalidOperationException)
        {
            return CustomerResult<Customer>.Failed(CustomerFailureKind.Unavailable);
        }
    }

    public Task<CustomerResult<Customer>> CreateAsync(
        string name,
        string phone,
        string address,
        string notes,
        CancellationToken cancellationToken = default) => SubmitAsync(
            Command.ForCustomerCreate(new CreateCustomer
            {
                Name = name,
                Phone = phone,
                Address = EmptyToNull(address)!,
                Notes = EmptyToNull(notes)!,
            }), cancellationToken);

    public Task<CustomerResult<Customer>> UpdateAsync(
        Customer current,
        string name,
        string phone,
        string address,
        string notes,
        CancellationToken cancellationToken = default) => SubmitAsync(
            Command.ForCustomerUpdate(new UpdateCustomer
            {
                CustomerId = current.Id,
                ExpectedRevision = current.Revision,
                Name = name,
                Phone = phone,
                Address = EmptyToNull(address)!,
                Notes = EmptyToNull(notes)!,
            }), cancellationToken);

    public async Task ActivateAsync(CancellationToken cancellationToken = default)
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        lock (stateLock)
        {
            if (active) return;
            active = true;
            synchronizationContext = SynchronizationContext.Current;
        }
        engine.StateChanged += ObserveEngineState;
        await RefreshSubscriptionAsync(cancellationToken);
    }

    public async Task DeactivateAsync()
    {
        lock (stateLock) active = false;
        engine.StateChanged -= ObserveEngineState;
        await DropSubscriptionAsync();
    }

    public async ValueTask DisposeAsync()
    {
        if (disposed) return;
        disposed = true;
        await DeactivateAsync();
        subscriptionGate.Dispose();
    }

    public static string ArabicMessage(CustomerFailureKind failure) => failure switch
    {
        CustomerFailureKind.Validation => "تحقق من بيانات العميل. لا تترك مسافات زائدة واستخدم رقماً صالحاً للهاتف.",
        CustomerFailureKind.RevisionConflict => "تغيرت بيانات العميل في مكان آخر. لم تُحفظ تعديلاتك. أغلق النافذة وراجع أحدث البيانات قبل المحاولة مرة أخرى.",
        CustomerFailureKind.Denied => "ليس لديك صلاحية لعرض بيانات العملاء أو تعديلها.",
        CustomerFailureKind.NotFound => "لم يعد سجل العميل متاحاً.",
        _ => "تعذر الاتصال ببيانات العملاء. حاول مرة أخرى.",
    };

    private async Task<CustomerResult<Customer>> SubmitAsync(Command command, CancellationToken cancellationToken)
    {
        try
        {
            var response = await engine.SubmitCommandAsync(command, Guid.NewGuid(), cancellationToken);
            return response.Outcome.Status == CommandOutcomeStatus.Succeeded
                && response.Outcome.Payload.Payload?.Customer is { } customer
                ? CustomerResult<Customer>.Success(customer)
                : CustomerResult<Customer>.Failed(MapFailure(response.Outcome.Payload.Code), ValidationFields(response.Outcome.Payload.Detail));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (EngineIpcException error)
        {
            return CustomerResult<Customer>.Failed(
                MapFailure(error.ContractError?.Code), ValidationFields(error.ContractError?.Detail));
        }
        catch (Exception error) when (error is IOException or InvalidOperationException)
        {
            return CustomerResult<Customer>.Failed(CustomerFailureKind.Unavailable);
        }
    }

    private void ObserveEngineState(EngineSupervisionSnapshot snapshot)
    {
        bool shouldRefresh;
        lock (stateLock)
        {
            shouldRefresh = active
                && snapshot.IpcHealth == EngineIpcHealthState.Connected
                && snapshot.LastLifecycle?.Ready == true
                && snapshot.Generation != subscribedGeneration;
        }
        if (shouldRefresh) _ = RefreshSubscriptionAsync(CancellationToken.None);
    }

    private async Task RefreshSubscriptionAsync(CancellationToken cancellationToken)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityCustomerV1)) return;
        await subscriptionGate.WaitAsync(cancellationToken);
        try
        {
            bool shouldStart;
            lock (stateLock)
            {
                shouldStart = active && !disposed;
                if (subscribedGeneration == engine.Snapshot.Generation && subscription is not null) return;
            }
            if (!shouldStart) return;
            await DropSubscriptionCoreAsync();
            var created = await engine.SubscribeAsync(
                Subscription.ForCustomerChangedSubscribe(new CustomerChanges()), cancellationToken);
            var pumpCancellation = new CancellationTokenSource();
            lock (stateLock)
            {
                if (!active || disposed)
                {
                    pumpCancellation.Cancel();
                    _ = created.DisposeAsync();
                    return;
                }
                subscription = created;
                subscriptionCancellation = pumpCancellation;
                subscribedGeneration = engine.Snapshot.Generation;
            }
            created.ResyncRequired += SignalFullRefresh;
            _ = PumpAsync(created, pumpCancellation.Token);
        }
        catch (EngineIpcException)
        {
            await DropSubscriptionCoreAsync();
        }
        finally
        {
            subscriptionGate.Release();
        }
    }

    private async Task DropSubscriptionAsync()
    {
        await subscriptionGate.WaitAsync();
        try { await DropSubscriptionCoreAsync(); }
        finally { subscriptionGate.Release(); }
    }

    private async Task DropSubscriptionCoreAsync()
    {
        IEngineSubscription? current;
        CancellationTokenSource? cancellation;
        lock (stateLock)
        {
            current = subscription;
            cancellation = subscriptionCancellation;
            subscription = null;
            subscriptionCancellation = null;
            subscribedGeneration = -1;
        }
        cancellation?.Cancel();
        cancellation?.Dispose();
        if (current is not null) await current.DisposeAsync();
    }

    private async Task PumpAsync(IEngineSubscription current, CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var delivered in current.ReadAllAsync(cancellationToken))
            {
                var notice = EngineContractCodec.DecodeEvent(delivered).AsCustomerChangedEvent();
                if (notice is not null) RaiseChanged(notice.CustomerId);
                current.Acknowledge(delivered);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (Exception error) when (error is EngineIpcException or IOException or InvalidDataException)
        {
            await DropSubscriptionAsync();
            SignalFullRefresh();
            await RetrySubscriptionAsync();
        }
    }

    private async Task RetrySubscriptionAsync()
    {
        foreach (var delay in new[] { 250, 500, 1_000, 2_000 })
        {
            await Task.Delay(delay);
            lock (stateLock)
            {
                if (!active || disposed || subscription is not null) return;
            }
            await RefreshSubscriptionAsync(CancellationToken.None);
        }
    }

    private void SignalFullRefresh() => RaiseChanged(null);

    private void RaiseChanged(Guid? customerId)
    {
        var context = synchronizationContext;
        if (context is null) Changed?.Invoke(this, customerId);
        else context.Post(_ => Changed?.Invoke(this, customerId), null);
    }

    private static string? EmptyToNull(string value) => string.IsNullOrWhiteSpace(value) ? null : value;

    private static CustomerFailureKind MapFailure(string? code) => code switch
    {
        ProtocolIds.ErrorCodes.EitmadErrorCustomerRevisionConflictV1 => CustomerFailureKind.RevisionConflict,
        ProtocolIds.ErrorCodes.EitmadErrorCustomerNotFoundV1 => CustomerFailureKind.NotFound,
        ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => CustomerFailureKind.Denied,
        ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => CustomerFailureKind.Validation,
        _ => CustomerFailureKind.Unavailable,
    };

    private static IReadOnlySet<string> ValidationFields(ErrorDetail? detail) =>
        detail is { Kind: DetailKind.Validation, Payload.Fields: { } fields }
            ? fields.ToHashSet(StringComparer.Ordinal)
            : new HashSet<string>();
}
