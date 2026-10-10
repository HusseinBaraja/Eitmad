using System.Globalization;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Features.Reception;

public sealed partial class SalesCatalogViewModel
{
    private QuotationDraftClient? draftClient;
    private QuotationDraft? savedDraft;
    private bool applyingDraft, isDraftBusy, draftConflict, hasUnsavedEdits = true, uncertainSave;
    private long draftSession;
    private Command? retryCommand;
    private string? retryFingerprint;
    private Guid retryKey;
    public bool IsLiveQuotation => catalogClient is not null;
    public bool AreCustomerDetailsReadOnly => IsLiveQuotation && !IsNewCustomer;
    public bool IsDraftBusy { get => isDraftBusy; private set { Set(ref isDraftBusy, value); RaiseDiscountState(); } }
    public bool CanReloadDraft => savedDraft is not null && !IsDraftBusy;
    public string DraftState => savedDraft is null ? "تعديلات محلية — لم يُؤكد حفظ مسودة" : QuotationDraftClient.SyncLabel(savedDraft) + (hasUnsavedEdits ? " — تعديلات غير محفوظة" : "");
    public string FutureActionsNotice => IsLiveQuotation ? "احفظ المسودة ثم أصدر العرض بعد تأكيد الشروط. تحويل العرض إلى طلب غير متاح بعد." : "";
    public string QuotationEditorGuidance => IsLiveQuotation ? "التعديلات محلية حتى تأكيد حفظ المسودة. اختر عميلاً محفوظاً؛ الأسعار والإجمالي من المحرك." : "معاينة عرض السعر — بيانات العميل تُحفظ في محرك الاعتماد";
    public void AttachDraftClient(QuotationDraftClient client)
    {
        draftClient = client; PublishPreview = null;
        client.Changed += DraftChanged; client.ApprovalChanged += ServerApprovalChanged; client.LifecycleChanged += LifecycleChanged; client.Invalidated += DraftInvalidated;
        Raise(nameof(FutureActionsNotice));
    }
    private void DraftInvalidated(object? sender, EventArgs e) { catalogActive = false; ClearCatalog(); QuotationLines.Clear(); ClearDraftSession(); }
    private void ServerApprovalChanged(object? sender, EventArgs e) { if (catalogActive) _ = RefreshApprovalAndLifecycleAsync(); }
    private void DraftChanged(object? sender, EventArgs e) { if (catalogActive && savedDraft is not null && !IsDraftBusy) _ = RefreshDraftStatusAsync(); }
    private async Task RefreshDraftStatusAsync()
    {
        var current = savedDraft!; var session = draftSession;
        var result = await draftClient!.GetAsync(current.Snapshot.Id);
        if (session != draftSession || savedDraft != current || !catalogActive || IsDraftBusy) return;
        if (!result.Succeeded) { QuotationNotice = QuotationDraftClient.Message(result.Failure); return; }
        if (result.Value!.Snapshot.Revision != current.Snapshot.Revision)
        {
            draftConflict = true;
            QuotationNotice = "تغيرت المسودة في مكان آخر. احتُفظ بتعديلاتك؛ أعد فتح النسخة المحفوظة للمراجعة.";
        }
        else
        {
            savedDraft = result.Value;
            draftConflict = savedDraft.SyncState is SyncState.Conflicted or SyncState.Rejected;
            if (draftConflict) QuotationNotice = QuotationDraftClient.SyncLabel(savedDraft);
        }
        RaiseDraftState();
    }
    private void ClearDraftSession()
    {
        DocumentInvalidated?.Invoke(this, EventArgs.Empty);
        quotationLifecycle = null; issueRetry = null; issueKey = Guid.Empty; lifecycleOnline = false; ++lifecycleRead;
        ++draftSession; ++approvalReadVersion; savedDraft = null; serverApproval = null; approvalCommand = null; approvalKey = Guid.Empty; retryCommand = null; retryFingerprint = null; draftConflict = uncertainSave = false; hasUnsavedEdits = true;
        SelectedCustomer = null; applyingCustomer = true; CustomerName = Phone = Address = Notes = ""; applyingCustomer = false;
        customerSearchCancellation?.Cancel(); ++customerSearchVersion; CustomerMatches.Clear();
        QuotationNumber = ""; RaiseDraftState();
    }
    private void RaiseDraftState() { Raise(nameof(DraftState)); Raise(nameof(CanReloadDraft)); RaiseDiscountState(); }
    public async Task<bool> OpenDraftAsync(Guid id)
    {
        if (draftClient is null || IsDraftBusy) return false;
        var session = draftSession; var version = evaluationVersion;
        IsDraftBusy = true; QuotationNotice = "جارٍ تحميل المسودة...";
        try
        {
            var result = await draftClient.GetAsync(id);
            if (session != draftSession || !catalogActive) return false;
            if (!result.Succeeded) { QuotationNotice = QuotationDraftClient.Message(result.Failure); return false; }
            if (version != evaluationVersion) { QuotationNotice = "تغيرت تعديلاتك أثناء التحميل. احتُفظ بها؛ أعد فتح المسودة للمراجعة."; return false; }
            ApplyDraft(result.Value!); await RefreshApprovalAsync(); await RefreshLifecycleAsync(); return true;
        }
        finally { IsDraftBusy = false; RaiseDraftState(); }
    }
    private void ApplyDraft(QuotationDraft draft)
    {
        ClearQuotationEvaluation(); applyingDraft = true;
        savedDraft = draft; draftConflict = draft.SyncState is SyncState.Conflicted or SyncState.Rejected;
        retryCommand = null; retryFingerprint = null; uncertainSave = hasUnsavedEdits = false;
        QuotationLines.Clear();
        foreach (var intent in draft.Snapshot.Intent.Lines)
            QuotationLines.Add(new(intent, draft.Snapshot.Evaluation.Lines.Single(line => line.Id == intent.Id)));
        var customer = draft.Snapshot.Evaluation.Customer;
        applyingCustomer = true;
        CustomerName = customer.Name; Phone = customer.Phone; Address = customer.Address ?? ""; Notes = "";
        applyingCustomer = false;
        SelectedCustomer = new(CustomerName, Phone, Address, Notes, customer.Id, customer.Revision);
        discountInput = (draft.Snapshot.Intent.DiscountBasisPoints / 100m).ToString(CultureInfo.InvariantCulture);
        discountPercent = draft.Snapshot.Intent.DiscountBasisPoints / 100m; isDiscountValid = true;
        evaluation = draft.Snapshot.Evaluation;
        QuotationNumber = "مسودة بدون رقم رسمي"; IsReviewingQuotation = true;
        applyingDraft = false;
        Raise(nameof(DiscountInput)); Raise(nameof(IsQuotationEmpty)); Raise(nameof(QuotationLabel)); RaiseEvaluation(); RaiseDraftState();
        QuotationNotice = DraftState + " — الأسعار المحفوظة؛ راجع أحدث الأسعار قبل حفظ أي تعديل.";
    }
    public Task<bool> ReloadDraftAsync() => savedDraft is { } current ? OpenDraftAsync(current.Snapshot.Id) : Task.FromResult(false);
    public async Task DisposeEditorAsync()
    {
        if (draftClient is not null) { draftClient.Changed -= DraftChanged; draftClient.ApprovalChanged -= ServerApprovalChanged; draftClient.LifecycleChanged -= LifecycleChanged; draftClient.Invalidated -= DraftInvalidated; }
        if (customerClient is not null) customerClient.Changed -= CustomerChanged;
        await DeactivateCatalogAsync();
        if (catalogClient is not null) { catalogClient.Changed -= CatalogChanged; catalogClient.ProjectionInvalidated -= CatalogInvalidated; await catalogClient.DisposeAsync(); }
    }
    public async Task<bool> SaveDraftAsync()
    {
        if (draftClient is null) return ReviewDraftSave();
        if (IsDraftBusy || !LifecycleEditable || draftConflict || !CheckRequiredFields() || !isDiscountValid || !catalogActive) return false;
        var lines = QuotationLines.Select(line => new QuotationLineIntent { Id = line.Id, Configuration = line.Intent! }).ToArray();
        if (lines.Any(line => line.Configuration is null)) { QuotationNotice = "راجع إعدادات الأصناف من الكتالوج."; return false; }
        var input = new EvaluateQuotation { Customer = SelectedCustomer?.Id is { } id ? new() { Id = id, Revision = SelectedCustomer.Revision } : null!,
            Lines = lines, DiscountBasisPoints = (long)(discountPercent * 100m) };
        var command = savedDraft is { } current
            ? Command.ForQuotationDraftUpdate(new() { DraftId = current.Snapshot.Id, ExpectedRevision = current.Snapshot.Revision, Intent = input })
            : Command.ForQuotationDraftCreate(new() { Intent = input });
        var fingerprint = JsonSerializer.Serialize(command);
        if (!uncertainSave && retryFingerprint != fingerprint) { retryCommand = command; retryFingerprint = fingerprint; retryKey = Guid.NewGuid(); }
        var savingCurrentInput = retryFingerprint == fingerprint;
        var session = draftSession; var version = evaluationVersion;
        IsDraftBusy = true; QuotationNotice = "جارٍ حفظ المسودة...";
        try
        {
            var result = await draftClient.SaveAsync(retryCommand!, retryKey);
            if (session != draftSession || !catalogActive) return false;
            if (!result.Succeeded)
            {
                uncertainSave = result.Failure == DraftFailure.Unavailable;
                if (!uncertainSave) { retryCommand = null; retryFingerprint = null; }
                if (result.Failure == DraftFailure.Conflict) draftConflict = true;
                QuotationNotice = result.Errors.Length > 0 ? EvaluationMessage(result.Errors[0]) + " — لم تُحفظ تعديلاتك." : QuotationDraftClient.Message(result.Failure);
                if (result.Errors.Length > 0) { evaluation = null; RaiseEvaluation(); }
                return false;
            }
            savedDraft = result.Value!; retryFingerprint = null; retryCommand = null; uncertainSave = false;
            hasUnsavedEdits = version != evaluationVersion || !savingCurrentInput;
            QuotationNumber = "مسودة بدون رقم رسمي";
            if (!hasUnsavedEdits) { evaluation = savedDraft.Snapshot.Evaluation; ApplyEvaluatedLines(evaluation); RaiseEvaluation(); }
            QuotationNotice = DraftState;
            await RefreshApprovalAsync(); await RefreshLifecycleAsync();
            return true;
        }
        finally { IsDraftBusy = false; RaiseDraftState(); }
    }
    private async Task EditSavedLineAsync(PreviewQuotationLine line)
    {
        if (catalogClient is null || !catalogActive) return;
        CloseSelection(); editingLine = line;
        var input = line.Intent;
        if (input is null) { QuotationNotice = "راجع إعدادات الصنف من الكتالوج."; return; }
        // SelectAsync reads current public definitions. The stored line stays unchanged until the user accepts the selection.
        var item = line.SavedEvaluation is { } evaluated ? line.Item with { Entry = new CatalogEntry { Price = evaluated.Price.Snapshot } } : line.Item;
        await SelectAsync(item);
        if (editingLine != line || !catalogActive) return;
        var target = JsonSerializer.Deserialize<PriceTarget>(JsonSerializer.Serialize(input.Selection.Target))!;
        if (ProductSelection is { } p)
        {
            p.SelectedVariant = p.Variants.FirstOrDefault(v => v.Id == target.AsProduct()?.VariantId); p.Quantity = (int)input.Selection.Quantity;
        }
        if (Selection is { } f)
        {
            f.SelectedSize = f.Sizes.FirstOrDefault(v => v.Id == target.AsFurniture()?.VariantId);
            f.SelectedColor = f.Colors.FirstOrDefault(v => v.Id == input.Selection.ColorId);
            f.SelectedHandle = f.Handles.FirstOrDefault(v => v.Id == input.Selection.HandleId);
            f.Quantity = (int)input.Selection.Quantity;
            if (input.Dimensions is { } d) { f.WidthCm = (d.WidthMm / 10m).ToString(CultureInfo.InvariantCulture); f.HeightCm = (d.HeightMm / 10m).ToString(CultureInfo.InvariantCulture); f.DepthCm = (d.DepthMm / 10m).ToString(CultureInfo.InvariantCulture); }
        }
        IsReviewingQuotation = false;
        SelectionNotice = "راجع السعر والخيارات الحالية. لا تتغير المسودة حتى حفظ التعديلات ثم حفظ كمسودة.";
        await CheckConfigurationAsync();
    }
}
