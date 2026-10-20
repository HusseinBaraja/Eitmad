using System.Windows;
using Eitmad.Platform.Windows.Installation;
namespace Eitmad.WindowsSetup;
public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        var window = new SetupWindow(new WindowsSetupBridge(System.IO.Path.GetFullPath(System.IO.Path.Combine(AppContext.BaseDirectory, ".."))));
        MainWindow = window; window.Show();
    }
}
