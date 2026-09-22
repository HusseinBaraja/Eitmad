using System.Collections.ObjectModel;

namespace Eitmad.WindowsShell.Features.Reception;

// Immutable selections from synthetic catalog fixtures; never durable quotation records.
public sealed record PreviewQuotationLine
{
    public Guid Id { get; init; } = Guid.NewGuid();
    public SalesCatalogItem Item { get; }
    public FurnitureSelectionViewModel? Furniture { get; }
    public ProductSelectionViewModel? Product { get; }
    public PreviewQuotationLine(FurnitureSelectionViewModel value)
    {
        Item = value.Item;
        Furniture = new(value.Item, value.Sizes, value.Colors, value.Handles)
        { SelectedSize = value.SelectedSize, SelectedColor = value.SelectedColor, SelectedHandle = value.SelectedHandle, Quantity = value.Quantity };
    }
    public PreviewQuotationLine(ProductSelectionViewModel value)
    {
        Item = value.Item;
        Product = new(value.Item, value.Variants) { SelectedVariant = value.SelectedVariant, Quantity = value.Quantity };
    }
    public string Name => Item.Name;
    public string Variant => Furniture?.SelectedSize?.Name ?? Product?.SelectedVariant?.Name ?? string.Empty;
    public string Dimensions => Furniture?.SelectedSize?.DimensionsLabel ?? string.Empty;
    public string? Color => Furniture?.SelectedColor?.Name;
    public string? Handle => Furniture?.SelectedHandle?.Name;
    public string Options => string.Join(" · ", new[] { Variant, Color is null ? null : "اللون: " + Color, Handle is null ? null : "المقبض: " + Handle }.Where(s => !string.IsNullOrEmpty(s)));
    public int Quantity => Furniture?.Quantity ?? Product!.Quantity;
    public decimal UnitPrice => Furniture?.UnitPrice ?? Product!.UnitPrice;
    public decimal LineTotal => Furniture?.LineTotal ?? Product!.LineTotal;
}

public sealed record PreviewCustomer(string Name, string Phone, string Address, string Notes, Guid? Id = null, long Revision = 0)
{
    public string Label => Name + " — " + Phone;

    public static PreviewCustomer FromContract(Eitmad.Contracts.Customer customer) =>
        new(customer.Name, customer.Phone, customer.Address ?? string.Empty, customer.Notes ?? string.Empty,
            customer.Id, customer.Revision);
}

public sealed partial class SalesCatalogViewModel
{
    private PreviewQuotationLine? editingLine;
    private string customerName = "", phone = "", address = "", notes = "", quotationNotice = "";
    private bool isNewCustomer;
    private PreviewCustomer? previousCustomer;
    private Features.Customers.CustomerClient? customerClient;
    private CancellationTokenSource? customerSearchCancellation;
    private long customerSearchVersion;
    private bool applyingCustomer;
    private bool isCustomerBusy;
    private bool isCustomerSaveBusy;
    private string customerOperationError = string.Empty;
    private IReadOnlySet<string> invalidCustomerFields = new HashSet<string>();
    public PreviewCustomer? SelectedCustomer { get; private set; }
    public ObservableCollection<PreviewCustomer> CustomerMatches { get; } = [];
    private string quotationNumber = "";
    public string QuotationNumber { get => quotationNumber; set { if (Set(ref quotationNumber, value)) Raise(nameof(QuotationHeading)); } }
    public string QuotationHeading => string.IsNullOrWhiteSpace(QuotationNumber) ? "عرض سعر جديد" : QuotationNumber;
    public bool IsQuotationEmpty => QuotationLines.Count == 0;
    public decimal Subtotal => QuotationLines.Sum(line => line.LineTotal);
    public decimal Discount => IsDiscountValid ? decimal.Round(Subtotal * (discountPercent / 100m), 0, MidpointRounding.AwayFromZero) : 0;
    public decimal FinalTotal => Subtotal - Discount;
    private bool showRequiredErrors;
    public string CustomerNameError => showRequiredErrors && string.IsNullOrWhiteSpace(CustomerName)
        ? "أدخل اسم العميل"
        : invalidCustomerFields.Contains("name") ? "تحقق من اسم العميل" : "";
    public string PhoneError => showRequiredErrors && string.IsNullOrWhiteSpace(Phone)
        ? "أدخل رقم الهاتف"
        : invalidCustomerFields.Contains("phone") ? "تحقق من رقم الهاتف" : "";
    public string ItemsError => showRequiredErrors && IsQuotationEmpty ? "أضف صنفاً واحداً على الأقل" : "";
    public bool IsCustomerBusy { get => isCustomerBusy; private set => Set(ref isCustomerBusy, value); }
    public bool IsCustomerSaveBusy { get => isCustomerSaveBusy; private set => Set(ref isCustomerSaveBusy, value); }
    public string CustomerOperationError { get => customerOperationError; private set => Set(ref customerOperationError, value); }
    public string CustomerName { get => customerName; set { if (Set(ref customerName, value)) { if (!applyingCustomer) InvalidateDiscountRequest(); CustomerInputChanged(value); Raise(nameof(CustomerNameError)); } } }
    public string Phone { get => phone; set { if (Set(ref phone, value)) { if (!applyingCustomer) InvalidateDiscountRequest(); CustomerInputChanged(value); Raise(nameof(PhoneError)); } } }
    public string Address { get => address; set { if (Set(ref address, value) && !applyingCustomer) InvalidateDiscountRequest(); } }
    public string Notes { get => notes; set { if (Set(ref notes, value) && !applyingCustomer) InvalidateDiscountRequest(); } }
    public bool IsNewCustomer { get => isNewCustomer; private set => Set(ref isNewCustomer, value); }
    public string QuotationNotice { get => quotationNotice; private set => Set(ref quotationNotice, value); }
    public void AttachCustomerClient(Features.Customers.CustomerClient client)
    {
        if (ReferenceEquals(customerClient, client)) return;
        if (customerClient is not null) customerClient.Changed -= CustomerChanged;
        customerClient = client;
        customerClient.Changed += CustomerChanged;
    }

