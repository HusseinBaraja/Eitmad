// Windows elevation transport. Only public coordinates enter argv; private JSON uses an ACL pipe.
using System;
using System.Diagnostics;
using System.IO;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Principal;
using System.Text;
using System.Threading.Tasks;

public static class EitmadInstallationChannel
{
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetNamedPipeClientProcessId(Microsoft.Win32.SafeHandles.SafePipeHandle pipe, out uint id);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetNamedPipeServerProcessId(Microsoft.Win32.SafeHandles.SafePipeHandle pipe, out uint id);

    public static NamedPipeServerStream Create(string name)
    {
        var acl = new PipeSecurity();
        acl.SetAccessRuleProtection(true, false);
        acl.AddAccessRule(new PipeAccessRule(new SecurityIdentifier("S-1-5-2"), PipeAccessRights.FullControl, AccessControlType.Deny));
        foreach (var sid in new[] { WindowsIdentity.GetCurrent().User, new SecurityIdentifier("S-1-5-32-544"), new SecurityIdentifier("S-1-5-18") })
            acl.AddAccessRule(new PipeAccessRule(sid, PipeAccessRights.ReadWrite, AccessControlType.Allow));
        return new NamedPipeServerStream(name, PipeDirection.InOut, 1, PipeTransmissionMode.Byte,
            PipeOptions.Asynchronous, 4096, 4096, acl);
    }

    public static void Accept(NamedPipeServerStream pipe, int childId)
    {
        var pending = pipe.BeginWaitForConnection(null, null);
        if (!pending.AsyncWaitHandle.WaitOne(180000)) throw new IOException("Elevation connection deadline elapsed.");
        pipe.EndWaitForConnection(pending);
        uint actual;
        if (!GetNamedPipeClientProcessId(pipe.SafePipeHandle, out actual) || actual != childId)
            throw new IOException("Elevation peer mismatch.");
    }

    public static NamedPipeClientStream Connect(string name, int parentId)
    {
        var pipe = new NamedPipeClientStream(".", name, PipeDirection.InOut, PipeOptions.Asynchronous,
            TokenImpersonationLevel.Anonymous);
        try
        {
            pipe.Connect(30000);
            uint actual;
            if (!GetNamedPipeServerProcessId(pipe.SafePipeHandle, out actual) || actual != parentId)
                throw new IOException("Elevation broker mismatch.");
            return pipe;
        }
        catch { pipe.Dispose(); throw; }
    }

    private static void Await(Task operation)
    {
        if (!operation.Wait(1200000)) throw new IOException("Installation channel deadline elapsed.");
        operation.GetAwaiter().GetResult();
    }

    private static byte[] ReadExact(Stream pipe, int size)
    {
        var bytes = new byte[size];
        int offset = 0;
        while (offset < size)
        {
            var read = pipe.ReadAsync(bytes, offset, size - offset);
            Await(read);
            if (read.Result == 0) throw new EndOfStreamException();
            offset += read.Result;
        }
        return bytes;
    }

    public static void Send(Stream pipe, string text)
    {
        var bytes = Encoding.UTF8.GetBytes(text);
        if (bytes.Length > 65536) throw new IOException("Private input exceeds its bound.");
        try
        {
            var length = BitConverter.GetBytes(bytes.Length);
            Await(pipe.WriteAsync(length, 0, length.Length));
            Await(pipe.WriteAsync(bytes, 0, bytes.Length));
            pipe.Flush();
        }
        finally { Array.Clear(bytes, 0, bytes.Length); }
    }

    public static string Receive(Stream pipe, int maximum)
    {
        int size = BitConverter.ToInt32(ReadExact(pipe, 4), 0);
        if (size < 0 || size > maximum) throw new IOException("Invalid installation frame.");
        var bytes = ReadExact(pipe, size);
        try { return new UTF8Encoding(false, true).GetString(bytes); }
        finally { Array.Clear(bytes, 0, bytes.Length); }
    }

    public static string ReadPrivateInput(TextReader input)
    {
        var line = new StringBuilder();
        while (true)
        {
            int character = input.Read();
            if (character < 0) throw new EndOfStreamException();
            if (character == '\n') break;
            if (line.Length >= 65536) throw new IOException("Private input exceeds its bound.");
            line.Append((char)character);
        }
        var value = line.ToString();
        if (Encoding.UTF8.GetByteCount(value) > 65536) throw new IOException("Private input exceeds its bound.");
        return value;
    }

    private static void WritePrivateInput(Process process, string value)
    {
        var input = Encoding.UTF8.GetBytes(value + "\n");
        try {
            process.StandardInput.BaseStream.Write(input, 0, input.Length);
            process.StandardInput.BaseStream.Flush();
        }
        finally { Array.Clear(input, 0, input.Length); }
    }

    public static int RunWorker(Stream pipe, string backend, Func<string, bool> requiresInput)
    {
        var request = Receive(pipe, 65536);
        var start = new ProcessStartInfo(backend, "install-windows-server") {
            UseShellExecute = false, CreateNoWindow = true,
            RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true,
            StandardOutputEncoding = Encoding.UTF8
        };
        using (var process = Process.Start(start))
        {
            var errors = process.StandardError.BaseStream.CopyToAsync(Stream.Null);
            WritePrivateInput(process, request);
            request = null;
            try
            {
                while (true)
                {
                    var line = new StringBuilder();
                    while (true)
                    {
                        var character = new char[1];
                        var read = process.StandardOutput.ReadAsync(character, 0, 1);
                        Await(read);
                        if (read.Result == 0 || character[0] == '\n') break;
                        if (line.Length >= 32768) throw new IOException("Invalid progress frame.");
                        line.Append(character[0]);
                    }
                    if (line.Length == 0) break;
                    var value = line.ToString();
                    Send(pipe, value);
                    if (requiresInput(value)) WritePrivateInput(process, Receive(pipe, 65536));
                }
                if (!process.WaitForExit(10000)) throw new IOException("Worker did not exit.");
                Await(errors);
                Send(pipe, "");
                Send(pipe, process.ExitCode == 0 ? "success" : "failed");
                return process.ExitCode;
            }
            catch
            {
                // Do not reset state or attempt another installation after a broken channel.
                // The Rust worker owns its cluster cleanup and retained progress.
                throw;
            }
            finally { process.StandardInput.Close(); }
        }
    }
}
