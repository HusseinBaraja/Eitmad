using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
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

public sealed class CustomerClient : IAsyncDisposable
{
    public const long SearchLimit = 20;
    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed changes;

    public CustomerClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        changes = new EngineChangeFeed(engine, ProtocolIds.Capabilities.EitmadCapabilityCustomerV1,
            Subscription.ForCustomerChangedSubscribe(new CustomerChanges()), notice =>
            {
                if (notice is null) Changed?.Invoke(this, null);
                else if (notice.AsCustomerChangedEvent() is { } customer) Changed?.Invoke(this, customer.CustomerId);
            }, () => Changed?.Invoke(this, null), refreshOnStart: false, notifyUnavailable: true);
    }

    public event EventHandler<Guid?>? Changed;

    public async Task<CustomerResult<IReadOnlyList<Customer>>> SearchAsync(
        string term,
        CancellationToken cancellationToken = default)
    {
        var result = await SearchPageAsync(term, null, cancellationToken);
        return result.Succeeded ? CustomerResult<IReadOnlyList<Customer>>.Success(result.Value!.Items)
            : CustomerResult<IReadOnlyList<Customer>>.Failed(result.Failure, result.InvalidFields);
    }

    public async Task<CustomerResult<CustomerPage>> SearchPageAsync(
        string term, CustomerCursor? after = null, CancellationToken cancellationToken = default)
    {
        var trimmed = term.Trim();
        if (!CustomerInputValidation.IsSearchTermValid(trimmed))
            return CustomerResult<CustomerPage>.Failed(CustomerFailureKind.Validation);
        return await RequestAsync(async () =>
        {
            var response = await engine.QueryAsync(Query.ForCustomerSearch(new SearchCustomers
            {
                Term = trimmed,
                Limit = SearchLimit,
                After = after!,
            }), cancellationToken);
            return Result(response.Outcome.Status, response.Outcome.Payload.AsCustomers(),
                response.Outcome.Payload.Code, response.Outcome.Payload.Detail);
        }, cancellationToken);
    }

    public Task<CustomerResult<Customer>> GetAsync(Guid customerId, CancellationToken cancellationToken = default, Guid? branchId = null) =>
        RequestAsync(async () =>
        {
            var response = await engine.QueryAsync(
                Query.ForCustomerGet(new GetCustomer { CustomerId = customerId, BranchId = branchId }), cancellationToken);
            return Result(response.Outcome.Status, response.Outcome.Payload.AsCustomer(),
                response.Outcome.Payload.Code, response.Outcome.Payload.Detail);
        }, cancellationToken);

    public Task<CustomerResult<Customer>> CreateAsync(
        string name,
        string phone,
        string address,
        string notes,
        CancellationToken cancellationToken = default) => ValidatedSubmitAsync(
            address, notes,
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
        CancellationToken cancellationToken = default) => ValidatedSubmitAsync(
            address, notes,
            Command.ForCustomerUpdate(new UpdateCustomer
            {
                CustomerId = current.Id,
                ExpectedRevision = current.Revision,
                Name = name,
                Phone = phone,
                Address = EmptyToNull(address)!,
                Notes = EmptyToNull(notes)!,
            }), cancellationToken);

    public Task ActivateAsync(CancellationToken cancellationToken = default) => changes.ActivateAsync(cancellationToken);

    public Task<CustomerResult<CustomerHistoryPage>> HistoryAsync(Customer customer,
        CustomerHistoryPage? previous = null, CancellationToken cancellationToken = default) =>
        RequestAsync(async () =>
        {
            var response = await engine.QueryAsync(Query.ForCustomerHistory(new GetCustomerHistory {
                Customer = new GetCustomer { CustomerId = customer.Id, BranchId = customer.Scope.Id },
                IncludeQuotations = previous is null || previous.QuotationsNext is not null,
                IncludeOrders = previous is null || previous.OrdersNext is not null,
                QuotationsAfter = previous?.QuotationsNext, OrdersAfter = previous?.OrdersNext,
            }), cancellationToken);
            return Result(response.Outcome.Status, response.Outcome.Payload.AsCustomerHistory(), response.Outcome.Payload.Code);
        }, cancellationToken);

    public Task DeactivateAsync() => changes.DeactivateAsync();

    public Task<CustomerResult<CustomerConflictPage>> ConflictsAsync(Customer customer, Guid? after = null, CancellationToken cancellationToken = default) =>
        RequestAsync(async () =>
        {
            var response = await engine.QueryAsync(Query.ForCustomerConflicts(new GetCustomerConflicts {
                Customer = new GetCustomer { CustomerId = customer.Id, BranchId = customer.Scope.Id }, After = after,
            }), cancellationToken);
            return Result(response.Outcome.Status, response.Outcome.Payload.AsCustomerConflicts(), response.Outcome.Payload.Code);
        }, cancellationToken);

    public Task<CustomerResult<ChangeRecord>> ResolveAsync(ResolveCustomerConflict command, Guid key, CancellationToken cancellationToken = default) =>
        RequestAsync(async () =>
        {
            var response = await engine.SubmitCommandAsync(Command.ForCustomerResolveConflict(command), key, cancellationToken);
            return Result(response.Outcome.Status, response.Outcome.Payload.AsCustomerConflictResolved(), response.Outcome.Payload.Code);
        }, cancellationToken);

    public ValueTask DisposeAsync() => changes.DisposeAsync();

    public static string ArabicMessage(CustomerFailureKind failure) => failure switch
    {
        CustomerFailureKind.Validation => "تحقق من بيانات العميل. لا تترك مسافات زائدة واستخدم رقماً صالحاً للهاتف.",
        CustomerFailureKind.RevisionConflict => "تغيرت بيانات العميل في مكان آخر. لم تُحفظ تعديلاتك. أغلق النافذة وراجع أحدث البيانات قبل المحاولة مرة أخرى.",
        CustomerFailureKind.Denied => "ليس لديك صلاحية لعرض بيانات العملاء أو تعديلها.",
        CustomerFailureKind.NotFound => "لم يعد سجل العميل متاحاً.",
        _ => "تعذر الاتصال ببيانات العملاء. حاول مرة أخرى.",
    };

    private Task<CustomerResult<Customer>> ValidatedSubmitAsync(
        string address, string notes, Command command, CancellationToken cancellationToken)
    {
        var normalizedAddress = EmptyToNull(address);
        var normalizedNotes = EmptyToNull(notes);
        if (normalizedAddress is not null && !CustomerInputValidation.IsAddressValid(normalizedAddress)
            || normalizedNotes is not null && !CustomerInputValidation.IsNotesValid(normalizedNotes))
            return Task.FromResult(CustomerResult<Customer>.Failed(CustomerFailureKind.Validation));
        return SubmitAsync(command, cancellationToken);
    }

    private Task<CustomerResult<Customer>> SubmitAsync(Command command, CancellationToken cancellationToken) =>
        RequestAsync(async () =>
        {
            var response = await engine.SubmitCommandAsync(command, Guid.NewGuid(), cancellationToken);
            var mutation = command.Kind == Command.CustomerCreateKind
                ? response.Outcome.Payload.AsCustomerCreated()
                : response.Outcome.Payload.AsCustomerUpdated();
            return Result(response.Outcome.Status, mutation?.Customer,
                response.Outcome.Payload.Code, response.Outcome.Payload.Detail);
        }, cancellationToken);

    private async Task<CustomerResult<T>> RequestAsync<T>(Func<Task<CustomerResult<T>>> request,
        CancellationToken cancellationToken) where T : class
    {
        try
        {
            return await request();
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (EngineIpcException error)
        {
            return CustomerResult<T>.Failed(
                MapFailure(error.ContractError?.Code), ValidationFields(error.ContractError?.Detail));
        }
        catch (Exception error) when (error is IOException or InvalidOperationException)
        {
            return CustomerResult<T>.Failed(CustomerFailureKind.Unavailable);
        }
    }

    private static CustomerResult<T> Result<T>(CommandOutcomeStatus status, T? value,
        string? code, ErrorDetail? detail = null) where T : class =>
        status == CommandOutcomeStatus.Succeeded && value is not null
            ? CustomerResult<T>.Success(value)
            : CustomerResult<T>.Failed(MapFailure(code), ValidationFields(detail));

    private static string? EmptyToNull(string value) => string.IsNullOrWhiteSpace(value) ? null : value.Trim();

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
