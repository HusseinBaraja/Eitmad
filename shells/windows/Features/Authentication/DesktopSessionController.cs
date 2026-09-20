using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Operations;

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

public interface IDesktopSessionController : IShellLifetimeCoordinator
{
    event EventHandler<SessionEndedEventArgs>? SessionEnded;
    Task StartAsync(CancellationToken cancellationToken = default);
    Task<AuthenticatedSurface> SignInAsync(string username, string password, CancellationToken cancellationToken = default);
    Task SignOutAsync(CancellationToken cancellationToken = default);
}

public sealed class DesktopSessionController : IDesktopSessionController
{
    private readonly IEngineShellBridge engine;
    private readonly OperationsCoordinator operations;
    private readonly SemaphoreSlim transition = new(1, 1);
    private CancellationTokenSource? expiryCancellation;
    private bool active;
    private bool disposed;

    public DesktopSessionController(IEngineShellBridge engine, OperationsCoordinator operations)
    {
        this.engine = engine;
        this.operations = operations;
    }

    public event EventHandler<SessionEndedEventArgs>? SessionEnded;

    public async Task StartAsync(CancellationToken cancellationToken = default)
    {
        engine.StateChanged += ObserveEngineState;
        await operations.StartAsync(cancellationToken);
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
            await DeactivateLocalStateAsync(cancellationToken);
            var session = await engine.SignInAsync(username, password, cancellationToken);
            try
            {
                var response = await engine.QueryAsync(
                    Query.ForPermissionsGetEffective(new GetEffectivePermissions()),
                    cancellationToken);
                var permissions = response.Outcome.Status == CommandOutcomeStatus.Succeeded
                    ? response.Outcome.Payload.AsEffectivePermissions()
                    : null;
                var surface = ResolveSurface(permissions ?? throw new SessionPermissionException());
                active = true;
                await operations.ActivateSessionAsync(cancellationToken);
                ScheduleExpiry(session.ExpiresAt);
                return surface;
            }
            catch
            {
                await DeactivateLocalStateAsync(CancellationToken.None);
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
            await DeactivateLocalStateAsync(cancellationToken);
            await engine.SignOutAsync(cancellationToken);
        }
        finally
        {
            transition.Release();
        }
    }

    public Task StopAsync(CancellationToken cancellationToken = default) => operations.StopAsync(cancellationToken);

    public async ValueTask DisposeAsync()
    {
        if (disposed) return;
        disposed = true;
        engine.StateChanged -= ObserveEngineState;
        expiryCancellation?.Cancel();
        expiryCancellation?.Dispose();
        expiryCancellation = null;
        await operations.DisposeAsync();
        transition.Dispose();
    }

    private static AuthenticatedSurface ResolveSurface(EffectivePermissions permissions)
    {
        var granted = (permissions.Permissions ?? [])
            .Where(permission => permission.Decision == PermissionDecision.Granted)
            .Select(permission => permission.Permission)
            .ToHashSet(StringComparer.Ordinal);
        if (granted.Contains(ProtocolIds.Permissions.EitmadPermissionCatalogDraftWriteV1))
            return AuthenticatedSurface.Manager;
        if (granted.Contains(ProtocolIds.Permissions.EitmadPermissionQuotationDraftWriteV1))
            return AuthenticatedSurface.Receptionist;
        throw new SessionPermissionException();
    }

    private async Task DeactivateLocalStateAsync(CancellationToken cancellationToken)
    {
        active = false;
        expiryCancellation?.Cancel();
        expiryCancellation?.Dispose();
        expiryCancellation = null;
        await operations.DeactivateSessionAsync(cancellationToken);
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
            await DeactivateLocalStateAsync(CancellationToken.None);
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
