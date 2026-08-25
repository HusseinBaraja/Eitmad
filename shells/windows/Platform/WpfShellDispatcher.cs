using System.Windows.Threading;
using Eitmad.WindowsShell.Features.Operations;

namespace Eitmad.WindowsShell.Platform;

public sealed class WpfShellDispatcher(Dispatcher dispatcher) : IShellDispatcher
{
    public void Invoke(Action action)
    {
        if (dispatcher.CheckAccess())
        {
            action();
        }
        else
        {
            dispatcher.Invoke(action);
        }
    }
}
