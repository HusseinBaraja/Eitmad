using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class ReceptionQuotationsRenderedTests
{
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
            Capture(window, "list");
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
                if (row.IsConverted || row.IsWaitingApproval || row.IsActive) Capture(window, row.Status.ToString());
            }
        });
    }

    private static void Capture(FrameworkElement element, string name)
    {
        var directory = Environment.GetEnvironmentVariable("EITMAD_QUOTATION_CAPTURE_DIR");
        if (string.IsNullOrEmpty(directory)) return;
        Directory.CreateDirectory(directory);
        var bitmap = new RenderTargetBitmap((int)element.ActualWidth, (int)element.ActualHeight, 96, 96, PixelFormats.Pbgra32);
        bitmap.Render(element);
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        using var stream = File.Create(Path.Combine(directory, name + ".png"));
        encoder.Save(stream);
    }
}
