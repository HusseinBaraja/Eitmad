using System.ComponentModel;
using System.Windows;
using Eitmad.WindowsShell.Features.Operations;

namespace Eitmad.WindowsShell.Platform;

public sealed class ShellLifetime : IDisposable
{
    private readonly System.Windows.Application application;
    private readonly Window window;
    private readonly IShellLifetimeCoordinator coordinator;
    private readonly TrayIcon tray;
    private int shutdownStarted;
    private bool disposed;

    public ShellLifetime(System.Windows.Application application, Window window, IShellLifetimeCoordinator coordinator)
    {
        this.application = application;
        this.window = window;
        this.coordinator = coordinator;
        tray = new TrayIcon(ShowWindow, () => _ = ShutdownAsync());
    }

    public void Start()
    {
        application.MainWindow = window;
        application.ShutdownMode = ShutdownMode.OnExplicitShutdown;
        window.Closing += HideOnClose;
        window.Show();
        tray.Show();
    }

    public async Task ShutdownAsync()
    {
        if (Interlocked.Exchange(ref shutdownStarted, 1) != 0)
        {
            return;
        }

        window.Closing -= HideOnClose;
        tray.Hide();
        try
        {
            await coordinator.StopAsync();
            await coordinator.DisposeAsync();
        }
        finally
        {
            application.Shutdown();
        }
    }

    private void HideOnClose(object? sender, CancelEventArgs eventArgs)
    {
        if (Volatile.Read(ref shutdownStarted) != 0)
        {
            return;
        }

        eventArgs.Cancel = true;
        window.Hide();
    }

    private void ShowWindow()
    {
        application.Dispatcher.Invoke(() =>
        {
            if (!window.IsVisible) window.Show();
            if (window.WindowState == WindowState.Minimized) window.WindowState = WindowState.Normal;
            window.Activate();
            window.Topmost = true;
            window.Topmost = false;
            window.Focus();
        });
    }

    public void Dispose()
    {
        if (disposed)
        {
            return;
        }

        disposed = true;
        tray.Dispose();
    }
}
