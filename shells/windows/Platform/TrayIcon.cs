using Drawing = System.Drawing;
using Forms = System.Windows.Forms;

namespace Eitmad.WindowsShell.Platform;

public sealed class TrayIcon : IDisposable
{
    private readonly Forms.NotifyIcon icon;
    private bool disposed;

    public TrayIcon(Action showWindow, Action exit)
    {
        var menu = new Forms.ContextMenuStrip { RightToLeft = Forms.RightToLeft.Yes };
        menu.Items.Add("فتح مركز العمليات", null, (_, _) => showWindow());
        menu.Items.Add("إنهاء الاعتماد", null, (_, _) => exit());
        icon = new Forms.NotifyIcon
        {
            Icon = Drawing.SystemIcons.Application,
            Text = "الاعتماد · مركز العمليات",
            ContextMenuStrip = menu,
        };
        icon.DoubleClick += (_, _) => showWindow();
    }

    public void Show() => icon.Visible = true;
    public void Hide() => icon.Visible = false;

    public void Dispose()
    {
        if (disposed)
        {
            return;
        }

        disposed = true;
        icon.Visible = false;
        icon.ContextMenuStrip?.Dispose();
        icon.Dispose();
    }
}
