using System.Globalization;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using UserControl = System.Windows.Controls.UserControl;
using FlowDirection = System.Windows.FlowDirection;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Features.Quotations;

public partial class QuotationDraftResolutionView : UserControl
{
    private readonly QuotationDraftClient client;
    private readonly Guid draftId, branchId;
    private QuotationDraftReview? page;
    private ConfirmQuotationDraftResolution? frozen;
    private bool busy, active = true;
    public event EventHandler? CloseRequested;
    private sealed record Source(string Label, QuotationDraftResolutionSource Value, QuotationDraftSnapshot Local, QuotationDraftSnapshot? Remote);
    private sealed record Field(string Label, string Value, FlowDirection Direction = FlowDirection.RightToLeft)
    {
        public TextAlignment Alignment => Direction == FlowDirection.LeftToRight ? TextAlignment.Right : TextAlignment.Left;
    }
    public QuotationDraftResolutionView(QuotationDraftClient client, Guid draftId, Guid branchId)
    {
        InitializeComponent(); this.client = client; this.draftId = draftId; this.branchId = branchId;
        client.Invalidated += Invalidated;
    }
    public void Detach() { active = false; client.Invalidated -= Invalidated; SetBusy(false); }
    private void Invalidated(object? sender, EventArgs e) { Detach(); Clear(); CloseRequested?.Invoke(this, EventArgs.Empty); }
    private void Clear() { Sources.ItemsSource = null; CurrentFields.ItemsSource = InputFields.ItemsSource = RemoteFields.ItemsSource = null; page = null; frozen = null; }
    public async Task LoadAsync(Guid? after = null)
    {
        if (busy || !active) return;
        SetBusy(true); Notice.Text = "جارٍ تحميل المراجعة من الخادم...";
        try {
            var result = await client.ReviewAsync(new() { DraftId = draftId, BranchId = branchId, After = after });
            if (!active) return;
            if (!result.Succeeded) { Clear(); Notice.Text = Message(result.Failure); return; }
            page = result.Value!; frozen = page.PendingResolution;
            var sources = page.Items.Select(v => new Source("تعارض محفوظ · المراجعة " + v.Local.Revision,
                QuotationDraftResolutionSource.ForConflict(new() { ConflictId = v.Conflict.ConflictId }), v.Local, v.Remote)).ToList();
            sources.AddRange(page.PendingReviews.Select(v => new Source("تعديل لاحق محفوظ · المراجعة " + v.Snapshot.Revision,
                QuotationDraftResolutionSource.ForPendingReview(v.Change), v.Snapshot, null)));
            Sources.ItemsSource = sources; Sources.SelectedIndex = sources.Count > 0 ? 0 : -1;
            CurrentFields.ItemsSource = Fields(page.Current);
            RevisionLabel.Text = "مراجعة الخادم الحالية: " + page.Current.Revision.ToString(CultureInfo.InvariantCulture);
            NextButton.Visibility = page.Next is null ? Visibility.Collapsed : Visibility.Visible;
            Choice.SelectedIndex = -1;
            Notice.Text = frozen is not null ? "لم تتأكد نتيجة القرار السابق. أعد إرسال القرار نفسه للتحقق من نتيجته."
                : sources.Count == 0 ? "لا توجد تعارضات أو تعديلات محفوظة للمراجعة." : "اختر نسخة ثم حدد قرارك.";
            if (frozen is not null) {
                Choice.SelectedIndex = frozen.Command.Choice == QuotationDraftResolutionChoice.KeepCurrent ? 0 : 1;
                var frozenSource = JsonSerializer.Serialize(frozen.Command.Source);
                Sources.SelectedItem = sources.FirstOrDefault(v => JsonSerializer.Serialize(v.Value) == frozenSource);
                Choice.SelectedIndex = frozen.Command.Choice == QuotationDraftResolutionChoice.KeepCurrent ? 0 : 1;
                ConfirmButton.Content = "إعادة إرسال القرار السابق";
            } else ConfirmButton.Content = "تأكيد حل التعارض";
        } finally { SetBusy(false); }
    }
    private void SetBusy(bool value)
    {
        busy = value;
        Sources.IsEnabled = Choice.IsEnabled = active && !busy && page is not null && frozen is null;
        ConfirmButton.IsEnabled = active && !busy && (frozen is not null || page?.CanResolve == true && Sources.SelectedItem is Source);
        ReloadButton.IsEnabled = active && !busy && frozen is null;
        NextButton.IsEnabled = active && !busy && frozen is null;
    }
    private void SourceChanged(object sender, SelectionChangedEventArgs e)
    {
        if (Sources.SelectedItem is not Source source) { InputFields.ItemsSource = RemoteFields.ItemsSource = null; return; }
        InputFields.ItemsSource = Fields(source.Local); RemoteFields.ItemsSource = source.Remote is { } remote ? Fields(remote) : null;
        Choice.SelectedIndex = -1;
    }
    private async void ConfirmClick(object sender, RoutedEventArgs e) => await ConfirmAsync();
    internal async Task ConfirmAsync()
    {
        if (busy || !active || page is null || frozen is null && !page.CanResolve) return;
        if (frozen is null) {
            if (Sources.SelectedItem is not Source source || Choice.SelectedIndex < 0) { Notice.Text = "اختر النسخة المطلوبة صراحة قبل التأكيد."; Choice.Focus(); return; }
            frozen = new() { Scope = page.Current.Evaluation.Scope, IdempotencyKey = Guid.NewGuid(),
                Command = new() { DraftId = draftId, BranchId = branchId, ExpectedServerRevision = page.Current.Revision,
                    Source = JsonSerializer.Deserialize<Dictionary<string, object>>(JsonSerializer.Serialize(source.Value))!, Choice = Choice.SelectedIndex == 0 ? QuotationDraftResolutionChoice.KeepCurrent : QuotationDraftResolutionChoice.UseInput } };
        }
        SetBusy(true); Notice.Text = "جارٍ تثبيت القرار على الخادم...";
        var result = await client.ResolveAsync(frozen.Command, frozen.IdempotencyKey);
        if (!active) return;
        if (result.Succeeded) { frozen = null; SetBusy(false); await LoadAsync(); Notice.Text = "ثبت الخادم القرار. راجع أي تعديلات محفوظة أخرى."; }
        else {
            if (result.Failure != DraftFailure.Unavailable) frozen = null;
            Notice.Text = Message(result.Failure); SetBusy(false);
            if (result.Failure == DraftFailure.Denied) Clear();
            if (result.Failure is DraftFailure.Conflict or DraftFailure.Invalid) ConfirmButton.IsEnabled = false;
        }
    }
    private async void ReloadClick(object sender, RoutedEventArgs e) => await LoadAsync();
    private async void NextClick(object sender, RoutedEventArgs e) { if (page?.Next is { } after) await LoadAsync(after); }
    private void CloseClick(object sender, RoutedEventArgs e) { Detach(); CloseRequested?.Invoke(this, EventArgs.Empty); }
    private static string Message(DraftFailure failure) => failure switch {
        DraftFailure.Conflict => "تغيرت المسودة أو حالة العرض. حدث المراجعة قبل قرار جديد.",
        DraftFailure.Invalid => "تعذر اعتماد المدخل. راجع بيانات العميل والأصناف والأسعار ثم احفظ تعديلاً جديداً.",
        DraftFailure.Denied => "هذه المراجعة متاحة لمدير مخول فقط.",
        _ => "تعذر تأكيد النتيجة. تبقى المدخلات محفوظة ويمكن إعادة إرسال القرار نفسه.",
    };
    private static Field[] Fields(QuotationDraftSnapshot snapshot)
    {
        var value = snapshot.Evaluation; var customer = value.Customer;
        var fields = new List<Field> {
            new("حالة المسودة", snapshot.Cancelled == true ? "ملغاة" : "مسودة"),
            new("العميل", customer?.Name ?? "غير متاح"), new("الهاتف", customer?.Phone ?? "—", FlowDirection.LeftToRight),
            new("العنوان", customer?.Address ?? "—"),
            new("مراجعة العميل", customer?.Revision.ToString(CultureInfo.InvariantCulture) ?? "—", FlowDirection.LeftToRight),
            new("نسبة الخصم", (value.DiscountBasisPoints / 100m).ToString("0.00", CultureInfo.InvariantCulture) + "%", FlowDirection.LeftToRight),
            new("الإجمالي · ر.ي", value.Totals?.TotalYer.ToString("N0", CultureInfo.InvariantCulture) ?? "غير متاح", FlowDirection.LeftToRight),
        };
        foreach (var line in value.Lines) {
            fields.Add(new("الصنف", line.Name + " · " + line.VariantName));
            fields.Add(new("الوصف", line.Description));
            fields.Add(new("اللون والمقبض", (line.ColorName ?? "—") + " · " + (line.HandleName ?? "—")));
            if (line.Dimensions is { } dimensions) fields.Add(new("الأبعاد · سم", SalesCatalogViewModel.DimensionsLabel(dimensions), FlowDirection.LeftToRight));
            fields.Add(new("الكمية · سعر الوحدة · الإجمالي (ر.ي)", $"{line.Quantity} · {line.Price.UnitPriceYer:N0} · {line.Price.TotalYer:N0}", FlowDirection.LeftToRight));
            fields.Add(new("مراجعة السعر", line.Price.Snapshot.Revision.ToString(CultureInfo.InvariantCulture), FlowDirection.LeftToRight));
        }
        return fields.ToArray();
    }
}
