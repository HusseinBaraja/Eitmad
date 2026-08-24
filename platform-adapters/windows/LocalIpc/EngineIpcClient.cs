using System.Buffers.Binary;
using System.Collections.Concurrent;
using System.IO.Pipes;
using System.Text.Json;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.LocalIpc;

public sealed class EngineIpcClient : IAsyncDisposable
{
    public const int MaximumFrameBytes = 8 * 1024 * 1024;
    public static TimeSpan DefaultConnectTimeout { get; } = TimeSpan.FromSeconds(5);
    public static TimeSpan DefaultRequestTimeout { get; } = TimeSpan.FromSeconds(30);

    private readonly NamedPipeClientStream pipe;
    private readonly SemaphoreSlim writeLock = new(1, 1);
    private readonly ConcurrentDictionary<Guid, TaskCompletionSource<IpcServerMessage>> pending = new();
    private readonly ConcurrentDictionary<Guid, EngineSubscription> subscriptions = new();
    private readonly ConcurrentDictionary<Guid, ConcurrentQueue<EventEnvelope>> earlySubscriptionEvents = new();
    private readonly CancellationTokenSource lifetime = new();
    private readonly Task reader;
    private bool disposed;

    private EngineIpcClient(NamedPipeClientStream pipe)
    {
        this.pipe = pipe;
        reader = ReadResponsesAsync();
    }

    public AuthorizationContext Authorization { get; private set; } = null!;
    public NegotiatedSession NegotiatedSession { get; private set; } = null!;
    public Task Completion => reader;

    public static async Task<EngineIpcClient> ConnectAsync(
        string pipeName,
        PeerHello peer,
        DevelopmentIdentityAssertion assertedAuthorization,
        string developmentBearerToken,
        TimeSpan? timeout = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(pipeName);
        ArgumentNullException.ThrowIfNull(peer);
        ArgumentNullException.ThrowIfNull(assertedAuthorization);
        ArgumentException.ThrowIfNullOrWhiteSpace(developmentBearerToken);

        var pipe = new NamedPipeClientStream(
            ".",
            pipeName,
            PipeDirection.InOut,
            PipeOptions.Asynchronous);
        using var connectCancellation = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        connectCancellation.CancelAfter(timeout ?? DefaultConnectTimeout);
        try
        {
            await pipe.ConnectAsync(connectCancellation.Token).ConfigureAwait(false);
        }
        catch (OperationCanceledException error) when (!cancellationToken.IsCancellationRequested)
        {
            pipe.Dispose();
            throw new EngineIpcException(
                EngineIpcFailureKind.EngineUnavailable,
                "The Rust engine IPC endpoint was unavailable before the connection deadline.",
                innerException: error);
        }
        catch (IOException error)
        {
            pipe.Dispose();
            throw new EngineIpcException(
                EngineIpcFailureKind.EngineUnavailable,
                "The Rust engine IPC endpoint is unavailable.",
                innerException: error);
        }

        var client = new EngineIpcClient(pipe);
        try
        {
            var requestId = Guid.NewGuid();
            var response = await client.SendFrameAsync(
                IpcClientMessage.IpcHandshakeKind,
                new HandshakeRequest
                {
                    RequestId = requestId,
                    CorrelationId = Guid.NewGuid(),
                    Peer = peer,
                    DevelopmentBearerToken = developmentBearerToken,
                    AssertedAuthorization = assertedAuthorization,
                },
                requestId,
                DefaultRequestTimeout,
                cancellationToken,
                commandOutcomeUnknown: false).ConfigureAwait(false);

            var handshake = response.AsIpcHandshakeResponse();
            if (handshake?.Outcome?.Payload is null)
            {
                throw ProtocolViolation("The engine returned an invalid handshake response.");
            }

            if (handshake.Outcome.Status != NegotiationOutcomeStatus.Accepted)
            {
                var kind = handshake.Outcome.Payload.Kind == FluffyKind.Negotiation
                    ? EngineIpcFailureKind.VersionMismatch
                    : EngineIpcFailureKind.AuthenticationRejected;
                throw new EngineIpcException(kind, "The engine rejected the local IPC handshake.");
            }

            client.Authorization = handshake.Outcome.Payload.Authorization
                ?? throw ProtocolViolation("The engine omitted the negotiated authorization session.");
            client.NegotiatedSession = handshake.Outcome.Payload.Negotiated
                ?? throw ProtocolViolation("The engine omitted protocol negotiation details.");
            return client;
        }
        catch
        {
            await client.DisposeAsync().ConfigureAwait(false);
            throw;
        }
    }

