using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class CatalogImageRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ImportedImagesRenderAndRemovalStagesOnlyANewReference(int width, int height)
    {
        var data = ProductsPresentationTests.Data();
        var reference = new CatalogImageRef { Id = Guid.NewGuid(), Kind = CatalogImageKind.Product, Sha256 = new string('0', 64) };
        data.Products.Single().Image = reference;
        var furnitureFixtures = new Furniture.FurnitureFixtures();
        furnitureFixtures.Seed();
        var furnitureData = furnitureFixtures.Snapshot();
        var furnitureQueries = furnitureFixtures.Engine().QueryHandler!;
        var engine = new FakeEngine();
        var pixels = Enumerable.Range(0, 24 * 16 * 3).Select(index => (byte)(index % 3 == 0 ? 190 : 90)).ToArray();
        byte[] png = [];
        WpfTestHost.Run(width, height, window =>
        {
            var bitmap = BitmapSource.Create(24, 16, 96, 96, PixelFormats.Rgb24, null, pixels, 72);
            using var content = new System.IO.MemoryStream();
            var encoder = new PngBitmapEncoder(); encoder.Frames.Add(BitmapFrame.Create(bitmap)); encoder.Save(content); png = content.ToArray();
        });
        engine.QueryHandler = query => new QueryResponseEnvelope
        {
            RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(),
            Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = query.Kind switch
            {
                Query.ProductCategoryListKind => QueryResult.ForProductCategories(data.Categories),
                Query.CatalogImageGetKind => QueryResult.ForCatalogImage(new CatalogImageChunk { Reference = query.AsCatalogImageGet()!.Reference, Offset = 0, TotalBytes = png.Length, Base64 = Convert.ToBase64String(png) }),
                Query.ProductListKind => QueryResult.ForProducts(new ProductPage { Items = data.Products.ToArray(), CanManage = true, CanReadCosts = true }),
                Query.FurnitureCategoryListKind => QueryResult.ForFurnitureCategories(furnitureData.Categories),
                Query.FurnitureListKind => QueryResult.ForFurnitures(new FurniturePage { Items = furnitureData.Furniture.ToArray(), CanManage = true, CanReadCosts = true }),
                _ => furnitureQueries(query).Outcome.Payload,
            } },
        };
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "ProductsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var products = WpfTestHost.Descendants<ProductsView>(window).Single();
            var deadline = DateTime.UtcNow.AddSeconds(5);
            while (products.ViewModel.VisibleProducts.FirstOrDefault()?.Image is null && DateTime.UtcNow < deadline) WpfTestHost.PumpDispatcher();
            var row = products.ViewModel.VisibleProducts.Single(); Assert.IsNotNull(row.Image);
            WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window, $"catalog-product-image-list-{width}x{height}");
            products.ViewModel.BeginEdit(row); WpfTestHost.CompleteLayout(window);
            var picker = WpfTestHost.FindByAutomationName<Button>(products, "اختيار صورة المنتج"); picker.BringIntoView(); picker.Focus(); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(picker.IsKeyboardFocusWithin);
            Assert.IsTrue(WpfTestHost.Descendants<Image>(products).Any(image => image.IsVisible && image.Source is not null));
            WpfTestHost.Capture(window, $"catalog-product-image-editor-{width}x{height}");
            WpfTestHost.FindByAutomationName<Button>(products, "إزالة صورة المنتج").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsNull(products.ViewModel.SaveInput().Image); Assert.IsNotNull(data.Products.Single().Image);
            products.ViewModel.CancelEditor();

            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var furniture = WpfTestHost.Descendants<FurnitureView>(window).Single();
            deadline = DateTime.UtcNow.AddSeconds(5);
            while ((furniture.ViewModel.IsLoading || furniture.ViewModel.VisibleFurniture.Count == 0) && DateTime.UtcNow < deadline) WpfTestHost.PumpDispatcher();
            var furnitureRow = furniture.ViewModel.VisibleFurniture.First(); furnitureRow.Image = row.Image;
            WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window, $"catalog-furniture-image-list-{width}x{height}");
            var preparation = furniture.PrepareEditorAsync(furnitureRow);
            while (!preparation.IsCompleted && DateTime.UtcNow < deadline) WpfTestHost.PumpDispatcher();
            Assert.IsTrue(preparation.GetAwaiter().GetResult());
            Assert.IsTrue(furniture.ViewModel.BeginEdit(furnitureRow));
            furniture.ViewModel.SetImportedImage(new CatalogImageRef { Id = reference.Id, Kind = CatalogImageKind.Furniture, Sha256 = reference.Sha256 }, row.Image);
            WpfTestHost.CompleteLayout(window);
            var remove = WpfTestHost.FindByAutomationName<Button>(furniture, "إزالة صورة الأثاث"); remove.BringIntoView(); remove.Focus(); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(remove.IsKeyboardFocusWithin);
            Assert.IsTrue(WpfTestHost.Descendants<Image>(furniture).Any(image => image.IsVisible && image.Source is not null));
            WpfTestHost.Capture(window, $"catalog-furniture-image-editor-{width}x{height}");
            remove.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Assert.IsNull(furniture.ViewModel.EditorImageReference);
            Console.WriteLine($"Catalog images rendered {window.ActualWidth}x{window.ActualHeight} DIP, scaling {VisualTreeHelper.GetDpi(window).DpiScaleX * 100}%.");
        }, engine: engine);
    }
}
