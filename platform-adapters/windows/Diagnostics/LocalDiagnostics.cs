using System.ComponentModel;
using System.Diagnostics;
using System.Reflection;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Threading.Channels;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.Diagnostics;

/// <summary>Native file mechanics for the Rust-owned, metadata-only local diagnostic contract.</summary>
public sealed class LocalDiagnosticRecorder : IDisposable
{
    internal static readonly JsonElement Policy = LoadPolicy();
    internal static readonly JsonSerializerOptions Json = new(Converter.Settings) { UnmappedMemberHandling = System.Text.Json.Serialization.JsonUnmappedMemberHandling.Disallow };
    private sealed record WriteItem(LocalDiagnosticEvent? Event, TaskCompletionSource? Flushed = null);
    private readonly Channel<WriteItem> queue;
    private readonly Task worker;
    private readonly string directory;
    private readonly LocalDiagnosticComponent component;
    private readonly Guid session = Guid.NewGuid();
    private long dropped;
    private long failures;
    private long sequence;
    private string? path;
    public long Dropped => Interlocked.Read(ref dropped);
    public long Failures => Interlocked.Read(ref failures);
    internal static int Limit(string name) => Policy.GetProperty(name).GetInt32();

    public LocalDiagnosticRecorder(string directory, LocalDiagnosticComponent component)
    {
        this.directory = Path.GetFullPath(directory); this.component = component;
        queue = Channel.CreateBounded<WriteItem>(new BoundedChannelOptions(Limit("queueCapacity")) { SingleReader = true, FullMode = BoundedChannelFullMode.Wait });
        worker = Task.Run(WriteAsync);
    }

    public void Record(LocalDiagnosticEvent record)
    {
        record.UtcMillis = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        if (!queue.Writer.TryWrite(new WriteItem(record))) Interlocked.Increment(ref dropped);
    }

    private async Task WriteAsync()
    {
        var build = Assembly.GetEntryAssembly()?.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion ?? "unknown";
        if (!Symbol(build)) build = "unknown";
        var hash = "";
        try { if (Environment.ProcessPath is { } executable) { await using var file = File.OpenRead(executable); hash = Convert.ToHexString(await SHA256.HashDataAsync(file)).ToLowerInvariant(); } }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException) { Interlocked.Increment(ref failures); }
        await foreach (var item in queue.Reader.ReadAllAsync().ConfigureAwait(false))
        {
            if (item.Flushed is { } flushed) { flushed.TrySetResult(); continue; }
            var record = item.Event!;
            record.Version = 1; record.Component = component; record.Session = session;
            record.Sequence = ++sequence; record.ProcessId = Environment.ProcessId;
            record.Build = build; record.BinarySha256 = hash; record.OsVersion = Environment.OSVersion.Version.ToString();
            record.Dropped = Interlocked.Exchange(ref dropped, 0);
            try { Append(record); }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException or JsonException or ArgumentException)
            { Interlocked.Increment(ref failures); Interlocked.Add(ref dropped, record.Dropped + 1); }
        }
    }

    private void Append(LocalDiagnosticEvent record)
    {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(record, Json);
        if (!Valid(record) || bytes.Length >= Limit("recordBytes")) throw new InvalidDataException("Invalid diagnostic metadata.");
        PlainPath(directory); Directory.CreateDirectory(directory);
        if (path is null || !File.Exists(path) || new FileInfo(path).Length + bytes.Length + 1 > Limit("fileBytes"))
        {
            var prefix = component.ToString().ToLowerInvariant() + "-";
            path = Path.Combine(directory, prefix + Guid.NewGuid().ToString("N") + ".jsonl");
            using (new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.Read)) { }
            var files = new DirectoryInfo(directory).GetFiles(prefix + "*.jsonl").Where(file => (file.Attributes & FileAttributes.ReparsePoint) == 0)
                .OrderByDescending(file => file.LastWriteTimeUtc).ToArray();
            foreach (var old in files.Where(file => file.FullName != path).Skip(Limit("filesPerComponent") - 1)) old.Delete();
        }
        PlainPath(path);
        using var output = new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete);
        output.Write(bytes); output.WriteByte((byte)'\n');
    }

    internal static bool Allowed(string list, string? value) => value is not null && Policy.GetProperty(list).EnumerateArray().Any(item => item.GetString() == value);
    internal static bool Symbol(string? value) => value is not null && Regex.IsMatch(value, Policy.GetProperty("symbolPattern").GetString()!, RegexOptions.CultureInvariant, TimeSpan.FromMilliseconds(100));
    internal static bool Valid(LocalDiagnosticEvent record) => record.Version == 1 && Enum.IsDefined(record.Component)
        && record.UtcMillis >= 0 && record.Sequence >= 0 && record.ProcessId >= 0 && record.Dropped >= 0
        && record.ElapsedMs is null or >= 0 && record.HttpStatus is null or >= 100 and <= 599
        && Allowed("phases", record.Phase) && Allowed("results", record.Result)
        && (record.ErrorCode is null || Allowed("errorCodes", record.ErrorCode))
        && (record.Operation is null || Allowed("operations", record.Operation))
        && record.Locations is not null && record.Locations.Length <= Limit("maxLocations") && record.Locations.All(Symbol)
        && (record.Build == "" || Symbol(record.Build)) && (record.OsVersion == "" || Symbol(record.OsVersion))
        && record.BinarySha256 is not null && (record.BinarySha256 == "" || record.BinarySha256.Length == 64 && record.BinarySha256.All(Uri.IsHexDigit));

    internal static void PlainPath(string path)
    {
        for (var current = Path.GetFullPath(path); current is not null; current = Path.GetDirectoryName(current))
            if ((File.Exists(current) || Directory.Exists(current)) && (File.GetAttributes(current) & FileAttributes.ReparsePoint) != 0)
                throw new IOException("Diagnostic paths cannot traverse reparse points.");
    }

    private static JsonElement LoadPolicy()
    {
        using var input = typeof(LocalDiagnosticRecorder).Assembly.GetManifestResourceStream("Eitmad.LocalDiagnosticPolicy.json")!;
        using var json = JsonDocument.Parse(input); return json.RootElement.Clone();
    }

    public async Task DrainAsync()
    {
        // Complete only at process shutdown; report capture uses the independent file snapshot.
        queue.Writer.TryComplete();
        await worker.WaitAsync(TimeSpan.FromSeconds(2)).ConfigureAwait(false);
    }
    public async Task FlushAsync()
    {
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(2));
        var done = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        await queue.Writer.WriteAsync(new WriteItem(null, done), timeout.Token).ConfigureAwait(false);
        await done.Task.WaitAsync(timeout.Token).ConfigureAwait(false);
    }
    public void Dispose() { queue.Writer.TryComplete(); try { worker.Wait(TimeSpan.FromSeconds(2)); } catch (AggregateException) { } }
}

