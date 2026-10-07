using System.IO;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Quotations;

public enum DraftFailure { None, Invalid, Conflict, Denied, NotFound, Unavailable }
public sealed record DraftResult<T>(T? Value, DraftFailure Failure, QuotationFieldError[] Errors) where T : class
{
    public bool Succeeded => Value is not null && Failure == DraftFailure.None;
}

/// <summary>Generated draft contracts only; Rust owns persistence, evaluation, scope, and sync.</summary>
public sealed class QuotationDraftClient : IAsyncDisposable
{
    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed changes;
    private SynchronizationContext? context;
    public QuotationDraftClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        changes = new(engine, ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1,
            Subscription.ForQuotationDraftChangedSubscribe(new()),
            _ => Changed?.Invoke(this, EventArgs.Empty), Invalidate,
            refreshOnStart: false, notifyUnavailable: true);
    }
    public event EventHandler? Changed;
    public event EventHandler? Invalidated;
    public Task ActivateAsync() { context = SynchronizationContext.Current; return changes.ActivateAsync(); }
    private void Invalidate()
    {
        if (context is null) Invalidated?.Invoke(this, EventArgs.Empty);
        else context.Post(_ => Invalidated?.Invoke(this, EventArgs.Empty), null);
    }
    public Task DeactivateAsync() => changes.DeactivateAsync();
    public ValueTask DisposeAsync() => changes.DisposeAsync();
    public Task<DraftResult<QuotationDraft>> GetAsync(Guid id, CancellationToken token = default) =>
        RequestAsync(async () => {
            var r = await engine.QueryAsync(Query.ForQuotationDraftGet(new() { DraftId = id }), token);
            return Result(r.Outcome.Status, r.Outcome.Payload.AsQuotationDraft(), r.Outcome.Payload.Code, r.Outcome.Payload.Detail);
        }, token);
    public Task<DraftResult<QuotationDraftPage>> ListAsync(Guid? after, CancellationToken token = default) =>
        RequestAsync(async () => {
            var r = await engine.QueryAsync(Query.ForQuotationDraftList(new() { After = after, Limit = 100 }), token);
            return Result(r.Outcome.Status, r.Outcome.Payload.AsQuotationDrafts(), r.Outcome.Payload.Code, r.Outcome.Payload.Detail);
        }, token);
    public Task<DraftResult<QuotationDraft>> SaveAsync(Command command, Guid key, CancellationToken token = default) =>
        RequestAsync(async () => {
            var r = await engine.SubmitCommandAsync(command, key, token);
            return Result(r.Outcome.Status, r.Outcome.Payload.AsQuotationDraftCreated() ?? r.Outcome.Payload.AsQuotationDraftUpdated(), r.Outcome.Payload.Code, r.Outcome.Payload.Detail);
        }, token);
    private async Task<DraftResult<T>> RequestAsync<T>(Func<Task<DraftResult<T>>> request, CancellationToken token) where T : class
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1)) return new(null, DraftFailure.Unavailable, []);
        try { return await request(); }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { throw; }
        catch (EngineIpcException e) { return Result<T>(CommandOutcomeStatus.Failed, null, e.ContractError?.Code, e.ContractError?.Detail); }
        catch (Exception e) when (e is IOException or InvalidOperationException or ObjectDisposedException or JsonException)
        { return new(null, DraftFailure.Unavailable, []); }
    }
    private static DraftResult<T> Result<T>(CommandOutcomeStatus status, T? value, string? code, ErrorDetail? detail) where T : class =>
        status == CommandOutcomeStatus.Succeeded && value is not null ? new(value, DraftFailure.None, []) : new(null, code switch {
            ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftConflictV1 => DraftFailure.Conflict,
            ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => DraftFailure.Invalid,
            ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => DraftFailure.Denied,
            ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftNotFoundV1 => DraftFailure.NotFound,
            _ => DraftFailure.Unavailable,
        }, detail?.Kind == DetailKind.QuotationDraftValidation ? detail.Payload.Errors ?? [] : []);
    public static string Message(DraftFailure failure) => failure switch {
        DraftFailure.Conflict => "تغيرت المسودة في مكان آخر أو تعارضت مزامنتها. لم تُحفظ تعديلاتك. أعد فتح النسخة المحفوظة للمراجعة.",
        DraftFailure.Invalid => "لم تُحفظ المسودة. راجع بيانات العميل والأصناف والأسعار.",
        DraftFailure.Denied => "ليس لديك صلاحية لعرض المسودات أو حفظها.",
        DraftFailure.NotFound => "المسودة غير متاحة.",
        _ => "تعذر تأكيد الحفظ أو تحميل المسودات. احتُفظ بتعديلاتك؛ أعد المحاولة.",
    };
    public static string SyncLabel(QuotationDraft draft) => draft.SyncState switch {
        SyncState.Confirmed => "مسودة محفوظة — أكد الخادم المزامنة",
        SyncState.Conflicted => "مسودة محفوظة محلياً — تعارض في المزامنة؛ التعديل غير متاح",
        SyncState.Rejected => "مسودة محفوظة محلياً — رفض الخادم المزامنة؛ التعديل غير متاح",
        _ => "مسودة محفوظة محلياً — بانتظار المزامنة",
    };
}
