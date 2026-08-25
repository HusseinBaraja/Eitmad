using System.IO;
using System.Windows;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.WindowsShell.Features.Operations;
using Eitmad.WindowsShell.Platform;

namespace Eitmad.WindowsShell;

public partial class App : System.Windows.Application
{
    private ShellLifetime? lifetime;

    protected override async void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        var enginePath = EnginePathResolver.Resolve(e.Args);
        var supervisor = new EngineSupervisor();
        var bridge = new WindowsEngineBridge(supervisor);
        var viewModel = new OperationsViewModel();
        var coordinator = new OperationsCoordinator(bridge, viewModel, new WpfShellDispatcher(Dispatcher));
        var window = new MainWindow(viewModel);
        lifetime = new ShellLifetime(this, window, coordinator);
        lifetime.Start();

        var runtimeDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Eitmad",
            "engine");
        try
        {
            await coordinator.StartAsync(new EngineLaunchRequest(
                enginePath,
                runtimeDirectory,
                DevelopmentIdentity.Create()));
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

internal static class EnginePathResolver
{
    public static string Resolve(IReadOnlyList<string> arguments)
    {
        var engineArgument = arguments
            .Select((value, index) => (value, index))
            .FirstOrDefault(item => item.value == "--engine");
        if (engineArgument.value is not null && engineArgument.index + 1 < arguments.Count)
        {
            return Path.GetFullPath(arguments[engineArgument.index + 1]);
        }

        var environmentPath = Environment.GetEnvironmentVariable("EITMAD_ENGINE_PATH");
        if (!string.IsNullOrWhiteSpace(environmentPath))
        {
            return Path.GetFullPath(environmentPath);
        }

        return Path.Combine(AppContext.BaseDirectory, "eitmad-engine-cli.exe");
    }
}

internal static class DevelopmentIdentity
{
    private static readonly Guid DevelopmentPrincipal = new("a0a8e326-d0ba-4e96-bc91-e486b53da9c2");
    private static readonly Guid DevelopmentService = new("2df1c605-133c-4a3f-b80a-b3333db18198");
    private static readonly Guid DevelopmentScope = new("a1588e27-2f73-4ff4-a316-0eb0ad7145c7");

    public static DevelopmentIdentityAssertion Create() => new()
    {
        TenantId = DevelopmentScope,
        Identity = new AuthenticatedIdentity
        {
            PrincipalId = DevelopmentPrincipal,
            PrincipalKind = PrincipalKind.Service,
            ServiceId = DevelopmentService,
        },
        Scope = new ScopeRef { Kind = "organization", Id = DevelopmentScope },
    };
}
