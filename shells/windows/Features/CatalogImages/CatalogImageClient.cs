using System.IO;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.CatalogImages;

/// <summary>Native presentation adapter. Rust imports, validates, authorizes, and stores each image.</summary>
public sealed class CatalogImageClient(IEngineShellBridge engine)
{
    /// <summary>Submits the native picker path to Rust and returns only a confirmed immutable reference.</summary>
    public async Task<CatalogImageRef?> ImportAsync(CatalogImageKind kind, string path, CancellationToken cancellation = default)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityCatalogImageV1)) return null;
        var response = await engine.SubmitCommandAsync(Command.ForCatalogImageImport(new ImportCatalogImage { Kind = kind, SourcePath = path }), Guid.NewGuid(), cancellation);
        return response.Outcome.Status == CommandOutcomeStatus.Succeeded ? response.Outcome.Payload.AsCatalogImageImported() : null;
    }

    /// <summary>Assembles bounded Rust chunks and decodes a frozen presentation image off the UI thread.</summary>
    public async Task<ImageSource?> LoadAsync(CatalogImageRef? reference, int decodeSize, CancellationToken cancellation = default)
    {
        if (reference is null || decodeSize <= 0 || !engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityCatalogImageV1)) return null;
        try
        {
            using var content = new MemoryStream();
            long? total = null;
            do
            {
                var response = await engine.QueryAsync(Query.ForCatalogImageGet(new GetCatalogImage { Reference = reference, Offset = checked((uint)content.Length) }), cancellation);
                if (response.Outcome.Status != CommandOutcomeStatus.Succeeded) return null;
                var chunk = response.Outcome.Payload.AsCatalogImage();
                // Presentation allocation guard; image validity is enforced by Rust.
                if (chunk is null || chunk.TotalBytes is <= 0 or > 8 * 1024 * 1024 || chunk.Offset != content.Length || (total is not null && total != chunk.TotalBytes)) return null;
                total = chunk.TotalBytes;
                var bytes = Convert.FromBase64String(chunk.Base64);
                if (bytes.Length is <= 0 or > 64 * 1024 || content.Length + bytes.Length > total) return null;
                content.Write(bytes);
            } while (content.Length < total);
            var data = content.ToArray();
            return await Task.Run<ImageSource>(() =>
            {
                using var source = new MemoryStream(data);
                var frame = BitmapDecoder.Create(source, BitmapCreateOptions.DelayCreation, BitmapCacheOption.None).Frames[0];
                var scale = Math.Min(1d, decodeSize / (double)Math.Max(frame.PixelWidth, frame.PixelHeight));
                source.Position = 0;
                var bitmap = new BitmapImage(); bitmap.BeginInit();
                bitmap.CacheOption = BitmapCacheOption.OnLoad;
                // Both dimensions must be explicit: a width alone can enlarge a narrow portrait.
                bitmap.DecodePixelWidth = Math.Max(1, (int)Math.Floor(frame.PixelWidth * scale));
                bitmap.DecodePixelHeight = Math.Max(1, (int)Math.Floor(frame.PixelHeight * scale));
                bitmap.StreamSource = source; bitmap.EndInit(); bitmap.Freeze();
                return bitmap;
            }, cancellation);
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { throw; }
        catch (Exception error) when (error is EngineIpcException or IOException or InvalidOperationException or ObjectDisposedException or FormatException or NotSupportedException) { return null; }
    }
}
