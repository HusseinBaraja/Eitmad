using System.Runtime.InteropServices;
using System.Runtime.Versioning;

namespace Eitmad.Platform.Windows.Installation;

[SupportedOSPlatform("windows")]
internal static class WindowsDesktopActivation
{
    // Activation returns the app's process ID. An Explorer helper exit cannot confirm activation.
    internal static uint Activate(string applicationId)
    {
        IApplicationActivationManager? manager = null;
        try
        {
            manager = (IApplicationActivationManager)new ApplicationActivationManager();
            // A foreground grant is optional when the user has switched to another window.
            _ = CoAllowSetForegroundWindow(manager, IntPtr.Zero);
            // Suppress the Windows error dialog so setup can show its own recovery action.
            const uint noErrorUi = 0x00000002;
            var result = manager.ActivateApplication(applicationId, null, noErrorUi, out var processId);
            if (result < 0)
                throw new IOException($"Windows desktop activation failed (HRESULT=0x{result:X8}).",
                    Marshal.GetExceptionForHR(result));
            if (processId == 0) throw new IOException("Windows did not return an activated desktop process.");
            return processId;
        }
        catch (Exception error) when (error is COMException or UnauthorizedAccessException)
        {
            throw new IOException($"Windows desktop activation failed (HRESULT=0x{error.HResult:X8}).", error);
        }
        finally { if (manager is not null) Marshal.ReleaseComObject(manager); }
    }

    [DllImport("ole32.dll", ExactSpelling = true)]
    private static extern int CoAllowSetForegroundWindow(
        [MarshalAs(UnmanagedType.IUnknown)] object unknown, IntPtr reserved);

    [ComImport, Guid("45BA127D-10A8-46EA-8AB7-56EA9078943C")]
    private class ApplicationActivationManager { }

    [ComImport, Guid("2E941141-7F97-4756-BA1D-9DECDE894A3D"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IApplicationActivationManager
    {
        [PreserveSig]
        int ActivateApplication([MarshalAs(UnmanagedType.LPWStr)] string applicationId,
            [MarshalAs(UnmanagedType.LPWStr)] string? arguments, uint options, out uint processId);
    }
}
