using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using Eitmad.WindowsShell.Features.RawMaterials;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class RawMaterialsRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void DurableListAndEditorsRenderAtBaselineSizes(int width, int height)
    {
        var engine = new FakeEngine();
        var scope = new ScopeRef { Kind = "organization", Id = Guid.NewGuid() };
        var category = new MaterialCategory { Id = Guid.NewGuid(), Scope = scope, Name = "أخشاب طبيعية", Revision = 1 };
        var unit = new MaterialUnit { Id = Guid.NewGuid(), Scope = scope, Name = "متر", Symbol = "م",
            Dimension = UnitDimension.Length, Numerator = 1, Denominator = 1, Revision = 1 };
        var material = new Material { Id = Guid.NewGuid(), Scope = scope, Name = "خشب زان مجفف",
            CategoryId = category.Id, UnitId = unit.Id, CurrentCostYer = 8_000, Revision = 1 };
        engine.QueryHandler = query => new QueryResponseEnvelope
        {
            RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(),
            Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded,
                Payload = query.Kind == Query.MaterialReferenceListKind
                    ? QueryResult.ForMaterialReferences(new MaterialReferences { Categories = [category], Units = [unit] })
                    : QueryResult.ForMaterials(new MaterialPage { Items = [material] }) },
        };
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "MaterialsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<RawMaterialsView>(window).Single();
            Assert.HasCount(1, view.ViewModel.VisibleMaterials);
            Assert.AreEqual("ر.ي 8,000", view.ViewModel.VisibleMaterials.Single().CostLabel);
            WpfTestHost.Capture(window, $"material-durable-list-{width}x{height}");

            view.ViewModel.BeginEdit(view.ViewModel.VisibleMaterials.Single());
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            WpfTestHost.Capture(window, $"material-durable-editor-{width}x{height}");

            var unitSelector = WpfTestHost.FindByAutomationName<ComboBox>(view, "وحدة المادة الخام");
            unitSelector.IsDropDownOpen = true;
            WpfTestHost.CompleteLayout(window);
            var popup = (Popup)unitSelector.Template.FindName("PART_Popup", unitSelector);
            WpfTestHost.Descendants<Button>(popup.Child).First(button => (string?)button.Tag == "unit")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(view.ViewModel.IsReferenceEditorOpen);
            Assert.IsTrue(view.ViewModel.IsUnitReference);
            WpfTestHost.Capture(window, $"material-durable-unit-{width}x{height}");
        }, engine: engine);
    }

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
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(PlacementMode.Right, action.ContextMenu.Placement);
            CollectionAssert.AreEquivalent(
                new[] { "تعديل", "تكرار", "أرشفة" },
                action.ContextMenu.Items.OfType<MenuItem>().Select(item => item.Header).Cast<string>().ToArray());
        });
    }

    [TestMethod]
    public void PointerRowActivationOpensTheExistingMaterial()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "MaterialsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<RawMaterialsView>(window).Single();
            var table = WpfTestHost.FindByName<DataGrid>(view, "MaterialsTable");
            var row = WpfTestHost.Descendants<DataGridRow>(table).First();
            row.RaiseEvent(new MouseButtonEventArgs(Mouse.PrimaryDevice, 0, MouseButton.Left)
            {
                RoutedEvent = Mouse.PreviewMouseUpEvent
            });
            WpfTestHost.PumpDispatcher();

            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(((RawMaterialListItem)row.DataContext).Name, view.ViewModel.EditorName);
        });
    }
}
