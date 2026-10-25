using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Runtime.Versioning;
using System.Text;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;

namespace Eitmad.Platform.Windows.Installation;

/// <summary>Retain immutable local files while Windows authenticates and uses a release catalog.</summary>
[SupportedOSPlatform("windows")]
public sealed class OfflinePayloadLease : IDisposable
{
    private readonly List<SafeFileHandle> handles = [];
    public string RootDirectory { get; private init; } = "";

    internal static OfflinePayloadLease RetainForLaunch(string root)
    {
        var lease = new OfflinePayloadLease { RootDirectory = Path.GetFullPath(root) };
        try { lease.RetainTree(lease.RootDirectory); return lease; }
        catch { lease.Dispose(); throw; }
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFile(string path, uint access, uint share, IntPtr security, uint mode, uint flags, IntPtr template);

    private void Retain(string path, bool directory)
    {
        // Files deny write/delete sharing. Directories deny renames without requiring
        // exclusive directory access on busy ancestors. Open reparse points themselves.
        var handle = CreateFile(path, directory ? 0u : 0x80000000u, directory ? 3u : 1u,
            IntPtr.Zero, 3, 0x00200000u | (directory ? 0x02000000u : 0), IntPtr.Zero);
        if (handle.IsInvalid) { var error = new Win32Exception(Marshal.GetLastWin32Error()); handle.Dispose(); throw new IOException("Cannot retain setup payload.", error); }
        handles.Add(handle);
        if ((File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0)
            throw new IOException("Setup payload cannot use reparse points.");
    }

    private void RetainTree(string directory)
    {
        Retain(directory, true);
        foreach (var path in Directory.EnumerateFileSystemEntries(directory))
        {
            var attributes = File.GetAttributes(path);
            if ((attributes & FileAttributes.ReparsePoint) != 0) throw new IOException("Setup payload cannot use reparse points.");
            if ((attributes & FileAttributes.Directory) != 0) RetainTree(path);
            else Retain(path, false);
        }
    }

    public static async Task<OfflinePayloadLease> AcquireAsync(string root, string anchor)
    {
        var lease = new OfflinePayloadLease();
        try
        {
            // Refuse indirect ancestry. Retained tree handles prevent replacement
            // without locking unrelated directories in the user's profile.
            for (var parent = Directory.GetParent(Path.GetFullPath(root)); parent is not null; parent = parent.Parent)
                if ((File.GetAttributes(parent.FullName) & FileAttributes.ReparsePoint) != 0)
                    throw new IOException("Setup payload cannot use reparse points.");
            lease.RetainTree(Path.GetFullPath(root));
            using var source = typeof(OfflinePayloadLease).Assembly.GetManifestResourceStream("Eitmad.VerifyOffline.ps1")
                ?? throw new IOException("Setup verifier unavailable.");
            using var reader = new StreamReader(source, Encoding.UTF8);
            var script = await reader.ReadToEndAsync();
            var start = new ProcessStartInfo(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Windows),
                "System32", "WindowsPowerShell", "v1.0", "powershell.exe")) {
                UseShellExecute = false, CreateNoWindow = true, RedirectStandardInput = true,
                RedirectStandardOutput = true, RedirectStandardError = true, StandardInputEncoding = new UTF8Encoding(false),
            };
            start.Environment.Remove("PSModulePath");
            foreach (var argument in new[] { "-NoProfile", "-NonInteractive", "-EncodedCommand", Convert.ToBase64String(Encoding.Unicode.GetBytes(script)) })
                start.ArgumentList.Add(argument);
            using var child = Process.Start(start) ?? throw new IOException("Setup verifier unavailable.");
            var output = child.StandardOutput.BaseStream.CopyToAsync(Stream.Null);
            var errors = child.StandardError.BaseStream.CopyToAsync(Stream.Null);
            await child.StandardInput.WriteAsync(JsonSerializer.Serialize(new { root = Path.GetFullPath(root), anchor, publisher = "" }));
            child.StandardInput.Close();
            await child.WaitForExitAsync(); await Task.WhenAll(output, errors);
            if (child.ExitCode != 0) throw new IOException("Setup payload authentication failed.");
            return lease;
        }
        catch { lease.Dispose(); throw; }
    }

    public void Dispose()
    {
        for (var index = handles.Count - 1; index >= 0; index--) handles[index].Dispose();
        handles.Clear();
    }
}
