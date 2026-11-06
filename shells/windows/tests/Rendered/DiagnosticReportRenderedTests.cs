using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Installation;
using Eitmad.WindowsSetup;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.TestDoubles;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class DiagnosticReportRenderedTests
{
    public TestContext TestContext { get; set; } = null!;

    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Report_is_accessible_on_sign_in_and_product_screens(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            Check(window, width, height, "sign-in");
            WpfTestHost.SignIn(window, "admin");
            Check(window, width, height, "manager");
        }, showSignIn: true);
    }

    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Report_is_accessible_during_guided_setup(int width, int height)
    {
        WpfTestHost.RunWindow(() => new SetupWindow(new SetupBackend(), WindowsSetupRole.Server) { Width = width, Height = height }, window => Check(window, width, height, "setup"));
    }

    [TestMethod]
    [DataRow(1920, 1080)] [DataRow(1338, 753)] [DataRow(720, 560)]
    public void Report_remains_accessible_in_separate_product_windows(int width, int height)
    {
        WpfTestHost.RunWindow(() => new OwnerPairingWindow(new FakeEngine()) { Width = width, Height = height },
            window => Check(window, width, height, "owner-pairing"));
        WpfTestHost.RunWindow(() => {
            var model = new SalesCatalogViewModel(new(), new());
            return new QuotationEditorWindow(model, new SalesCatalogView { DataContext = model }) { Width = width, Height = height };
        }, window => Check(window, width, height, "quotation-editor"));
        WpfTestHost.RunWindow(() => new Window {
            Width = width, Height = height, FlowDirection = FlowDirection.RightToLeft,
            Content = DiagnosticReportButton.WithContent(new PrintPreview {
                Document = new System.Windows.Documents.FlowDocument(new System.Windows.Documents.Paragraph(new System.Windows.Documents.Run("معاينة تجريبية")))
            })
        }, window => Check(window, width, height, "print-preview"));
    }

    private void Check(Window window, int width, int height, string screen)
    {
        if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
        WpfTestHost.CompleteLayout(window);
        var dpi = VisualTreeHelper.GetDpi(window);
        var dimensions = $"Display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scale {dpi.DpiScaleX * 100:0}%; requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; screen {screen}.";
        TestContext.WriteLine(dimensions);
        if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } captures)
        {
            System.IO.Directory.CreateDirectory(captures);
            System.IO.File.AppendAllText(System.IO.Path.Combine(captures, "dimensions.txt"), dimensions + Environment.NewLine);
        }
        var control = WpfTestHost.Descendants<DiagnosticReportButton>(window).Single();
        var button = (Button)control.FindName("SaveButton");
        Assert.IsTrue(button.IsVisible && button.IsEnabled && button.Focus());
        Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(button)));
        Assert.AreEqual(FlowDirection.RightToLeft, control.FlowDirection);
        var position = button.TranslatePoint(new Point(), window);
        Assert.IsTrue(position.X >= 0 && position.Y >= 0 && position.X + button.ActualWidth <= window.ActualWidth && position.Y + button.ActualHeight <= window.ActualHeight);
        var center = button.TranslatePoint(new Point(button.ActualWidth / 2, button.ActualHeight / 2), window);
        var hit = window.InputHitTest(center) as DependencyObject;
        Assert.IsTrue(hit == button || hit is not null && WpfTestHost.Ancestor<Button>(hit) == button, "The report button must remain above the screen content.");
        WpfTestHost.CompleteLayout(window);
        WpfTestHost.Capture(window, $"diagnostic-{screen}-{width}");
    }

    private sealed class SetupBackend : IWindowsSetupBridge
    {
        public Task<WindowsInstallationState?> InspectAsync() => Task.FromResult<WindowsInstallationState?>(null);
        public Task InstallAsync(GuidedSetupAction action, InitializeWorkshop? workshop, IProgress<WindowsInstallProgress> progress) => Task.CompletedTask;
        public Task RetryAsync(string? ownerPassword, IProgress<WindowsInstallProgress> progress) => Task.CompletedTask;
        public Task OpenDesktopAsync() => Task.CompletedTask;
    }
}
