using System.IO;
using System.Threading.Channels;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Tests.TestDoubles;

internal sealed class FakeSubscription : IEngineSubscription
{
    private readonly Action onDispose;
    private readonly Channel<EventEnvelope> events = Channel.CreateUnbounded<EventEnvelope>();

    public FakeSubscription(Action onDispose) => this.onDispose = onDispose;

    public event Action? ResyncRequired;

    public IAsyncEnumerable<EventEnvelope> ReadAllAsync(CancellationToken cancellationToken = default) =>
        events.Reader.ReadAllAsync(cancellationToken);

    public void Acknowledge(EventEnvelope delivered)
    {
    }

    public void Publish(EventEnvelope envelope) => events.Writer.TryWrite(envelope);

    public void FailRead() => events.Writer.TryComplete(new IOException("Synthetic subscription read failure."));

    public void SignalResync() => ResyncRequired?.Invoke();

    public ValueTask DisposeAsync()
    {
        events.Writer.TryComplete();
        onDispose();
        return ValueTask.CompletedTask;
    }
}
