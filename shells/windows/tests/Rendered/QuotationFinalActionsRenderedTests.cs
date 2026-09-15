using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class QuotationFinalActionsRenderedTests
{
    [TestMethod]
    public void RequiredFieldsPreviewBackAndCustomerOnlyPagination()
    {
        WpfTestHost.Run(1200, 950, window =>
        {
            var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel());
            var view = new CurrentQuotationView { DataContext = model };
            window.Content = view;
            WpfTestHost.CompleteLayout(window);
            var save = WpfTestHost.FindByAutomationName<Button>(view, "حفظ عرض السعر");
            save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsTrue(model.ItemsError.Length > 0);
            Assert.IsTrue(model.CustomerNameError.Length > 0);
            Assert.IsTrue(model.PhoneError.Length > 0);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "ContinueButton").IsKeyboardFocusWithin);
            model.Select(model.VisibleItems.Single(item => item.Name == "وسادة فندقية"));
            model.AddProductSelection(); model.CloseSelection();
            save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "CustomerNameInput").IsKeyboardFocusWithin);
            Capture(window, "required-fields");
            model.CustomerName = "عميل تجريبي";
            save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
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
            Capture(window, "final-actions");
            Exception? failure = null;
            window.Dispatcher.BeginInvoke(DispatcherPriority.ApplicationIdle, new Action(() =>
            {
                var modal = window.OwnedWindows.Cast<Window>().Single();
                try
                {
                    WpfTestHost.CompleteLayout(modal);
                    var preview = (PrintPreview)modal.Content;
                    Assert.IsTrue(WpfTestHost.FindByName<Button>(preview, "PrintButton").IsKeyboardFocusWithin);
                    var text = new TextRange(preview.Document.ContentStart, preview.Document.ContentEnd).Text;
                    Assert.IsFalse(text.Contains(model.Notes));
                    Assert.IsTrue(text.Contains(model.FinalTotal.ToString("N0", System.Globalization.CultureInfo.InvariantCulture) + " YER"));
                    Assert.IsTrue(text.Contains(model.QuotationLines[1].Options));
                    Capture(modal, "customer-preview");
                    var back = WpfTestHost.FindByAutomationName<Button>(preview, "رجوع");
                    back.Focus(); Assert.IsTrue(back.IsKeyboardFocusWithin);
                    back.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                }
                catch (Exception error) { failure = error; }
                finally { modal.Close(); }
            }));
            var print = WpfTestHost.FindByName<Button>(view, "PrintPreviewButton");
            print.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            if (failure is not null) throw failure;
            Assert.IsTrue(print.IsKeyboardFocusWithin);
            Assert.HasCount(2, model.QuotationLines);
            for (var i = 0; i < 45; i++) model.DuplicateLine(model.QuotationLines[0]);
            var document = QuotationCustomerDocument.Create(model, new DateTime(2026, 9, 15));
            var paginator = ((IDocumentPaginatorSource)document).DocumentPaginator;
            paginator.ComputePageCount();
            Assert.IsTrue(paginator.PageCount > 1);
            for (var page = 0; page < paginator.PageCount; page++) Assert.AreNotSame(DocumentPage.Missing, paginator.GetPage(page));
            model.DiscountInput = "10";
            Assert.IsFalse(model.CanPreviewCustomer);
            Assert.ThrowsExactly<InvalidOperationException>(() => QuotationCustomerDocument.Create(model, DateTime.Today));
        });
    }
    private static void Capture(FrameworkElement element, string name)
    {
        var directory = Environment.GetEnvironmentVariable("EITMAD_CATALOG_CAPTURE_DIR");
        if (string.IsNullOrEmpty(directory)) return;
        Directory.CreateDirectory(directory);
        var bitmap = new RenderTargetBitmap((int)element.ActualWidth, (int)element.ActualHeight, 96, 96, PixelFormats.Pbgra32);
        bitmap.Render(element);
        var encoder = new PngBitmapEncoder(); encoder.Frames.Add(BitmapFrame.Create(bitmap));
        using var stream = File.Create(Path.Combine(directory, name + ".png")); encoder.Save(stream);
    }
}
