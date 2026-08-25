using System.IO;
using System.Windows;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Operations;
using Eitmad.WindowsShell.Platform;

namespace Eitmad.WindowsShell;

public partial class App : System.Windows.Application
{
    private ShellLifetime? lifetime;

    protected override async void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        var bridge = WindowsEngineBridge.Create(e.Args);
        var viewModel = new OperationsViewModel();
        var coordinator = new OperationsCoordinator(bridge, viewModel, new WpfShellDispatcher(Dispatcher));
        var window = new MainWindow(viewModel);
        lifetime = new ShellLifetime(this, window, coordinator);
        lifetime.Start();

        try
        {
            await coordinator.StartAsync();
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or InvalidOperationException)
        {
            viewModel.ObserveStartupFailure(error.Message);
        }
    }

    protected override void OnExit(ExitEventArgs e)
    {
        lifetime?.Dispose();
        base.OnExit(e);
    }
}
