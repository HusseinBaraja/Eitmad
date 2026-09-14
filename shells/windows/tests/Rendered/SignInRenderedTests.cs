using System.IO;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class SignInRenderedTests
{
    [TestMethod]
    public void UsernameShowsFullLineWithoutVerticalClipping()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            var input = WpfTestHost.FindByName<TextBox>(window, "UsernameBox");
            WpfTestHost.FindByName<PasswordBox>(window, "PasswordInput").Password = "sample";
            foreach (var text in new[] { "admin", "مستخدم.تجريبي" })
            {
                input.Text = text;
                WpfTestHost.CompleteLayout(window);
                var host = (ScrollViewer)input.Template.FindName("PART_ContentHost", input);
                Assert.IsTrue(host.ViewportHeight > 0);
                Assert.IsTrue(host.ExtentHeight <= host.ViewportHeight + 1,
                    $"Username line is clipped: extent {host.ExtentHeight}, viewport {host.ViewportHeight}.");
            }

            var directory = Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR");
            if (string.IsNullOrEmpty(directory)) return;
            Directory.CreateDirectory(directory);
            var bitmap = new RenderTargetBitmap((int)window.ActualWidth, (int)window.ActualHeight, 96, 96, PixelFormats.Pbgra32);
            bitmap.Render(window);
            var encoder = new PngBitmapEncoder();
            encoder.Frames.Add(BitmapFrame.Create(bitmap));
            using var stream = File.Create(Path.Combine(directory, "sign-in.png"));
            encoder.Save(stream);
        }, showSignIn: true);
    }
}
