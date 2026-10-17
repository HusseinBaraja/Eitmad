using System.Buffers.Binary;
using System.IO.Pipes;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;

internal static class IpcCancellationScenario
{
    public static async Task StalledShutdownStillCleansUpProcess()
    {
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        await using var server = new NamedPipeServerStream("fake-1001", PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
        var fixture = new SupervisorFixture();
        fixture.Launcher.LifecycleOutput = JsonSerializer.Serialize(new LifecycleSnapshot
        {
            Identity = new() { InstanceId = Guid.NewGuid() }, State = LifecycleState.Ready,
            Ready = true, Live = true, Checks = [],
        }, Converter.Settings) + "\n";
        var serve = StallShutdownAsync(server, deadline.Token);
        await fixture.Supervisor.StartAsync(fixture.Request);
        while (!fixture.Supervisor.IpcConnected) await Task.Delay(5, deadline.Token);
        var process = fixture.Launcher.Current;
        var stop = fixture.Supervisor.StopAsync();
        await serve;
        process.Exit(0);
        await stop.WaitAsync(deadline.Token);
        Assert.True(process.InputClosed, "IPC timeout must still close the stdin lifetime pipe");
        Assert.True(process.Disposed, "IPC timeout must still dispose the owned process");
        Assert.Equal(EngineSupervisionState.Stopped, fixture.Supervisor.Snapshot.State, "IPC timeout cleanup reaches stopped");
        Assert.False(fixture.Group.Terminated, "stdin fallback allows a clean process exit");
        await fixture.Supervisor.DisposeAsync();
    }

    private static async Task StallShutdownAsync(NamedPipeServerStream server, CancellationToken token)
    {
        await server.WaitForConnectionAsync(token);
        await HandshakeAsync(server, token);
        using var shutdown = await ReadFrameAsync(server, token);
        Assert.Equal(IpcClientMessage.IpcShutdownKind, shutdown.RootElement.GetProperty("kind").GetString(), "typed shutdown request");
        Assert.Equal(0, await server.ReadAsync(new byte[1], token), "IPC shutdown deadline disposes the stalled transport");
    }

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

    public static async Task BlockedWriteExpiresAndReleasesQueuedRequests()
    {
        foreach (var command in new[] { false, true })
            foreach (var cancel in new[] { false, true })
                await BlockedWriteAsync(command, cancel);
    }

    private static async Task BlockedWriteAsync(bool command, bool cancel)
    {
        var name = $"eitmad-deadline-{Guid.NewGuid():N}";
        using var safety = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        using var cancellation = new CancellationTokenSource();
        using var queuedCancellation = new CancellationTokenSource();
        await using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous, 4096, 4096);
        var started = new TaskCompletionSource<int>(TaskCreationOptions.RunContinuationsAsynchronously);
        var stalled = StallAfterHeaderAsync(server, started, safety.Token);
        await using var client = await EngineIpcClient.ConnectAsync(name, new PeerHello(), "synthetic-bootstrap", cancellationToken: safety.Token);
        var payload = new string('x', 1024 * 1024);
        Task blocked = command
            ? client.SendCommandAsync(CommandRequest(payload), TimeSpan.FromMilliseconds(500), cancellation.Token)
            : client.SendQueryAsync(Request(payload), TimeSpan.FromMilliseconds(500), cancellation.Token);
        var length = await started.Task.WaitAsync(safety.Token);
        if (cancel) cancellation.Cancel();
        var queued = client.SendQueryAsync(Request("queued"), TimeSpan.FromSeconds(5));
        var queuedCommand = client.SendCommandAsync(CommandRequest("queued"), TimeSpan.FromSeconds(5));
        var expired = client.SendCommandAsync(CommandRequest("expired"), TimeSpan.FromMilliseconds(100));
        var cancelled = client.SendCommandAsync(CommandRequest("cancelled"), cancellationToken: queuedCancellation.Token);
        queuedCancellation.Cancel();
        await ExpectCancellationAsync(cancelled, queuedCancellation.Token);
        await ExpectFailureAsync(expired, EngineIpcFailureKind.Timeout);
        Assert.False(client.Completion.IsCompleted, "a queued timeout or cancellation must not close the active writer");
        if (command) await ExpectFailureAsync(blocked, EngineIpcFailureKind.CommandOutcomeUnknown);
        else if (cancel) await ExpectCancellationAsync(blocked, cancellation.Token);
        else await ExpectFailureAsync(blocked, EngineIpcFailureKind.Timeout);
        await ExpectFailureAsync(queued, EngineIpcFailureKind.ConnectionLost);
        await ExpectFailureAsync(queuedCommand, EngineIpcFailureKind.ConnectionLost);
        await client.Completion.WaitAsync(TimeSpan.FromSeconds(2));
        await stalled;
        await ExpectFailureAsync(client.SendQueryAsync(Request("closed")), EngineIpcFailureKind.ConnectionLost);
        // Drain only after all requests ended: the stopped reader must not help them finish.
        var received = 0;
        var buffer = new byte[4096];
        int count;
        while ((count = await server.ReadAsync(buffer, safety.Token)) != 0) received += count;
        Assert.True(received < length, "an unfinished frame must end at EOF, with no following live frame");
    }

