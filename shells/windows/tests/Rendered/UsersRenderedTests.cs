using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Users;
namespace Eitmad.WindowsShell.Tests.Rendered;
[TestClass]
public sealed class UsersRenderedTests
{
    [TestMethod]
    public void UsersNavigationFiltersAndDialogsWorkInTheRenderedShell()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<UsersView>(window).Single();
            var table = WpfTestHost.FindByName<OperationsTable>(view, "UsersTable");
            Assert.IsTrue(view.IsVisible);
            Assert.AreEqual(4, table.Items.Count);
            var capture = Environment.GetEnvironmentVariable("EITMAD_USERS_CAPTURE");
            if (!string.IsNullOrEmpty(capture))
            {
                var bitmap = new RenderTargetBitmap((int)window.ActualWidth, (int)window.ActualHeight, 96, 96, PixelFormats.Pbgra32);
                bitmap.Render(window);
                var encoder = new PngBitmapEncoder(); encoder.Frames.Add(BitmapFrame.Create(bitmap));
                using var stream = File.Create(capture); encoder.Save(stream);
            }
            var search = WpfTestHost.FindByName<TextBox>(view, "UsersSearchBox");
            search.Text = "احمد"; WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(1, table.Items.Count);
            var edit = WpfTestHost.FindByAutomationName<Button>(view, "تعديل المستخدم أحمد علي");
            edit.Focus(); edit.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "UserNameInput").IsKeyboardFocusWithin);
            WpfTestHost.Descendants<DialogHost>(view).Single(dialog => dialog.IsOpen).RequestClose();
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(edit.IsKeyboardFocusWithin);
            WpfTestHost.FindByAutomationName<Button>(view, "تعطيل المستخدم أحمد علي").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "CancelDeactivateButton").IsKeyboardFocusWithin);
            WpfTestHost.FindByAutomationName<Button>(view, "تأكيد التعطيل في المعاينة").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsFalse(view.ViewModel.VisibleUsers.Single().IsActive);
        });
    }
}
