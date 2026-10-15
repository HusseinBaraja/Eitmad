using System.Buffers.Binary;
using System.IO.Pipes;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;

internal static class IpcCancellationScenario
{
    public static async Task CancelledWritePreservesTheNextFrame()
    {
        var name = $"eitmad-cancellation-{Guid.NewGuid():N}";
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        using var requestCancellation = new CancellationTokenSource();
        await using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous, 4096, 4096);
        var serve = ServeAsync(server, requestCancellation, deadline.Token);
        await using var client = await EngineIpcClient.ConnectAsync(name, new PeerHello(), "synthetic-bootstrap", cancellationToken: deadline.Token);
        var cancelled = client.SendQueryAsync(Request(new string('x', 1024 * 1024)), cancellationToken: requestCancellation.Token);
        await Assert.ThrowsAsync<OperationCanceledException>(() => cancelled, "cancelled caller still observes cancellation");
        var next = await client.SendQueryAsync(Request("next"), cancellationToken: deadline.Token);
        Assert.Equal(CommandOutcomeStatus.Succeeded, next.Outcome.Status, "next IPC request remains readable");
        await serve;
    }

    private static QueryEnvelope Request(string value) => new()
    {
        RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Authorization = new(), ProtocolVersion = new(),
        Query = new() { ["kind"] = "synthetic", ["value"] = value },
    };

    private static async Task ServeAsync(NamedPipeServerStream server, CancellationTokenSource cancelled, CancellationToken token)
    {
        await server.WaitForConnectionAsync(token);
        using var hello = await ReadFrameAsync(server, token);
        await ReplyAsync(server, IpcServerMessage.IpcHandshakeResponseKind, new HandshakeResponse
        {
            RequestId = hello.RootElement.GetProperty("payload").GetProperty("requestId").GetGuid(),
            Outcome = new() { Status = HandshakeOutcomeStatus.Accepted, Payload = new()
            {
                Authorization = new() { SessionId = Guid.NewGuid() },
                Negotiated = new() { Protocol = new() { Major = 1 }, Capabilities = [], Schemas = [] },
            } },
        }, token);
        var header = new byte[4];
        await server.ReadExactlyAsync(header, token);
        // Cancel only after the frame has started, while its large body is under backpressure.
        cancelled.Cancel();
        await Task.Delay(100, token);
        var body = new byte[BinaryPrimitives.ReadUInt32LittleEndian(header)];
        await server.ReadExactlyAsync(body, token);
        using var first = JsonDocument.Parse(body);
        Assert.Equal(1024 * 1024, first.RootElement.GetProperty("payload").GetProperty("query").GetProperty("value").GetString()!.Length,
            "cancelled frame body is complete");
        using var next = await ReadFrameAsync(server, token);
        await ReplyAsync(server, IpcServerMessage.IpcQueryResponseKind, new QueryResponseEnvelope
        {
            RequestId = next.RootElement.GetProperty("payload").GetProperty("requestId").GetGuid(),
            Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = new() },
        }, token);
    }

    private static async Task<JsonDocument> ReadFrameAsync(Stream stream, CancellationToken token)
    {
        var header = new byte[4];
        await stream.ReadExactlyAsync(header, token);
        var body = new byte[BinaryPrimitives.ReadUInt32LittleEndian(header)];
        await stream.ReadExactlyAsync(body, token);
        return JsonDocument.Parse(body);
    }

    private static async Task ReplyAsync(Stream stream, string kind, object payload, CancellationToken token)
    {
        var body = JsonSerializer.SerializeToUtf8Bytes(new { kind, payload }, Converter.Settings);
        var header = new byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(header, (uint)body.Length);
        await stream.WriteAsync(header, token);
        await stream.WriteAsync(body, token);
    }
}