    private void CustomerInputChanged(string value)
    {
        if (applyingCustomer) return;
        SelectedCustomer = null;
        invalidCustomerFields = new HashSet<string>();
        CustomerOperationError = string.Empty;
        QueueCustomerSearch(value);
    }

    private void QueueCustomerSearch(string value)
    {
        customerSearchCancellation?.Cancel();
        customerSearchCancellation?.Dispose();
        customerSearchCancellation = new CancellationTokenSource();
        var version = ++customerSearchVersion;
        if (IsNewCustomer || string.IsNullOrWhiteSpace(value) || customerClient is null)
        {
            CustomerMatches.Clear();
            IsCustomerBusy = false;
            return;
        }
        _ = SearchCustomersAsync(value.Trim(), version, customerSearchCancellation.Token);
    }

    private async Task SearchCustomersAsync(string term, long version, CancellationToken cancellationToken)
    {
        IsCustomerBusy = true;
        try
        {
            var result = await customerClient!.SearchAsync(term, cancellationToken);
            if (cancellationToken.IsCancellationRequested || version != customerSearchVersion) return;
            CustomerMatches.Clear();
            if (result.Succeeded)
                foreach (var customer in result.Value!) CustomerMatches.Add(PreviewCustomer.FromContract(customer));
            else CustomerOperationError = Features.Customers.CustomerClient.ArabicMessage(result.Failure);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        finally
        {
            if (version == customerSearchVersion) IsCustomerBusy = false;
        }
    }
    public void AttachCustomer(PreviewCustomer customer, bool preserveDiscountRequest = false)
    {
        if (!preserveDiscountRequest) InvalidateDiscountRequest();
        applyingCustomer = true;
        CustomerName = customer.Name; Phone = customer.Phone; Address = customer.Address; Notes = customer.Notes;
        applyingCustomer = false;
        SelectedCustomer = customer;
        invalidCustomerFields = new HashSet<string>();
        Raise(nameof(CustomerNameError));
        Raise(nameof(PhoneError));
        CustomerMatches.Clear();
        CustomerOperationError = string.Empty;
        QuotationNotice = "تم اختيار العميل";
    }
    public void BeginNewCustomer()
    {
        previousCustomer = new(CustomerName, Phone, Address, Notes);
        IsNewCustomer = true;
        SelectedCustomer = null;
        CustomerName = Phone = Address = Notes = "";
        CustomerMatches.Clear(); CustomerOperationError = QuotationNotice = "";
    }
    public void CancelNewCustomer()
    {
        IsNewCustomer = false;
        if (previousCustomer is not null) AttachCustomer(previousCustomer);
        QuotationNotice = "";
    }
    public async Task<bool> SaveNewCustomerAsync(CancellationToken cancellationToken = default)
    {
        if (IsCustomerSaveBusy) return false;
        if (!CheckCustomer()) return false;
        if (customerClient is null)
        {
            CustomerOperationError = "تعذر الاتصال ببيانات العملاء. حاول مرة أخرى.";
            return false;
        }
        IsCustomerSaveBusy = true;
        IsCustomerBusy = true;
        try
        {
            CustomerOperationError = string.Empty;
            var result = await customerClient.CreateAsync(CustomerName, Phone, Address, Notes, cancellationToken);
            if (!result.Succeeded)
            {
                invalidCustomerFields = result.InvalidFields;
                CustomerOperationError = Features.Customers.CustomerClient.ArabicMessage(result.Failure);
                Raise(nameof(CustomerNameError));
                Raise(nameof(PhoneError));
                return false;
            }
            IsNewCustomer = false;
            AttachCustomer(PreviewCustomer.FromContract(result.Value!));
            return true;
        }
        finally
        {
            IsCustomerSaveBusy = false;
            IsCustomerBusy = false;
        }
    }

    private void CustomerChanged(object? sender, Guid? customerId)
    {
        if (IsNewCustomer) return;
        if (SelectedCustomer?.Id is { } selectedId && (customerId is null || customerId == selectedId))
        {
            _ = RefreshSelectedCustomerAsync(selectedId);
        }
        var term = CustomerName.Length > 0 ? CustomerName : Phone;
        if (term.Length > 0) QueueCustomerSearch(term);
    }

    private async Task RefreshSelectedCustomerAsync(Guid selectedId)
    {
        if (customerClient is null) return;
        var result = await customerClient.GetAsync(selectedId);
        if (SelectedCustomer?.Id != selectedId) return;
        if (!result.Succeeded)
        {
            SelectedCustomer = null;
            CustomerOperationError = Features.Customers.CustomerClient.ArabicMessage(result.Failure);
            return;
        }
        var previousRevision = SelectedCustomer.Revision;
        var previousNotice = QuotationNotice;
        AttachCustomer(PreviewCustomer.FromContract(result.Value!), preserveDiscountRequest: true);
        if (previousRevision > 0 && result.Value!.Revision > previousRevision)
            QuotationNotice = "تغيرت بيانات العميل. راجع أحدث البيانات قبل المتابعة.";
        else QuotationNotice = previousNotice;
    }
    private bool CheckCustomer()
    {
        showRequiredErrors = true;
        Raise(nameof(CustomerNameError)); Raise(nameof(PhoneError));
        var nameValid = Features.Customers.CustomerInputValidation.IsNameValid(CustomerName);
        var phoneValid = Features.Customers.CustomerInputValidation.IsPhoneValid(Phone);
        if (nameValid && phoneValid) return true;
        var invalidFields = new HashSet<string>();
        if (!nameValid && !string.IsNullOrWhiteSpace(CustomerName)) invalidFields.Add("name");
        if (!phoneValid && !string.IsNullOrWhiteSpace(Phone)) invalidFields.Add("phone");
        invalidCustomerFields = invalidFields;
        Raise(nameof(CustomerNameError)); Raise(nameof(PhoneError));
        QuotationNotice = "تحقق من اسم العميل ورقم الهاتف للمتابعة"; return false;
    }
    public bool ReviewSave()
    {
        if (!CheckRequiredFields()) return false;
        if (!CanSaveQuotation) { QuotationNotice = DiscountError.Length > 0 ? DiscountError : DiscountStatus; return false; }
        PublishPreview?.Invoke(this, false);
        QuotationNotice = "المعاينة مكتملة — حفظ عرض السعر غير متاح بعد، ولم يُحفظ عرض السعر"; return true;
    }
    public bool CheckRequiredFields()
    {
        var customerValid = CheckCustomer();
        Raise(nameof(ItemsError));
        if (!customerValid || IsQuotationEmpty)
        {
            QuotationNotice = "أكمل الحقول المطلوبة وأضف صنفاً واحداً على الأقل";
            return false;
        }
        return true;
    }
    public bool CanPreviewCustomer => CanSaveQuotation && !IsQuotationEmpty;
    private void RefreshQuotation()
    {
        InvalidateDiscountRequest();
        foreach (var property in new[] { nameof(QuotationLabel), nameof(IsQuotationEmpty), nameof(Subtotal), nameof(FinalTotal), nameof(ItemsError), nameof(CanPreviewCustomer) }) Raise(property);
        QuotationNotice = "";
    }
    private void StoreLine(PreviewQuotationLine line)
    {
        if (editingLine is null) QuotationLines.Add(line);
        else
        {
            var index = QuotationLines.IndexOf(editingLine);
            if (index >= 0) QuotationLines[index] = line with { Id = editingLine.Id };
            CloseSelection();
        }
    }
    public void EditLine(PreviewQuotationLine line)
    {
        if (!QuotationLines.Contains(line)) return;
        CloseSelection(); editingLine = line;
        if (line.Furniture is { } f) Selection = new(f.Item, f.Sizes, f.Colors, f.Handles)
        { SelectedSize = f.SelectedSize, SelectedColor = f.SelectedColor, SelectedHandle = f.SelectedHandle, Quantity = f.Quantity, IsEditing = true };
        if (line.Product is { } p) ProductSelection = new(p.Item, p.Variants)
        { SelectedVariant = p.SelectedVariant, Quantity = p.Quantity, IsEditing = true };
        IsReviewingQuotation = false;
    }
    public void DuplicateLine(PreviewQuotationLine line) { if (QuotationLines.Contains(line)) QuotationLines.Insert(QuotationLines.IndexOf(line) + 1, line with { Id = Guid.NewGuid() }); }
}
