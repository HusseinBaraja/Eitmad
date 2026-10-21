using System.Diagnostics;
using System.Runtime.Versioning;
using System.Security.Principal;
using System.Text;
using System.Text.Json;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.Installation;

public interface IWindowsSetupBridge
{
    Task<WindowsInstallationState?> InspectAsync();
    Task InstallAsync(GuidedSetupAction action, InitializeWorkshop? workshop, SetupBackupDestinations? backup, IProgress<WindowsInstallProgress> progress);
    Task RetryAsync(string? ownerPassword, IProgress<WindowsInstallProgress> progress);
    Task OpenDesktopAsync();
}

/// <summary>Private child-process transport. Rust owns all installation and settings policy.</summary>
[SupportedOSPlatform("windows")]
public sealed class WindowsSetupBridge : IWindowsSetupBridge
{
    private readonly string bundleDirectory;
    private readonly string backend;
    public WindowsSetupBridge(string bundleDirectory)
    {
        this.bundleDirectory = Path.GetFullPath(bundleDirectory);
        backend = Path.Combine(this.bundleDirectory, "server", "application", "eitmad-server.exe");
    }
    private GuidedWindowsSetup Request(GuidedSetupAction action, InitializeWorkshop? workshop = null, SetupBackupDestinations? backup = null, string? ownerPassword = null) =>
        new() { Version = 1, Action = action, BundleDirectory = bundleDirectory,
            IntendedUserSid = WindowsIdentity.GetCurrent().User?.Value ?? throw new InvalidOperationException("Windows identity unavailable."),
            Workshop = workshop!, Backup = backup!, OwnerPassword = ownerPassword! };
    private Process Start() => Process.Start(new ProcessStartInfo(backend) {
        ArgumentList = { "guided-windows-setup" }, UseShellExecute = false, CreateNoWindow = true,
        RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true,
        StandardInputEncoding = new UTF8Encoding(false), StandardOutputEncoding = Encoding.UTF8,
    }) ?? throw new IOException("Setup backend unavailable.");
    public async Task<WindowsInstallationState?> InspectAsync()
    {
        using var payload = await Task.Run(() => OfflinePayloadLease.AcquireAsync(bundleDirectory, Path.Combine(bundleDirectory, "setup", "EitmadSetup.exe")));
        using var child = Start();
        var errors = child.StandardError.BaseStream.CopyToAsync(Stream.Null);
        await JsonSerializer.SerializeAsync(child.StandardInput.BaseStream, Request(GuidedSetupAction.Inspect), Converter.Settings);
        child.StandardInput.Close();
        var output = await ReadBoundedLineAsync(child.StandardOutput);
        await child.WaitForExitAsync(); await errors;
        if (child.ExitCode != 0 || output is null) throw new IOException("Installation state unavailable.");
        return JsonSerializer.Deserialize<WindowsInstallationState>(output, Converter.Settings);
    }
    public Task InstallAsync(GuidedSetupAction action, InitializeWorkshop? workshop, SetupBackupDestinations? backup, IProgress<WindowsInstallProgress> progress) =>
        RunAsync(action, workshop, backup, null, progress);
    public Task RetryAsync(string? ownerPassword, IProgress<WindowsInstallProgress> progress) =>
        RunAsync(GuidedSetupAction.Retry, null, null, ownerPassword, progress);
    private async Task RunAsync(GuidedSetupAction action, InitializeWorkshop? workshop, SetupBackupDestinations? backup, string? ownerPassword, IProgress<WindowsInstallProgress> progress)
    {
        using var payload = await Task.Run(() => OfflinePayloadLease.AcquireAsync(bundleDirectory, Path.Combine(bundleDirectory, "setup", "EitmadSetup.exe")));
        using var child = Start();
        var errors = child.StandardError.BaseStream.CopyToAsync(Stream.Null);
        var request = Request(action, workshop, backup, ownerPassword);
        ownerPassword = null;
        try { await JsonSerializer.SerializeAsync(child.StandardInput.BaseStream, request, Converter.Settings); }
        finally { if (workshop is not null) workshop.OwnerPassword = ""; request.Workshop = null!; request.OwnerPassword = null!; child.StandardInput.Close(); }
        long sequence = 0;
        WindowsInstallProgress? last = null;
        while (await ReadBoundedLineAsync(child.StandardOutput) is { } line)
        {
            var next = JsonSerializer.Deserialize<WindowsInstallProgress>(line, Converter.Settings)
                ?? throw new IOException("Invalid installation progress.");
            if (next.Version != 1 || next.Sequence != sequence + 1 || last?.Stage == WindowsInstallStage.Complete)
                throw new IOException("Invalid installation order.");
            sequence = next.Sequence; last = next; progress.Report(next);
        }
        await child.WaitForExitAsync(); await errors;
        if (child.ExitCode != 0 || last?.Stage != WindowsInstallStage.Complete || last.Status != WindowsInstallStatus.Succeeded)
            throw new IOException("Installation was not confirmed. Preserve retained state.");
    }
    private static async Task<string?> ReadBoundedLineAsync(StreamReader reader)
    {
        var line = new StringBuilder(); var character = new char[1];
        while (await reader.ReadAsync(character) != 0)
        {
            if (character[0] == '\n') return line.ToString();
            if (line.Length >= 4096) throw new IOException("Installation output exceeded its limit.");
            line.Append(character[0]);
        }
        return line.Length == 0 ? null : throw new IOException("Incomplete installation output.");
    }
    public async Task OpenDesktopAsync()
    {
        var systemRoot = Environment.GetFolderPath(Environment.SpecialFolder.Windows);
        var start = new ProcessStartInfo(Path.Combine(systemRoot, "System32", "WindowsPowerShell", "v1.0", "powershell.exe")) {
            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true,
        };
        foreach (var arg in new[] { "-NoProfile", "-NonInteractive", "-Command", "$p=Get-AppxPackage -Name Eitmad.Desktop; if (@($p).Count -ne 1) { exit 1 }; Start-Process explorer.exe -ArgumentList ('shell:AppsFolder\'+$p.PackageFamilyName+'!Desktop') -WindowStyle Hidden" }) start.ArgumentList.Add(arg);
        using var child = Process.Start(start) ?? throw new IOException("Desktop unavailable.");
        var output = child.StandardOutput.BaseStream.CopyToAsync(Stream.Null); var errors = child.StandardError.BaseStream.CopyToAsync(Stream.Null);
        await child.WaitForExitAsync(); await Task.WhenAll(output, errors);
        if (child.ExitCode != 0) throw new IOException("Desktop launch was not confirmed.");
    }
}
