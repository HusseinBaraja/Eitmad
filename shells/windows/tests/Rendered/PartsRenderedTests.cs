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
    public void CreateWizardRendersAccessibleStepsAndMaterialPicker()
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
            Assert.IsTrue(editorName.IsKeyboardFocusWithin);
            Assert.AreEqual(1, view.ViewModel.CurrentStep);

            editorName.Text = "جانب خزانة";
            WpfTestHost.FindByAutomationName<Button>(view, "التالي إلى المواد الخام")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(2, view.ViewModel.CurrentStep);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Border>(view, "MaterialsStep").Visibility);

            WpfTestHost.FindByAutomationName<Button>(view, "إضافة مادة خام")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            Assert.IsTrue(view.ViewModel.IsMaterialPickerOpen);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "MaterialSearchBox").IsKeyboardFocusWithin);

            WpfTestHost.Descendants<Button>(view)
                .First(button => AutomationProperties.GetName(button) == "اختيار المادة الخام")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.HasCount(1, view.ViewModel.SelectedMaterials);

            WpfTestHost.FindByAutomationName<Button>(view, "التالي إلى المراجعة")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(3, view.ViewModel.CurrentStep);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Border>(view, "ReviewStep").Visibility);

            WpfTestHost.FindByAutomationName<Button>(view, "حفظ الجزء")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
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
