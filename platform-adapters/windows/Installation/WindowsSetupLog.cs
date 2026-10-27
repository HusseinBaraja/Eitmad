using System.Text;

namespace Eitmad.Platform.Windows.Installation;

/// <summary>Writes an operator-selected diagnostic export without changing installation state.</summary>
public static class WindowsSetupLog
{
    public static Task SaveAsync(string path, string content) =>
        File.WriteAllTextAsync(path, content, new UTF8Encoding(true));
}
