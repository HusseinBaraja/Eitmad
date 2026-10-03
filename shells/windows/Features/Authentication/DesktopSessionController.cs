using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Authentication;

public enum AuthenticatedSurface
{
    Manager,
    Receptionist,
}

public enum SessionEndReason
{
    Expired,
    ConnectionLost,
}

public sealed class SessionEndedEventArgs(SessionEndReason reason) : EventArgs
{
    public SessionEndReason Reason { get; } = reason;
}

public sealed class SessionPermissionException : Exception;

public interface IDesktopSessionController : IAsyncDisposable
{
    event EventHandler<SessionEndedEventArgs>? SessionEnded;
    Task StartAsync(CancellationToken cancellationToken = default);
    Task StopAsync(CancellationToken cancellationToken = default);
    Task<AuthenticatedSurface> SignInAsync(string username, string password, CancellationToken cancellationToken = default);
    Task SignOutAsync(CancellationToken cancellationToken = default);
}

public sealed class DesktopSessionController : IDesktopSessionController
{
    private readonly IEngineShellBridge engine;
    private readonly SemaphoreSlim transition = new(1, 1);
    private CancellationTokenSource? expiryCancellation;
    private bool active;
    private bool disposed;

    public DesktopSessionController(IEngineShellBridge engine)
    {
        this.engine = engine;
    }

    public event EventHandler<SessionEndedEventArgs>? SessionEnded;

    public async Task StartAsync(CancellationToken cancellationToken = default)
    {
        engine.StateChanged += ObserveEngineState;
        await engine.StartAsync(cancellationToken);
    }

    public async Task<AuthenticatedSurface> SignInAsync(
        string username,
        string password,
        CancellationToken cancellationToken = default)
    {
        await transition.WaitAsync(cancellationToken);
        try
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            DeactivateLocalState();
            var session = await engine.SignInAsync(username, password, cancellationToken);
            try
            {
                var surface = ResolveSurface(session.AccountRole);
                active = true;
                ScheduleExpiry(session.ExpiresAt);
                return surface;
            }
            catch
            {
                DeactivateLocalState();
                try { await engine.SignOutAsync(CancellationToken.None); }
                catch (EngineIpcException) { }
                throw;
            }
        }
        finally
        {
            transition.Release();
        }
    }

    public async Task SignOutAsync(CancellationToken cancellationToken = default)
    {
        await transition.WaitAsync(cancellationToken);
        try
        {
            DeactivateLocalState();
            await engine.SignOutAsync(cancellationToken);
        }
        finally
        {
            transition.Release();
        }
    }

    public Task StopAsync(CancellationToken cancellationToken = default) => engine.StopAsync(cancellationToken);

    public async ValueTask DisposeAsync()
    {
        if (disposed) return;
        disposed = true;
        engine.StateChanged -= ObserveEngineState;
        expiryCancellation?.Cancel();
        expiryCancellation?.Dispose();
        expiryCancellation = null;
        await engine.DisposeAsync();
        transition.Dispose();
    }

    private static AuthenticatedSurface ResolveSurface(DesktopAccountRole role) => role switch
    {
        DesktopAccountRole.Manager => AuthenticatedSurface.Manager,
        DesktopAccountRole.Receptionist => AuthenticatedSurface.Receptionist,
        _ => throw new SessionPermissionException(),
    };

    private void DeactivateLocalState()
    {
        active = false;
        expiryCancellation?.Cancel();
        expiryCancellation?.Dispose();
        expiryCancellation = null;
    }

    private void ScheduleExpiry(long? expiresAt)
    {
        if (expiresAt is null) return;
        expiryCancellation = new CancellationTokenSource();
        var delay = DateTimeOffset.FromUnixTimeMilliseconds(expiresAt.Value) - DateTimeOffset.UtcNow;
        _ = ExpireAsync(delay > TimeSpan.Zero ? delay : TimeSpan.Zero, expiryCancellation.Token);
    }

    private async Task ExpireAsync(TimeSpan delay, CancellationToken cancellationToken)
    {
        try
        {
            await Task.Delay(delay, cancellationToken);
            await EndSessionAsync(SessionEndReason.Expired);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
    }

    private void ObserveEngineState(EngineSupervisionSnapshot snapshot)
    {
        if (active && snapshot.IpcHealth != EngineIpcHealthState.Connected)
        {
            _ = EndSessionAsync(SessionEndReason.ConnectionLost);
        }
    }

    private async Task EndSessionAsync(SessionEndReason reason)
    {
        await transition.WaitAsync();
        try
        {
            if (!active) return;
            DeactivateLocalState();
            try { await engine.SignOutAsync(CancellationToken.None); }
            catch (EngineIpcException) { }
        }
        finally
        {
            transition.Release();
        }
        SessionEnded?.Invoke(this, new SessionEndedEventArgs(reason));
    }
}