public static class LocalDiagnostics
{
    public static string UserDirectory => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Eitmad", "Logs");
    public static string ServerDirectory => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData), "EitmadHost", "diagnostics");
    private static LocalDiagnosticRecorder? recorder;
    public static void Initialize(LocalDiagnosticComponent component)
    {
        recorder ??= new LocalDiagnosticRecorder(UserDirectory, component);
        Record("Process", "Started");
        AppDomain.CurrentDomain.UnhandledException += (_, args) => { if (args.ExceptionObject is Exception error) Error("Crash", error); recorder?.Dispose(); };
        TaskScheduler.UnobservedTaskException += (_, args) => Error("Crash", args.Exception);
    }
    public static void Record(string phase, string result, Guid? correlation = null, string? operation = null,
        string? errorCode = null, long? elapsedMs = null, long? nativeCode = null, string[]? locations = null)
    {
        recorder?.Record(new LocalDiagnosticEvent { Phase = phase, Result = result, Correlation = correlation,
            Operation = operation!, ErrorCode = errorCode!, ElapsedMs = elapsedMs, NativeCode = nativeCode, Locations = locations ?? [] });
    }
    public static void Error(string phase, Exception error, Guid? correlation = null)
        => recorder?.Record(ExceptionEvent(phase, error, correlation));

    internal static LocalDiagnosticEvent ExceptionEvent(string phase, Exception error, Guid? correlation = null)
    {
        var locations = new List<string>();
        long nativeCode = error.HResult;
        // Exception.Message, Data, ToString and argument values are intentionally never read.
        for (var cause = error; cause is not null && locations.Count < LocalDiagnosticRecorder.Limit("maxLocations"); cause = cause.InnerException)
        {
            if (cause is Win32Exception native) nativeCode = native.NativeErrorCode;
            locations.Add(cause.GetType().FullName ?? "Exception");
            foreach (var frame in new StackTrace(cause, false).GetFrames())
            {
                var method = frame.GetMethod();
                var symbol = method?.DeclaringType?.FullName + "." + method?.Name;
                if (LocalDiagnosticRecorder.Symbol(symbol)) locations.Add(symbol);
                if (locations.Count >= LocalDiagnosticRecorder.Limit("maxLocations")) break;
            }
        }
        return new LocalDiagnosticEvent { Phase = phase, Result = "Failed", Correlation = correlation,
            NativeCode = nativeCode, Locations = locations.Where(LocalDiagnosticRecorder.Symbol).ToArray() };
    }
    public static void RecordNativeSetupFailure(WindowsInstallProgress progress)
    {
        if (progress.Details is not { } details || progress.Status is not (WindowsInstallStatus.Failed or WindowsInstallStatus.Blocked)) return;
        var start = details.IndexOf('{');
        if (start < 0) return;
        try
        {
            using var json = JsonDocument.Parse(details[start..]);
            var root = json.RootElement;
            if (root.ValueKind != JsonValueKind.Object) return;
            long? native = root.TryGetProperty("hresult", out var code) && code.ValueKind == JsonValueKind.Number
                && code.TryGetInt64(out var value) && value is >= int.MinValue and <= int.MaxValue ? value : null;
            var locations = new List<string>();
            if (root.TryGetProperty("exceptionType", out var type) && type.ValueKind == JsonValueKind.String
                && LocalDiagnosticRecorder.Symbol(type.GetString())) locations.Add(type.GetString()!);
            if (root.TryGetProperty("script", out var script) && script.ValueKind == JsonValueKind.String)
            {
                var name = Path.GetFileName(script.GetString());
                if (LocalDiagnosticRecorder.Symbol(name))
                {
                    var line = root.TryGetProperty("line", out var number) && number.ValueKind == JsonValueKind.Number
                        && number.TryGetInt32(out var lineNumber) ? lineNumber : 0;
                    locations.Add(name + ":" + line);
                }
            }
            Record(progress.Stage.ToString(), progress.Status.ToString(), nativeCode: native, locations: locations.ToArray());
        }
        catch (JsonException) { /* Raw diagnostics are not copied into persistent support evidence. */ }
    }
    public static long CaptureFailures => recorder?.Failures ?? 0;
    public static long PendingDropped => recorder?.Dropped ?? 0;
    public static Task FlushAsync() => recorder?.FlushAsync() ?? Task.CompletedTask;
    public static void Stop() { Record("Process", "Stopped"); recorder?.Dispose(); }
}
