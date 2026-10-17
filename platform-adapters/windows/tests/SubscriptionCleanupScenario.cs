using System.IO.Pipes;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;

internal static partial class IpcCancellationScenario
{
    public static async Task SubscriptionDeadlineIncludesSessionAdmission()
    {
        using var safety = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        var token = safety.Token;
        var name = $"eitmad-subscribe-deadline-{Guid.NewGuid():N}";
        await using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
        var accept = AcceptAsync(server, token);
        await using var client = await EngineIpcClient.ConnectAsync(name, new(), "synthetic-bootstrap", cancellationToken: token);
        await accept;
        var signOut = client.SignOutAsync(token);
        using var session = await ReadFrameAsync(server, token);
        await ExpectFailureAsync(client.SubscribeAsync(SubscriptionRequest(), TimeSpan.FromMilliseconds(100)), EngineIpcFailureKind.Timeout);
        using var cancelled = new CancellationTokenSource();
        var waiting = client.SubscribeAsync(SubscriptionRequest(), cancellationToken: cancelled.Token);
        cancelled.Cancel();
        await ExpectCancellationAsync(waiting, cancelled.Token);
        await ProbeAsync(client, server, token);
        Assert.False(client.Completion.IsCompleted, "Expiry or cancellation before session admission preserves the transport.");

        var subscribing = client.SubscribeAsync(SubscriptionRequest(), TimeSpan.FromMilliseconds(700));
        await Task.Delay(450, token);
        await ReplyAsync(server, IpcServerMessage.IpcDesktopSessionResponseKind, new DesktopSessionResponse
        {
            RequestId = session.RootElement.GetProperty("payload").GetProperty("requestId").GetGuid(),
            Status = DesktopSessionStatus.SignedOut,
        }, token);
        await signOut;
        using var request = await ReadFrameAsync(server, token);
        Assert.Equal(IpcClientMessage.IpcSubscribeKind, request.RootElement.GetProperty("kind").GetString(), "Only the admitted subscription is sent.");
        await ExpectFailureAsync(subscribing.WaitAsync(TimeSpan.FromMilliseconds(500)), EngineIpcFailureKind.Timeout);
        await client.Completion.WaitAsync(TimeSpan.FromSeconds(2));
    }

    private static SubscriptionEnvelope SubscriptionRequest() => new()
    {
        RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Authorization = new(), ProtocolVersion = new(),
        Subscription = new() { ["kind"] = "synthetic" },
    };

    public static async Task ContainmentReleasesSessionRequestsAndTheirLock()
    {
        using var safety = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        var name = $"eitmad-session-drain-{Guid.NewGuid():N}";
        await using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
        var accept = AcceptAsync(server, safety.Token);
        await using var client = await EngineIpcClient.ConnectAsync(name, new PeerHello(), "synthetic-bootstrap", cancellationToken: safety.Token);
        await accept;
        var oldSignIn = client.SignInAsync("synthetic", "synthetic");
        using var signInFrame = await ReadFrameAsync(server, safety.Token);
        var signOut = client.SignOutAsync(); // Queued behind a session operation with no reply.
        Assert.False(signOut.IsCompleted, "sign-out waits for the session lock");
        await client.DisposeAsync(); // The same concrete transport disposal used by supervisor stop.
        await ExpectFailureAsync(oldSignIn, EngineIpcFailureKind.ConnectionLost);
        await Assert.ThrowsAsync<ObjectDisposedException>(() => signOut.WaitAsync(TimeSpan.FromSeconds(2)),
            "queued sign-out completes after transport disposal");
        Assert.Equal(1, PrivateField<SemaphoreSlim>(client, "sessionChangeLock").CurrentCount,
            "containment releases the session transition lock before another account can enter");
    }

