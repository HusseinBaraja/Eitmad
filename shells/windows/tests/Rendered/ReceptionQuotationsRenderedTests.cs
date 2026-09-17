using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class ReceptionQuotationsRenderedTests
{
    [TestMethod]
    public void NewQuotationWindowStartsInCatalogAndCanReturnToReview()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var view = new QuotationsView();
            view.ConfigureReceptionist(quotation => QuotationPreviewProjection.Create(quotation,
                new Features.Furniture.FurnitureViewModel(), new Features.Products.ProductsViewModel()));
            window.Content = view;
            WpfTestHost.CompleteLayout(window);
            Exception? failure = null;
            var inspected = false;
            window.Dispatcher.BeginInvoke(System.Windows.Threading.DispatcherPriority.ApplicationIdle, new Action(() =>
            {
                var child = window.OwnedWindows.Cast<Window>().Single();
                try
                {
                    var catalog = (SalesCatalogView)child.Content;
                    var model = (SalesCatalogViewModel)catalog.DataContext;
                    WpfTestHost.CompleteLayout(child);
                    Assert.IsFalse(model.IsReviewingQuotation);
                    Assert.IsTrue(WpfTestHost.FindByName<TextBox>(catalog, "CatalogSearch").IsKeyboardFocusWithin);
                    WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار وسادة فندقية").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    WpfTestHost.CompleteLayout(child);
                    var detail = WpfTestHost.FindByName<ProductSelectionView>(catalog, "ProductSelectionView");
                    WpfTestHost.FindByName<Button>(detail, "AddButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    WpfTestHost.CompleteLayout(child);
                    Assert.IsFalse(model.IsReviewingQuotation);
                    WpfTestHost.Capture(child, "quotation-new-catalog-selection");
                    var header = WpfTestHost.Descendants<Controls.PageHeader>(catalog).Single();
                    WpfTestHost.Descendants<Button>(header).Single(button => button.IsVisible).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    WpfTestHost.CompleteLayout(child);
                    Assert.IsTrue(model.IsReviewingQuotation);
                    var continueButton = WpfTestHost.FindByAutomationName<Button>(catalog, "متابعة اختيار المنتجات");
                    Assert.IsTrue(continueButton.IsKeyboardFocusWithin);
                    continueButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    WpfTestHost.CompleteLayout(child);
                    Assert.IsTrue(WpfTestHost.FindByName<Button>(detail, "BackButton").IsKeyboardFocusWithin);
                    inspected = true;
                }
                catch (Exception exception) { failure = exception; }
                finally { child.Close(); }
            }));
            WpfTestHost.FindByAutomationName<Button>(view, "+ عرض سعر جديد").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            if (failure is not null) throw failure;
            Assert.IsTrue(inspected);
        });
    }

    [TestMethod]
    public void ConversionConfirmsSnapshotAndCancelPreservesQuotation()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var view = new QuotationsView();
            view.ConfigureReceptionist(_ => throw new InvalidOperationException("Conversion must not reopen the editor."));
            window.Content = view;
            var quotation = view.ViewModel.VisibleQuotations.Single(row => row.IsActive);
            view.ViewModel.OpenQuotation(quotation);
            WpfTestHost.CompleteLayout(window);
            var trigger = WpfTestHost.FindByAutomationName<Button>(view, "تحويل عرض السعر إلى طلب");
            trigger.Focus();
            trigger.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var dialog = WpfTestHost.FindByName<Eitmad.WindowsShell.Controls.DialogHost>(view, "ConversionDialog");
            var cancel = WpfTestHost.FindByName<Button>(view, "CancelConversionButton");
            Assert.IsTrue(dialog.IsOpen);
            Assert.IsTrue(cancel.IsKeyboardFocused);
            Assert.AreSame(quotation, dialog.DataContext);
            WpfTestHost.Capture(window, "quotation-conversion-confirmation");
            cancel.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(dialog.IsOpen);
            Assert.IsTrue(trigger.IsKeyboardFocused);
            trigger.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            dialog.RequestClose();
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(dialog.IsOpen);
            trigger.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Exception? failure = null;
            var inspected = false;
            window.Dispatcher.BeginInvoke(System.Windows.Threading.DispatcherPriority.ApplicationIdle, new Action(() =>
            {
                var child = window.OwnedWindows.Cast<Window>().Single();
                try
                {
                    var orders = (Eitmad.WindowsShell.Features.Orders.OrdersView)child.Content;
                    var order = orders.ViewModel.SelectedOrder!;
                    Assert.AreEqual(quotation.Customer, order.Customer);
                    Assert.AreEqual(quotation.Discount, order.Discount);
                    Assert.AreEqual(quotation.FinalTotal, order.FinalTotal);
                    CollectionAssert.AreEqual(quotation.Items.Select(line => (line.FurnitureName, line.Variant, line.Color, line.Handle, line.Quantity, line.UnitPrice)).ToArray(),
                        order.Items.Select(line => (line.Product, line.Variant, line.Color, line.Handle, line.Quantity, line.SellingPrice)).ToArray());
                    Assert.IsTrue(WpfTestHost.FindByName<Button>(orders, "BackToOrdersButton").IsKeyboardFocused);
                    inspected = true;
                }
                catch (Exception exception) { failure = exception; }
                finally { child.Close(); }
            }));
            WpfTestHost.FindByName<Button>(view, "ConfirmConversionButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            if (failure is not null) throw failure;
            Assert.IsTrue(inspected);
            Assert.AreEqual(QuotationStatus.Active, quotation.Status);
        });
    }

    [TestMethod]
    public void ReceptionListFiltersAndDetailActionsRespectStatus()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "QuotationsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.FindByName<QuotationsView>(reception, "ReceptionQuotations");
            Assert.IsTrue(view.IsVisible);
            Assert.IsTrue(view.ViewModel.IsReceptionist);
            WpfTestHost.Capture(window, "quotation-list");
            var rows = view.ViewModel.VisibleQuotations.ToArray();
            view.ViewModel.SearchText = "٠٠٠٠٠٠٠٤٣";
            Assert.AreEqual(QuotationStatus.WaitingApproval, view.ViewModel.VisibleQuotations.Single().Status);
            view.ViewModel.SearchText = "";
            foreach (var row in rows)
            {
                view.ViewModel.OpenQuotation(row);
                WpfTestHost.CompleteLayout(view);
                Assert.AreEqual(row.CanEdit, WpfTestHost.FindByAutomationName<Button>(view, "تعديل عرض السعر").IsVisible);
                Assert.AreEqual(row.CanPrint, WpfTestHost.FindByAutomationName<Button>(view, "طباعة عرض السعر").IsVisible);
                Assert.AreEqual(row.IsConverted, WpfTestHost.FindByAutomationName<Button>(view, "فتح الطلب").IsVisible);
                Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Border>(view, "ApprovalSection").Visibility);
                if (row.IsConverted || row.IsWaitingApproval || row.IsActive) WpfTestHost.Capture(window, "quotation-" + row.Status);
            }
        });
    }
}
