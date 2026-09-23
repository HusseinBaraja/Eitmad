using System.Collections.ObjectModel;
using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Features.Reception;

/// <summary>Window-scoped synthetic handoffs. Never calls IPC, saves data, or grants authority.</summary>
public sealed class ReceptionHandoffPreview
{
    public ObservableCollection<QuotationListItem> Quotations { get; }
    private int nextNumber = 144;
    public ReceptionHandoffPreview(ObservableCollection<QuotationListItem> quotations) => Quotations = quotations;

    public SalesCatalogViewModel Attach(SalesCatalogViewModel editor)
    {
        editor.PublishPreview = Publish;
        return editor;
    }

    private void Publish(SalesCatalogViewModel editor, bool requestApproval)
    {
        var previous = Quotations.FirstOrDefault(item => item.Id == editor.PreviewId);
        var snapshot = new QuotationListItem(editor.PreviewId,
            previous?.Number ?? $"QT-PREVIEW-{nextNumber++:0000}", editor.CustomerName,
            DateOnly.FromDateTime(DateTime.Today), requestApproval ? QuotationStatus.WaitingApproval : QuotationStatus.Draft,
            editor.Discount, editor.QuotationLines.Select(line => new QuotationLineItem(line.Name, line.Variant,
                line.Color ?? "—", line.Handle ?? "—", line.Quantity, line.UnitPrice)
            {
                IsFurniture = line.Furniture is not null, Dimensions = line.Dimensions,
                ThumbnailKind = line.Item.ThumbnailKind, Image = line.Item.Image,
            }).ToArray(),
            requestApproval || editor.IsDiscountApproved || editor.IsDiscountRejected, editor.Phone)
        {
            CustomerId = editor.SelectedCustomer?.Id,
            Address = editor.Address, Notes = editor.Notes,
            NeedsApprovalToComplete = editor.RequiresDiscountApproval,
            ReceptionActivity = requestApproval ? "طلب خصم من الاستقبال" : previous is null ? "جديد من الاستقبال" : "عُدّل في الاستقبال",
        };
        if (editor.IsDiscountApproved) snapshot.DecideDiscount(DiscountApprovalDecision.Approved);
        else if (editor.IsDiscountRejected && !requestApproval) snapshot.DecideDiscount(DiscountApprovalDecision.Rejected);
        if (previous is null) Quotations.Insert(0, snapshot);
        else Quotations[Quotations.IndexOf(previous)] = snapshot;
        editor.QuotationNumber = snapshot.Number;
        editor.ObserveApproval(snapshot);
    }
}