    public async Task<CommandResponseEnvelope> SendCommandAsync(
        CommandEnvelope request,
        TimeSpan? timeout = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        ApplySession(request.Authorization, request.ProtocolVersion);
        var response = await SendFrameAsync(
            IpcClientMessage.IpcCommandKind,
            request,
            request.RequestId,
            timeout ?? DefaultRequestTimeout,
            cancellationToken,
            commandOutcomeUnknown: true).ConfigureAwait(false);
        EnsureKind(response, IpcServerMessage.IpcCommandResponseKind);
        return response.AsIpcCommandResponse()
            ?? throw ProtocolViolation("The engine returned an invalid command response.");
    }

    public async Task<QueryResponseEnvelope> SendQueryAsync(
        QueryEnvelope request,
        TimeSpan? timeout = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        ApplySession(request.Authorization, request.ProtocolVersion);
        var response = await SendFrameAsync(
            IpcClientMessage.IpcQueryKind,
            request,
            request.RequestId,
            timeout ?? DefaultRequestTimeout,
            cancellationToken,
            commandOutcomeUnknown: false).ConfigureAwait(false);
        EnsureKind(response, IpcServerMessage.IpcQueryResponseKind);
        return response.AsIpcQueryResponse()
            ?? throw ProtocolViolation("The engine returned an invalid query response.");
    }

    public async Task<EngineSubscription> SubscribeAsync(
        SubscriptionEnvelope request,
        TimeSpan? timeout = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        ApplySession(request.Authorization, request.ProtocolVersion);
        var earlyEvents = new ConcurrentQueue<EventEnvelope>();
        if (!earlySubscriptionEvents.TryAdd(request.CorrelationId, earlyEvents))
        {
            throw ProtocolViolation("A subscription with the same correlation identifier is pending.");
        }
        try
        {
            var response = await SendFrameAsync(
                IpcClientMessage.IpcSubscribeKind,
                request,
                request.RequestId,
                timeout ?? DefaultRequestTimeout,
                cancellationToken,
                commandOutcomeUnknown: false).ConfigureAwait(false);
            EnsureKind(response, IpcServerMessage.IpcSubscribeResponseKind);
            var subscribeResponse = response.AsIpcSubscribeResponse()
                ?? throw ProtocolViolation("The engine returned an invalid subscription response.");
            var outcome = subscribeResponse.Outcome;
            if (outcome.Status != CommandOutcomeStatus.Succeeded)
            {
                var error = ConvertPayload<ContractError>(outcome.Payload);
                var kind = error.Code == ProtocolIds.ErrorCodes.EitmadErrorIpcSubscriptionResyncRequiredV1
                    ? EngineIpcFailureKind.ResyncRequired
                    : EngineIpcFailureKind.SubscriptionUnsupported;
                throw new EngineIpcException(kind, "The engine rejected the subscription.", error);
            }

            var accepted = outcome.Payload;
            if (accepted?.SubscriptionId is not { } subscriptionId
                || accepted.StreamCursor is not { } streamCursor
                || accepted.Resumed is not { } resumed)
            {
                throw ProtocolViolation("The engine returned an incomplete subscription acknowledgement.");
            }

            var subscription = new EngineSubscription(subscriptionId, streamCursor, resumed);
            if (!subscriptions.TryAdd(subscriptionId, subscription))
            {
                throw ProtocolViolation("The engine reused an active subscription identifier.");
            }
            while (earlyEvents.TryDequeue(out var delivered))
            {
                if (delivered.SubscriptionId != subscriptionId || !subscription.TryPublish(delivered))
                {
                    throw ProtocolViolation("The engine emitted an invalid early subscription event.");
                }
            }

            return subscription;
        }
        finally
        {
            earlySubscriptionEvents.TryRemove(request.CorrelationId, out _);
        }
    }

    public async Task UnsubscribeAsync(
        EngineSubscription subscription,
        TimeSpan? timeout = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(subscription);
        var requestId = Guid.NewGuid();
        var response = await SendFrameAsync(
            IpcClientMessage.IpcUnsubscribeKind,
            new UnsubscribeRequest
            {
                RequestId = requestId,
                CorrelationId = Guid.NewGuid(),
                SubscriptionId = subscription.SubscriptionId,
            },
            requestId,
            timeout ?? DefaultRequestTimeout,
            cancellationToken,
            commandOutcomeUnknown: false).ConfigureAwait(false);
        EnsureKind(response, IpcServerMessage.IpcUnsubscribeResponseKind);
        if (response.AsIpcUnsubscribeResponse()?.Accepted != true)
        {
            throw ProtocolViolation("The engine did not recognize the active subscription.");
        }

        subscriptions.TryRemove(subscription.SubscriptionId, out _);
        subscription.Complete();
    }

