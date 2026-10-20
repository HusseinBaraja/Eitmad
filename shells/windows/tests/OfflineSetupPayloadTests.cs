using System.IO;
using Eitmad.Platform.Windows.Installation;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests;

[TestClass]
public sealed class OfflineSetupPayloadTests
{
    [TestMethod]
    public async Task Launcher_refuses_a_missing_or_unsigned_catalog_before_starting_the_backend()
    {
        var root = Path.Combine(Path.GetTempPath(), "eitmad-bootstrap-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(Path.Combine(root, "setup"));
        Directory.CreateDirectory(Path.Combine(root, "server", "application"));
        try
        {
            // A real executable would produce non-JSON output if the launch guard were skipped.
            var source = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Windows), "System32", "whoami.exe");
            File.Copy(source, Path.Combine(root, "setup", "EitmadSetup.exe"));
            File.Copy(source, Path.Combine(root, "server", "application", "eitmad-server.exe"));
            var bridge = new WindowsSetupBridge(root);
            var missing = await Assert.ThrowsAsync<IOException>(() => bridge.InspectAsync());
            Assert.AreEqual("Setup payload authentication failed.", missing.Message);
            File.WriteAllText(Path.Combine(root, "offline.cat"), "Synthetic unsigned catalog");
            var unsigned = await Assert.ThrowsAsync<IOException>(() => bridge.InspectAsync());
            Assert.AreEqual("Setup payload authentication failed.", unsigned.Message);
            // Failed verification releases all file and directory handles.
            File.WriteAllText(Path.Combine(root, "server", "application", "eitmad-server.exe"), "released");
        }
        finally { Directory.Delete(root, true); }
    }
}
