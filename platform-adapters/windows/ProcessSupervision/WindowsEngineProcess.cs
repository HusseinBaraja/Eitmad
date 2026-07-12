using System.Diagnostics;

namespace Eitmad.Platform.Windows.ProcessSupervision;

internal sealed class WindowsEngineProcessLauncher : IEngineProcessLauncher
{
    public IEngineProcess Launch(EngineLaunchRequest request, long generation, IProcessGroup group)
    {
        _ = generation;
        var executable = Path.GetFullPath(request.EngineExecutablePath);
        if (!File.Exists(executable))
        {
            throw new FileNotFoundException("The packaged engine executable was not found.", executable);
        }

        var startInfo = new ProcessStartInfo
        {
            FileName = executable,
            WorkingDirectory = Path.GetDirectoryName(executable)!,
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
        };
        startInfo.ArgumentList.Add("run");
        startInfo.ArgumentList.Add("--mode");
        startInfo.ArgumentList.Add("supervised");
        startInfo.ArgumentList.Add("--supervisor-pid");
        startInfo.ArgumentList.Add(Environment.ProcessId.ToString(System.Globalization.CultureInfo.InvariantCulture));
        if (request.RuntimeDirectory is not null)
        {
            startInfo.ArgumentList.Add("--runtime-directory");
            startInfo.ArgumentList.Add(Path.GetFullPath(request.RuntimeDirectory));
        }

        var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException("Windows did not start the engine process.");
        var engineProcess = new WindowsEngineProcess(process);
        try
        {
            group.Assign(engineProcess);
            return engineProcess;
        }
        catch
        {
            engineProcess.Kill();
            engineProcess.DisposeAsync().AsTask().GetAwaiter().GetResult();
            throw;
        }
    }
}

internal sealed class WindowsEngineProcess(Process process) : IEngineProcess
{
    public int ProcessId => process.Id;
    public nint NativeHandle => process.Handle;
    public TextReader StandardOutput => process.StandardOutput;
    public TextReader StandardError => process.StandardError;
    public TextWriter StandardInput => process.StandardInput;

    public async Task<int> WaitForExitAsync(CancellationToken cancellationToken)
    {
        await process.WaitForExitAsync(cancellationToken).ConfigureAwait(false);
        return process.ExitCode;
    }

    public void Kill()
    {
        if (!process.HasExited)
        {
            process.Kill(entireProcessTree: false);
        }
    }

    public ValueTask DisposeAsync()
    {
        process.Dispose();
        return ValueTask.CompletedTask;
    }
}
