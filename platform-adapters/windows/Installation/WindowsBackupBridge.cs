using System.Diagnostics;
using System.Runtime.Versioning;
using System.Text;
using System.Text.Json;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.Installation;

public interface IWindowsBackupBridge
{
    Task<ServerBackupManagement> ManageAsync(ServerBackupAction action, SetupBackupDestinations? destinations = null);
}

/// <summary>Uses the authenticated setup payload and its privileged Windows channel. Rust owns state and policy.</summary>
[SupportedOSPlatform("windows")]
public sealed class WindowsBackupBridge(string bundleDirectory) : IWindowsBackupBridge
{
    public async Task<ServerBackupManagement> ManageAsync(ServerBackupAction action, SetupBackupDestinations? destinations = null)
    {
        using var payload = await Task.Run(() => OfflinePayloadLease.AcquireAsync(bundleDirectory, Path.Combine(bundleDirectory, "setup", "EitmadSetup.exe")));
        var start = new ProcessStartInfo(Path.Combine(bundleDirectory, "server", "application", "eitmad-server.exe")) {
            ArgumentList = { "windows-backup-management" }, UseShellExecute = false, CreateNoWindow = true,
            RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true,
            StandardInputEncoding = new UTF8Encoding(false), StandardOutputEncoding = Encoding.UTF8,
        };
        using var child = Process.Start(start) ?? throw new IOException("Backup backend unavailable.");
        var errors = child.StandardError.BaseStream.CopyToAsync(Stream.Null);
        await JsonSerializer.SerializeAsync(child.StandardInput.BaseStream,
            new ManageServerBackup { Version = 1, Action = action, Destinations = destinations! }, Converter.Settings);
        child.StandardInput.Close();
        var bytes = new MemoryStream();
        var buffer = new byte[4096];
        try {
            int read;
            while ((read = await child.StandardOutput.BaseStream.ReadAsync(buffer)) != 0) {
                if (bytes.Length + read > 32768) throw new IOException("Backup response exceeded its limit.");
                await bytes.WriteAsync(buffer.AsMemory(0, read));
            }
            await child.WaitForExitAsync(); await errors;
            if (child.ExitCode != 0) throw new IOException("Windows backup administration was not confirmed.");
            var state = JsonSerializer.Deserialize<ServerBackupManagement>(bytes.ToArray(), Converter.Settings);
            if (state?.Version != 1 || state.Status?.Version != 1 || state.Destinations is null || state.Status.DeploymentId == Guid.Empty)
                throw new IOException("Unsupported backup response.");
            return state;
        }
        catch { if (!child.HasExited) child.Kill(entireProcessTree: true); throw; }
        finally { bytes.Dispose(); }
    }
}
