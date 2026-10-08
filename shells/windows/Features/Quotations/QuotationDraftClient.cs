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
    private readonly EngineChangeFeed approvals;
    private readonly EngineChangeFeed lifecycle;
    public bool SupportsLifecycle => engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1);
    public bool SupportsApprovals => engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityQuotationApprovalV1);
    private SynchronizationContext? context;
    public QuotationDraftClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        lifecycle = new(engine, ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1,
            Subscription.ForQuotationChangedSubscribe(new()), _ => LifecycleChanged?.Invoke(this, EventArgs.Empty), Invalidate,
            refreshOnStart: false, notifyUnavailable: true);
        approvals = new(engine, ProtocolIds.Capabilities.EitmadCapabilityQuotationApprovalV1,
            Subscription.ForQuotationApprovalChangedSubscribe(new()),
            _ => ApprovalChanged?.Invoke(this, EventArgs.Empty), Invalidate,
            refreshOnStart: false, notifyUnavailable: true);
        changes = new(engine, ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1,
            Subscription.ForQuotationDraftChangedSubscribe(new()),
            _ => Changed?.Invoke(this, EventArgs.Empty), Invalidate,
            refreshOnStart: false, notifyUnavailable: true);
    }
    public event EventHandler? Changed;
    public event EventHandler? ApprovalChanged;
    public event EventHandler? LifecycleChanged;
    public event EventHandler? Invalidated;
    public async Task ActivateAsync() { context = SynchronizationContext.Current; await changes.ActivateAsync(); await approvals.ActivateAsync(); await lifecycle.ActivateAsync(); }
    private void Invalidate()
    {
        if (context is null) Invalidated?.Invoke(this, EventArgs.Empty);
        else context.Post(_ => Invalidated?.Invoke(this, EventArgs.Empty), null);
    }
    public async Task DeactivateAsync() { await changes.DeactivateAsync(); await approvals.DeactivateAsync(); await lifecycle.DeactivateAsync(); }
    public async ValueTask DisposeAsync() { await changes.DisposeAsync(); await approvals.DisposeAsync(); await lifecycle.DisposeAsync(); }
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
    public Task<DraftResult<DiscountApprovalPage>> ApprovalsAsync(Guid? after, CancellationToken token = default) =>
        RequestAsync(async () => {
            if (!SupportsApprovals) return new DraftResult<DiscountApprovalPage>(null, DraftFailure.Unavailable, []);
            var r = await engine.QueryAsync(Query.ForQuotationApprovalList(new() { After = after, Limit = 100 }), token);
            return Result(r.Outcome.Status, r.Outcome.Payload.AsDiscountApprovals(), r.Outcome.Payload.Code, r.Outcome.Payload.Detail);
        }, token);
    public Task<DraftResult<DiscountApproval>> ApprovalAsync(Command command, Guid key, CancellationToken token = default) =>
        RequestAsync(async () => {
            if (!SupportsApprovals) return new DraftResult<DiscountApproval>(null, DraftFailure.Unavailable, []);
            var r = await engine.SubmitCommandAsync(command, key, token);
            return Result(r.Outcome.Status, r.Outcome.Payload.AsDiscountApproval(), r.Outcome.Payload.Code, r.Outcome.Payload.Detail);
        }, token);
    public Task<DraftResult<QuotationPage>> QuotationsAsync(Guid? after, CancellationToken token = default) =>
        RequestAsync(async () => {
            if (!SupportsLifecycle) return new DraftResult<QuotationPage>(null, DraftFailure.Unavailable, []);
            var r = await engine.QueryAsync(Query.ForQuotationList(new() { After = after, Limit = 100 }), token);
            return Result(r.Outcome.Status, r.Outcome.Payload.AsQuotations(), r.Outcome.Payload.Code, r.Outcome.Payload.Detail);
        }, token);
    public Task<DraftResult<QuotationRecord>> TransitionAsync(Command command, Guid key, CancellationToken token = default) =>
        RequestAsync(async () => {
            if (!SupportsLifecycle) return new DraftResult<QuotationRecord>(null, DraftFailure.Unavailable, []);
            var r = await engine.SubmitCommandAsync(command, key, token);
            return Result(r.Outcome.Status, r.Outcome.Payload.AsQuotation(), r.Outcome.Payload.Code, r.Outcome.Payload.Detail);
        }, token);
    public static string LifecycleMessage(DraftFailure failure) => failure switch {
        DraftFailure.Conflict => "تغير العرض أو السعر أو الموافقة. حدّث العرض وراجع الشروط قبل المتابعة.",
        DraftFailure.Invalid => "تحقق من مدة الصلاحية وسبب الإلغاء.",
        DraftFailure.Denied => "ليس لديك صلاحية لهذه العملية.",
        _ => "تعذر تأكيد العملية من الخادم. أعد محاولة العملية نفسها لتأكيد نتيجتها.",
    };
    public static string ApprovalMessage(DraftFailure failure) => failure switch {
        DraftFailure.Invalid => "أدخل سبب الرفض وتحقق من بيانات الطلب.",
        DraftFailure.Conflict => "تغيرت شروط العرض أو اتُخذ قرار سابق. حدّث الطلب قبل المتابعة.",
        DraftFailure.Denied => "ليس لديك صلاحية لهذا القرار، ولا يمكن لصاحب الطلب اتخاذ القرار.",
        _ => "تعذر تأكيد العملية من الخادم. لم يتغير القرار المعروض؛ أعد المحاولة.",
    };
    public static string ApprovalLabel(DiscountApproval? value) => value?.State switch {
        DiscountApprovalState.Pending => "بانتظار موافقة المدير",
        DiscountApprovalState.Approved => "وافق المدير على الخصم",
        DiscountApprovalState.Rejected => "رفض المدير الخصم",
        DiscountApprovalState.Invalidated => "تغيرت شروط العرض — يلزم طلب موافقة جديد",
        _ => "",
    };
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
            ProtocolIds.ErrorCodes.EitmadErrorQuotationStateConflictV1 or ProtocolIds.ErrorCodes.EitmadErrorQuotationStalePriceV1 or ProtocolIds.ErrorCodes.EitmadErrorQuotationApprovalRequiredV1 or ProtocolIds.ErrorCodes.EitmadErrorQuotationApprovalConflictV1 or ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftConflictV1 => DraftFailure.Conflict,
            ProtocolIds.ErrorCodes.EitmadErrorQuotationInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorQuotationApprovalInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => DraftFailure.Invalid,
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
