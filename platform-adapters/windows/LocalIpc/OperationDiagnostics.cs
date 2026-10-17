using System.Diagnostics;
using System.Text.Json;
using System.Threading.Channels;

namespace Eitmad.Platform.Windows.LocalIpc;

public enum DiagnosticPhase { Activation, Home, Drafts, Orders, WorkOrders, Customers, Catalog, AccountSwitch, SignIn, IpcRequest, IpcLock, IpcSend, IpcReply, Worker, DriverAdmission, CredentialAdmission, CredentialRefresh, Dns, Connect, Tls, Send, Receive, Confirmation, Cleanup }
public enum DiagnosticSource { Unspecified, Quotations, Orders, WorkOrders, Approvals, Catalog, Pricing, Images }
public enum DiagnosticResult { Started, Succeeded, GateBusy, Poisoned, Deadline, Cancelled, Network, IoTimeout, Denied, Invalid, Server, Failed }

/// <summary>Opt-in bounded metadata only. Never accepts error messages or payload values.</summary>
public sealed class OperationDiagnostics : IDisposable
{
    private static readonly AsyncLocal<Guid?> Current = new();
    private static int events;
    private static readonly Channel<string> Output = CreateOutput();
    internal static Channel<string> CreateOutput(TextWriter? sink = null)
    {
        var channel = Channel.CreateBounded<string>(new BoundedChannelOptions(256) { FullMode = BoundedChannelFullMode.Wait, SingleReader = true });
        _ = Task.Run(async () =>
        {
            await foreach (var line in channel.Reader.ReadAllAsync().ConfigureAwait(false))
            {
                try { await (sink ?? Console.Error).WriteLineAsync(line).ConfigureAwait(false); }
                catch (IOException) { }
            }
        });
        return channel;
    }
    private readonly Guid? previous;
    private readonly Guid operation;
    private readonly Guid correlation;
    private readonly DiagnosticPhase phase;
    private readonly long started = Stopwatch.GetTimestamp();
    private bool disposed;
    private DiagnosticResult result = DiagnosticResult.Failed;

    public OperationDiagnostics(DiagnosticPhase phase, Guid correlation = default)
    {
        previous = Current.Value;
        operation = previous ?? Guid.NewGuid();
        Current.Value = operation;
        this.correlation = correlation == Guid.Empty ? operation : correlation;
        this.phase = phase;
        Write(operation, this.correlation, phase, DiagnosticResult.Started, 0);
    }

    public void Finish(DiagnosticResult result = DiagnosticResult.Succeeded) => this.result = result;
    public void Dispose()
    {
        if (disposed) return;
        disposed = true;
        Write(operation, correlation, phase, result, (long)Stopwatch.GetElapsedTime(started).TotalMilliseconds);
        Current.Value = previous;
    }

    private static void Write(Guid operation, Guid correlation, DiagnosticPhase phase, DiagnosticResult result, long elapsedMs, DiagnosticSource source = DiagnosticSource.Unspecified)
    {
        if (Environment.GetEnvironmentVariable("EITMAD_DIAGNOSTICS") != "1") return;
        var count = Volatile.Read(ref events);
        while (true)
        {
            if (count >= 4096) return;
            var observed = Interlocked.CompareExchange(ref events, count + 1, count);
            if (observed == count) break;
            count = observed;
        }
        try
        {
            Output.Writer.TryWrite(JsonSerializer.Serialize(new { diagnostic = "eitmad.operation.v1", operation, correlation, source = source.ToString(), phase = phase.ToString(), result = result.ToString(), elapsedMs }));
        }
        catch (IOException) { /* Diagnostic output cannot change product behavior. */ }
    }

    /// <summary>Re-emits only validated engine metadata; never forwards a raw stderr line.</summary>
    internal static bool ForwardEngineLine(string line)
    {
        if (line.Length > 1024) return false;
        try
        {
            using var json = JsonDocument.Parse(line);
            var root = json.RootElement;
            if (root.ValueKind != JsonValueKind.Object || !root.TryGetProperty("diagnostic", out var tag) || tag.GetString() != "eitmad.operation.v1") return false;
            if (!root.TryGetProperty("correlation", out var id) || !id.TryGetGuid(out var correlation)
                || !root.TryGetProperty("phase", out var p) || p.ValueKind != JsonValueKind.String || !Enum.TryParse<DiagnosticPhase>(p.GetString(), out var phase) || !Enum.IsDefined(phase)
                || !root.TryGetProperty("result", out var r) || r.ValueKind != JsonValueKind.String || !Enum.TryParse<DiagnosticResult>(r.GetString(), out var result) || !Enum.IsDefined(result)
                || !root.TryGetProperty("elapsedMs", out var ms) || !ms.TryGetInt64(out var elapsed) || elapsed < 0) return false;
            if (!root.TryGetProperty("source", out var sourceValue) || sourceValue.ValueKind != JsonValueKind.String
                || !Enum.TryParse<DiagnosticSource>(sourceValue.GetString(), out var source) || !Enum.IsDefined(source)) return false;
            Write(Guid.Empty, correlation, phase, result, elapsed, source);
            return true;
        }
        catch (Exception error) when (error is JsonException or InvalidOperationException or FormatException) { return false; }
    }

    public static async Task MeasureAsync(DiagnosticPhase phase, Func<Task> action, CancellationToken cancellationToken = default)
    {
        using var span = new OperationDiagnostics(phase);
        try
        {
            await action().ConfigureAwait(false);
            span.Finish(cancellationToken.IsCancellationRequested ? DiagnosticResult.Cancelled : DiagnosticResult.Succeeded);
        }
        catch (OperationCanceledException) { span.Finish(DiagnosticResult.Cancelled); throw; }
    }

    /// <summary>Retains the original typed error first when cleanup also fails.</summary>
    public static async Task RunWithCleanupAsync(Func<Task> action, Func<ValueTask> cleanup)
    {
        Exception? primary = null;
        try { await action().ConfigureAwait(false); }
        catch (Exception error) { primary = error; }
        try { await cleanup().ConfigureAwait(false); }
        catch (Exception secondary) when (primary is not null) { throw new AggregateException(primary, secondary); }
        if (primary is not null) System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(primary).Throw();
    }
}
