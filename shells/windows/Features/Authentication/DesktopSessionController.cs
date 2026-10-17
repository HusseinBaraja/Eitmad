using Eitmad.Contracts;
using System.ComponentModel;
using System.IO;
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
    RecoveryFailed,
}

public sealed class SessionEndedEventArgs(SessionEndReason reason) : EventArgs
{
    public SessionEndReason Reason { get; } = reason;
}

public sealed class SessionPermissionException : Exception;
public sealed class SessionRecoveryException(Exception innerException)
    : Exception("The previous engine session could not be safely recovered.", innerException);

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
    internal static readonly TimeSpan RecoveryTimeout = TimeSpan.FromSeconds(5);
    internal TimeSpan SessionEndTimeout { get; init; } = RecoveryTimeout;
    private readonly IEngineShellBridge engine;
    private readonly SemaphoreSlim transition = new(1, 1);
    private CancellationTokenSource? expiryCancellation;
    private bool active;
    private bool disposed;
    private SessionRecoveryException? recoveryFailure;

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
            if (recoveryFailure is not null) throw recoveryFailure;
            DeactivateLocalState();
            await AwaitEngineReadyAsync(cancellationToken);
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

    public async Task StopAsync(CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        DeactivateLocalState();
        try { await engine.StopAsync(cancellationToken); }
        catch (Exception error) when (IsCleanupFailure(error))
        {
            // A stopped snapshot alone does not prove that failed OS cleanup contained
            // the old process. This controller cannot authorize another sign-in.
            recoveryFailure ??= new SessionRecoveryException(error);
            throw recoveryFailure;
        }
        if (recoveryFailure is not null) throw recoveryFailure;
    }

    internal static bool IsCleanupFailure(Exception error) => error is EngineIpcException or IOException
        or ObjectDisposedException or Win32Exception or SessionRecoveryException
        || error is AggregateException aggregate && aggregate.InnerExceptions.All(IsCleanupFailure);

    private async Task AwaitEngineReadyAsync(CancellationToken cancellationToken)
    {
        static bool Ready(EngineSupervisionSnapshot snapshot) =>
            snapshot.IpcHealth == EngineIpcHealthState.Connected && snapshot.LastLifecycle?.Ready == true;
        if (Ready(engine.Snapshot)) return;
        var ready = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        void Observe(EngineSupervisionSnapshot snapshot)
        {
            if (Ready(snapshot)) ready.TrySetResult();
            else if (snapshot.State is EngineSupervisionState.Faulted or EngineSupervisionState.RestartExhausted
                || snapshot.IpcHealth == EngineIpcHealthState.ReconnectExhausted)
                ready.TrySetException(new EngineIpcException(EngineIpcFailureKind.EngineUnavailable,
                    "The engine could not become ready for sign-in."));
        }
        engine.StateChanged += Observe;
        try
        {
            if (engine.Snapshot.State == EngineSupervisionState.Stopped) await engine.StartAsync(cancellationToken);
            Observe(engine.Snapshot);
            await ready.Task.WaitAsync(RecoveryTimeout, cancellationToken);
        }
        catch (TimeoutException error)
        {
            throw new EngineIpcException(EngineIpcFailureKind.Timeout,
                "The engine did not become ready for sign-in within the recovery budget.", innerException: error);
        }
        finally { engine.StateChanged -= Observe; }
    }

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
            using var budget = new CancellationTokenSource(SessionEndTimeout);
            var signOut = engine.SignOutAsync(budget.Token);
            try { await signOut.WaitAsync(budget.Token); }
            catch (Exception error) when (error is EngineIpcException or System.IO.IOException
                or ObjectDisposedException or OperationCanceledException)
            {
                try
                {
                    await StopAsync(CancellationToken.None);
                    // Concrete IPC disposal releases pending sends and the session lock.
                    await ObserveSignOutEndAsync(signOut);
                    reason = SessionEndReason.ConnectionLost;
                }
                catch (SessionRecoveryException)
                {
                    _ = ObserveSignOutEndAsync(signOut);
                    reason = SessionEndReason.RecoveryFailed;
                }
            }
        }
        finally
        {
            transition.Release();
        }
        SessionEnded?.Invoke(this, new SessionEndedEventArgs(reason));
    }

    private static async Task ObserveSignOutEndAsync(Task signOut)
    {
        try { await signOut; }
        catch (Exception error) when (error is EngineIpcException or IOException
            or ObjectDisposedException or OperationCanceledException) { }
    }
}
