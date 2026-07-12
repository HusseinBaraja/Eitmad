using System.ComponentModel;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace Eitmad.Platform.Windows.ProcessSupervision;

internal sealed class WindowsProcessGroupFactory : IProcessGroupFactory
{
    public IProcessGroup Create() => new WindowsJobObject();
}

internal sealed class WindowsJobObject : IProcessGroup
{
    private const uint JobObjectLimitKillOnJobClose = 0x00002000;
    private readonly SafeJobHandle handle;

    public WindowsJobObject()
    {
        handle = NativeMethods.CreateJobObject(nint.Zero, null);
        if (handle.IsInvalid)
        {
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Windows could not create the engine Job Object.");
        }

        var information = new JobObjectExtendedLimitInformation
        {
            BasicLimitInformation = new JobObjectBasicLimitInformation
            {
                LimitFlags = JobObjectLimitKillOnJobClose,
            },
        };
        var size = Marshal.SizeOf<JobObjectExtendedLimitInformation>();
        var pointer = Marshal.AllocHGlobal(size);
        try
        {
            Marshal.StructureToPtr(information, pointer, false);
            if (!NativeMethods.SetInformationJobObject(handle, 9, pointer, (uint)size))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Windows could not configure engine process containment.");
            }
        }
        finally
        {
            Marshal.FreeHGlobal(pointer);
        }
    }

    public void Assign(IEngineProcess process)
    {
        if (!NativeMethods.AssignProcessToJobObject(handle, process.NativeHandle))
        {
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Windows could not add the engine to its Job Object.");
        }
    }

    public void Terminate()
    {
        if (!NativeMethods.TerminateJobObject(handle, 1))
        {
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Windows could not terminate the engine Job Object.");
        }
    }

    public void Dispose() => handle.Dispose();

    [StructLayout(LayoutKind.Sequential)]
    private struct JobObjectBasicLimitInformation
    {
        public long PerProcessUserTimeLimit;
        public long PerJobUserTimeLimit;
        public uint LimitFlags;
        public nuint MinimumWorkingSetSize;
        public nuint MaximumWorkingSetSize;
        public uint ActiveProcessLimit;
        public nint Affinity;
        public uint PriorityClass;
        public uint SchedulingClass;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct IoCounters
    {
        public ulong ReadOperationCount;
        public ulong WriteOperationCount;
        public ulong OtherOperationCount;
        public ulong ReadTransferCount;
        public ulong WriteTransferCount;
        public ulong OtherTransferCount;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JobObjectExtendedLimitInformation
    {
        public JobObjectBasicLimitInformation BasicLimitInformation;
        public IoCounters IoInfo;
        public nuint ProcessMemoryLimit;
        public nuint JobMemoryLimit;
        public nuint PeakProcessMemoryUsed;
        public nuint PeakJobMemoryUsed;
    }

    private sealed class SafeJobHandle : SafeHandleZeroOrMinusOneIsInvalid
    {
        public SafeJobHandle() : base(true) { }
        protected override bool ReleaseHandle() => NativeMethods.CloseHandle(handle);
    }

    private static class NativeMethods
    {
        [DllImport("kernel32.dll", EntryPoint = "CreateJobObjectW", SetLastError = true, CharSet = CharSet.Unicode)]
        internal static extern SafeJobHandle CreateJobObject(nint securityAttributes, string? name);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool SetInformationJobObject(
            SafeJobHandle job,
            int informationClass,
            nint information,
            uint informationLength);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool AssignProcessToJobObject(SafeJobHandle job, nint process);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool TerminateJobObject(SafeJobHandle job, uint exitCode);

        [DllImport("kernel32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool CloseHandle(nint handle);
    }
}
