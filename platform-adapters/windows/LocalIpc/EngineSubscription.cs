using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.LocalIpc;

public sealed class EngineSubscription
{
    internal const int Capacity = 256;

    private readonly Channel<EventEnvelope> events = Channel.CreateBounded<EventEnvelope>(
        new BoundedChannelOptions(Capacity)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = false,
            SingleWriter = true,
        });
    private readonly object gate = new();
    private Guid processedCursor;
    private long processedSequence;

    internal EngineSubscription(Guid subscriptionId, Guid streamCursor, bool resumed)
    {
        SubscriptionId = subscriptionId;
        processedCursor = streamCursor;
        Resumed = resumed;
    }

    public Guid SubscriptionId { get; }
    public bool Resumed { get; }

    public Guid ProcessedCursor
    {
        get
        {
            lock (gate)
            {
                return processedCursor;
            }
        }
    }

    public async IAsyncEnumerable<EventEnvelope> ReadAllAsync(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await foreach (var item in events.Reader.ReadAllAsync(cancellationToken).ConfigureAwait(false))
        {
            yield return item;
        }
    }

    public void Acknowledge(EventEnvelope delivered)
    {
        ArgumentNullException.ThrowIfNull(delivered);
        if (delivered.SubscriptionId != SubscriptionId)
        {
            throw new ArgumentException("The event belongs to another subscription.", nameof(delivered));
        }

        lock (gate)
        {
            if (delivered.Sequence > processedSequence)
            {
                processedCursor = delivered.Cursor;
                processedSequence = delivered.Sequence;
            }
        }
    }

    internal bool TryPublish(EventEnvelope delivered) => events.Writer.TryWrite(delivered);

    internal void Complete(Exception? error = null) => events.Writer.TryComplete(error);
}
