using System.IO.Pipes;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;

internal static partial class IpcCancellationScenario
{
    public static async Task DiscoveryRevalidationCancellationPreservesIdentityAndTransport()
    {
        using var safety = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        using var cancellation = new CancellationTokenSource();
        var name = $"discovery-cancellation-{Guid.NewGuid():N}";
        var selected = Guid.NewGuid();
        await using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
        var handshake = Task.Run(async () => {
            await server.WaitForConnectionAsync(safety.Token);
            await HandshakeAsync(server, safety.Token);
        });
        await using var client = await EngineIpcClient.ConnectAsync(name, new PeerHello(), "synthetic-bootstrap", cancellationToken: safety.Token);
        await handshake;
        var pending = client.DiscoverServersAsync(cancellation.Token, selected);
        using var scan = await ReadFrameAsync(server, safety.Token);
        Assert.Equal(IpcClientMessage.IpcDiscoverServersKind, scan.RootElement.GetProperty("kind").GetString()!, "typed discovery request");
        var payload = scan.RootElement.GetProperty("payload");
        Assert.Equal(selected, payload.GetProperty("selectedServerId").GetGuid(), "Rust receives the selected identity");
        cancellation.Cancel();
        using var cancel = await ReadFrameAsync(server, safety.Token);
        Assert.Equal(IpcClientMessage.IpcCancelServerDiscoveryKind, cancel.RootElement.GetProperty("kind").GetString()!, "typed discovery cancellation");
        Assert.Equal(payload.GetRawText(), cancel.RootElement.GetProperty("payload").GetRawText(), "cancellation retains the original IDs and selection");
        try { await pending; throw new InvalidOperationException("Cancellation was expected."); }
        catch (OperationCanceledException) { }
        await ReplyAsync(server, IpcServerMessage.IpcServerDiscoveryKind, new ServerDiscoveryResult {
            RequestId = payload.GetProperty("requestId").GetGuid(), CorrelationId = payload.GetProperty("correlationId").GetGuid(),
            Status = DiscoveryStatus.Cancelled, Servers = [] }, safety.Token);
        var next = client.DiscoverServersAsync(safety.Token, selected);
        using var retry = await ReadFrameAsync(server, safety.Token);
        var retryPayload = retry.RootElement.GetProperty("payload");
        await ReplyAsync(server, IpcServerMessage.IpcServerDiscoveryKind, new ServerDiscoveryResult {
            RequestId = retryPayload.GetProperty("requestId").GetGuid(), CorrelationId = retryPayload.GetProperty("correlationId").GetGuid(),
            Status = DiscoveryStatus.ServerMissing, Servers = [] }, safety.Token);
        Assert.Equal(DiscoveryStatus.ServerMissing, (await next).Status, "cancelled scan does not poison the next request");
        Assert.False(client.Completion.IsCompleted, "the transport remains connected");
    }
}
