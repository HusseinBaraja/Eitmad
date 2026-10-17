using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Tests.Rendered;

public sealed partial class WorkflowEnvironmentTests
{
    [TestMethod]
    public void PreparedFamilySelectionKeepsOriginalAmongLaterPracticeOrders()
    {
        var quotation = FamilySelectionQuotation("QT-2026-00001");
        var original = FamilySelectionOrder(quotation, OrderState.Ready);
        var practice = Enumerable.Range(2, 101).Select(index =>
            FamilySelectionOrder(FamilySelectionQuotation($"QT-2026-{index:D5}"), OrderState.Delivered)).ToList();
        // Another Ready story must not replace the original either.
        practice.Add(FamilySelectionOrder(FamilySelectionQuotation("QT-2026-00103"), OrderState.Ready));
        practice.Add(original);
        var selected = SelectPreparedFamilyPurchase(practice.Select(order => order.Source), practice);
        Assert.AreSame(original, selected.Order);
        Assert.AreSame(quotation, selected.Quotation);
        practice.Reverse();
        Assert.AreSame(original, SelectPreparedFamilyPurchase(practice.Select(order => order.Source), practice).Order);
    }

    [TestMethod]
    [DataRow("missing quotation")]
    [DataRow("ambiguous quotation")]
    [DataRow("missing order")]
    [DataRow("ambiguous order")]
    [DataRow("wrong source identity")]
    [DataRow("wrong source number")]
    [DataRow("delivered original")]
    [DataRow("delivery on ready original")]
    public void PreparedFamilySelectionRejectsInvalidOriginalDespiteReadyPractice(string failure)
    {
        var quotation = FamilySelectionQuotation("QT-2026-00001");
        var original = FamilySelectionOrder(quotation, OrderState.Ready);
        var practice = FamilySelectionOrder(FamilySelectionQuotation("QT-2026-00002"), OrderState.Ready);
        var quotations = new List<QuotationRecord> { quotation, practice.Source };
        var orders = new List<OrderRecord> { practice, original };
        switch (failure)
        {
            case "missing quotation": quotations.Remove(quotation); break;
            case "ambiguous quotation": quotations.Add(FamilySelectionQuotation(quotation.Number)); break;
            case "missing order": orders.Remove(original); break;
            case "ambiguous order": orders.Add(FamilySelectionOrder(quotation, OrderState.Ready)); break;
            case "wrong source identity": original.Source = FamilySelectionQuotation(quotation.Number); break;
            case "wrong source number": original.Source = FamilySelectionQuotation("QT-2026-00009");
                original.Source.Quotation.Id = quotation.Quotation.Id; break;
            case "delivered original": original.State = OrderState.Delivered; break;
            case "delivery on ready original": original.Delivery = new(); break;
        }
        Assert.ThrowsExactly<AssertFailedException>(() => SelectPreparedFamilyPurchase(quotations, orders));
    }

    private static QuotationRecord FamilySelectionQuotation(string number) => new()
    {
        Number = number, Quotation = new() { Id = Guid.NewGuid() }, State = QuotationState.Converted,
    };

    private static OrderRecord FamilySelectionOrder(QuotationRecord source, OrderState state) => new()
    {
        Id = Guid.NewGuid(), Source = source, State = state,
    };
}
