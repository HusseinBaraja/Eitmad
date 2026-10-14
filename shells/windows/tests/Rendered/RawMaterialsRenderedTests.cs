using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.RawMaterials;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class RawMaterialsRenderedTests
{
    [TestMethod]
    public void InputErrorsStayWithTheirFieldsAndAuthorityFailuresUseSummary()
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
        WpfTestHost.Run(1920, 1080, window =>
        {
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
            cost.Text = "8.5";
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
                cost.Text = invalid;
                Submit();
                Assert.AreEqual(0, submissions, invalid);
                Assert.AreEqual(invalid, cost.Text);
                Assert.IsFalse(string.IsNullOrEmpty(model.EditorCostError));
            }
            cost.Text = "8500";
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
                cost.Text = input;
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
                var message = (TextBlock)field.Template.FindName("Error", field);
                Assert.AreEqual(AutomationLiveSetting.Assertive, AutomationProperties.GetLiveSetting(message));
                Assert.AreEqual(Visibility.Visible, message.Visibility);
                var messageBounds = message.TransformToAncestor(field).TransformBounds(new Rect(message.RenderSize));
                var inputBounds = input.TransformToAncestor(field).TransformBounds(new Rect(input.RenderSize));
                Assert.IsTrue(messageBounds.Top >= inputBounds.Bottom && messageBounds.Top - inputBounds.Bottom <= 12);
                AssertInside(field, WpfTestHost.Descendants<ScrollContentPresenter>(WpfTestHost.FindByName<ScrollViewer>(view, "EditorScrollViewer")).First());
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
            Console.WriteLine($"Materials rendered {window.ActualWidth}x{window.ActualHeight} DIP; scaling {VisualTreeHelper.GetDpi(window).DpiScaleX * 100}%; primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP.");
            WpfTestHost.FindByName<Button>(window, "MaterialsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<RawMaterialsView>(window).Single();
            Assert.HasCount(1, view.ViewModel.VisibleMaterials);
            Assert.AreEqual("ر.ي 8,000", view.ViewModel.VisibleMaterials.Single().CostLabel);
            WpfTestHost.Capture(window, $"material-durable-list-{width}x{height}");

            view.ViewModel.BeginEdit(view.ViewModel.VisibleMaterials.Single());
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            var editor = WpfTestHost.FindByName<Grid>(view, "EditorSurface");
            var scroll = WpfTestHost.FindByName<ScrollViewer>(view, "EditorScrollViewer");
            var save = WpfTestHost.FindByName<Button>(view, "SaveEditorButton");
            var cancel = WpfTestHost.FindByName<Button>(view, "CancelEditorButton");
            var cost = WpfTestHost.FindByName<TextBox>(view, "EditorCostBox");
            var viewport = WpfTestHost.Descendants<ScrollContentPresenter>(scroll).First();
            AssertInside(save, editor);
            AssertInside(cancel, editor);
            AssertInside(cost, viewport);
            WpfTestHost.Capture(window, $"material-durable-editor-{width}x{height}");

            var actionPosition = save.TranslatePoint(new Point(), editor);
            scroll.ScrollToBottom();
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(actionPosition, save.TranslatePoint(new Point(), editor));
            scroll.ScrollToTop();
            cost.Text = "8.5";
            save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(view.ViewModel.HasEditorError);
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.AreEqual("8.5", cost.Text);
            var costField = WpfTestHost.FindByName<FormField>(view, "EditorCostField");
            AssertInside(costField, viewport);
            AssertInside(save, editor);
            AssertInside(cancel, editor);
            WpfTestHost.Capture(window, $"material-durable-editor-error-{width}x{height}");
            if (width == 720)
            {
                cost.FontSize = 24;
                WpfTestHost.CompleteLayout(window);
                scroll.ScrollToTop();
                view.ViewModel.Fail("أدخل تكلفة صحيحة بالريال اليمني ثم حاول الحفظ مرة أخرى.");
                WpfTestHost.CompleteLayout(window);
                AssertInside(costField, viewport);
                AssertInside(save, editor);
                AssertInside(cancel, editor);
            }
            view.ViewModel.BeginEdit(view.ViewModel.VisibleMaterials.Single());

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
                    new MaterialUnit { Id = unit.Id, Scope = scope, Name = "متر طولي", Dimension = unit.Dimension,
                        Numerator = unit.Numerator, Denominator = unit.Denominator, Revision = 2 },
                    new MaterialUnit { Id = Guid.NewGuid(), Scope = scope, Name = unit.Name, Dimension = unit.Dimension,
                        Numerator = unit.Numerator, Denominator = unit.Denominator, Revision = 1 },
                    new MaterialUnit { Id = Guid.NewGuid(), Scope = scope, Name = "سنتيمتر مربع", Dimension = UnitDimension.Area,
                        Numerator = 1, Denominator = 10_000, Revision = 1 },
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
            var conversion = WpfTestHost.FindByName<StackPanel>(view, "UnitConversionGroup");
            var referenceScroll = WpfTestHost.Ancestor<ScrollViewer>(conversion);
            referenceScroll.ScrollToBottom();
            WpfTestHost.CompleteLayout(window);
            AssertInside(conversion, WpfTestHost.Descendants<ScrollContentPresenter>(referenceScroll).First());
            WpfTestHost.Capture(window, $"material-unit-conversion-count-{width}x{height}");

            view.ViewModel.ReferenceDimension = UnitDimension.Length;
            WpfTestHost.CompleteLayout(window);
            referenceScroll.ScrollToBottom();
            WpfTestHost.CompleteLayout(window);
            AssertInside(conversion, WpfTestHost.Descendants<ScrollContentPresenter>(referenceScroll).First());
            WpfTestHost.Capture(window, $"material-unit-conversion-length-{width}x{height}");
            view.ViewModel.ReferenceDimension = UnitDimension.Area;
            WpfTestHost.CompleteLayout(window);
            referenceScroll.ScrollToBottom();
            WpfTestHost.CompleteLayout(window);
            AssertInside(conversion, WpfTestHost.Descendants<ScrollContentPresenter>(referenceScroll).First());
            WpfTestHost.Capture(window, $"material-unit-conversion-area-{width}x{height}");
            if (width == 720)
            {
                foreach (var field in WpfTestHost.Descendants<FormField>(conversion))
                    ControlOptions.SetHighContrast(field, true);
                WpfTestHost.CompleteLayout(window);
                referenceScroll.ScrollToBottom();
                WpfTestHost.CompleteLayout(window);
                AssertInside(conversion, WpfTestHost.Descendants<ScrollContentPresenter>(referenceScroll).First());
                WpfTestHost.Capture(window, "material-unit-conversion-larger-text-system-colors-720x560");
            }
            view.ViewModel.CancelReferenceEditor();
            view.ViewModel.BeginManageUnits();
            view.ViewModel.BeginEditReference(view.ViewModel.Units.Single(item => item.Dimension == UnitDimension.Area));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("1", WpfTestHost.FindByName<TextBox>(view, "UnitNumeratorBox").Text);
            Assert.AreEqual("10000", WpfTestHost.FindByName<TextBox>(view, "UnitDenominatorBox").Text);
        }, engine: engine);
    }

    private static void AssertInside(FrameworkElement element, FrameworkElement viewport)
    {
        var bounds = element.TransformToAncestor(viewport)
            .TransformBounds(new Rect(element.RenderSize));
        Assert.IsTrue(bounds.Width > 0 && bounds.Height > 0
            && bounds.Left >= -1 && bounds.Top >= -1
            && bounds.Right <= viewport.ActualWidth + 1 && bounds.Bottom <= viewport.ActualHeight + 1,
            $"{element.Name} is outside the viewport: {bounds}, viewport {viewport.RenderSize}.");
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
