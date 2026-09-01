using System.Threading.Channels;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Tests.TestDoubles;

internal sealed class FakeSubscription : IEngineSubscription
{
    private readonly Channel<EventEnvelope> events = Channel.CreateUnbounded<EventEnvelope>();

    public event Action? ResyncRequired;

    public IAsyncEnumerable<EventEnvelope> ReadAllAsync(CancellationToken cancellationToken = default) =>
        events.Reader.ReadAllAsync(cancellationToken);

    public void Acknowledge(EventEnvelope delivered)
    {
    }

    public void SignalResync() => ResyncRequired?.Invoke();

    public ValueTask DisposeAsync()
    {
        events.Writer.TryComplete();
        return ValueTask.CompletedTask;
    }
}
