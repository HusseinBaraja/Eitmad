using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.WindowsShell.Features.Authentication;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class DesktopEnrollmentRenderedTests
{
    public TestContext TestContext { get; set; } = null!;

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void FailedEnrollmentRetainsInputsAndOffersKeyboardRetry(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
            var view = window.SignInSurface;
            var setup = WpfTestHost.FindByName<Expander>(view, "ConnectionSetup");
            setup.IsExpanded = true;
            WpfTestHost.CompleteLayout(window);
            var endpoint = WpfTestHost.FindByName<TextBox>(view, "ServerEndpoint");
            endpoint.Text = "https://server.example.test/";
            WpfTestHost.FindByName<TextBox>(view, "TrustCertificate").Text = @"C:\Setup\trusted-ca.pem";
            WpfTestHost.FindByName<TextBox>(view, "TenantCode").Text = "synthetic-organization";
            WpfTestHost.FindByName<TextBox>(view, "OrganizationId").Text = Guid.NewGuid().ToString();
            WpfTestHost.FindByName<TextBox>(view, "BranchId").Text = Guid.NewGuid().ToString();
            DesktopEnrollmentSettings? sent = null;
            view.AuthenticateAsync = (_, _, settings, _) => {
                sent = settings;
                return Task.FromException<AuthenticatedSurface>(new EngineIpcException(EngineIpcFailureKind.AuthenticationRejected, "Synthetic trust rejection."));
            };
            WpfTestHost.CompleteLayout(window);
            var dpi = VisualTreeHelper.GetDpi(window);
            TestContext.WriteLine($"Enrollment render requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%; display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP.");
            Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
            endpoint.BringIntoView(); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(endpoint.Focus()); Assert.IsTrue(endpoint.IsKeyboardFocused);
            Assert.AreEqual(FlowDirection.LeftToRight, endpoint.FlowDirection);
            Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(endpoint)));
            WpfTestHost.Capture(window, $"desktop-enrollment-setup-{width}");
            WpfTestHost.SignIn(window, "مدير الاختبار");
            Assert.IsNotNull(sent);
            Assert.AreEqual(endpoint.Text, sent.Endpoint);
            Assert.AreEqual("", WpfTestHost.FindByName<PasswordBox>(view, "PasswordInput").Password);
            Assert.IsTrue(setup.IsExpanded);
            var error = WpfTestHost.FindByName<TextBlock>(view, "SignInError");
            Assert.AreEqual(Visibility.Visible, error.Visibility);
            WpfTestHost.CompleteLayout(window);
            var retry = WpfTestHost.FindByName<Button>(view, "SignInButton");
            Assert.IsTrue(retry.IsEnabled); Assert.IsTrue(retry.Focus());
            WpfTestHost.Capture(window, $"desktop-enrollment-failure-{width}");
        }, showSignIn: true);
    }
}
