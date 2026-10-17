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
    private readonly SemaphoreSlim sessionChangeLock = new(1, 1);
    private readonly ConcurrentDictionary<Guid, TaskCompletionSource<IpcServerMessage>> pending = new();
    private readonly ConcurrentDictionary<Guid, EngineSubscription> subscriptions = new();
    private readonly ConcurrentDictionary<Guid, ConcurrentQueue<EventEnvelope>> earlySubscriptionEvents = new();
    private readonly object subscriptionHandoff = new();
    private readonly object transportGate = new();
    private readonly CancellationTokenSource lifetime = new();
    private readonly Task reader;
    private bool disposed;
    private EngineIpcException? transportFailure;
    private AuthorizationContext processAuthorization = null!;

    private EngineIpcClient(NamedPipeClientStream pipe)
    {
        this.pipe = pipe;
        reader = ReadResponsesAsync();
    }

    public AuthorizationContext Authorization { get; private set; } = null!;
    private AuthorizationContext? customerAuthorization;
    private DesktopAccountRole? accountRole;
    public AuthorizationContext AuthorizationForApprovals => accountRole == DesktopAccountRole.Receptionist ? AuthorizationForCustomer : Authorization;
    public AuthorizationContext AuthorizationForCustomer => customerAuthorization ?? Authorization;
    public NegotiatedSession NegotiatedSession { get; private set; } = null!;
    public Task Completion => reader;

    public static async Task<EngineIpcClient> ConnectAsync(
        string pipeName,
        PeerHello peer,
        string bootstrapToken,
        TimeSpan? timeout = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(pipeName);
        ArgumentNullException.ThrowIfNull(peer);
        ArgumentException.ThrowIfNullOrWhiteSpace(bootstrapToken);

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
                    BootstrapToken = bootstrapToken,
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

            if (handshake.Outcome.Status != HandshakeOutcomeStatus.Accepted)
            {
                var kind = handshake.Outcome.Payload.Payload is not null
                    ? EngineIpcFailureKind.VersionMismatch
                    : EngineIpcFailureKind.AuthenticationRejected;
                throw new EngineIpcException(kind, "The engine rejected the local IPC handshake.");
            }

            client.Authorization = handshake.Outcome.Payload.Authorization
                ?? throw ProtocolViolation("The engine omitted the negotiated authorization session.");
            client.processAuthorization = client.Authorization;
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

    public async Task<DesktopSessionState> SignInAsync(
        string username,
        string password,
        CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(username);
        ArgumentException.ThrowIfNullOrWhiteSpace(password);
        await sessionChangeLock.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            ClearSessionActivity();
            var requestId = Guid.NewGuid();
            var response = await SendFrameAsync(
                IpcClientMessage.IpcDesktopSignInKind,
                new DesktopSignInRequest { RequestId = requestId, CorrelationId = Guid.NewGuid(), Username = username, Password = password },
                requestId, DefaultRequestTimeout, cancellationToken, commandOutcomeUnknown: false).ConfigureAwait(false);
            EnsureKind(response, IpcServerMessage.IpcDesktopSessionResponseKind);
            var result = response.AsIpcDesktopSessionResponse()
                ?? throw ProtocolViolation("The engine returned an invalid desktop session response.");
            if (result.Status != DesktopSessionStatus.Active || result.State?.Authorization is null)
            {
                Authorization = processAuthorization;
                customerAuthorization = null;
                accountRole = null;
                throw new EngineIpcException(EngineIpcFailureKind.AuthenticationRejected,
                    "The engine rejected desktop sign-in.", result.Error);
            }
            Authorization = result.State.Authorization;
            customerAuthorization = result.State.CustomerAuthorization;
            accountRole = result.State.AccountRole;
            return result.State;
        }
        finally
        {
            sessionChangeLock.Release();
        }
    }

    public async Task<DesktopSessionState?> GetSessionStateAsync(CancellationToken cancellationToken = default)
    {
        var result = await SendDesktopSessionRequestAsync(IpcClientMessage.IpcDesktopSessionStateKind, cancellationToken)
            .ConfigureAwait(false);
        if (result.Status == DesktopSessionStatus.Failed)
            throw new EngineIpcException(EngineIpcFailureKind.AuthenticationRejected,
                "The engine could not read the desktop session.", result.Error);
        if (result.State?.Authorization is null)
        {
            ClearSessionActivity();
        }
        Authorization = result.State?.Authorization ?? processAuthorization;
        customerAuthorization = result.State?.CustomerAuthorization;
        accountRole = result.State?.AccountRole;
        return result.State;
    }

    public async Task SignOutAsync(CancellationToken cancellationToken = default)
    {
        await sessionChangeLock.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            ClearSessionActivity();
            var result = await SendDesktopSessionRequestAsync(IpcClientMessage.IpcDesktopSignOutKind, cancellationToken)
                .ConfigureAwait(false);
            if (result.Status != DesktopSessionStatus.SignedOut)
                throw new EngineIpcException(EngineIpcFailureKind.AuthenticationRejected,
                    "The engine could not close the desktop session.", result.Error);
            Authorization = processAuthorization;
            customerAuthorization = null;
            accountRole = null;
        }
        finally
        {
            sessionChangeLock.Release();
        }
    }

    internal void ClearSessionActivity()
    {
        var error = new EngineIpcException(
            EngineIpcFailureKind.SessionChanged,
            "The desktop session changed before the operation completed.");
        FailPending(error);
        lock (subscriptionHandoff)
        {
            earlySubscriptionEvents.Clear();
        }
    }

    private async Task<DesktopSessionResponse> SendDesktopSessionRequestAsync(string kind, CancellationToken cancellationToken)
    {
        var requestId = Guid.NewGuid();
        var response = await SendFrameAsync(kind,
            new DesktopSessionRequest { RequestId = requestId, CorrelationId = Guid.NewGuid() },
            requestId, DefaultRequestTimeout, cancellationToken, commandOutcomeUnknown: false).ConfigureAwait(false);
        EnsureKind(response, IpcServerMessage.IpcDesktopSessionResponseKind);
        return response.AsIpcDesktopSessionResponse()
            ?? throw ProtocolViolation("The engine returned an invalid desktop session response.");
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
            lock (subscriptionHandoff)
            {
                if (!subscriptions.TryAdd(subscriptionId, subscription))
                {
                    throw ProtocolViolation("The engine reused an active subscription identifier.");
                }
                try
                {
                    while (earlyEvents.TryDequeue(out var delivered))
                    {
                        if (delivered.SubscriptionId != subscriptionId || !subscription.TryPublish(delivered))
                            throw ProtocolViolation("The engine emitted an invalid early subscription event.");
                    }
                }
                catch
                {
                    subscriptions.TryRemove(subscriptionId, out _);
                    subscription.Complete();
                    throw;
                }
                earlySubscriptionEvents.TryRemove(request.CorrelationId, out _);
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
        if (timeout <= TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(timeout));
        }

        CancellationToken transportToken;
        lock (transportGate)
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            if (transportFailure is { } failure) throw failure;
            transportToken = lifetime.Token;
        }
        using var deadline = new CancellationTokenSource(timeout);
        using var requestCancellation = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, deadline.Token, transportToken);
        using var frameCancellation = CancellationTokenSource.CreateLinkedTokenSource(deadline.Token, transportToken);
        var frameStarted = false;

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

            await writeLock.WaitAsync(requestCancellation.Token).ConfigureAwait(false);
            try
            {
                requestCancellation.Token.ThrowIfCancellationRequested();
                // A session transition can invalidate this request while it waits for the writer.
                if (completion.Task.IsCompleted) return await completion.Task.ConfigureAwait(false);
                var header = new byte[sizeof(uint)];
                BinaryPrimitives.WriteUInt32LittleEndian(header, (uint)bytes.Length);
                // Caller cancellation cannot truncate a live frame. Its original deadline
                // still bounds transmission; an interrupted frame invalidates the transport.
                frameStarted = true;
                try
                {
                    await pipe.WriteAsync(header, frameCancellation.Token).ConfigureAwait(false);
                    await pipe.WriteAsync(bytes, frameCancellation.Token).ConfigureAwait(false);
                    await pipe.FlushAsync(frameCancellation.Token).ConfigureAwait(false);
                }
                catch
                {
                    InvalidateTransport(new EngineIpcException(EngineIpcFailureKind.ConnectionLost,
                        "The IPC transport closed because a request frame could not finish."));
                    throw;
                }
            }
            finally
            {
                writeLock.Release();
            }

            return await completion.Task.WaitAsync(requestCancellation.Token).ConfigureAwait(false);
        }
        catch (Exception error) when (error is OperationCanceledException or IOException or ObjectDisposedException or EngineIpcException)
        {
            if (commandOutcomeUnknown && frameStarted)
            {
                throw new EngineIpcException(
                    EngineIpcFailureKind.CommandOutcomeUnknown,
                    "The command response was not confirmed; its final outcome is unknown. Retry only with the same idempotency key.",
                    innerException: error);
            }
            cancellationToken.ThrowIfCancellationRequested();
            if (deadline.IsCancellationRequested)
                throw new EngineIpcException(EngineIpcFailureKind.Timeout,
                    "The IPC request deadline elapsed.", innerException: error);
            if (transportToken.IsCancellationRequested)
            {
                lock (transportGate) throw transportFailure!;
            }
            throw;
        }
        finally
        {
            pending.TryRemove(requestId, out _);
            // Observe a pending failure even when cancellation won the response wait.
            completion.TrySetCanceled();
            _ = completion.Task.Exception;
        }
    }

    /// <summary>Routes typed replies and subscription frames; session-changing closures invalidate pending consumers.</summary>
    private async Task ReadResponsesAsync()
    {
        var transportToken = lifetime.Token;
        try
        {
            var header = new byte[sizeof(uint)];
            while (!transportToken.IsCancellationRequested)
            {
                await pipe.ReadExactlyAsync(header, transportToken).ConfigureAwait(false);
                var length = BinaryPrimitives.ReadUInt32LittleEndian(header);
                if (length > MaximumFrameBytes)
                {
                    throw ProtocolViolation("The engine returned an oversized IPC frame.");
                }

                var payload = new byte[length];
                await pipe.ReadExactlyAsync(payload, transportToken).ConfigureAwait(false);
                var message = JsonSerializer.Deserialize<IpcServerMessage>(payload, Converter.Settings)
                    ?? throw ProtocolViolation("The engine returned an empty IPC response.");
                if (message.Kind == IpcServerMessage.IpcEventKind)
                {
                    var delivered = message.AsIpcEvent()
                        ?? throw ProtocolViolation("The engine emitted an invalid event frame.");
                    lock (subscriptionHandoff)
                    {
                        if (earlySubscriptionEvents.TryGetValue(delivered.CorrelationId, out var earlyEvents))
                        {
                            if (earlyEvents.Count >= EngineSubscription.Capacity)
                                throw ProtocolViolation("The engine exceeded the early subscription event capacity.");
                            earlyEvents.Enqueue(delivered);
                            continue;
                        }
                        if (!subscriptions.TryGetValue(delivered.SubscriptionId, out var subscription))
                            throw ProtocolViolation("The engine emitted an event for an unknown subscription.");
                        if (!subscription.TryPublish(delivered))
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
                        closed?.Reason switch
                        {
                            SubscriptionCloseReason.ProjectionInvalidated or SubscriptionCloseReason.AuthorizationRevoked => EngineIpcFailureKind.SessionChanged,
                            SubscriptionCloseReason.EngineStopping => EngineIpcFailureKind.EngineStopping,
                            _ => EngineIpcFailureKind.SubscriptionBackpressure,
                        },
                        "The engine closed the subscription; its projection may require a fresh query.");
                    if (closed?.SubscriptionId is { } subscriptionId
                        && subscriptions.TryRemove(subscriptionId, out var subscription))
                    {
                        subscription.Complete(error);
                    }
                    continue;
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
        catch (OperationCanceledException) when (transportToken.IsCancellationRequested)
        {
        }
        catch (ObjectDisposedException) when (transportToken.IsCancellationRequested)
        {
        }
        catch (Exception error) when (error is IOException or EndOfStreamException or JsonException)
        {
            InvalidateTransport(new EngineIpcException(
                EngineIpcFailureKind.ConnectionLost,
                "The engine IPC connection closed before pending requests completed.",
                innerException: error));
        }
        catch (EngineIpcException error)
        {
            InvalidateTransport(error);
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
        IpcServerMessage.IpcDesktopSessionResponseKind => message.AsIpcDesktopSessionResponse()?.RequestId,
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

    private void InvalidateTransport(EngineIpcException error)
    {
        lock (transportGate)
        {
            if (transportFailure is not null) return;
            transportFailure = error;
            FailPending(error);
            lifetime.Cancel();
            pipe.Dispose();
        }
    }

    public async ValueTask DisposeAsync()
    {
        lock (transportGate)
        {
            if (disposed) return;
            disposed = true;
        }

        InvalidateTransport(new EngineIpcException(EngineIpcFailureKind.ConnectionLost,
            "The engine IPC transport was disposed before pending requests completed."));
        try
        {
            await reader.ConfigureAwait(false);
        }
        catch (ObjectDisposedException)
        {
        }
        lifetime.Dispose();
        // In-flight calls can still release these managed semaphores after the reader exits.
        // No wait handles are created; let them be collected with this client.
        foreach (var subscription in subscriptions.Values)
        {
            subscription.Complete();
        }
        subscriptions.Clear();
    }
}
