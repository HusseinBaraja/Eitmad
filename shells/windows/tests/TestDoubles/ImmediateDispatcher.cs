using Eitmad.WindowsShell.Features.Operations;

namespace Eitmad.WindowsShell.Tests.TestDoubles;

internal sealed class ImmediateDispatcher : IShellDispatcher
{
    public void Invoke(Action action) => action();
}
