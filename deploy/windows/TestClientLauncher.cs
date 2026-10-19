using System;
using System.Diagnostics;
using System.IO;

// The test MSIX must use the same isolated runtime from Start and direct launch.
internal static class TestClientLauncher
{
    [STAThread]
    private static void Main()
    {
        string directory = AppDomain.CurrentDomain.BaseDirectory;
        string runtime = Path.Combine(Environment.GetFolderPath(
            Environment.SpecialFolder.LocalApplicationData), "Eitmad", "TestClient", "engine");
        Process.Start(new ProcessStartInfo
        {
            FileName = Path.Combine(directory, "Eitmad.WindowsShell.exe"),
            WorkingDirectory = directory,
            Arguments = "--runtime-directory \"" + runtime + "\"",
            UseShellExecute = false
        });
    }
}
