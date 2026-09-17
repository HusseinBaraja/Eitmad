using System.Windows;
using System.Windows.Controls;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Users;
namespace Eitmad.WindowsShell.Tests.Rendered;
[TestClass]
public sealed class UsersRenderedTests
{
    [TestMethod]
    [DataRow(720d)]
    [DataRow(1338d)]
    public void EditorUsesAvailableWidthAndKeepsEnlargedControlsReachable(double width)
    {
        WpfTestHost.Run(width, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var view = WpfTestHost.Descendants<UsersView>(window).Single();
            view.ViewModel.BeginEdit(view.ViewModel.VisibleUsers[0]);
            WpfTestHost.CompleteLayout(view);
            var content = WpfTestHost.FindByName<StackPanel>(view, "EditorContent");
            // The old left-aligned StackPanel shrank to the text's desired width.
            Assert.AreEqual(Math.Min(640, view.ActualWidth - 56), content.ActualWidth, 2);
            var role = WpfTestHost.FindByName<ComboBox>(view, "UserRoleInput");
            var status = WpfTestHost.FindByName<ComboBox>(view, "UserStatusInput");
            var rolePoint = role.TranslatePoint(new Point(), content);
            var statusPoint = status.TranslatePoint(new Point(), content);
            if (width == 720) Assert.IsTrue(statusPoint.Y > rolePoint.Y + role.ActualHeight);
            else Assert.AreEqual(rolePoint.Y, statusPoint.Y, 1);
            foreach (var control in WpfTestHost.Descendants<Control>(content)) control.FontSize *= 1.5;
            foreach (var text in WpfTestHost.Descendants<TextBlock>(content)) text.FontSize *= 1.5;
            WpfTestHost.CompleteLayout(view);
            var save = WpfTestHost.FindByAutomationName<Button>(view, "حفظ");
            save.BringIntoView();
            WpfTestHost.CompleteLayout(view);
            var point = save.TranslatePoint(new Point(), view);
            Assert.IsTrue(point.X >= 0 && point.X + save.ActualWidth <= view.ActualWidth);
            Assert.IsTrue(point.Y >= 0 && point.Y + save.ActualHeight <= view.ActualHeight);
            WpfTestHost.Capture(window, $"user-editor-{width}");
        });
    }
    [TestMethod]
    public void UsersNavigationFiltersAndEditorPageWorkInTheRenderedShell()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<UsersView>(window).Single();
            var table = WpfTestHost.FindByName<OperationsTable>(view, "UsersTable");
            Assert.IsTrue(view.IsVisible);
            Assert.AreEqual(4, table.Items.Count);
            WpfTestHost.Capture(window, "users-list");
            var search = WpfTestHost.FindByName<TextBox>(view, "UsersSearchBox");
            search.Text = "احمد"; WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(1, table.Items.Count);
            var edit = WpfTestHost.FindByAutomationName<Button>(view, "تعديل المستخدم أحمد علي");
            edit.Focus(); edit.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "UserNameInput").IsKeyboardFocusWithin);
            Assert.IsFalse(table.IsVisible);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "UsernameInput").IsReadOnly);
            Assert.IsFalse(WpfTestHost.Descendants<DialogHost>(view).Any(dialog => dialog.IsOpen));
            var name = WpfTestHost.FindByName<TextBox>(view, "UserNameInput");
            name.MoveFocus(new System.Windows.Input.TraversalRequest(System.Windows.Input.FocusNavigationDirection.Next));
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "UsernameInput").IsKeyboardFocusWithin);
            var role = WpfTestHost.FindByName<ComboBox>(view, "UserRoleInput");
            role.Focus(); role.IsDropDownOpen = true; WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(role.IsDropDownOpen);
            role.IsDropDownOpen = false;
            WpfTestHost.Capture(window, "users-editor");
            WpfTestHost.FindByAutomationName<Button>(view, "إلغاء تعديل المستخدم").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(edit.IsKeyboardFocusWithin);
            WpfTestHost.FindByAutomationName<Button>(view, "تعطيل المستخدم أحمد علي").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "CancelDeactivateButton").IsKeyboardFocusWithin);
            WpfTestHost.FindByAutomationName<Button>(view, "تأكيد التعطيل في المعاينة").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsFalse(view.ViewModel.VisibleUsers.Single().IsActive);
            WpfTestHost.FindByAutomationName<Button>(view, "إضافة مستخدم").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsFalse(WpfTestHost.FindByName<TextBox>(view, "UsernameInput").IsReadOnly);
            WpfTestHost.FindByName<TextBox>(view, "UserNameInput").Text = "أحمد سالم";
            WpfTestHost.FindByName<TextBox>(view, "UsernameInput").Text = "a.salem";
            WpfTestHost.FindByName<ComboBox>(view, "UserRoleInput").SelectedItem = "النجار";
            WpfTestHost.FindByName<ComboBox>(view, "UserStatusInput").SelectedItem = "غير نشط";
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(table.IsVisible);
            Assert.IsTrue(view.ViewModel.VisibleUsers.Any(user => user.Username == "a.salem" && !user.IsActive && user.Role == "النجار"));
        });
    }
}
