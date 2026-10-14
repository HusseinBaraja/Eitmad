using System.Windows;
using System.Windows.Controls;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Users;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Tests.TestDoubles;
namespace Eitmad.WindowsShell.Tests.Rendered;
[TestClass]
public sealed class UsersRenderedTests
{
    private static FakeEngine Engine()
    {
        var engine = new FakeEngine();
        engine.DesktopAccounts.AddRange([
            new DesktopAccountSummary { AccountId = Guid.NewGuid(), UserId = Guid.NewGuid(), DisplayName = "محمد سالم", Username = "m.salem", Role = DesktopAccountRole.Manager, Active = true, Revision = 1 },
            new DesktopAccountSummary { AccountId = Guid.NewGuid(), UserId = Guid.NewGuid(), DisplayName = "أحمد علي", Username = "a.ali", Role = DesktopAccountRole.Receptionist, Active = true, Revision = 1 },
        ]);
        return engine;
    }

    [TestMethod]
    public void RefreshNoticeAndRetryFitWithEnlargedTextAndSystemColorsAtMinimumSize()
    {
        var engine = Engine();
        WpfTestHost.Run(720, 560, window =>
        {
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<UsersView>(window).Single();
            engine.ThrowQueries = true;
            Assert.IsFalse(view.ViewModel.LoadAsync().GetAwaiter().GetResult());
            view.Resources[SystemParameters.HighContrastKey] = true;
            WpfTestHost.CompleteLayout(window);
            var notice = WpfTestHost.FindByName<FeedbackNotice>(view, "UsersLoadNotice");
            var retry = WpfTestHost.FindByName<Button>(view, "RetryUsersButton");
            notice.FontSize *= 1.5;
            retry.FontSize *= 1.5;
            WpfTestHost.CompleteLayout(window);
            AssertTextFitsView(WpfTestHost.Descendants<TextBlock>(notice).Single(text => text.Text == notice.Message), view);
            var bounds = retry.TransformToAncestor(view).TransformBounds(new Rect(retry.RenderSize));
            Assert.IsTrue(retry.IsVisible && retry.IsEnabled);
            Assert.IsTrue(bounds.Left >= 0 && bounds.Right <= view.ActualWidth);
            Assert.IsTrue(bounds.Top >= 0 && bounds.Bottom <= view.ActualHeight);
            WpfTestHost.Capture(window, "ui41-users-refresh-system-colors-large-text-720");
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920d, 1080d)]
    [DataRow(1338d, 753d)]
    [DataRow(720d, 560d)]
    public void FailedRefreshLabelsRetainedRowsAndRetryRestoresCurrentState(double width, double height)
    {
        var engine = Engine();
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            if (width == 1920) window.WindowState = WindowState.Maximized;
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<UsersView>(window).Single();
            var table = WpfTestHost.FindByName<OperationsTable>(view, "UsersTable");
            Assert.AreEqual(2, table.Items.Count);
            engine.ThrowQueries = true;
            Assert.IsFalse(view.ViewModel.LoadAsync().GetAwaiter().GetResult());
            WpfTestHost.CompleteLayout(window);

            var dpi = System.Windows.Media.VisualTreeHelper.GetDpi(window);
            var geometry = $"Requested {width}x{height}; actual window {window.ActualWidth}x{window.ActualHeight} DIP; primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX * 100:0}%; WPF RenderTargetBitmap at 96 DPI; synthetic account responses.";
            Console.WriteLine(geometry);
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
            {
                System.IO.Directory.CreateDirectory(directory);
                System.IO.File.WriteAllText(System.IO.Path.Combine(directory, $"ui41-{width}.txt"), geometry);
            }
            var notice = WpfTestHost.FindByName<FeedbackNotice>(view, "UsersLoadNotice");
            var noticeText = WpfTestHost.Descendants<TextBlock>(notice).Single(text => text.Text == notice.Message);
            AssertTextFitsView(noticeText, view);
            Assert.AreEqual(2, table.Items.Count);
            Assert.IsTrue(WpfTestHost.Descendants<StatusBadge>(table).All(badge => badge.Text == "غير مؤكد" && badge.Tone == PresentationTone.Neutral));
            Assert.IsFalse(WpfTestHost.FindByAutomationName<Button>(view, "إضافة مستخدم").IsEnabled);
            Assert.IsFalse(WpfTestHost.FindByAutomationName<Button>(view, "تعديل المستخدم محمد سالم").IsEnabled);
            Assert.IsFalse(WpfTestHost.FindByAutomationName<Button>(view, "تعطيل المستخدم محمد سالم").IsEnabled);
            var retry = WpfTestHost.FindByName<Button>(view, "RetryUsersButton");
            Assert.IsTrue(retry.IsVisible && retry.IsEnabled);
            var retryBounds = retry.TransformToAncestor(view).TransformBounds(new Rect(retry.RenderSize));
            Assert.IsTrue(retryBounds.Left >= 0 && retryBounds.Right <= view.ActualWidth);
            Assert.IsTrue(retryBounds.Top >= 0 && retryBounds.Bottom <= view.ActualHeight);
            Assert.IsTrue(retry.Focus());
            Assert.IsTrue(retry.IsKeyboardFocused);
            WpfTestHost.Capture(window, $"ui41-users-refresh-failed-{width}");

            // Another failed retry must keep the warning and last known rows.
            retry.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(view.ViewModel.HasLoadFailure);
            engine.ThrowQueries = false;
            engine.DesktopAccounts[0].Active = false;
            retry.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(view.ViewModel.HasLoadFailure);
            Assert.IsFalse(notice.IsVisible);
            Assert.IsFalse(retry.IsVisible);
            Assert.IsTrue(WpfTestHost.FindByAutomationName<Button>(view, "تعديل المستخدم محمد سالم").IsEnabled);
            Assert.IsFalse(WpfTestHost.FindByAutomationName<Button>(view, "تعطيل المستخدم محمد سالم").IsEnabled);
            Assert.IsTrue(WpfTestHost.Descendants<StatusBadge>(table).Any(badge => badge.Text == "غير نشط"));
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920d, 1080d)]
    [DataRow(1338d, 753d)]
    [DataRow(720d, 560d)]
    public void CreateValidationIdentifiesPasswordAndAuthorityFailureUsesFormSummary(double width, double height)
    {
        var engine = Engine();
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            if (width == 1920) window.WindowState = WindowState.Maximized;
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<UsersView>(window).Single();
            var dpi = System.Windows.Media.VisualTreeHelper.GetDpi(window);
            var geometry = $"Requested {width}x{height}; actual window {window.ActualWidth}x{window.ActualHeight} DIP; primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX * 100:0}%; WPF RenderTargetBitmap at 96 DPI; synthetic account responses.";
            Console.WriteLine(geometry);
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
            {
                System.IO.Directory.CreateDirectory(directory);
                System.IO.File.WriteAllText(System.IO.Path.Combine(directory, $"ui40-{width}.txt"), geometry);
            }

            WpfTestHost.FindByAutomationName<Button>(view, "إضافة مستخدم").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var name = WpfTestHost.FindByName<TextBox>(view, "UserNameInput");
            var username = WpfTestHost.FindByName<TextBox>(view, "UsernameInput");
            var password = WpfTestHost.FindByName<PasswordBox>(view, "UserPasswordInput");
            var nameField = WpfTestHost.Ancestor<FormField>(name);
            var passwordField = WpfTestHost.Ancestor<FormField>(password);
            var save = WpfTestHost.FindByAutomationName<Button>(view, "حفظ");
            name.Text = "مستخدم تجريبي";
            username.Text = "synthetic.user";
            WpfTestHost.CompleteLayout(window);
            foreach (var value in new[] { "", "short" })
            {
                password.Password = value;
                save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(password.IsKeyboardFocusWithin);
                Assert.IsNull(engine.LastCommand);
                Assert.AreEqual("", nameField.ErrorText);
                Assert.AreEqual("", WpfTestHost.Ancestor<FormField>(username).ErrorText);
                Assert.IsTrue(passwordField.ErrorText.Length > 0);
                Assert.AreEqual(value, password.Password);
                Assert.AreEqual("مستخدم تجريبي", name.Text);
                Assert.AreEqual("synthetic.user", username.Text);
                if (value.Length == 0 && width == 720)
                {
                    WpfTestHost.Ancestor<ScrollViewer>(password).ScrollToEnd();
                    WpfTestHost.CompleteLayout(window);
                    var saveBounds = save.TransformToAncestor(view).TransformBounds(new Rect(save.RenderSize));
                    Assert.IsTrue(saveBounds.Top >= 0 && saveBounds.Bottom <= view.ActualHeight);
                    AssertTextFitsView((TextBlock)passwordField.Template.FindName("Error", passwordField), view);
                    WpfTestHost.Capture(window, "ui40-empty-password-error-720");
                }
            }
            var error = (TextBlock)passwordField.Template.FindName("Error", passwordField);
            error.BringIntoView();
            WpfTestHost.CompleteLayout(window);
            AssertTextFitsView(error, view);
            WpfTestHost.Capture(window, $"ui40-password-error-{width}");

            password.Password = "synthetic-password";
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("", passwordField.ErrorText);
            engine.CommandHandler = _ => new CommandResponseEnvelope
            {
                Outcome = new CommandOutcome
                {
                    Status = CommandOutcomeStatus.Failed,
                    Payload = new CommandResult { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 },
                },
            };
            save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var summary = WpfTestHost.FindByName<TextBlock>(view, "EditorFailureSummary");
            Assert.IsTrue(summary.IsVisible);
            AssertTextFitsView(summary, view);
            Assert.IsTrue(WpfTestHost.Descendants<FormField>(view).Where(field => field.IsVisible).All(field => field.ErrorText.Length == 0));
            Assert.AreEqual("synthetic-password", password.Password);
            Assert.AreEqual("مستخدم تجريبي", name.Text);
            Assert.AreEqual("synthetic.user", username.Text);
            WpfTestHost.Capture(window, $"ui40-authority-summary-{width}");
        }, engine: engine);
    }

    private static void AssertTextFitsView(TextBlock text, FrameworkElement view)
    {
        var bounds = text.TransformToAncestor(view).TransformBounds(new Rect(text.RenderSize));
        Assert.IsTrue(text.IsVisible && text.ActualWidth > 0 && text.ActualHeight > 0);
        Assert.IsTrue(bounds.Left >= -1 && bounds.Right <= view.ActualWidth + 1);
        Assert.IsTrue(bounds.Top >= -1 && bounds.Bottom <= view.ActualHeight + 1);
    }

    [TestMethod]
    [DataRow(1920d, 1080d)]
    [DataRow(1338d, 753d)]
    [DataRow(720d, 560d)]
    public void SameDisplayNameShowsDistinctLoginIdentitiesInRowsAndConfirmation(double width, double height)
    {
        var engine = Engine();
        engine.DesktopAccounts[1].DisplayName = engine.DesktopAccounts[0].DisplayName;
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            if (width == 1920) window.WindowState = WindowState.Maximized;
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<UsersView>(window).Single();
            var table = WpfTestHost.FindByName<OperationsTable>(view, "UsersTable");
            var dpi = System.Windows.Media.VisualTreeHelper.GetDpi(window);
            var geometry = $"Requested {width}x{height}; actual window {window.ActualWidth}x{window.ActualHeight} DIP; primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX * 100:0}%; WPF RenderTargetBitmap at 96 DPI; synthetic account responses.";
            Console.WriteLine(geometry);
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
            {
                System.IO.Directory.CreateDirectory(directory);
                System.IO.File.WriteAllText(System.IO.Path.Combine(directory, $"ui39-{width}.txt"), geometry);
            }

            foreach (var user in view.ViewModel.VisibleUsers)
            {
                table.ScrollIntoView(user);
                WpfTestHost.CompleteLayout(window);
                var row = (DataGridRow)table.ItemContainerGenerator.ContainerFromItem(user);
                Assert.IsNotNull(row);
                Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(row).Any(text => text.IsVisible && text.Text == user.Name));
                AssertIdentityFits(row, user.Username);
                AssertIdentityFits(table, user.Username);
                if (width == 720) WpfTestHost.Capture(window, $"ui39-users-list-{user.Username}-720");
            }
            WpfTestHost.Capture(window, $"ui39-users-list-{width}");
            foreach (var user in view.ViewModel.VisibleUsers)
            {
                view.ViewModel.BeginDeactivation(user);
                WpfTestHost.CompleteLayout(window);
                var dialog = WpfTestHost.Descendants<DialogHost>(view).Single(host => host.IsOpen);
                Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(dialog).Any(text => text.IsVisible && text.Text == user.Name));
                AssertIdentityFits(dialog, user.Username);
                WpfTestHost.Capture(window, $"ui39-users-deactivate-{user.Username}-{width}");
                if (width == 720 && user.Username == "a.ali")
                {
                    view.Resources[SystemParameters.HighContrastKey] = true;
                    view.Resources["MutedBrush"] = SystemColors.GrayTextBrush;
                    WpfTestHost.CompleteLayout(window);
                    AssertIdentityFits(dialog, user.Username);
                    WpfTestHost.Capture(window, "ui39-users-deactivate-system-colors-720");
                }
                view.ViewModel.CancelDeactivation();
                WpfTestHost.CompleteLayout(window);
            }
        }, engine: engine);
    }

    private static void AssertIdentityFits(FrameworkElement container, string username)
    {
        var identity = WpfTestHost.Descendants<TextBlock>(container).Single(text => text.IsVisible && text.Text == username);
        Assert.AreEqual(FlowDirection.LeftToRight, identity.FlowDirection);
        var bounds = identity.TransformToAncestor(container).TransformBounds(new Rect(identity.RenderSize));
        Assert.IsTrue(identity.ActualWidth > 0 && identity.ActualHeight > 0);
        Assert.IsTrue(bounds.Left >= -1 && bounds.Right <= container.ActualWidth + 1);
        Assert.IsTrue(bounds.Top >= -1 && bounds.Bottom <= container.ActualHeight + 1);
    }

    [TestMethod]
    [DataRow(720d)]
    [DataRow(1338d)]
    public void EditorUsesAvailableWidthAndKeepsEnlargedControlsReachable(double width)
    {
        WpfTestHost.Run(width, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var view = WpfTestHost.Descendants<UsersView>(window).Single();
            view.ViewModel.BeginEdit();
            WpfTestHost.CompleteLayout(view);
            var content = WpfTestHost.FindByName<StackPanel>(view, "EditorContent");
            // The old left-aligned StackPanel shrank to the text's desired width.
            Assert.AreEqual(Math.Min(640, view.ActualWidth - 56), content.ActualWidth, 2);
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
        }, engine: Engine());
    }
    [TestMethod]
    [DataRow(720d, 560d)]
    [DataRow(1338d, 753d)]
    [DataRow(1536d, 864d)]
    public void ListKeepsActionsAndRoleReachableWithoutHorizontalScrolling(double width, double height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "UsersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<UsersView>(window).Single();
            var table = WpfTestHost.FindByName<OperationsTable>(view, "UsersTable");
            var scroll = (ScrollViewer)table.Template.FindName("DG_ScrollViewer", table);
            Assert.AreEqual(0, scroll.ScrollableWidth, 1);
            var edit = WpfTestHost.FindByAutomationName<Button>(view, "تعديل المستخدم محمد سالم");
            var deactivate = WpfTestHost.FindByAutomationName<Button>(view, "تعطيل المستخدم محمد سالم");
            foreach (var action in new[] { edit, deactivate })
            {
                var point = action.TranslatePoint(new Point(), table);
                Assert.IsTrue(point.X >= 0 && point.X + action.ActualWidth <= table.ActualWidth);
            }
            table.SelectedIndex = 0;
            if (width == 720)
            {
                var sort = WpfTestHost.FindByAutomationName<Button>(table, "ترتيب حسب الدور");
                Assert.IsTrue(sort.Focus());
                sort.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(System.ComponentModel.ListSortDirection.Ascending, table.Columns[1].SortDirection);
            }
            Console.WriteLine($"Users: {window.ActualWidth}x{window.ActualHeight} DIP; scaling {System.Windows.Media.VisualTreeHelper.GetDpi(window).DpiScaleX * 100}%.");
            WpfTestHost.Capture(window, $"ui04-users-{width}x{height}");
            WpfTestHost.FindByAutomationName<Button>(view, "تعديل المستخدم محمد سالم")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
        }, engine: Engine());
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
            Assert.AreEqual(2, table.Items.Count);
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
            WpfTestHost.FindByAutomationName<Button>(view, "تأكيد تعطيل المستخدم").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsFalse(view.ViewModel.VisibleUsers.Single().IsActive);
            WpfTestHost.FindByAutomationName<Button>(view, "إضافة مستخدم").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsFalse(WpfTestHost.FindByName<TextBox>(view, "UsernameInput").IsReadOnly);
            WpfTestHost.FindByName<TextBox>(view, "UserNameInput").Text = "أحمد سالم";
            WpfTestHost.FindByName<TextBox>(view, "UsernameInput").Text = "a.salem";
            WpfTestHost.FindByName<PasswordBox>(view, "UserPasswordInput").Password = "synthetic-password";
            WpfTestHost.FindByName<ComboBox>(view, "UserRoleInput").SelectedItem = "موظف الاستقبال";
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(table.IsVisible);
            Assert.IsTrue(view.ViewModel.VisibleUsers.Any(user => user.Username == "a.salem" && user.IsActive && user.Role == "موظف الاستقبال"));
        }, engine: Engine());
    }
}
