using System.Windows;
using System.Windows.Controls;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.WorkOrders;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class PreviewHandoffRenderedTests
{
    [TestMethod]
    [DataRow(1338)]
    [DataRow(780)]
    public void ReceptionRequestReachesInboxAndDecisionsGateTheExactEditor(double width)
    {
        WpfTestHost.Run(width, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            var manager = WpfTestHost.FindByName<QuotationsView>(window, "QuotationsSurface");
            var editor = reception.Handoffs.Attach(new SalesCatalogViewModel(new Features.Furniture.FurnitureViewModel(), new Features.Products.ProductsViewModel()));
            editor.Select(editor.VisibleItems.Single(item => item.Name == "وسادة فندقية"));
            Assert.IsTrue(editor.AddProductSelection());
            editor.CustomerName = "عميل اختبار الموافقة"; editor.Phone = "000000001";
            editor.Address = "عنوان تجريبي"; editor.Notes = "ملاحظة داخلية";
            editor.DiscountInput = "10";
            Assert.IsTrue(editor.ReviewDraftSave());
            var draft = reception.Handoffs.Quotations.Single(item => item.Id == editor.PreviewId);
            Assert.IsFalse(draft.CanPrint);
            Assert.IsFalse(draft.HasPendingDiscountApproval);
            StringAssert.Contains(draft.ReceptionActivity, "جديد");
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            reception.Visibility = Visibility.Visible;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "QuotationsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var receptionQuotes = WpfTestHost.FindByName<QuotationsView>(reception, "ReceptionQuotations");
            receptionQuotes.ViewModel.OpenQuotation(draft);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByAutomationName<Button>(receptionQuotes, "فتح تفاصيل عميل عرض السعر").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var customerDetail = WpfTestHost.FindByName<Features.Customers.CustomerDetailView>(reception, "CustomerDetail");
            Assert.AreEqual(editor.CustomerName, ((Features.Customers.CustomerPreview)customerDetail.DataContext).Name);
            WpfTestHost.FindByName<Button>(customerDetail, "BackButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            reception.Visibility = Visibility.Collapsed;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Visible;
            WpfTestHost.CompleteLayout(window);
            editor.RequestDiscountApproval();
            var request = reception.Handoffs.Quotations.Single(item => item.Id == editor.PreviewId);
            Assert.IsTrue(request.HasPendingDiscountApproval);
            WpfTestHost.Descendants<Button>(window).Single(button => (string?)button.Tag == "الموافقات").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(manager.IsVisible);
            Assert.IsTrue(manager.ViewModel.ApprovalsOnly);
            Assert.IsTrue(manager.ViewModel.VisibleQuotations.Contains(request));
            WpfTestHost.Capture(window, $"handoff-inbox-{width}");
            manager.ViewModel.OpenQuotation(request);
            WpfTestHost.CompleteLayout(window);
            var approve = WpfTestHost.FindByAutomationName<Button>(manager, "الموافقة على خصم عرض السعر");
            Assert.IsTrue(approve.Focus());
            Assert.IsTrue(approve.IsKeyboardFocused);
            WpfTestHost.Capture(window, $"handoff-approval-{width}");
            if (width == 780)
            {
                window.SetValue(Controls.ControlOptions.HighContrastProperty, true);
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "handoff-approval-system-colors");
                window.ClearValue(Controls.ControlOptions.HighContrastProperty);
            }
            approve.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(editor.IsDiscountApproved);
            Assert.IsTrue(editor.CanSaveQuotation);
            Assert.IsFalse(manager.ViewModel.VisibleQuotations.Contains(request));
            Assert.IsTrue(WpfTestHost.FindByName<Button>(manager, "BackToQuotationsButton").IsKeyboardFocused);
            Assert.IsTrue(editor.ReviewSave());
            var accepted = reception.Handoffs.Quotations.Single(item => item.Id == editor.PreviewId);
            Assert.IsTrue(accepted.CanPrint);
            var reopened = QuotationPreviewProjection.Create(accepted, new Features.Furniture.FurnitureViewModel(), new Features.Products.ProductsViewModel());
            Assert.IsTrue(reopened.CanSaveQuotation);
            Assert.IsFalse(accepted.Items.Single().IsFurniture);
            Assert.IsNotNull(reopened.QuotationLines.Single().Product);
            Assert.IsNull(reopened.QuotationLines.Single().Furniture);
            Assert.AreEqual(editor.Address, reopened.Address);
            Assert.AreEqual(editor.Notes, reopened.Notes);
            var detail = new CurrentQuotationView { DataContext = editor };
            window.Content = detail;
            WpfTestHost.CompleteLayout(window);
            var save = WpfTestHost.FindByAutomationName<Button>(detail, "حفظ عرض السعر");
            save.BringIntoView(); WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"handoff-approved-editor-{width}");
            editor.DiscountInput = "11";
            Assert.IsFalse(editor.CanSaveQuotation);
            editor.RequestDiscountApproval();
            var revised = reception.Handoffs.Quotations.Single(item => item.Id == editor.PreviewId);
            revised.DecideDiscount(DiscountApprovalDecision.Rejected);
            Assert.IsTrue(editor.IsDiscountRejected);
            Assert.IsFalse(editor.CanSaveQuotation);
            Assert.IsFalse(revised.CanPrint);
            Assert.IsTrue(revised.CanEdit);
            Assert.IsTrue(editor.ReviewDraftSave());
            Assert.IsTrue(editor.IsDiscountRejected);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"handoff-rejected-editor-{width}");
            editor.RequestDiscountApproval();
            var oldRequest = reception.Handoffs.Quotations.Single(item => item.Id == editor.PreviewId);
            editor.DuplicateLine(editor.QuotationLines[0]);
            oldRequest.DecideDiscount(DiscountApprovalDecision.Approved);
            Assert.IsFalse(editor.CanSaveQuotation, "A decision for the previous item snapshot must not approve edited items.");
            editor.DiscountInput = "4";
            Assert.IsTrue(editor.ReviewSave());
            Assert.IsFalse(editor.IsDiscountPending);
            Assert.AreEqual(1, reception.Handoffs.Quotations.Count(item => item.Id == editor.PreviewId));
            StringAssert.Contains(reception.Handoffs.Quotations.Single(item => item.Id == editor.PreviewId).ReceptionActivity, "عُدّل");
        });
    }

    [TestMethod]
    public void OrderProductionLinksAndReadyNoticeUseTheSamePreviewOrder()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var orders = WpfTestHost.FindByName<OrdersView>(window, "OrdersSurface");
            var work = WpfTestHost.FindByName<WorkOrdersView>(window, "WorkOrdersSurface");
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            var order = orders.ViewModel.PreviewOrders.Single(item => item.IsInProduction);
            WpfTestHost.FindByName<Button>(window, "OrdersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            orders.ViewModel.OpenOrder(order);
            WpfTestHost.CompleteLayout(window);
            Assert.IsNotNull(order.OriginalQuotation);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(orders, "OriginalQuotationButton").IsEnabled);
            WpfTestHost.Capture(window, "handoff-order-production");
            WpfTestHost.FindByAutomationName<Button>(orders, "فتح تصنيع الطلب").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(work.IsVisible);
            var production = work.ViewModel.SelectedWorkOrder!;
            Assert.AreEqual(order.Number, production.OrderNumber);
            Assert.AreEqual(order.Customer, production.Customer);
            Assert.AreEqual(order.Items.Count(item => item.IsFurniture), production.Furniture.Count);
            Assert.IsFalse(production.Furniture.Any(item => item.Name == "مرتبة الراحة"));
            Assert.IsTrue(WpfTestHost.FindByName<Button>(work, "BackToWorkOrdersButton").IsKeyboardFocused);
            WpfTestHost.Capture(window, "handoff-work-order-link");
            var back = WpfTestHost.FindByAutomationName<Button>(work, "فتح الطلب المرتبط");
            Assert.IsTrue(back.Focus());
            back.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(orders.IsVisible);
            Assert.AreEqual(order.Id, orders.ViewModel.SelectedOrder!.Id);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(orders, "BackToOrdersButton").IsKeyboardFocused);
            WpfTestHost.FindByAutomationName<Button>(orders, "فتح تصنيع الطلب").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(work, "AdvanceStatusButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(production.IsCompleted);
            Assert.IsFalse(work.ViewModel.AdvanceSelectedStatus());
            var notice = reception.PreviewOrders.ViewModel.NewReadyOrders.Single();
            Assert.AreEqual(order.Id, notice.Id);
            Assert.IsTrue(notice.IsReady);
            Assert.AreEqual(production.Number, notice.ReadyFromWorkOrder);
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            reception.Visibility = Visibility.Visible;
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(reception.IsVisible);
            WpfTestHost.Capture(window, "handoff-ready-home");
            var ready = WpfTestHost.FindByAutomationName<Button>(reception, $"مراجعة الطلب الجاهز {order.Number}");
            Assert.IsTrue(ready.Focus());
            ready.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(order.Id, reception.PreviewOrders.ViewModel.SelectedOrder!.Id);
            WpfTestHost.Capture(window, "handoff-newly-ready-order");
            WpfTestHost.FindByAutomationName<Button>(reception.PreviewOrders, "تمت مراجعة تنبيه جاهزية الطلب").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.AreEqual(0, reception.PreviewOrders.ViewModel.NewReadyOrders.Count);
            Assert.IsTrue(reception.PreviewOrders.ViewModel.SelectedOrder.IsReady);
            Assert.IsFalse(reception.PreviewOrders.ViewModel.SelectedOrder.IsNewlyReady);
        });
    }
}
