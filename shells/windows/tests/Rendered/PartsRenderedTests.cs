using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using Eitmad.WindowsShell.Features.Parts;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class PartsRenderedTests
{
    [TestMethod]
    public void NavigationRendersAccessibleControlsAndCreateFocus()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PartsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<PartsView>(window).Single();
            var search = WpfTestHost.FindByName<TextBox>(view, "PartsSearchBox");
            Assert.AreEqual("البحث عن جزء", AutomationProperties.GetName(search));
            var addButton = WpfTestHost.FindByAutomationName<Button>(view, "إضافة جزء");
            addButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();

            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Grid>(view, "EditorSurface").Visibility);
            var editorName = WpfTestHost.FindByName<TextBox>(view, "EditorNameBox");
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(editorName.Focus());
            Assert.IsTrue(editorName.IsKeyboardFocusWithin);
        });
    }

    [TestMethod]
    public void RowActionPopupUsesMousePlacementAndNonDestructiveActions()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PartsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PartsView>(window).Single();
            var action = WpfTestHost.FindByAutomationName<Button>(view, "إجراءات الجزء");

            action.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();

            Assert.IsNotNull(action.ContextMenu);
            Assert.IsTrue(action.ContextMenu.IsOpen);
            Assert.AreSame(action, action.ContextMenu.PlacementTarget);
            Assert.AreEqual(PlacementMode.MousePoint, action.ContextMenu.Placement);
            CollectionAssert.AreEquivalent(
                new[] { "تعديل", "تكرار", "أرشفة" },
                action.ContextMenu.Items.OfType<MenuItem>().Select(item => item.Header).Cast<string>().ToArray());
        });
    }
}
