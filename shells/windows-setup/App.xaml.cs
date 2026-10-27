using System.Windows;
using Eitmad.Platform.Windows.Installation;
using Eitmad.Contracts;
namespace Eitmad.WindowsSetup;
public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        if (typeof(App).Assembly.GetManifestResourceInfo("Eitmad.InstallerPayload.zip") is not null)
        {
            var installer = new InstallerWindow(); MainWindow = installer;
            installer.Loaded += async (_, _) => await installer.StartAsync();
            installer.Show(); return;
        }
        var bundle = System.IO.Path.GetFullPath(System.IO.Path.Combine(AppContext.BaseDirectory, ".."));
        Window window;
        if (e.Args.Length == 2 && e.Args[0] == "--role" && e.Args[1] is "server" or "client")
            window = new SetupWindow(new WindowsSetupBridge(bundle),
                e.Args[1] == "server" ? WindowsSetupRole.Server : WindowsSetupRole.Client);
        else
        {
            MessageBox.Show("افتح EitmadServerSetup.exe لإعداد الخادم، أو EitmadClientSetup.exe لجهاز الموظف.",
                "إعداد الاعتماد", MessageBoxButton.OK, MessageBoxImage.Information,
                MessageBoxResult.OK, MessageBoxOptions.RtlReading | MessageBoxOptions.RightAlign);
            Shutdown(); return;
        }
        MainWindow = window; window.Show();
    }
}
