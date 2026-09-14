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

public sealed record PreviewCustomer(string Name, string Phone, string Address, string Notes)
{
    public string Label => Name + " — " + Phone;
}

public sealed partial class SalesCatalogViewModel
{
    private PreviewQuotationLine? editingLine;
    private string customerName = "", phone = "", address = "", notes = "", quotationNotice = "";
    private bool isNewCustomer;
    private PreviewCustomer? previousCustomer;
    private readonly List<PreviewCustomer> customers = [new("عميل تجريبي", "000000000", "عنوان تجريبي", "")];
    public ObservableCollection<PreviewCustomer> CustomerMatches { get; } = [];
    public string QuotationNumber { get; init; } = "";
    public string QuotationHeading => string.IsNullOrWhiteSpace(QuotationNumber) ? "عرض سعر جديد" : QuotationNumber;
    public bool IsQuotationEmpty => QuotationLines.Count == 0;
    public decimal Subtotal => QuotationLines.Sum(line => line.LineTotal);
    // No discount policy exists in the engine yet. The preview starts without a discount.
    public decimal Discount => 0;
    public decimal FinalTotal => Subtotal - Discount;
    public string CustomerName { get => customerName; set { if (Set(ref customerName, value)) MatchCustomers(); } }
    public string Phone { get => phone; set { if (Set(ref phone, value)) MatchCustomers(); } }
    public string Address { get => address; set => Set(ref address, value); }
    public string Notes { get => notes; set => Set(ref notes, value); }
    public bool IsNewCustomer { get => isNewCustomer; private set => Set(ref isNewCustomer, value); }
    public string QuotationNotice { get => quotationNotice; private set => Set(ref quotationNotice, value); }
    private void MatchCustomers()
    {
        CustomerMatches.Clear();
        if (IsNewCustomer) return;
        var name = PreviewText.NormalizeSearch(CustomerName.Trim());
        foreach (var customer in customers.Where(c => (name.Length > 0 && PreviewText.NormalizeSearch(c.Name).Contains(name, StringComparison.OrdinalIgnoreCase)) || (Phone.Trim().Length > 0 && c.Phone.Contains(Phone.Trim(), StringComparison.Ordinal)))) CustomerMatches.Add(customer);
    }
    public void AttachCustomer(PreviewCustomer customer)
    {
        CustomerName = customer.Name; Phone = customer.Phone; Address = customer.Address; Notes = customer.Notes;
        CustomerMatches.Clear();
        QuotationNotice = "تم اختيار العميل للمعاينة فقط";
    }
    public void BeginNewCustomer()
    {
        previousCustomer = new(CustomerName, Phone, Address, Notes);
        IsNewCustomer = true;
        CustomerName = Phone = Address = Notes = "";
        CustomerMatches.Clear(); QuotationNotice = "";
    }
    public void CancelNewCustomer()
    {
        IsNewCustomer = false;
        if (previousCustomer is not null) AttachCustomer(previousCustomer);
        QuotationNotice = "";
    }
    public bool SaveNewCustomer()
    {
        if (!CheckCustomer()) return false;
        var customer = new PreviewCustomer(CustomerName, Phone, Address, Notes);
        customers.Add(customer); IsNewCustomer = false; AttachCustomer(customer); return true;
    }
    private bool CheckCustomer()
    {
        if (!string.IsNullOrWhiteSpace(CustomerName) && !string.IsNullOrWhiteSpace(Phone)) return true;
        QuotationNotice = "أدخل اسم العميل ورقم الهاتف للمتابعة"; return false;
    }
    public bool ReviewSave()
    {
        if (IsQuotationEmpty) { QuotationNotice = "أضف صنفاً إلى عرض السعر أولاً"; return false; }
        if (!CheckCustomer()) return false;
        QuotationNotice = "المعاينة مكتملة — حفظ عرض السعر غير متاح بعد، ولم تُحفظ البيانات"; return true;
    }
    private void RefreshQuotation()
    {
        foreach (var property in new[] { nameof(QuotationLabel), nameof(IsQuotationEmpty), nameof(Subtotal), nameof(FinalTotal) }) Raise(property);
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
    public void RemoveLine(PreviewQuotationLine line) => QuotationLines.Remove(line);
}
