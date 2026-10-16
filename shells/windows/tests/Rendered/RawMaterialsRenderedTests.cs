using System.Windows;
using System.Windows.Media;
using Eitmad.WindowsShell.Controls;
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
    public void InputErrorsStayWithTheirFieldsAndAuthorityFailuresUseSummary(double width, double height)
    {
        var data = RawMaterials.MaterialFixtures.Snapshot();
        var submissions = 0;
        var engine = new FakeEngine
        {
            QueryHandler = query => new QueryResponseEnvelope
            {
                Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded,
                    Payload = query.Kind == Query.MaterialReferenceListKind
                        ? QueryResult.ForMaterialReferences(data.References)
                        : QueryResult.ForMaterials(new MaterialPage { Items = data.Materials.ToArray() }) },
            },
            CommandHandler = _ =>
            {
                submissions++;
                return new CommandResponseEnvelope
                {
                    Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Failed,
                        Payload = new CommandResult { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 } },
                };
            },
        };
        WpfTestHost.Run(width, height, window =>
        {
            if (width == 1920) { window.WindowState = WindowState.Maximized; WpfTestHost.CompleteLayout(window); }
            Console.WriteLine($"Materials validation: {window.ActualWidth}x{window.ActualHeight} DIP; scaling {VisualTreeHelper.GetDpi(window).DpiScaleX * 100}%; display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP.");
            WpfTestHost.FindByName<Button>(window, "MaterialsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<RawMaterialsView>(window).Single();
            var model = view.ViewModel;
            var material = model.VisibleMaterials.First();
            model.BeginEdit(material);
            model.EditorCost = 8.5m;
            WpfTestHost.CompleteLayout(window);
            var cost = WpfTestHost.FindByName<TextBox>(view, "EditorCostBox");
            var category = WpfTestHost.FindByAutomationName<ComboBox>(view, "تصنيف المادة الخام");
            var unit = WpfTestHost.FindByAutomationName<ComboBox>(view, "وحدة المادة الخام");
            var save = WpfTestHost.FindByName<Button>(view, "SaveEditorButton");
            var summary = WpfTestHost.FindByName<TextBlock>(view, "EditorSummary");
            void Submit() { save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window); }
            cost.SetCurrentValue(TextBox.TextProperty, "8.5");
            Submit();
            Assert.AreEqual(0, submissions);
            Assert.AreEqual("8.5", cost.Text);
            Assert.AreEqual(material.CategoryId, category.SelectedValue);
            Assert.AreEqual(material.UnitId, unit.SelectedValue);
            Assert.AreEqual(string.Empty, model.EditorCategoryError);
            Assert.AreEqual(string.Empty, model.EditorUnitError);
            Assert.AreEqual(Visibility.Collapsed, summary.Visibility);
            CheckError("EditorCostField", cost, model.EditorCostError);
            WpfTestHost.Capture(window, "material-field-error");

            foreach (var invalid in new[] { "", "8,5", "٨٫٥", "1e3", "9223372036854775808" })
            {
                cost.SetCurrentValue(TextBox.TextProperty, invalid);
                Submit();
                Assert.AreEqual(0, submissions, invalid);
                Assert.AreEqual(invalid, cost.Text);
                Assert.IsFalse(string.IsNullOrEmpty(model.EditorCostError));
            }
            cost.SetCurrentValue(TextBox.TextProperty, "8500");
            category.SelectedIndex = -1;
            unit.SelectedIndex = -1;
            Submit();
            Assert.AreEqual(0, submissions);
            Assert.AreEqual(string.Empty, model.EditorCostError);
            CheckError("EditorCategoryField", category, model.EditorCategoryError);
            CheckError("EditorUnitField", unit, model.EditorUnitError);

            category.SelectedValue = material.CategoryId;
            Submit();
            Assert.AreEqual(0, submissions);
            Assert.AreEqual(string.Empty, model.EditorCategoryError);
            Assert.IsFalse(string.IsNullOrEmpty(model.EditorUnitError));
            unit.SelectedValue = material.UnitId;
            foreach (var (input, expected) in new[] { ("٨٥٠٠", 8500L), ("۰", 0L), ("9223372036854775807", long.MaxValue) })
            {
                cost.SetCurrentValue(TextBox.TextProperty, input);
                cost.GetBindingExpression(TextBox.TextProperty)!.UpdateSource();
                Assert.IsFalse(Validation.GetHasError(cost), "Valid whole-YER text must not show a culture conversion error.");
                if (expected == 8500L) WpfTestHost.Capture(window, $"material-valid-cost-{width}");
                Submit();
                Assert.AreEqual(expected, engine.LastCommand!.AsMaterialSave()!.CurrentCostYer);
                Assert.IsTrue(model.IsEditorOpen);
                Assert.IsTrue(model.HasEditorSummaryError);
                Assert.AreEqual(Visibility.Visible, summary.Visibility);
                Assert.AreEqual(MaterialClient.ArabicMessage(MaterialFailureKind.Denied), summary.Text);
                Assert.AreEqual(string.Empty, model.EditorCategoryError);
                Assert.AreEqual(string.Empty, model.EditorUnitError);
                Assert.AreEqual(string.Empty, model.EditorCostError);
            }
            Assert.AreEqual(3, submissions);
            model.BeginEdit(material);
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(model.HasEditorError);

            void CheckError(string fieldName, Control input, string error)
            {
                Assert.IsFalse(string.IsNullOrEmpty(error));
                var field = WpfTestHost.FindByName<FormField>(view, fieldName);
                Assert.AreEqual(error, field.ErrorText);
                Assert.AreEqual(error, AutomationProperties.GetItemStatus(input));
                Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(input)));
                Assert.IsNotNull(AutomationProperties.GetLabeledBy(input));
            }
        }, engine: engine);
    }

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

            var categorySelector = WpfTestHost.FindByAutomationName<ComboBox>(view, "تصنيف المادة الخام");
            var unitSelector = WpfTestHost.FindByAutomationName<ComboBox>(view, "وحدة المادة الخام");
            Assert.AreEqual(category.Id, categorySelector.SelectedValue);
            Assert.AreEqual(unit.Id, unitSelector.SelectedValue);
            view.ViewModel.ApplyDurableData(new MaterialReferences
            {
                Categories =
                [
                    new MaterialCategory { Id = category.Id, Scope = scope, Name = "خشب طبيعي", Revision = 2 },
                    new MaterialCategory { Id = Guid.NewGuid(), Scope = scope, Name = category.Name, Revision = 1 },
                ],
                Units =
                [
                    new MaterialUnit { Id = unit.Id, Scope = scope, Name = "متر طولي", Revision = 2 },
                    new MaterialUnit { Id = Guid.NewGuid(), Scope = scope, Name = unit.Name, Revision = 1 },
                ],
            }, [material]);
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(category.Id, categorySelector.SelectedValue);
            Assert.AreEqual(unit.Id, unitSelector.SelectedValue);

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