    public static async Task ClosingSubscriptionDiscardsEventsWithoutBreakingTransport()
    {
        foreach (var scenario in new[] { "unsubscribe", "early-close", "session", "pending-session" })
        {
            var earlyClose = scenario == "early-close";
            var sessionClose = scenario is "session" or "pending-session";
            using var safety = new CancellationTokenSource(TimeSpan.FromSeconds(10));
            var token = safety.Token;
            var name = $"eitmad-unsubscribe-{Guid.NewGuid():N}";
            await using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
                PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
            var accept = AcceptAsync(server, token);
            await using var client = await EngineIpcClient.ConnectAsync(name, new PeerHello(), "synthetic-bootstrap", cancellationToken: token);
            await accept;
            var request = SubscriptionRequest();
            var subscribing = client.SubscribeAsync(request, cancellationToken: token);
            using var sent = await ReadFrameAsync(server, token);
            var pendingSession = scenario == "pending-session" ? client.SignOutAsync(token) : null;
            if (pendingSession is not null)
                Assert.False(pendingSession.IsCompleted, "session transition waits for the subscription acknowledgement");
            var id = Guid.NewGuid();
            var notice = new EventEnvelope { SubscriptionId = id, CorrelationId = request.CorrelationId, Cursor = Guid.NewGuid(), Sequence = 1 };
            // Exercise the acknowledged-subscription handoff before its continuation runs.
            await ReplyAsync(server, IpcServerMessage.IpcEventKind, notice, token);
            if (earlyClose)
                await ReplyAsync(server, IpcServerMessage.IpcSubscriptionClosedKind, new SubscriptionClosedEnvelope
                {
                    SubscriptionId = id, CorrelationId = request.CorrelationId, Reason = SubscriptionCloseReason.AuthorizationRevoked,
                }, token);
            await ReplyAsync(server, IpcServerMessage.IpcSubscribeResponseKind, new SubscriptionResponseEnvelope
            {
                RequestId = request.RequestId, CorrelationId = request.CorrelationId,
                Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = new() { SubscriptionId = id, StreamCursor = Guid.NewGuid(), Resumed = false } },
            }, token);
            var subscription = await subscribing.WaitAsync(token);
            if (earlyClose)
            {
                await using var reader = subscription.ReadAllAsync(token).GetAsyncEnumerator();
                Assert.True(await reader.MoveNextAsync(), "early event survives handoff");
                await ExpectFailureAsync(reader.MoveNextAsync().AsTask(), EngineIpcFailureKind.SessionChanged);
                await client.UnsubscribeAsync(subscription, cancellationToken: token);
                // Already-closed cleanup sends no redundant request.
            }
            else
            {
                var closing = pendingSession ?? (sessionClose ? client.SignOutAsync(token) : client.UnsubscribeAsync(subscription, cancellationToken: token));
                using var unsubscribe = await ReadFrameAsync(server, token);
                Assert.Equal(sessionClose ? IpcClientMessage.IpcDesktopSignOutKind : IpcClientMessage.IpcUnsubscribeKind,
                    unsubscribe.RootElement.GetProperty("kind").GetString(), "typed cleanup request");
                await using var reader = subscription.ReadAllAsync(token).GetAsyncEnumerator();
                if (sessionClose) await ExpectFailureAsync(reader.MoveNextAsync().AsTask(), EngineIpcFailureKind.SessionChanged);
                else Assert.False(await reader.MoveNextAsync(), "buffered delivery stops before unsubscribe acknowledgement");
                // More than a queue's capacity must be discarded while the reply is withheld.
                for (var index = 0; index <= EngineSubscription.Capacity; index++)
                    await ReplyAsync(server, IpcServerMessage.IpcEventKind, notice, token);
                await ProbeAsync(client, server, token);
                Assert.False(closing.IsCompleted, "the peer still withholds unsubscribe");
                await ReplyAsync(server, IpcServerMessage.IpcSubscriptionClosedKind, new SubscriptionClosedEnvelope
                {
                    SubscriptionId = id, CorrelationId = request.CorrelationId, Reason = SubscriptionCloseReason.ClientRequested,
                }, token);
                var requestId = unsubscribe.RootElement.GetProperty("payload").GetProperty("requestId").GetGuid();
                if (sessionClose)
                    await ReplyAsync(server, IpcServerMessage.IpcDesktopSessionResponseKind, new DesktopSessionResponse
                    {
                        RequestId = requestId, Status = DesktopSessionStatus.SignedOut,
                    }, token);
                else
                    await ReplyAsync(server, IpcServerMessage.IpcUnsubscribeResponseKind, new UnsubscribeResponse
                    {
                        RequestId = requestId, SubscriptionId = id, Accepted = true,
                    }, token);
                await closing.WaitAsync(token);
            }
            var signOut = client.SignOutAsync(token);
            using var signOutFrame = await ReadFrameAsync(server, token);
            Assert.Equal(IpcClientMessage.IpcDesktopSignOutKind, signOutFrame.RootElement.GetProperty("kind").GetString(), "healthy sign-out after subscription cleanup");
            await ReplyAsync(server, IpcServerMessage.IpcDesktopSessionResponseKind, new DesktopSessionResponse
            {
                RequestId = signOutFrame.RootElement.GetProperty("payload").GetProperty("requestId").GetGuid(), Status = DesktopSessionStatus.SignedOut,
            }, token);
            await signOut.WaitAsync(token);
            await ProbeAsync(client, server, token);
            Assert.False(client.Completion.IsCompleted, "event cleanup preserves the healthy transport");
        }
    }

    private static async Task ProbeAsync(EngineIpcClient client, NamedPipeServerStream server, CancellationToken token)
    {
        var query = client.SendQueryAsync(Request("healthy"), cancellationToken: token);
        using var sent = await ReadFrameAsync(server, token);
        Assert.Equal(IpcClientMessage.IpcQueryKind, sent.RootElement.GetProperty("kind").GetString(), "healthy query frame");
        await ReplyAsync(server, IpcServerMessage.IpcQueryResponseKind, new QueryResponseEnvelope
        {
            RequestId = sent.RootElement.GetProperty("payload").GetProperty("requestId").GetGuid(),
            Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = new() },
        }, token);
        await query.WaitAsync(token);
    }
}