    private static async Task StallAfterHeaderAsync(NamedPipeServerStream server, TaskCompletionSource<int> started, CancellationToken token)
    {
        await server.WaitForConnectionAsync(token);
        await HandshakeAsync(server, token);
        var header = new byte[4];
        await server.ReadExactlyAsync(header, token);
        started.SetResult((int)BinaryPrimitives.ReadUInt32LittleEndian(header));
        // Keep the connection open without consuming the body until the client has closed it.
    }

    public static async Task ReplyWaitUsesTheRemainingDeadline()
    {
        var name = $"eitmad-reply-deadline-{Guid.NewGuid():N}";
        using var safety = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        using var cancellation = new CancellationTokenSource();
        await using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous, 4096, 4096);
        var started = new TaskCompletionSource<int>(TaskCreationOptions.RunContinuationsAsynchronously);
        var handshake = StallAfterHeaderAsync(server, started, safety.Token);
        await using var client = await EngineIpcClient.ConnectAsync(name, new PeerHello(), "synthetic-bootstrap", cancellationToken: safety.Token);
        var first = client.SendQueryAsync(Request(new string('x', 1024 * 1024)), cancellationToken: cancellation.Token);
        var length = await started.Task.WaitAsync(safety.Token);
        cancellation.Cancel();
        var queued = client.SendQueryAsync(Request("reply"), TimeSpan.FromMilliseconds(700));
        await Task.Delay(450, safety.Token);
        await server.ReadExactlyAsync(new byte[length], safety.Token);
        using var request = await ReadFrameAsync(server, safety.Token);
        await ExpectCancellationAsync(first, cancellation.Token);
        // Only about 250 ms remain. A fresh 700 ms reply timeout fails this bound.
        await ExpectFailureAsync(queued.WaitAsync(TimeSpan.FromMilliseconds(500)), EngineIpcFailureKind.Timeout);
        Assert.False(client.Completion.IsCompleted, "reply timeout preserves a completely written stream");
        await handshake;
    }

    public static async Task UnconfirmedCommandsKeepUnknownOutcomeAndIdempotency()
    {
        var name = $"eitmad-command-outcome-{Guid.NewGuid():N}";
        using var safety = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        await using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
        var accept = AcceptAsync(server, safety.Token);
        await using var client = await EngineIpcClient.ConnectAsync(name, new PeerHello(), "synthetic-bootstrap", cancellationToken: safety.Token);
        await accept;
        var command = CommandRequest("unchanged");
        var timedOut = client.SendCommandAsync(command, TimeSpan.FromMilliseconds(100));
        using var sent = await ReadFrameAsync(server, safety.Token);
        await ExpectFailureAsync(timedOut, EngineIpcFailureKind.CommandOutcomeUnknown);
        using var cancellation = new CancellationTokenSource();
        command.RequestId = Guid.NewGuid();
        var cancelled = client.SendCommandAsync(command, cancellationToken: cancellation.Token);
        using var retry = await ReadFrameAsync(server, safety.Token);
        cancellation.Cancel();
        await ExpectFailureAsync(cancelled, EngineIpcFailureKind.CommandOutcomeUnknown);
        Assert.Equal(sent.RootElement.GetProperty("payload").GetProperty("idempotencyKey").GetGuid(),
            retry.RootElement.GetProperty("payload").GetProperty("idempotencyKey").GetGuid(), "exact retry retains idempotency key");
        Assert.True(sent.RootElement.GetProperty("payload").GetProperty("command").GetRawText()
            == retry.RootElement.GetProperty("payload").GetProperty("command").GetRawText(), "exact retry retains command payload");
        command.RequestId = Guid.NewGuid();
        var lost = client.SendCommandAsync(command);
        using var last = await ReadFrameAsync(server, safety.Token);
        server.Disconnect();
        await ExpectFailureAsync(lost, EngineIpcFailureKind.CommandOutcomeUnknown);
    }

    public static async Task SessionChangeDoesNotSendQueuedAccountRequests()
    {
        var name = $"eitmad-session-queue-{Guid.NewGuid():N}";
        using var safety = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        await using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous, 4096, 4096);
        var started = new TaskCompletionSource<int>(TaskCreationOptions.RunContinuationsAsynchronously);
        var handshake = StallAfterHeaderAsync(server, started, safety.Token);
        await using var client = await EngineIpcClient.ConnectAsync(name, new PeerHello(), "synthetic-bootstrap", cancellationToken: safety.Token);
        var first = client.SendCommandAsync(CommandRequest(new string('x', 1024 * 1024)), cancellationToken: safety.Token);
        var length = await started.Task.WaitAsync(safety.Token);
        var queued = client.SendQueryAsync(Request("old-account"), cancellationToken: safety.Token);
        var signOut = client.SignOutAsync(safety.Token);
        await server.ReadExactlyAsync(new byte[length], safety.Token);
        using var next = await ReadFrameAsync(server, safety.Token);
        Assert.Equal(IpcClientMessage.IpcDesktopSignOutKind, next.RootElement.GetProperty("kind").GetString(),
            "a queued prior-account request must not be transmitted after session invalidation");
        await ReplyAsync(server, IpcServerMessage.IpcDesktopSessionResponseKind, new DesktopSessionResponse
        {
            RequestId = next.RootElement.GetProperty("payload").GetProperty("requestId").GetGuid(),
            Status = DesktopSessionStatus.SignedOut,
        }, safety.Token);
        await ExpectFailureAsync(first, EngineIpcFailureKind.CommandOutcomeUnknown);
        await ExpectFailureAsync(queued, EngineIpcFailureKind.SessionChanged);
        await signOut;
        await handshake;
    }

    private static async Task AcceptAsync(NamedPipeServerStream server, CancellationToken token)
    {
        await server.WaitForConnectionAsync(token);
        await HandshakeAsync(server, token);
    }

    private static async Task ExpectCancellationAsync(Task request, CancellationToken token)
    {
        try
        {
            await request.WaitAsync(TimeSpan.FromSeconds(2));
        }
        catch (OperationCanceledException error)
        {
            Assert.Equal(token, error.CancellationToken, "caller cancellation retains its token");
            return;
        }
        throw new InvalidOperationException("Expected caller cancellation.");
    }

    private static async Task ExpectFailureAsync(Task request, EngineIpcFailureKind kind)
    {
        try
        {
            await request.WaitAsync(TimeSpan.FromSeconds(2));
        }
        catch (EngineIpcException error)
        {
            Assert.Equal(kind, error.Kind, "IPC failure classification");
            return;
        }
        throw new InvalidOperationException($"Expected IPC failure {kind}.");
    }

    private static QueryEnvelope Request(string value) => new()
    {
        RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Authorization = new(), ProtocolVersion = new(),
        Query = new() { ["kind"] = "synthetic", ["value"] = value },
    };

    private static CommandEnvelope CommandRequest(string value) => new()
    {
        RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Authorization = new(), ProtocolVersion = new(),
        IdempotencyKey = Guid.NewGuid(), Command = new() { ["kind"] = "synthetic", ["value"] = value },
    };

    private static async Task ServeAsync(NamedPipeServerStream server, CancellationTokenSource cancelled, CancellationToken token)
    {
        await server.WaitForConnectionAsync(token);
        await HandshakeAsync(server, token);
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

    private static async Task HandshakeAsync(NamedPipeServerStream server, CancellationToken token)
    {
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
