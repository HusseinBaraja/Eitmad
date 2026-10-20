using System.Windows;
using System.Windows.Controls;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Reception;
using UserControl = System.Windows.Controls.UserControl;
using FlowDirection = System.Windows.FlowDirection;

namespace Eitmad.WindowsShell.Features.Customers;

public partial class CustomerResolutionView : UserControl
{
    private CustomerClient? client;
    private Customer? customer;
    private CustomerConflictPage? page;
    private ResolveCustomerConflict? frozen;
    private Guid key;
    private CancellationTokenSource? cancellation;
    private int generation;
    private bool busy;
    public Task LastOperation { get; private set; } = Task.CompletedTask;
    public CustomerResolutionView() => InitializeComponent();

    public Task OpenAsync(Customer value, CustomerClient customerClient)
    {
        ClearSession(); customer = value; client = customerClient; Dialog.IsOpen = true;
        return LastOperation = LoadAsync();
    }
    public void ClearSession()
    {
        ++generation; cancellation?.Cancel(); cancellation?.Dispose(); cancellation = null;
        Dialog.IsOpen = false; Sources.ItemsSource = null; Fields.ItemsSource = null;
        customer = null; client = null; page = null; frozen = null; busy = false; Notice.Text = RevisionLabel.Text = "";
    }
    private async Task LoadAsync(Guid? after = null)
    {
        if (customer is null || client is null) return;
        var version = ++generation;
        cancellation?.Cancel(); cancellation?.Dispose(); cancellation = new();
        var token = cancellation.Token; busy = true; UpdateActions(); Notice.Text = "جارٍ تحميل النسختين من الخادم...";
        try {
            var result = await client.ConflictsAsync(customer, after, token);
            if (version != generation || token.IsCancellationRequested) return;
            Sources.ItemsSource = null; Fields.ItemsSource = null;
            if (!result.Succeeded) { page = null; Notice.Text = CustomerClient.ArabicMessage(result.Failure); return; }
            page = result.Value!;
            var options = page.Items.Select((v, i) => new ResolutionOption($"تعارض {i + 1}",
                CustomerResolutionSource.ForConflict(new CustomerConflictSource { ConflictId = v.Conflict.ConflictId }), v.Local, v.Remote)).ToList();
            options.AddRange(page.PendingReviews.Select((v, i) => new ResolutionOption($"تعديل لاحق محفوظ للمراجعة {i + 1}",
                CustomerResolutionSource.ForPendingReview(v.Change), v.Contact, CurrentPayload(page.Current))));
            frozen = page.PendingResolution?.Command; key = page.PendingResolution?.IdempotencyKey ?? Guid.NewGuid();
            Sources.ItemsSource = options;
            if (frozen is not null) {
                Fields.ItemsSource = MakeFields(frozen.Contact); Fields.IsEnabled = false;
                Notice.Text = "نتيجة القرار السابق غير مؤكدة. أعد إرسال القرار نفسه للتحقق؛ لا تغيّر القيم.";
            } else {
                Fields.IsEnabled = true;
                Notice.Text = options.Count == 0 ? "لا توجد تعارضات أو تعديلات معلقة للمراجعة." : "اختر تعارضاً ثم حدد القيمة النهائية لكل حقل.";
            }
            RevisionLabel.Text = $"إصدار الخادم الحالي: {page.Current.Revision}";
            NextButton.Visibility = page.Next is null ? Visibility.Collapsed : Visibility.Visible;
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { }
        finally { if (version == generation) { busy = false; UpdateActions(); } }
    }
    private void SourceChanged(object sender, SelectionChangedEventArgs e)
    {
        if (frozen is not null || page is null || Sources.SelectedItem is not ResolutionOption option) return;
        Fields.ItemsSource = new[] {
            new ResolutionField("الاسم", option.Local.Name, option.Remote.Name, page.Current.Name),
            new ResolutionField("رقم الهاتف", option.Local.Phone, option.Remote.Phone, page.Current.Phone, true),
            new ResolutionField("العنوان", option.Local.Address, option.Remote.Address, page.Current.Address),
            new ResolutionField("ملاحظات", option.Local.Notes, option.Remote.Notes, page.Current.Notes),
        };
        UpdateActions();
    }
    private void ConfirmClick(object sender, RoutedEventArgs e) => LastOperation = ConfirmAsync();
    public async Task ConfirmAsync()
    {
        if (busy || page is null || customer is null || client is null) return;
        if (frozen is null) {
            if (Sources.SelectedItem is not ResolutionOption option || Fields.ItemsSource is not ResolutionField[] fields || fields.Any(v => v.Choice < 0)) {
                Notice.Text = "حدد القيمة النهائية لكل حقل قبل الاعتماد."; return;
            }
            var contact = new CustomerContact { Name = fields[0].Value, Phone = fields[1].Value, Address = Optional(fields[2].Value)!, Notes = Optional(fields[3].Value)! };
            if (!CustomerInputValidation.IsNameValid(contact.Name) || !CustomerInputValidation.IsPhoneValid(contact.Phone)
                || contact.Address is not null && !CustomerInputValidation.IsAddressValid(contact.Address)
                || contact.Notes is not null && !CustomerInputValidation.IsNotesValid(contact.Notes)) { Notice.Text = "تحقق من القيم النهائية للعميل."; return; }
            frozen = new ResolveCustomerConflict { Customer = new GetCustomer { CustomerId = customer.Id, BranchId = customer.Scope.Id },
                ExpectedServerRevision = page.Current.Revision,
                Source = JsonSerializer.Deserialize<Dictionary<string, object>>(JsonSerializer.Serialize(option.Source))!, Contact = contact };
            key = Guid.NewGuid();
        }
        var version = generation; busy = true; UpdateActions(); Fields.IsEnabled = false;
        Notice.Text = "جارٍ التحقق من إصدار الخادم واعتماد القرار...";
        var result = await client.ResolveAsync(frozen, key);
        if (version != generation) return;
        busy = false;
        if (result.Succeeded) {
            frozen = null; Fields.ItemsSource = null; page = null;
            Notice.Text = "اعتمد الخادم القرار. التعديلات اللاحقة محفوظة لمراجعة مستقلة. حدّث المقارنة للمتابعة.";
        } else if (result.Failure == CustomerFailureKind.Unavailable) {
            Notice.Text = "نتيجة القرار غير مؤكدة. أعد إرسال القرار نفسه للتحقق؛ سيبقى محفوظاً بعد إعادة التشغيل.";
        } else {
            frozen = null; page = null; Fields.ItemsSource = null;
            Notice.Text = result.Failure == CustomerFailureKind.RevisionConflict
                ? "تغير إصدار الخادم. لم يُعتمد هذا القرار. حدّث المقارنة وراجع القيم قبل اعتماد قرار جديد."
                : CustomerClient.ArabicMessage(result.Failure);
        }
        UpdateActions();
    }
    private void UpdateActions()
    {
        ConfirmButton.IsEnabled = !busy && page is not null && (frozen is not null || Sources.SelectedItem is ResolutionOption);
        ConfirmButton.Content = frozen is null ? "اعتماد القيم المختارة" : "إعادة إرسال القرار نفسه";
        Sources.IsEnabled = !busy && frozen is null; ReloadButton.IsEnabled = !busy && frozen is null;
        NextButton.IsEnabled = !busy && frozen is null;
    }
    private void ReloadClick(object sender, RoutedEventArgs e) => LastOperation = LoadAsync();
    private void NextClick(object sender, RoutedEventArgs e) => LastOperation = LoadAsync(page?.Next);
    private void CloseClick(object? sender, RoutedEventArgs e) => ClearSession();
    private static string? Optional(string value) => string.IsNullOrWhiteSpace(value) ? null : value.Trim();
    private static CustomerSyncPayload CurrentPayload(Customer c) => new() { CustomerId = c.Id, Name = c.Name, Phone = c.Phone, Address = c.Address, Notes = c.Notes, Revision = c.Revision };
    private static ResolutionField[] MakeFields(CustomerContact c) => [new("الاسم", c.Name, comparison: false), new("رقم الهاتف", c.Phone, ltr: true, comparison: false), new("العنوان", c.Address, comparison: false), new("ملاحظات", c.Notes, comparison: false)];
    private sealed record ResolutionOption(string Label, CustomerResolutionSource Source, CustomerSyncPayload Local, CustomerSyncPayload Remote);
}

public sealed class ResolutionField(string label, string? local, string? remote = null, string? current = null, bool ltr = false, bool comparison = true) : ObservableObject
{
    private int choice = -1;
    private string value = local ?? "";
    public Visibility ComparisonVisibility => comparison ? Visibility.Visible : Visibility.Collapsed;
    public string Label => label;
    public string LocalDisplay => local ?? "غير مضاف";
    public string RemoteDisplay => remote ?? "غير مضاف";
    public string CurrentDisplay => current ?? "غير مضاف";
    public FlowDirection Direction => ltr ? FlowDirection.LeftToRight : FlowDirection.RightToLeft;
    public string ChoiceName => $"اختيار مصدر {label}";
    public string ValueName => $"القيمة النهائية: {label}";
    public string[] Choices => ["التعديل المحلي", "الخادم وقت التعارض", "الخادم الحالي", "قيمة مصححة"];
    public int Choice { get => choice; set { choice = value; Value = value switch { 0 => local ?? "", 1 => remote ?? "", 2 => current ?? "", _ => "" }; Raise(); Raise(nameof(IsReadOnly)); } }
    public string Value { get => value; set { this.value = value; Raise(); } }
    public bool IsReadOnly => choice != 3;
}