    public async Task RequestShutdownAsync(
        TimeSpan? timeout = null,
        CancellationToken cancellationToken = default)
    {
        var requestId = Guid.NewGuid();
        var response = await SendFrameAsync(
            IpcClientMessage.IpcShutdownKind,
            new ShutdownRequest
            {
                RequestId = requestId,
                CorrelationId = Guid.NewGuid(),
            },
            requestId,
            timeout ?? DefaultRequestTimeout,
            cancellationToken,
            commandOutcomeUnknown: false).ConfigureAwait(false);
        EnsureKind(response, IpcServerMessage.IpcShutdownResponseKind);
        if (response.AsIpcShutdownResponse()?.Accepted != true)
        {
            throw new EngineIpcException(
                EngineIpcFailureKind.EngineStopping,
                "The engine did not accept the shutdown request.");
        }
    }

    private async Task<IpcServerMessage> SendFrameAsync(
        string kind,
        object payload,
        Guid requestId,
        TimeSpan timeout,
        CancellationToken cancellationToken,
        bool commandOutcomeUnknown)
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        if (timeout <= TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(timeout));
        }

        var completion = new TaskCompletionSource<IpcServerMessage>(
            TaskCreationOptions.RunContinuationsAsynchronously);
        if (!pending.TryAdd(requestId, completion))
        {
            throw ProtocolViolation("A request with the same identifier is already pending.");
        }

        try
        {
            var message = new IpcClientMessage { Kind = kind, Payload = payload };
            var bytes = JsonSerializer.SerializeToUtf8Bytes(message, Converter.Settings);
            if (bytes.Length > MaximumFrameBytes)
            {
                throw new EngineIpcException(
                    EngineIpcFailureKind.PayloadTooLarge,
                    $"The IPC payload exceeds {MaximumFrameBytes} bytes.");
            }

            await writeLock.WaitAsync(cancellationToken).ConfigureAwait(false);
            try
            {
                var header = new byte[sizeof(uint)];
                BinaryPrimitives.WriteUInt32LittleEndian(header, (uint)bytes.Length);
                await pipe.WriteAsync(header, cancellationToken).ConfigureAwait(false);
                await pipe.WriteAsync(bytes, cancellationToken).ConfigureAwait(false);
                await pipe.FlushAsync(cancellationToken).ConfigureAwait(false);
            }
            finally
            {
                writeLock.Release();
            }

            using var requestCancellation = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            requestCancellation.CancelAfter(timeout);
            try
            {
                return await completion.Task.WaitAsync(requestCancellation.Token).ConfigureAwait(false);
            }
            catch (OperationCanceledException error) when (!cancellationToken.IsCancellationRequested)
            {
                throw new EngineIpcException(
                    commandOutcomeUnknown
                        ? EngineIpcFailureKind.CommandOutcomeUnknown
                        : EngineIpcFailureKind.Timeout,
                    commandOutcomeUnknown
                        ? "The command deadline elapsed; its final outcome is unknown. Retry only with the same idempotency key."
                        : "The IPC request deadline elapsed.",
                    innerException: error);
            }
        }
        finally
        {
            pending.TryRemove(requestId, out _);
        }
    }

    private async Task ReadResponsesAsync()
    {
        try
        {
            var header = new byte[sizeof(uint)];
            while (!lifetime.IsCancellationRequested)
            {
                await pipe.ReadExactlyAsync(header, lifetime.Token).ConfigureAwait(false);
                var length = BinaryPrimitives.ReadUInt32LittleEndian(header);
                if (length > MaximumFrameBytes)
                {
                    throw ProtocolViolation("The engine returned an oversized IPC frame.");
                }

                var payload = new byte[length];
                await pipe.ReadExactlyAsync(payload, lifetime.Token).ConfigureAwait(false);
                var message = JsonSerializer.Deserialize<IpcServerMessage>(payload, Converter.Settings)
                    ?? throw ProtocolViolation("The engine returned an empty IPC response.");
                if (message.Kind == IpcServerMessage.IpcEventKind)
                {
                    var delivered = message.AsIpcEvent()
                        ?? throw ProtocolViolation("The engine emitted an invalid event frame.");
                    if (!subscriptions.TryGetValue(delivered.SubscriptionId, out var subscription))
                    {
                        if (earlySubscriptionEvents.TryGetValue(delivered.CorrelationId, out var earlyEvents)
                            && earlyEvents.Count < EngineSubscription.Capacity)
                        {
                            earlyEvents.Enqueue(delivered);
                            continue;
                        }
                        throw ProtocolViolation("The engine emitted an event for an unknown subscription.");
                    }
                    if (!subscription.TryPublish(delivered))
                    {
                        throw new EngineIpcException(
                            EngineIpcFailureKind.SubscriptionBackpressure,
                            "The shell event consumer exceeded its bounded subscription queue.");
                    }
                    continue;
                }
                if (message.Kind == IpcServerMessage.IpcSubscriptionClosedKind)
                {
                    var closed = message.AsIpcSubscriptionClosed();
                    var error = new EngineIpcException(
                        EngineIpcFailureKind.SubscriptionBackpressure,
                        "The engine closed a subscription because replay could not preserve a discrete event.");
                    if (closed?.SubscriptionId is { } subscriptionId
                        && subscriptions.TryRemove(subscriptionId, out var subscription))
                    {
                        subscription.Complete(error);
                    }
                    throw error;
                }
                var requestId = RequestIdOf(message);
                if (requestId is { } id && pending.TryRemove(id, out var completion))
                {
                    completion.TrySetResult(message);
                }
                else if (message.Kind == IpcServerMessage.IpcFailureKind)
                {
                    FailPending(new EngineIpcException(
                        EngineIpcFailureKind.ProtocolViolation,
                        "The engine returned an uncorrelated structured IPC failure.",
                        message.AsIpcFailure()?.Error));
                }
            }
        }
        catch (OperationCanceledException) when (lifetime.IsCancellationRequested)
        {
        }
        catch (Exception error) when (error is IOException or EndOfStreamException or JsonException)
        {
            FailPending(new EngineIpcException(
                EngineIpcFailureKind.ConnectionLost,
                "The engine IPC connection closed before pending requests completed.",
                innerException: error));
        }
        catch (EngineIpcException error)
        {
            FailPending(error);
        }
    }

    private void ApplySession(AuthorizationContext authorization, ProtocolVersion protocol)
    {
        ArgumentNullException.ThrowIfNull(authorization);
        authorization.SessionId = Authorization.SessionId;
        protocol.Major = NegotiatedSession.Protocol.Major;
        protocol.Minor = NegotiatedSession.Protocol.Minor;
    }

    private static void EnsureKind(IpcServerMessage response, string expected)
    {
        if (response.Kind != expected)
        {
            throw ProtocolViolation("The engine response kind did not match the request.");
        }
    }

    private static Guid? RequestIdOf(IpcServerMessage message) => message.Kind switch
    {
        IpcServerMessage.IpcHandshakeResponseKind => message.AsIpcHandshakeResponse()?.RequestId,
        IpcServerMessage.IpcCommandResponseKind => message.AsIpcCommandResponse()?.RequestId,
        IpcServerMessage.IpcQueryResponseKind => message.AsIpcQueryResponse()?.RequestId,
        IpcServerMessage.IpcSubscribeResponseKind => message.AsIpcSubscribeResponse()?.RequestId,
        IpcServerMessage.IpcUnsubscribeResponseKind => message.AsIpcUnsubscribeResponse()?.RequestId,
        IpcServerMessage.IpcShutdownResponseKind => message.AsIpcShutdownResponse()?.RequestId,
        _ => null,
    };

    private static T ConvertPayload<T>(object value)
    {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(value, Converter.Settings);
        return JsonSerializer.Deserialize<T>(bytes, Converter.Settings)
            ?? throw ProtocolViolation("A generated IPC payload could not be converted.");
    }

    private void FailPending(EngineIpcException error)
    {
        foreach (var pair in pending)
        {
            if (pending.TryRemove(pair.Key, out var completion))
            {
                completion.TrySetException(error);
            }
        }
        foreach (var pair in subscriptions)
        {
            if (subscriptions.TryRemove(pair.Key, out var subscription))
            {
                subscription.Complete(error);
            }
        }
    }

    private static EngineIpcException ProtocolViolation(string message) =>
        new(EngineIpcFailureKind.ProtocolViolation, message);

    public async ValueTask DisposeAsync()
    {
        if (disposed)
        {
            return;
        }

        disposed = true;
        lifetime.Cancel();
        pipe.Dispose();
        try
        {
            await reader.ConfigureAwait(false);
        }
        catch (ObjectDisposedException)
        {
        }
        lifetime.Dispose();
        writeLock.Dispose();
        foreach (var subscription in subscriptions.Values)
        {
            subscription.Complete();
        }
        subscriptions.Clear();
    }
}
