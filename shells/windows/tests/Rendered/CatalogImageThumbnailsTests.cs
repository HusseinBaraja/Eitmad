using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.CatalogImages;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class CatalogImageThumbnailsTests
{
    [TestMethod]
    public void RefreshReusesReferenceCacheAndSessionClearRequiresAuthorizedReads()
    {
        WpfTestHost.Run(720, 560, _ =>
        {
            var reads = 0;
            var engine = ImageEngine(() => ++reads);
            var client = new CatalogImageClient(engine);
            var thumbnails = new CatalogImageThumbnails();
            var reference = Reference();
            (Guid, CatalogImageRef)[] rows = [(Guid.NewGuid(), reference), (Guid.NewGuid(), reference)];
            var applied = new Dictionary<Guid, ImageSource?>();
            Complete(thumbnails.ApplyAsync(client, rows, (id, image) => applied[id] = image, default));
            Assert.AreEqual(1, reads);
            Assert.AreEqual(2, applied.Count);
            Assert.IsNotNull(applied[rows[0].Item1]);
            applied.Clear();
            var refresh = thumbnails.ApplyAsync(client, rows, (id, image) => applied[id] = image, default);
            Assert.AreEqual(2, applied.Count, "Cached images must apply before any asynchronous load.");
            Complete(refresh);
            Assert.AreEqual(1, reads);
            reference = new CatalogImageRef { Id = reference.Id, Kind = reference.Kind, Sha256 = new string('1', 64) };
            Complete(thumbnails.ApplyAsync(client, [(rows[0].Item1, reference)], (id, image) => applied[id] = image, default));
            Assert.AreEqual(2, reads, "A changed digest must miss the cache.");
            thumbnails.Clear();
            Complete(thumbnails.ApplyAsync(client, rows, (id, image) => applied[id] = image, default));
            Assert.AreEqual(3, reads);
        });
    }

    [TestMethod]
    public void OldSessionLoadsCannotRepopulateCacheAndWorkersStayBounded()
    {
        WpfTestHost.Run(720, 560, _ =>
        {
            var reads = 0;
            var started = 0;
            var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            var engine = ImageEngine(() => ++reads);
            engine.QueryBarrier = _ => { ++started; return release.Task; };
            var client = new CatalogImageClient(engine);
            var thumbnails = new CatalogImageThumbnails();
            (Guid, CatalogImageRef)[] rows = Enumerable.Range(0, 3).Select(_ => (Guid.NewGuid(), Reference())).ToArray();
            var applied = 0;
            var load = thumbnails.ApplyAsync(client, rows, (_, _) => ++applied, default);
            Assert.AreEqual(2, started);
            thumbnails.Clear();
            release.SetResult(); Complete(load);
            Assert.AreEqual(0, applied);
            Assert.AreEqual(2, started, "Old-session queued work must not start after invalidation.");
            engine.QueryBarrier = null;
            Complete(thumbnails.ApplyAsync(client, [rows[0]], (_, _) => ++applied, default));
            Assert.AreEqual(3, reads, "The late completion must not have populated the new session cache.");
            Assert.AreEqual(1, applied);
        });
    }

    [TestMethod]
    public void BoundRowHashSetsSurviveThumbnailAndBindingChanges()
    {
        WpfTestHost.Run(720, 560, _ =>
        {
            var product = new ProductListItem(Guid.NewGuid(), "مرتبة", "مراتب", 100, "مفرد", "", false, true);
            var furniture = new FurnitureListItem(Guid.NewGuid(), "كرسي", "كراسي", 1, 100, "", false, false);
            var products = new HashSet<ProductListItem> { product };
            var furnishings = new HashSet<FurnitureListItem> { furniture };
            product.PropertyChanged += (_, _) => { };
            furniture.PropertyChanged += (_, _) => { };
            product.Image = BitmapSource.Create(1, 1, 96, 96, PixelFormats.Gray8, null, new byte[] { 1 }, 1);
            furniture.Image = product.Image;
            Assert.IsTrue(products.Remove(product));
            Assert.IsTrue(furnishings.Remove(furniture));
        });
    }

    private static CatalogImageRef Reference() => new() { Id = Guid.NewGuid(), Kind = CatalogImageKind.Product, Sha256 = new string('0', 64) };

    private static FakeEngine ImageEngine(Action read)
    {
        using var content = new System.IO.MemoryStream();
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(BitmapSource.Create(1, 1, 96, 96, PixelFormats.Gray8, null, new byte[] { 1 }, 1)));
        encoder.Save(content);
        var png = content.ToArray();
        return new FakeEngine { QueryHandler = query =>
        {
            read();
            return new QueryResponseEnvelope { RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(),
                Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = QueryResult.ForCatalogImage(
                    new CatalogImageChunk { Reference = query.AsCatalogImageGet()!.Reference, Offset = 0, TotalBytes = png.Length, Base64 = Convert.ToBase64String(png) }) } };
        } };
    }

    private static void Complete(Task task)
    {
        var deadline = DateTime.UtcNow.AddSeconds(5);
        while (!task.IsCompleted && DateTime.UtcNow < deadline) WpfTestHost.PumpDispatcher();
        Assert.IsTrue(task.IsCompleted);
        task.GetAwaiter().GetResult();
    }
}
