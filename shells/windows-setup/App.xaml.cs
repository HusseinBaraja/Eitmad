using System.Windows;
using Eitmad.Platform.Windows.Installation;
namespace Eitmad.WindowsSetup;
public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        var bundle = System.IO.Path.GetFullPath(System.IO.Path.Combine(AppContext.BaseDirectory, ".."));
        var backups = new WindowsBackupBridge(bundle);
        Window window = e.Args.SequenceEqual(new[] { "--backups" }) ? new BackupWindow(backups)
            : new SetupWindow(new WindowsSetupBridge(bundle), backups);
        MainWindow = window; window.Show();
    }
}
