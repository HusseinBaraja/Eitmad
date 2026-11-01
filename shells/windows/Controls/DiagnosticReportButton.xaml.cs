using System.Diagnostics;
using System.IO;
using System.Windows;
using Eitmad.Platform.Windows.Diagnostics;

namespace Eitmad.WindowsShell.Controls;

public partial class DiagnosticReportButton : System.Windows.Controls.UserControl
{
    private string? saved;
    public DiagnosticReportButton() => InitializeComponent();
    internal static System.Windows.Controls.Grid WithContent(UIElement content)
    {
        var panel = new System.Windows.Controls.Grid();
        panel.RowDefinitions.Add(new() { Height = new GridLength(1, GridUnitType.Star) });
        panel.RowDefinitions.Add(new() { Height = GridLength.Auto });
        panel.Children.Add(new System.Windows.Controls.Border { Child = content, ClipToBounds = true });
        var report = new DiagnosticReportButton();
        System.Windows.Controls.Grid.SetRow(report, 1);
        panel.Children.Add(report);
        return panel;
    }
    private async void SaveClick(object sender, RoutedEventArgs args) => await SaveAsync();
    public async Task SaveAsync()
    {
        if (!SaveButton.IsEnabled) return;
        SaveButton.IsEnabled = false; OpenDownloadsButton.Visibility = Visibility.Collapsed;
        Notice.Text = "جارٍ تجهيز تقرير السجلات…";
        try
        {
            saved = await DiagnosticReport.SaveToDownloadsAsync();
            Notice.Text = "حُفظ تقرير السجلات في مجلد التنزيلات.";
            OpenDownloadsButton.Visibility = Visibility.Visible;
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or System.ComponentModel.Win32Exception
            or System.Runtime.InteropServices.COMException or System.Text.Json.JsonException
            or System.Threading.Channels.ChannelClosedException)
        {
            LocalDiagnostics.Error("Export", error);
            Notice.Text = "تعذر حفظ تقرير السجلات. تحقق من مساحة مجلد التنزيلات وصلاحياته، ثم أعد المحاولة.";
        }
        finally { SaveButton.IsEnabled = true; }
    }
    private void OpenDownloadsClick(object sender, RoutedEventArgs args)
    {
        if (saved is null) return;
        try { Process.Start(new ProcessStartInfo(Path.GetDirectoryName(saved)!) { UseShellExecute = true }); }
        catch (Exception error) when (error is IOException or System.ComponentModel.Win32Exception)
        { LocalDiagnostics.Error("Export", error); Notice.Text = "تعذر فتح المجلد. افتح التنزيلات من مستكشف الملفات."; }
    }
}
