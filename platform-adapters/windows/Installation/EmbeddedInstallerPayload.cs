using System.IO.Compression;
using System.Runtime.Versioning;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;

namespace Eitmad.Platform.Windows.Installation;

/// <summary>Extract the archive pinned by a signed installer; retain it for restart and recovery.</summary>
[SupportedOSPlatform("windows")]
public static class EmbeddedInstallerPayload
{
    private const long MaximumExpandedBytes = 4L * 1024 * 1024 * 1024;

    public static async Task<OfflinePayloadLease> PrepareAsync(Stream payload, string expectedSha256, string parent)
    {
        if (expectedSha256.Length != 64 || !expectedSha256.All(Uri.IsHexDigit))
            throw new IOException("Installer payload digest is missing.");
        // A bounded local copy permits hashing resource streams that cannot seek.
        PlainAncestors(parent);
        var staging = Path.Combine(parent, "staging-" + Guid.NewGuid().ToString("N"));
        CreatePrivateDirectory(staging);
        var archivePath = Path.Combine(staging, "payload.zip");
        using (var copy = new FileStream(archivePath, FileMode.CreateNew, FileAccess.ReadWrite, FileShare.None,
            1024 * 1024, FileOptions.Asynchronous))
        {
            await CopyBoundedAsync(payload, copy, MaximumExpandedBytes);
            copy.Position = 0;
            if (!Convert.ToHexString(await SHA256.HashDataAsync(copy)).Equals(expectedSha256, StringComparison.OrdinalIgnoreCase))
                throw new IOException("Installer payload digest differs.");
        }
        var destination = Path.Combine(parent, expectedSha256.ToLowerInvariant());
        using var archive = ZipFile.OpenRead(archivePath);
        var entries = ValidateEntries(archive);
        if (Directory.Exists(destination))
        {
            var retained = await RetainVerifiedAsync(destination, entries);
            // Only this invocation's verified archive and empty staging directory are disposable.
            return Complete(retained, archive, archivePath, staging);
        }
        var extracted = Path.Combine(staging, "files");
        CreatePrivateDirectory(extracted);
        foreach (var entry in entries)
        {
            var path = Path.Combine(extracted, entry.FullName.Replace('/', Path.DirectorySeparatorChar));
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            using var input = entry.Open();
            using var output = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None, 1024 * 1024, true);
            if (await CopyBoundedAsync(input, output, entry.Length) != entry.Length)
                throw new IOException("Installer extraction is incomplete.");
        }
        // Never replace another installation or a retained extraction. Failed staging remains for inspection.
        Directory.Move(extracted, destination);
        var completed = await RetainVerifiedAsync(destination, entries);
        return Complete(completed, archive, archivePath, staging);
    }

    private static OfflinePayloadLease Complete(OfflinePayloadLease lease, ZipArchive archive, string archivePath, string staging)
    {
        try { archive.Dispose(); File.Delete(archivePath); Directory.Delete(staging); return lease; }
        catch { lease.Dispose(); throw; }
    }

    private static List<ZipArchiveEntry> ValidateEntries(ZipArchive archive)
    {
        if (archive.Entries.Count is 0 or > 20000) throw new IOException("Invalid installer archive size.");
        var names = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        long bytes = 0;
        foreach (var entry in archive.Entries)
        {
            if (!SafeName(entry.FullName) || !names.Add(entry.FullName) ||
                ((entry.ExternalAttributes >> 16) & 0xF000) is not (0 or 0x8000) ||
                (entry.ExternalAttributes & (int)FileAttributes.ReparsePoint) != 0)
                throw new IOException("Unsafe installer archive entry.");
            if (entry.Length < 0 || entry.Length > MaximumExpandedBytes - bytes)
                throw new IOException("Installer archive exceeds its size limit.");
            bytes += entry.Length;
        }
        if (!names.Contains("setup/EitmadSetup.exe") || !names.Contains("offline.cat") || !names.Contains("setup-bundle.json"))
            throw new IOException("Installer archive is incomplete.");
        return archive.Entries.ToList();
    }

    private static bool SafeName(string name) => name.Length is > 0 and <= 240 &&
        name.Split('/').All(part => part.Length > 0 && part is not ("." or "..") &&
            !part.EndsWith('.') && !part.EndsWith(' ') &&
            part.IndexOfAny(Path.GetInvalidFileNameChars()) < 0 &&
            !System.Text.RegularExpressions.Regex.IsMatch(part, @"^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(\.|$)",
                System.Text.RegularExpressions.RegexOptions.IgnoreCase | System.Text.RegularExpressions.RegexOptions.CultureInvariant));

    private static async Task VerifyExistingAsync(string root, List<ZipArchiveEntry> entries)
    {
        PlainAncestors(root);
        if (CountPlainFiles(root) != entries.Count) throw new IOException("Retained installer payload differs.");
        foreach (var entry in entries)
        {
            var path = Path.Combine(root, entry.FullName.Replace('/', Path.DirectorySeparatorChar));
            using var stored = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
            using var expected = entry.Open();
            var observedHash = await SHA256.HashDataAsync(stored);
            var expectedHash = await SHA256.HashDataAsync(expected);
            if (stored.Length != entry.Length || !CryptographicOperations.FixedTimeEquals(observedHash, expectedHash))
                throw new IOException("Retained installer payload differs.");
        }
    }

    private static int CountPlainFiles(string root)
    {
        var count = 0;
        foreach (var path in Directory.EnumerateFileSystemEntries(root))
        {
            var attributes = File.GetAttributes(path);
            if ((attributes & FileAttributes.ReparsePoint) != 0) throw new IOException("Retained installer payload contains a link.");
            count += (attributes & FileAttributes.Directory) != 0 ? CountPlainFiles(path) : 1;
        }
        return count;
    }

    private static async Task<OfflinePayloadLease> RetainVerifiedAsync(string root, List<ZipArchiveEntry> entries)
    {
        var lease = OfflinePayloadLease.RetainForLaunch(root);
        try { await VerifyExistingAsync(root, entries); return lease; }
        catch { lease.Dispose(); throw; }
    }

    private static async Task<long> CopyBoundedAsync(Stream input, Stream output, long limit)
    {
        var buffer = new byte[1024 * 1024];
        long written = 0;
        int count;
        while ((count = await input.ReadAsync(buffer)) != 0)
        {
            if (count > limit - written) throw new IOException("Installer data exceeds its size limit.");
            await output.WriteAsync(buffer.AsMemory(0, count)); written += count;
        }
        return written;
    }

    internal static void CreatePrivateDirectory(string path)
    {
        PlainAncestors(path);
        var security = new DirectorySecurity();
        security.SetAccessRuleProtection(true, false);
        var user = WindowsIdentity.GetCurrent().User ?? throw new IOException("Windows identity unavailable.");
        security.SetOwner(user);
        foreach (var sid in new[] { user, new SecurityIdentifier("S-1-5-18"), new SecurityIdentifier("S-1-5-32-544") })
            security.AddAccessRule(new FileSystemAccessRule(sid, FileSystemRights.FullControl,
                InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit, PropagationFlags.None, AccessControlType.Allow));
        new DirectoryInfo(path).Create(security);
        PlainAncestors(path);
    }

    private static void PlainAncestors(string path)
    {
        for (var current = new DirectoryInfo(Path.GetFullPath(path)); current is not null; current = current.Parent)
            if (current.Exists && (current.Attributes & FileAttributes.ReparsePoint) != 0)
                throw new IOException("Installer paths cannot contain links or junctions.");
    }
}
