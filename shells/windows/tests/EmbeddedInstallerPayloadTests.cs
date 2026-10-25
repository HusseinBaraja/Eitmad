using System.IO;
using System.IO.Compression;
using System.Security.Cryptography;
using Eitmad.Platform.Windows.Installation;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests;

[TestClass]
public sealed class EmbeddedInstallerPayloadTests
{
    private string root = "";
    [TestInitialize]
    public void CreateRoot() => root = Path.Combine(Path.GetTempPath(), "eitmad-extraction-" + Guid.NewGuid().ToString("N"));
    [TestCleanup]
    public void RemoveRoot() { if (Directory.Exists(root)) Directory.Delete(root, true); }

    private static byte[] Payload(string? extra = null, int attributes = 0)
    {
        using var bytes = new MemoryStream();
        using (var archive = new ZipArchive(bytes, ZipArchiveMode.Create, true))
        {
            foreach (var name in new[] { "setup/EitmadSetup.exe", "offline.cat", "setup-bundle.json" })
            { using var writer = new StreamWriter(archive.CreateEntry(name).Open()); writer.Write("synthetic " + name); }
            if (extra is not null)
            {
                var entry = archive.CreateEntry(extra); entry.ExternalAttributes = attributes;
                using var writer = new StreamWriter(entry.Open()); writer.Write("synthetic extra");
            }
        }
        return bytes.ToArray();
    }

    private Task<OfflinePayloadLease> Extract(byte[] bytes, string? digest = null) =>
        EmbeddedInstallerPayload.PrepareAsync(new MemoryStream(bytes), digest ?? Convert.ToHexString(SHA256.HashData(bytes)), root);

    [TestMethod]
    public async Task Verified_payload_is_reused_and_retained_until_the_child_finishes()
    {
        var bytes = Payload(); string destination;
        using (var lease = await Extract(bytes))
        {
            destination = lease.RootDirectory;
            Assert.AreEqual("synthetic setup/EitmadSetup.exe", File.ReadAllText(Path.Combine(destination, "setup/EitmadSetup.exe")));
            Assert.ThrowsExactly<IOException>(() => File.WriteAllText(Path.Combine(destination, "setup/EitmadSetup.exe"), "changed"));
            Assert.ThrowsExactly<IOException>(() => Directory.Move(destination, destination + "-moved"));
            using var reused = await Extract(bytes);
            Assert.AreEqual(destination, reused.RootDirectory);
        }
        File.WriteAllText(Path.Combine(destination, "setup/EitmadSetup.exe"), "changed");
        await Assert.ThrowsAsync<IOException>(() => Extract(bytes));
        Assert.AreEqual("changed", File.ReadAllText(Path.Combine(destination, "setup/EitmadSetup.exe")));
    }

    [TestMethod]
    public async Task Payload_digest_must_match_before_any_code_is_extracted()
    {
        await Assert.ThrowsAsync<IOException>(() => Extract(Payload(), new string('0', 64)));
        Assert.IsFalse(Directory.GetFiles(root, "*.exe", SearchOption.AllDirectories).Any());
    }

    [TestMethod]
    [DataRow("../escaped.exe", 0)] [DataRow("setup/../../escaped.exe", 0)]
    [DataRow("/escaped.exe", 0)] [DataRow("C:/escaped.exe", 0)] [DataRow("setup\\escaped.exe", 0)]
    [DataRow("setup/EitmadSetup.EXE", 0)] [DataRow("setup/NUL.txt", 0)] [DataRow("setup/extra. ", 0)]
    [DataRow("setup/link.exe", unchecked((int)0xA0000000))]
    public async Task Unsafe_archive_entries_are_rejected_before_extraction(string entry, int attributes)
    {
        await Assert.ThrowsAsync<IOException>(() => Extract(Payload(entry, attributes)));
        Assert.IsFalse(Directory.GetFiles(root, "*.exe", SearchOption.AllDirectories).Any());
    }

    [TestMethod]
    public async Task An_unexpected_file_in_a_retained_payload_is_preserved_and_refused()
    {
        var bytes = Payload(); string destination;
        using (var lease = await Extract(bytes)) destination = lease.RootDirectory;
        var extra = Path.Combine(destination, "unexpected.dll"); File.WriteAllText(extra, "preserve");
        await Assert.ThrowsAsync<IOException>(() => Extract(bytes));
        Assert.AreEqual("preserve", File.ReadAllText(extra));
    }

    [TestMethod]
    public void Unsigned_installer_is_refused_by_Windows_trust_policy()
    {
        Directory.CreateDirectory(root);
        var executable = Path.Combine(root, "unsigned.exe"); File.WriteAllText(executable, "synthetic unsigned executable");
        Assert.ThrowsExactly<IOException>(() => AuthenticodeTrust.Verify(executable));
    }
}
