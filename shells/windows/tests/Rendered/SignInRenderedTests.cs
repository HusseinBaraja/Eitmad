using System.Windows.Controls;

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

            WpfTestHost.Capture(window, "sign-in");
        }, showSignIn: true);
    }
}
