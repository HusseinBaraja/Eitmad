using System.Windows.Media;
using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.CatalogImages;

/// <summary>Retains at most 128 decoded thumbnails within one authorized view session.</summary>
internal sealed class CatalogImageThumbnails
{
    private readonly Dictionary<(Guid Id, string Sha256), ImageSource> cache = [];
    private readonly SemaphoreSlim workers = new(2);
    private long generation;

    /// <summary>Invalidates cached and in-flight images when session authority changes.</summary>
    public void Clear() { ++generation; cache.Clear(); }

    /// <summary>Applies cached images first and loads distinct misses with two worker slots.</summary>
    public async Task ApplyAsync(
        CatalogImageClient images,
        IEnumerable<(Guid RecordId, CatalogImageRef Reference)> records,
        Action<Guid, ImageSource?> apply,
        CancellationToken cancellation)
    {
        var version = generation;
        var pending = new List<IGrouping<(Guid Id, string Sha256), (Guid RecordId, CatalogImageRef Reference)>>();
        foreach (var group in records.GroupBy(record => (record.Reference.Id, record.Reference.Sha256)))
        {
            if (cache.TryGetValue(group.Key, out var cached))
                foreach (var record in group) apply(record.RecordId, cached);
            else pending.Add(group);
        }
        await Task.WhenAll(pending.Select(async group =>
        {
            await workers.WaitAsync(cancellation);
            try
            {
                cancellation.ThrowIfCancellationRequested();
                if (version != generation) return;
                var thumbnail = await images.LoadAsync(group.First().Reference, 96, cancellation);
                cancellation.ThrowIfCancellationRequested();
                if (version != generation) return;
                if (thumbnail is not null)
                {
                    if (cache.Count == 128) cache.Remove(cache.Keys.First());
                    cache[group.Key] = thumbnail;
                }
                foreach (var record in group) apply(record.RecordId, thumbnail);
            }
            finally { workers.Release(); }
        }));
    }
}
