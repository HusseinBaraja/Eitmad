using System.Collections.ObjectModel;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Features.Quotations;

public sealed partial class QuotationsViewModel
{
    private QuotationDraftClient? draftClient;
    private CancellationTokenSource? draftLoad;
    private long loadVersion, approvalSession;
    private bool draftsActive;
    private string listState = "المسودات غير متاحة.";
    public string ListState { get => listState; private set { Set(ref listState, value); Raise(nameof(ListSubtitle)); } }
    internal Task LastDraftLoad { get; private set; } = Task.CompletedTask;
    public void AttachDraftClient(QuotationDraftClient client)
    {
        draftClient = client;
        UsePreviewQuotations(new ObservableCollection<QuotationListItem>());
        client.Changed += DraftsChanged;
        client.ApprovalChanged += DraftsChanged;
        client.Invalidated += DraftsInvalidated;
        Raise(nameof(ListSubtitle)); Raise(nameof(EmptyDescription));
    }
    private void DraftsChanged(object? sender, EventArgs e) { if (draftsActive) LastDraftLoad = LoadDraftsAsync(); }
    private void DraftsInvalidated(object? sender, EventArgs e) => ClearDrafts();
    public async Task ActivateDraftsAsync()
    {
        if (draftClient is null) return;
        draftsActive = true;
        await draftClient.ActivateAsync();
        LastDraftLoad = LoadDraftsAsync(); await LastDraftLoad;
    }
    public void ClearDrafts()
    {
        if (draftClient is null) return;
        ++approvalSession; decisionIntent = null; decisionKey = Guid.Empty; ApprovalReason = ""; DecisionNotice = ""; isApprovalBusy = false; Raise(nameof(CanDecideApproval));
        draftsActive = false; ++loadVersion; draftLoad?.Cancel();
        CloseQuotation(); quotations.Clear(); RefreshVisibleQuotations(); ListState = "المسودات غير متاحة.";
    }
    public async Task DeactivateDraftsAsync() { ClearDrafts(); if (draftClient is not null) await draftClient.DeactivateAsync(); }
    private async Task LoadDraftsAsync()
    {
        draftLoad?.Cancel(); draftLoad?.Dispose(); draftLoad = new();
        var token = draftLoad.Token; var version = ++loadVersion;
        ListState = "جارٍ تحميل المسودات...";
        try
        {
            var rows = new List<QuotationListItem>(); Guid? after = null;
            do
            {
                var result = await draftClient!.ListAsync(after, token);
                if (token.IsCancellationRequested || version != loadVersion || !draftsActive) return;
                if (!result.Succeeded)
                {
                    // Remove protected content on denial and unavailable reads; never substitute samples.
                    CloseQuotation(); quotations.Clear(); RefreshVisibleQuotations();
                    ListState = QuotationDraftClient.Message(result.Failure); return;
                }
                rows.AddRange(result.Value!.Items.Select(draft => Project(draft))); after = result.Value.Next;
            } while (after is not null);
            if (draftClient!.SupportsApprovals) {
                var approvals = new List<DiscountApproval>(); Guid? cursor = null;
                do {
                    var result = await draftClient.ApprovalsAsync(cursor, token);
                    if (token.IsCancellationRequested || version != loadVersion || !draftsActive) return;
                    if (!result.Succeeded) { DecisionNotice = QuotationDraftClient.ApprovalMessage(result.Failure); break; }
                    approvals.AddRange(result.Value!.Items); cursor = result.Value.Next;
                } while (cursor is not null);
                if (approvals.Any(a => a.RequestId == decisionRequest && a.State != DiscountApprovalState.Pending)) decisionIntent = null;
                foreach (var approval in approvals) {
                    var index = rows.FindIndex(row => row.Id == approval.Quotation.Id);
                    var draft = index >= 0 && (IsReceptionist || approval.State != DiscountApprovalState.Pending) ? rows[index].Draft! : new QuotationDraft { Scope = approval.Scope, Snapshot = approval.Quotation, UpdatedAt = approval.RequestedAt, SyncState = SyncState.Confirmed };
                    var row = Project(draft, approval);
                    if (index >= 0) rows[index] = row; else rows.Add(row);
                }
            }
            var selected = SelectedQuotation?.Id;
            quotations.Clear(); foreach (var row in rows) quotations.Add(row);
            if (selected is { } id) SelectedQuotation = quotations.FirstOrDefault(row => row.Id == id);
            RefreshVisibleQuotations();
            ListState = "المسودات وطلبات الخصم — الإصدار والطباعة والتحويل غير متاحة بعد.";
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { }
    }
    internal static QuotationListItem Project(QuotationDraft draft, DiscountApproval? approval = null)
    {
        var value = draft.Snapshot.Evaluation; var customer = value.Customer;
        return new(draft.Snapshot.Id, "بدون رقم رسمي", customer.Name,
            DateOnly.FromDateTime(DateTimeOffset.FromUnixTimeMilliseconds(draft.UpdatedAt).LocalDateTime),
            QuotationStatus.Draft, value.Totals.DiscountYer,
            value.Lines.Select(line => new QuotationLineItem(line.Name, line.VariantName, line.ColorName ?? "—", line.HandleName ?? "—", (int)line.Quantity, line.Price.UnitPriceYer) {
                EvaluatedTotal = line.Price.TotalYer, IsFurniture = line.Dimensions is not null,
                Dimensions = line.Dimensions is { } d ? SalesCatalogViewModel.DimensionsLabel(d) : "",
            }).ToArray(), phone: customer.Phone) {
                Draft = draft, Approval = approval, CustomerId = customer.Id, Address = customer.Address ?? "",
                NeedsApprovalToComplete = value.Totals.ApprovalRequired, ReceptionActivity = QuotationDraftClient.SyncLabel(draft),
            };
    }
}
