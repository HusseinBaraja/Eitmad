using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class AccountSwitchRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void CancelKeepsTheAccountAndDialogFitsTheWindow(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.SignIn(window, "manager");
            MainWindow.SwitchAccountCommand.Execute(null, window);
            WpfTestHost.CompleteLayout(window);
            var dialog = window.AccountSwitchDialog;
            var cancel = window.CancelAccountSwitchButton;
            Assert.IsTrue(dialog.IsOpen);
            Assert.IsTrue(dialog.IsKeyboardFocusWithin);
            Assert.IsTrue(cancel.IsKeyboardFocused);
            Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(cancel)));
            Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(window.ConfirmAccountSwitchButton)));
            Assert.IsTrue(dialog.DialogWidth <= window.ActualWidth);
            foreach (var button in new[] { cancel, window.ConfirmAccountSwitchButton })
            {
                var bounds = button.TransformToAncestor(window).TransformBounds(new Rect(button.RenderSize));
                Assert.IsTrue(bounds.Left >= 0 && bounds.Top >= 0 && bounds.Right <= window.ActualWidth && bounds.Bottom <= window.ActualHeight);
            }
            var dpi = VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"Switch dialog: window {window.ActualWidth} x {window.ActualHeight}, scaling {dpi.DpiScaleX * 100}%, display {SystemParameters.PrimaryScreenWidth} x {SystemParameters.PrimaryScreenHeight} DIPs.");
            WpfTestHost.Capture(window, $"account-switch-{width}x{height}");
            dialog.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(window), 0, Key.Escape) { RoutedEvent = Keyboard.KeyDownEvent });
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(dialog.IsOpen);
            Assert.AreEqual(Visibility.Collapsed, window.SignInSurface.Visibility);
        }, showSignIn: true);
    }
}
