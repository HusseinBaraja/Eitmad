using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;

namespace Eitmad.Platform.Windows.ProcessSupervision;

public sealed class SupervisedEngineSubscription : IAsyncDisposable
{
    private readonly object gate = new();
    private Channel<EventEnvelope> events = CreateEventChannel();
    private CancellationTokenSource? attachmentCancellation;
    private EngineSubscription? attached;
    private Guid? processedCursor;
    private bool disposed;

    internal SupervisedEngineSubscription(Subscription contract)
    {
        Contract = contract;
        RegistrationId = Guid.NewGuid();
    }

    public event Action? ResyncRequired;

    public Guid RegistrationId { get; }

    public Guid? ProcessedCursor
    {
        get
        {
            lock (gate)
            {
                return processedCursor;
            }
        }
    }

    internal Subscription Contract { get; }

    internal EngineSubscription? Attached
    {
        get
        {
            lock (gate)
            {
                return attached;
            }
        }
    }

    public async IAsyncEnumerable<EventEnvelope> ReadAllAsync(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        ChannelReader<EventEnvelope> reader;
        lock (gate)
        {
            reader = events.Reader;
        }
        await foreach (var item in reader.ReadAllAsync(cancellationToken).ConfigureAwait(false))
        {
            yield return item;
        }
    }

    public void Acknowledge(EventEnvelope delivered)
    {
        ArgumentNullException.ThrowIfNull(delivered);
        lock (gate)
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            processedCursor = delivered.Cursor;
        }
    }

    internal void Attach(EngineSubscription subscription, bool resetCursor)
    {
        CancellationToken cancellationToken;
        Channel<EventEnvelope> attachedEvents;
        lock (gate)
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            attachmentCancellation?.Cancel();
            attachmentCancellation?.Dispose();
            attachmentCancellation = new CancellationTokenSource();
            cancellationToken = attachmentCancellation.Token;
            attached = subscription;
            if (events.Reader.Completion.IsCompleted)
            {
                events = CreateEventChannel();
            }
            attachedEvents = events;
            if (resetCursor || processedCursor is null)
            {
                processedCursor = subscription.ProcessedCursor;
            }
        }

        _ = PumpAsync(subscription, attachedEvents, cancellationToken);
    }

    internal void SignalResyncRequired() => ResyncRequired?.Invoke();

    private async Task PumpAsync(
        EngineSubscription subscription,
        Channel<EventEnvelope> attachedEvents,
        CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var delivered in subscription.ReadAllAsync(cancellationToken).ConfigureAwait(false))
            {
                if (!attachedEvents.Writer.TryWrite(delivered))
                {
                    attachedEvents.Writer.TryComplete(new EngineIpcException(
                        EngineIpcFailureKind.SubscriptionBackpressure,
                        "The supervised event consumer exceeded its bounded queue."));
                    return;
                }
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (EngineIpcException)
        {
            // The supervisor reconnects and attaches a replacement stream.
        }
    }

    public ValueTask DisposeAsync()
    {
        lock (gate)
        {
            if (disposed)
            {
                return ValueTask.CompletedTask;
            }
            disposed = true;
            attachmentCancellation?.Cancel();
            attachmentCancellation?.Dispose();
            attachmentCancellation = null;
            attached = null;
            events.Writer.TryComplete();
        }
        return ValueTask.CompletedTask;
    }

    private static Channel<EventEnvelope> CreateEventChannel() =>
        Channel.CreateBounded<EventEnvelope>(
            new BoundedChannelOptions(EngineSubscription.Capacity)
            {
                FullMode = BoundedChannelFullMode.Wait,
                SingleReader = false,
                SingleWriter = true,
            });
}
