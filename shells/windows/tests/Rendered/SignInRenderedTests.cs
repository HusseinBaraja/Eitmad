using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class SignInRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void FormAndConnectionFailureStayWithinScrollableViewport(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            var view = window.SignInSurface;
            var username = WpfTestHost.FindByName<TextBox>(view, "UsernameBox");
            username.Text = "مستخدم.تجريبي";
            WpfTestHost.FindByName<PasswordBox>(view, "PasswordInput").Password = "synthetic-password";
            var scroll = WpfTestHost.Ancestor<ScrollViewer>(username);
            WpfTestHost.CompleteLayout(window);
            RecordGeometry(window, width, height);
            AssertFormWithinViewport(view, scroll);
            WpfTestHost.Capture(window, $"sign-in-{width}x{height}");

            view.ShowConnectionFailure();
            WpfTestHost.CompleteLayout(window);
            AssertFormWithinViewport(view, scroll);
            WpfTestHost.Capture(window, $"sign-in-failure-{width}x{height}");

            // Vertical overflow must remain reachable without hiding horizontal content.
            var button = WpfTestHost.FindByName<Button>(view, "SignInButton");
            button.BringIntoView();
            WpfTestHost.CompleteLayout(window);
            var bounds = button.TransformToAncestor(scroll).TransformBounds(new Rect(button.RenderSize));
            Assert.IsTrue(bounds.Top >= -1 && bounds.Bottom <= scroll.ViewportHeight + 1,
                "The sign-in action must be reachable by vertical scrolling.");
        }, showSignIn: true);
    }

    private static void AssertFormWithinViewport(FrameworkElement view, ScrollViewer scroll)
    {
        foreach (var element in WpfTestHost.Descendants<FrameworkElement>((DependencyObject)scroll.Content)
                     .Where(element => element.IsVisible && element is TextBlock or TextBox or PasswordBox or Button or Canvas))
        {
            var bounds = element.TransformToAncestor(scroll).TransformBounds(new Rect(element.RenderSize));
            Assert.IsTrue(bounds.Left >= -1 && bounds.Right <= scroll.ViewportWidth + 1,
                $"{element.GetType().Name} {element.Name} is horizontally clipped: {bounds}; viewport {scroll.ViewportWidth}.");
        }
        Assert.IsTrue(scroll.ActualWidth <= view.ActualWidth + 1,
            "The form viewport must fit inside the sign-in surface.");
    }

    private static void RecordGeometry(Window window, int width, int height)
    {
        var dpi = VisualTreeHelper.GetDpi(window);
        var geometry = $"Requested: {width} x {height}; actual outer window: {window.ActualWidth} x {window.ActualHeight} DIP; "
            + $"scaling: {dpi.DpiScaleX * 100}%; display: {SystemParameters.PrimaryScreenWidth} x {SystemParameters.PrimaryScreenHeight} DIP.";
        Console.WriteLine(geometry);
        if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is not { Length: > 0 } directory) return;
        Directory.CreateDirectory(directory);
        File.WriteAllText(Path.Combine(directory, $"geometry-sign-in-{width}x{height}.txt"), geometry);
    }

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
