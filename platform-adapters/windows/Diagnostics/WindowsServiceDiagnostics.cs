using System.Runtime.InteropServices;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.Diagnostics;

internal static class WindowsServiceDiagnostics
{
    internal static LocalDiagnosticService[] Read() => [Read("EitmadServer"), Read("EitmadPostgres")];

    private static LocalDiagnosticService Read(string name)
    {
        var result = new LocalDiagnosticService { Name = name, State = "Unavailable" };
        if (!OperatingSystem.IsWindows()) return result;
        var manager = OpenSCManager(null, null, 1);
        if (manager == IntPtr.Zero) { result.NativeCode = Marshal.GetLastWin32Error(); return result; }
        try
        {
            var service = OpenService(manager, name, 4);
            if (service == IntPtr.Zero)
            {
                result.NativeCode = Marshal.GetLastWin32Error();
                result.State = result.NativeCode == 1060 ? "NotInstalled" : result.NativeCode == 5 ? "Denied" : "Unavailable";
                return result;
            }
            try
            {
                if (!QueryServiceStatus(service, out var status)) { result.NativeCode = Marshal.GetLastWin32Error(); return result; }
                result.State = status.State switch { 1 => "Stopped", 2 => "StartPending", 3 => "StopPending", 4 => "Running", 5 => "ContinuePending", 6 => "PausePending", 7 => "Paused", _ => "Unavailable" };
                result.ServiceExitCode = status.Win32ExitCode;
                return result;
            }
            finally { CloseServiceHandle(service); }
        }
        finally { CloseServiceHandle(manager); }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ServiceStatus { public uint Type, State, ControlsAccepted, Win32ExitCode, SpecificExitCode, Checkpoint, WaitHint; }
    [DllImport("advapi32.dll", EntryPoint = "OpenSCManagerW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr OpenSCManager(string? machine, string? database, uint access);
    [DllImport("advapi32.dll", EntryPoint = "OpenServiceW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr OpenService(IntPtr manager, string name, uint access);
    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool QueryServiceStatus(IntPtr service, out ServiceStatus status);
    [DllImport("advapi32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseServiceHandle(IntPtr handle);
}
