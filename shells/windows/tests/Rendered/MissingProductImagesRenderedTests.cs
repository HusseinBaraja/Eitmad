using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.WorkOrders;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.WorkOrders;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class MissingProductImagesRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void MattressUsesNeutralFallbackAcrossCategoryChangesAndImageUpdates(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            PrepareWindow(window, width);
            var view = new ProductsView();
            window.Content = view;
            var data = ProductsPresentationTests.Data();
            data.Categories.Items[0].Name = "مستلزمات النوم الجديدة";
            data.Products[0].CategoryName = data.Categories.Items[0].Name;
            view.ViewModel.ApplyDurableData(data);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"missing-mattress-{width}");
            var row = view.ViewModel.VisibleProducts.Single();
            var placeholder = WpfTestHost.Descendants<TextBlock>(view).Single(text =>
                ReferenceEquals(text.DataContext, row) && text.Text == "الصورة غير متاحة");
            Assert.IsTrue(placeholder.IsVisible, "An unfamiliar category must show an image-missing state.");

            foreach (var category in new[] { "المراتب", "الوسائد", "الإضاءة", "فئة أعيدت تسميتها" })
            {
                data.Categories.Items[0].Name = category;
                data.Products[0].CategoryName = category;
                view.ViewModel.ApplyDurableData(data);
                WpfTestHost.CompleteLayout(window);
                row = view.ViewModel.VisibleProducts.Single();
                placeholder = WpfTestHost.Descendants<TextBlock>(view).Single(text =>
                    ReferenceEquals(text.DataContext, row) && text.Text == "الصورة غير متاحة");
                Assert.IsTrue(placeholder.IsVisible, "Editable category text must never choose a product drawing.");
            }

            // A synthetic decoded image proves binding priority.
            var image = BitmapSource.Create(2, 2, 96, 96, PixelFormats.Rgb24, null,
                new byte[] { 180, 90, 45, 180, 90, 45, 180, 90, 45, 180, 90, 45 }, 6);
            image.Freeze();
            view.ViewModel.ApplyImage(row.Id, image);
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(placeholder.IsVisible);
            var renderedImage = WpfTestHost.Descendants<Image>(view).Single(control => ReferenceEquals(control.DataContext, row));
            Assert.AreSame(image, renderedImage.Source);
            Assert.IsTrue(renderedImage.IsVisible);
            Assert.AreEqual(row.Name, AutomationProperties.GetName(renderedImage));
            WpfTestHost.Capture(window, $"loaded-mattress-{width}");

            view.ViewModel.ApplyImage(row.Id, null);
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(placeholder.IsVisible, "A missing or failed image must restore the neutral state.");
            if (width == 1338)
            {
                var table = WpfTestHost.Descendants<OperationsTable>(view).Single();
                ControlOptions.SetHighContrast(table, true);
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "missing-mattress-system-colors");
                Assert.AreEqual(SystemColors.WindowTextBrush, placeholder.Foreground);
            }
            Console.WriteLine("Product proof: 5 category names, loaded image priority, null-image recovery.");
        });
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void LiveBedAndTableKeepSpecificationsWithoutInventedPictures(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            PrepareWindow(window, width);
            var view = new WorkOrdersView();
            window.Content = view;
            var fixture = new WorkOrderAuthorityTests.Fixture();
            var pictures = new List<FrameworkElement>();
            foreach (var (name, key) in new[] { ("سرير مزدوج تجريبي", "bed"), ("طاولة طعام تجريبية", "table") })
            {
                fixture.Work.Furniture[0].Name = name;
                var work = WorkOrdersViewModel.Project(fixture.Work);
                view.ViewModel.OpenWorkOrder(work);
                WpfTestHost.CompleteLayout(window);
                var picture = WpfTestHost.FindByAutomationName<Border>(view, $"صورة {name}");
                picture.BringIntoView();
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, $"missing-work-{key}-{width}");
                pictures.Add(picture);
                Assert.AreEqual("180 × 220 × 60 سم", work.Furniture.Single().Dimensions);
                Assert.AreEqual("جوزي", work.Furniture.Single().Color);
                Assert.AreEqual("نحاسي", work.Furniture.Single().Handle);
                Assert.AreEqual("2", work.Furniture.Single().QuantityLabel);
            }
            foreach (var picture in pictures)
            {
                Assert.IsFalse(WpfTestHost.Descendants<Canvas>(picture).Any(), "The live projection has no item image; it must not invent furniture.");
                Assert.AreEqual("الصورة غير متاحة", WpfTestHost.Descendants<TextBlock>(picture).Single().Text);
            }
            Console.WriteLine("Work-order proof: bed and table have neutral image states; dimensions, color, handle and quantity preserved.");
        });
    }

    private static void PrepareWindow(Window window, int width)
    {
        var dpi = VisualTreeHelper.GetDpi(window);
        Console.WriteLine($"Display: {SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX}x{SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY} px; scaling {dpi.DpiScaleX * 100}%.");
        if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
        WpfTestHost.CompleteLayout(window);
        Console.WriteLine($"Actual window: {window.ActualWidth}x{window.ActualHeight} DIP; requested width {width}.");
    }
}
