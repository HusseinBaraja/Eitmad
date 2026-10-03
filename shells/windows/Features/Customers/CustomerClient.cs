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

/// <summary>Thin typed adapter over Rust-owned customer commands, queries, and change events.</summary>
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
            }, refreshOnStart: false);
    }

    public event EventHandler<Guid?>? Changed;

    public async Task<CustomerResult<IReadOnlyList<Customer>>> SearchAsync(
        string term,
        CancellationToken cancellationToken = default)
    {
        var trimmed = term.Trim();
        if (!CustomerInputValidation.IsSearchTermValid(trimmed))
            return CustomerResult<IReadOnlyList<Customer>>.Failed(CustomerFailureKind.Validation);
        try
        {
            var response = await engine.QueryAsync(Query.ForCustomerSearch(new SearchCustomers
            {
                Term = trimmed,
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

    public Task DeactivateAsync() => changes.DeactivateAsync();

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

    private async Task<CustomerResult<Customer>> SubmitAsync(Command command, CancellationToken cancellationToken)
    {
        try
        {
            var response = await engine.SubmitCommandAsync(command, Guid.NewGuid(), cancellationToken);
            var mutation = command.Kind == Command.CustomerCreateKind
                ? response.Outcome.Payload.AsCustomerCreated()
                : response.Outcome.Payload.AsCustomerUpdated();
            return response.Outcome.Status == CommandOutcomeStatus.Succeeded
                && mutation?.Customer is { } customer
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
