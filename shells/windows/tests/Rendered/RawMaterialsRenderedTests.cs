using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using Eitmad.WindowsShell.Features.RawMaterials;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class RawMaterialsRenderedTests
{
    [TestMethod]
    public void NavigationRendersAccessibleControlsAndCreateFocus()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "MaterialsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<RawMaterialsView>(window).Single();
            var addButton = WpfTestHost.FindByAutomationName<Button>(view, "إضافة مادة خام");
            Assert.AreEqual("إضافة مادة خام", AutomationProperties.GetName(addButton));
            addButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();

            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Grid>(view, "EditorSurface").Visibility);
            var editorName = WpfTestHost.FindByName<TextBox>(view, "EditorNameBox");
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(editorName.IsKeyboardFocusWithin);
        });
    }

    [TestMethod]
    public void RowActionPopupUsesItsTargetAndNonDestructiveActions()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "MaterialsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<RawMaterialsView>(window).Single();
            var action = WpfTestHost.FindByAutomationName<Button>(view, "إجراءات المادة الخام");

            action.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();

            Assert.IsNotNull(action.ContextMenu);
            Assert.IsTrue(action.ContextMenu.IsOpen);
            Assert.AreSame(action, action.ContextMenu.PlacementTarget);
            Assert.AreEqual(PlacementMode.Right, action.ContextMenu.Placement);
            CollectionAssert.AreEquivalent(
                new[] { "تعديل", "تكرار", "أرشفة" },
                action.ContextMenu.Items.OfType<MenuItem>().Select(item => item.Header).Cast<string>().ToArray());
        });
    }
}
