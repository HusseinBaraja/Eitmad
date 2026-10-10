using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Media;
using System.Windows.Threading;
using System.Collections.ObjectModel;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class QuotationFinalActionsRenderedTests
{
    [TestMethod]
    public void RequiredFieldsPreviewBackAndCustomerOnlyPagination()
    {
        WpfTestHost.Run(1200, 950, window =>
        {
            var model = new SalesCatalogViewModel(Eitmad.WindowsShell.Tests.Furniture.FurnitureFixtures.SalesModel(), new ProductsViewModel());
            var view = new CurrentQuotationView { DataContext = model };
            window.Content = view;
            WpfTestHost.CompleteLayout(window);
            var issue = WpfTestHost.FindByAutomationName<Button>(view, "إصدار عرض السعر");
            Assert.IsFalse(issue.IsEnabled);
            var draft = WpfTestHost.FindByAutomationName<Button>(view, "حفظ كمسودة");
            draft.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsTrue(model.ItemsError.Length > 0);
            Assert.IsTrue(model.CustomerNameError.Length > 0);
            Assert.IsTrue(model.PhoneError.Length > 0);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "ContinueButton").IsKeyboardFocusWithin);
            Products.SalesCatalogPresentationTests.AddHistoricalProductLine(model);model.CloseSelection();
            draft.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "CustomerNameInput").IsKeyboardFocusWithin);
            draft.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "CustomerNameInput").IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, "quotation-required-fields");
            model.CustomerName = "عميل تجريبي";
            draft.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "PhoneInput").IsKeyboardFocusWithin);
            model.Phone = "000000000"; model.Address = "عنوان تجريبي"; model.Notes = "INTERNAL_ONLY_SENTINEL";
            Assert.AreEqual("", model.CustomerNameError); Assert.AreEqual("", model.PhoneError);
            model.Select(model.VisibleItems.First(item => item.Name == "خزانة السكينة"));
            model.Selection!.SelectedSize = model.Selection.Sizes[0];
            model.Selection.SelectedColor = model.Selection.Colors[0];
            model.Selection.SelectedHandle = model.Selection.Handles[0];
            model.AddSelection(); model.CloseSelection();
            model.DiscountInput = "5";
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Descendants<ScrollViewer>(view).First().ScrollToBottom();
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "quotation-final-actions");
            Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "PrintPreviewButton").IsEnabled);
            var row = new QuotationListItem(Guid.NewGuid(), "QT-PREVIEW-00001", model.CustomerName,
                new DateOnly(2026, 9, 15), QuotationStatus.Active, model.Discount,
                model.QuotationLines.Select(line => new QuotationLineItem(line.Name, line.Variant,
                    line.Color ?? "—", line.Handle ?? "—", line.Quantity, line.UnitPrice)
                    { IsFurniture = line.IsFurniture, Dimensions = line.Dimensions }).ToArray(), phone: model.Phone);
            var reviewed = new QuotationsView();
            reviewed.ConfigureReceptionist(_ => model);
            reviewed.ViewModel.UsePreviewQuotations(new ObservableCollection<QuotationListItem> { row });
            reviewed.ViewModel.OpenQuotation(row);
            window.Content = reviewed;
            WpfTestHost.CompleteLayout(window);
            Exception? failure = null;
            window.Dispatcher.BeginInvoke(DispatcherPriority.ApplicationIdle, new Action(() =>
            {
                var modal = window.OwnedWindows.Cast<Window>().Single();
                try
                {
                    WpfTestHost.CompleteLayout(modal);
                    var preview = (PrintPreview)modal.Content;
                    Assert.IsFalse(preview.CanPrint);
                    Assert.IsTrue(preview.BackButton.IsKeyboardFocusWithin);
                    var text = new TextRange(preview.Document.ContentStart, preview.Document.ContentEnd).Text;
                    Assert.IsFalse(text.Contains(model.Notes));
                    Assert.IsTrue(text.Contains(model.FinalTotal.ToString("N0", System.Globalization.CultureInfo.InvariantCulture) + " ر.ي"));
                    Assert.IsTrue(text.Contains(model.QuotationLines[1].Options));
                    WpfTestHost.Capture(modal, "quotation-customer-preview");
                    var back = WpfTestHost.FindByAutomationName<Button>(preview, "رجوع");
                    back.Focus(); Assert.IsTrue(back.IsKeyboardFocusWithin);
                    back.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                }
                catch (Exception error) { failure = error; }
                finally { modal.Close(); }
            }));
            var print = WpfTestHost.FindByAutomationName<Button>(reviewed, "طباعة عرض السعر");
            print.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            if (failure is not null) throw failure;
            Assert.IsTrue(WpfTestHost.FindByName<Button>(reviewed, "BackToQuotationsButton").IsKeyboardFocusWithin);
            Assert.HasCount(2, model.QuotationLines);
            for (var i = 0; i < 45; i++) model.DuplicateLine(model.QuotationLines[0]);
            var document = QuotationCustomerDocument.CreateExistingPreview(model, new DateTime(2026, 9, 15));
            var paginator = ((IDocumentPaginatorSource)document).DocumentPaginator;
            paginator.ComputePageCount();
            Assert.IsTrue(paginator.PageCount > 1);
            for (var page = 0; page < paginator.PageCount; page++) Assert.AreNotSame(DocumentPage.Missing, paginator.GetPage(page));
            model.DiscountInput = "10";
            Assert.IsFalse(model.CanPreviewCustomer);
            Assert.ThrowsExactly<InvalidOperationException>(() => QuotationCustomerDocument.Create(model, DateTime.Today));
        });
    }
}
