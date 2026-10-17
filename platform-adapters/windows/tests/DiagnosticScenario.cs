using System.Diagnostics;
using System.Text;
using System.Text.Json;
using Eitmad.Platform.Windows.LocalIpc;

internal static partial class IpcCancellationScenario
{
    public static async Task DiagnosticsRetainCorrelationAndRedactPayloads()
    {
        var previous = Environment.GetEnvironmentVariable("EITMAD_DIAGNOSTICS");
        var stderr = Console.Error;
        using var buffer = new StringWriter();
        var capture = TextWriter.Synchronized(buffer);
        Environment.SetEnvironmentVariable("EITMAD_DIAGNOSTICS", "1");
        Console.SetError(capture);
        try
        {
            using var safety = new CancellationTokenSource(TimeSpan.FromSeconds(5));
            var name = $"eitmad-diagnostic-{Guid.NewGuid():N}";
            await using var server = new System.IO.Pipes.NamedPipeServerStream(name, System.IO.Pipes.PipeDirection.InOut, 1,
                System.IO.Pipes.PipeTransmissionMode.Byte, System.IO.Pipes.PipeOptions.Asynchronous);
            var accepting = AcceptAsync(server, safety.Token);
            await using var client = await EngineIpcClient.ConnectAsync(name, new(), "synthetic-bootstrap", cancellationToken: safety.Token);
            await accepting;
            var request = Request("private-synthetic-customer-value");
            var correlation = request.CorrelationId;
            using (var activation = new OperationDiagnostics(DiagnosticPhase.Activation))
            {
                var waiting = client.SendQueryAsync(request, TimeSpan.FromMilliseconds(100));
                using var frame = await ReadFrameAsync(server, safety.Token);
                Assert.Equal(correlation, frame.RootElement.GetProperty("payload").GetProperty("correlationId").GetGuid(), "wire correlation is unchanged");
                await ExpectFailureAsync(waiting, EngineIpcFailureKind.Timeout);
                activation.Finish();
            }
            var forwarded = JsonSerializer.Serialize(new { diagnostic = "eitmad.operation.v1", correlation,
                source = "Approvals", phase = "CredentialAdmission", result = "GateBusy", elapsedMs = 0, token = "never-forward-this-token" });
            Assert.True(OperationDiagnostics.ForwardEngineLine(forwarded), "safe engine metadata accepted");
            Assert.False(OperationDiagnostics.ForwardEngineLine(forwarded.Replace("GateBusy", "never-forward-this-token")), "unrecognized metadata rejected");
            string text;
            do
            {
                await Task.Delay(5, safety.Token);
                lock (capture) text = buffer.ToString();
            } while (!text.Contains("CredentialAdmission", StringComparison.Ordinal));
            Assert.False(text.Contains("private-synthetic-customer-value", StringComparison.Ordinal), "customer value redacted");
            Assert.False(text.Contains("synthetic-bootstrap", StringComparison.Ordinal), "bootstrap redacted");
            Assert.False(text.Contains("never-forward-this-token", StringComparison.Ordinal), "engine extra fields and invalid result redacted");
            var records = text.Split('\n', StringSplitOptions.RemoveEmptyEntries).Select(line => JsonDocument.Parse(line)).ToArray();
            try
            {
                var related = records.Where(r => r.RootElement.GetProperty("correlation").GetGuid() == correlation
                    && r.RootElement.GetProperty("phase").GetString() != "CredentialAdmission").ToArray();
                Assert.True(related.Any(r => r.RootElement.GetProperty("phase").GetString() == "IpcReply"
                    && r.RootElement.GetProperty("result").GetString() == "Deadline"), "specific reply deadline is retained");
                var operation = related[0].RootElement.GetProperty("operation").GetGuid();
                Assert.True(related.All(r => r.RootElement.GetProperty("operation").GetGuid() == operation), "lock/send/reply keep activation operation");
                Assert.True(records.Any(r => r.RootElement.GetProperty("phase").GetString() == "Activation"
                    && r.RootElement.GetProperty("operation").GetGuid() == operation), "IPC joins activation");
            }
            finally { foreach (var record in records) record.Dispose(); }
        }
        finally { Console.SetError(stderr); Environment.SetEnvironmentVariable("EITMAD_DIAGNOSTICS", previous); }
    }

    public static async Task DiagnosticQueueNeverWaitsForBlockedStderr()
    {
        using var sink = new BlockedWriter();
        var output = OperationDiagnostics.CreateOutput(sink);
        Assert.True(output.Writer.TryWrite("synthetic"), "first item admitted");
        await sink.Entered.Task.WaitAsync(TimeSpan.FromSeconds(2));
        var clock = Stopwatch.StartNew();
        var dropped = 0;
        for (var i = 0; i < 1024; i++) if (!output.Writer.TryWrite("synthetic")) dropped++;
        Assert.True(dropped > 0 && clock.Elapsed < TimeSpan.FromSeconds(1), "full output drops without waiting for stderr");
        sink.Release.TrySetResult();
        output.Writer.Complete();
    }

    private sealed class BlockedWriter : TextWriter
    {
        public override Encoding Encoding => Encoding.UTF8;
        public TaskCompletionSource Entered { get; } = new(TaskCreationOptions.RunContinuationsAsynchronously);
        public TaskCompletionSource Release { get; } = new(TaskCreationOptions.RunContinuationsAsynchronously);
        public override Task WriteLineAsync(string? value) { Entered.TrySetResult(); return Release.Task; }
    }
}
