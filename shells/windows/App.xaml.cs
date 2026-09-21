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
        var sessions = new Features.Authentication.DesktopSessionController(bridge, coordinator);
        var window = CreateWindow(viewModel, sessions);
        lifetime = new ShellLifetime(this, window, sessions);
        lifetime.Start();

        MainWindow CreateWindow(OperationsViewModel model, Features.Authentication.DesktopSessionController controller)
        {
            var created = new MainWindow(model, controller, engine: bridge);
            created.AccountSessionCleared += (_, _) =>
            {
                if (lifetime is not null) lifetime.ReplaceWindow(CreateWindow(model, controller));
            };
            return created;
        }

        try
        {
            await sessions.StartAsync();
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or InvalidOperationException)
        {
            viewModel.ObserveStartupFailure(error.Message);
            window.SignInSurface.ShowConnectionFailure();
        }
    }

    protected override void OnExit(ExitEventArgs e)
    {
        lifetime?.Dispose();
        base.OnExit(e);
    }
}
