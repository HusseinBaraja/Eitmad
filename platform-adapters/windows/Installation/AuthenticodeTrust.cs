using System.Runtime.InteropServices;
using System.Runtime.Versioning;

namespace Eitmad.Platform.Windows.Installation;

/// <summary>Use Windows trust policy before executing an embedded installer payload.</summary>
[SupportedOSPlatform("windows")]
public static class AuthenticodeTrust
{
    /// <summary>Prevent replacement of the installer while its publisher and payload are verified.</summary>
    public static FileStream RetainInstaller(string path) =>
        new(path, FileMode.Open, FileAccess.Read, FileShare.Read);

    [DllImport("wintrust.dll", ExactSpelling = true, CharSet = CharSet.Unicode)]
    private static extern int WinVerifyTrust(nint window, [In] ref Guid action, ref TrustData data);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct FileInfo
    {
        public uint Size;
        [MarshalAs(UnmanagedType.LPWStr)] public string Path;
        public nint Handle;
        public nint Subject;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct TrustData
    {
        public uint Size;
        public nint Policy, Sip;
        public uint Ui, Revocation, Choice;
        public nint File;
        public uint StateAction;
        public nint State, Url;
        public uint Flags, Context;
        public nint SignatureSettings;
    }

    public static void Verify(string path)
    {
        using var source = RetainInstaller(path);
        Verify(source);
    }

    public static void Verify(FileStream source)
    {
        var file = new FileInfo { Size = (uint)Marshal.SizeOf<FileInfo>(), Path = source.Name,
            Handle = source.SafeFileHandle.DangerousGetHandle() };
        var pointer = Marshal.AllocHGlobal(Marshal.SizeOf<FileInfo>());
        Marshal.StructureToPtr(file, pointer, false);
        var data = new TrustData {
            Size = (uint)Marshal.SizeOf<TrustData>(), Ui = 2, Revocation = 1,
            Choice = 1, File = pointer, StateAction = 1, Flags = 0x80 | 0x2000,
        };
        var action = new Guid("00AAC56B-CD44-11d0-8CC2-00C04FC295EE");
        try
        {
            if (WinVerifyTrust(new nint(-1), ref action, ref data) != 0)
                throw new IOException("Installer signature is not trusted by Windows.");
        }
        finally
        {
            data.StateAction = 2;
            _ = WinVerifyTrust(new nint(-1), ref action, ref data);
            Marshal.DestroyStructure<FileInfo>(pointer);
            Marshal.FreeHGlobal(pointer);
            GC.KeepAlive(source);
        }
    }
}
