using System.IO;
using System.Windows;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Platform;

namespace Eitmad.WindowsShell;

public partial class App : System.Windows.Application
{
    private ShellLifetime? lifetime;

    protected override async void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        var bridge = WindowsEngineBridge.Create(e.Args);
        var sessions = new Features.Authentication.DesktopSessionController(bridge);
        var window = CreateWindow(sessions);
        lifetime = new ShellLifetime(this, window, sessions);
        lifetime.Start();

        MainWindow CreateWindow(Features.Authentication.DesktopSessionController controller)
        {
            var created = new MainWindow(controller, engine: bridge);
            created.AccountSessionCleared += (_, _) =>
            {
                if (lifetime is not null) lifetime.ReplaceWindow(CreateWindow(controller));
            };
            return created;
        }

        try
        {
            await sessions.StartAsync();
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or InvalidOperationException)
        {
            window.SignInSurface.ShowConnectionFailure();
        }
    }

    protected override void OnExit(ExitEventArgs e)
    {
        lifetime?.Dispose();
        base.OnExit(e);
    }
}
