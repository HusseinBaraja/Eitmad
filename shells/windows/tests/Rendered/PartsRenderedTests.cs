using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using Eitmad.WindowsShell.Features.Parts;
using Eitmad.WindowsShell.Tests.Parts;
using Eitmad.Contracts;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class PartsRenderedTests
{
    [TestMethod]
    public void MaterialPickerClosePreservesCompositionAndSelectionReturnsFocus()
    {
        var fixture = new PartFixtures();
        var engine = fixture.Engine();
        var query = engine.QueryHandler!;
        engine.QueryHandler = input => input.Kind == Query.MaterialListKind && input.AsMaterialList()!.Term.Length > 0
            ? PartFixtures.Success(QueryResult.ForMaterials(new MaterialPage { Items = [] })) : query(input);
        WpfTestHost.Run(1920, 1080, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PartsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PartsView>(window).Single();
            view.ViewModel.BeginCreate();
            view.ViewModel.EditorName = "جانب خزانة تجريبي";
            Assert.IsTrue(view.ViewModel.MoveToMaterials());
            view.ViewModel.AddMaterial(view.ViewModel.FilteredMaterials[0]);
            var retained = view.ViewModel.SelectedMaterials[0];
            retained.Quantity = "1.2";
            WpfTestHost.CompleteLayout(window);
            var invoker = WpfTestHost.FindByAutomationName<Button>(view, "إضافة مادة خام");
            foreach (var noMatch in new[] { false, true })
            {
                invoker.Focus();
                invoker.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                var search = WpfTestHost.FindByName<TextBox>(view, "MaterialSearchBox");
                Assert.IsTrue(search.IsKeyboardFocused);
                if (noMatch) search.Text = "مادة غير موجودة";
                WpfTestHost.CompleteLayout(window);
                Assert.HasCount(noMatch ? 0 : 1, view.ViewModel.FilteredMaterials);
                var dialog = WpfTestHost.Descendants<Controls.DialogHost>(view).Single(host => host.IsOpen);
                var close = WpfTestHost.FindByAutomationName<Button>(dialog, "إغلاق اختيار المادة الخام");
                Assert.IsTrue(close.IsVisible && close.IsEnabled);
                var focusStops = WpfTestHost.Descendants<UIElement>(dialog).Count(element => element.IsVisible && element.IsEnabled && element.Focusable);
                for (var index = 0; index < focusStops && !close.IsKeyboardFocused; index++)
                {
                    ((UIElement)Keyboard.FocusedElement).MoveFocus(new TraversalRequest(FocusNavigationDirection.Next));
                    Assert.IsTrue(dialog.IsKeyboardFocusWithin);
                }
                Assert.IsTrue(close.IsKeyboardFocused, "The footer is reachable within the dialog focus scope.");
                Assert.IsTrue(((Border)close.Template.FindName("FocusRing", close)).IsVisible);
                close.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                Assert.IsFalse(view.ViewModel.IsMaterialPickerOpen);
                Assert.IsTrue(invoker.IsKeyboardFocused);
                Assert.HasCount(1, view.ViewModel.SelectedMaterials);
                Assert.AreSame(retained, view.ViewModel.SelectedMaterials[0]);
                Assert.AreEqual("1.2", retained.Quantity);
            }
            invoker.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var picker = WpfTestHost.Descendants<Controls.DialogHost>(view).Single(host => host.IsOpen);
            picker.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(picker), 0, Key.Escape)
            { RoutedEvent = Keyboard.KeyDownEvent });
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(view.ViewModel.IsMaterialPickerOpen);
            Assert.IsTrue(invoker.IsKeyboardFocused);
            Assert.HasCount(1, view.ViewModel.SelectedMaterials);
            Assert.AreSame(retained, view.ViewModel.SelectedMaterials[0]);
            Assert.AreEqual("1.2", retained.Quantity);
            invoker.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var choice = view.ViewModel.FilteredMaterials.Single();
            WpfTestHost.FindByAutomationName<Button>(view, "اختيار المادة الخام").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(view.ViewModel.IsMaterialPickerOpen);
            Assert.IsTrue(invoker.IsKeyboardFocused);
            Assert.HasCount(2, view.ViewModel.SelectedMaterials);
            Assert.AreSame(retained, view.ViewModel.SelectedMaterials[0]);
            Assert.AreEqual(choice.Id, view.ViewModel.SelectedMaterials[1].Material.Id);
            Assert.IsNull(engine.LastCommand, "Picker actions must not submit saved product state.");
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void CreateWizardRendersAccessibleStepsAndMaterialPicker(int width, int height)
    {
        var fixture = new PartFixtures();
        var engine = fixture.Engine();
        WpfTestHost.Run(width, height, window =>
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
            Console.WriteLine($"Parts rendered window: {window.ActualWidth}x{window.ActualHeight} DIP; display scaling {VisualTreeHelper.GetDpi(window).DpiScaleX * 100}%.");
            WpfTestHost.Capture(window,$"part-information-{width}x{height}");

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
            WpfTestHost.Capture(window,$"part-picker-{width}x{height}");

            WpfTestHost.Descendants<Button>(view)
                .First(button => AutomationProperties.GetName(button) == "اختيار المادة الخام")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.HasCount(1, view.ViewModel.SelectedMaterials);
            view.ViewModel.SelectedMaterials[0].Quantity = "1.2";
            view.ViewModel.OpenMaterialPicker();
            view.ViewModel.AddMaterial(view.ViewModel.FilteredMaterials.Single());
            view.ViewModel.SelectedMaterials[1].Quantity = "3";
            WpfTestHost.CompleteLayout(window);
            var unitSelector = WpfTestHost.FindByAutomationName<ComboBox>(view,"وحدة كمية المادة");
            unitSelector.Focus();
            Assert.IsTrue(unitSelector.IsKeyboardFocusWithin);
            unitSelector.IsDropDownOpen = true; WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(unitSelector.IsDropDownOpen);
            unitSelector.IsDropDownOpen = false;
            WpfTestHost.Capture(window,$"part-materials-{width}x{height}");

            WpfTestHost.FindByAutomationName<Button>(view, "التالي إلى المراجعة")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(3, view.ViewModel.CurrentStep);
            Assert.AreEqual("9,450",view.ViewModel.TotalPartCostLabel);
            WpfTestHost.Capture(window,$"part-review-{width}x{height}");
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Border>(view, "ReviewStep").Visibility);

            if (width == 1338)
            {
                var accepted = engine.CommandHandler;
                engine.CommandHandler = _ => new CommandResponseEnvelope
                {
                    RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(),
                    Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Failed, Payload = new CommandResult { Code = ProtocolIds.ErrorCodes.EitmadErrorPartRevisionConflictV1 } },
                };
                var save = WpfTestHost.FindByAutomationName<Button>(view,"حفظ الجزء");
                save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(view);
                Assert.IsTrue(view.ViewModel.IsEditorOpen);
                Assert.AreEqual("1.2",view.ViewModel.SelectedMaterials[0].Quantity);
                Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(WpfTestHost.FindByName<Border>(view,"ReviewStep"))
                    .Any(t => t.Text == view.ViewModel.EditorError && t.IsVisible));
                engine.CommandHandler = _ => throw new System.IO.IOException("synthetic lost response");
                save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(view);
                Assert.IsTrue(view.ViewModel.SavePending);
                Assert.IsFalse(WpfTestHost.FindByAutomationName<Button>(view,"السابق إلى المواد الخام").IsEnabled);
                var retryKey = engine.LastIdempotencyKey;
                engine.CommandHandler = accepted;
                save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(view);
                Assert.AreEqual(retryKey,engine.LastIdempotencyKey);
            }
            else
            {

            WpfTestHost.FindByAutomationName<Button>(view, "حفظ الجزء")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            }
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            var sent = engine.LastCommand!.AsPartSave()!;
            Assert.AreEqual("1.2",sent.Usages[0].Quantity);
            Assert.HasCount(2,sent.Usages);
            var saved = view.ViewModel.VisibleParts.Single(p => p.Name == "جانب خزانة");
            Assert.AreEqual(PartsViewModel.AllCategories,WpfTestHost.FindByAutomationName<ComboBox>(view,"تصفية الفئة").SelectedItem);
            Assert.AreEqual(PartsViewModel.AllCategories,WpfTestHost.FindByAutomationName<ComboBox>(view,"تصفية الفئة").SelectionBoxItem);
            WpfTestHost.Capture(window,$"part-list-{width}x{height}");
            view.ViewModel.BeginEdit(saved);
            Assert.HasCount(2,view.ViewModel.SelectedMaterials);
            Assert.AreEqual("1.2",view.ViewModel.SelectedMaterials[0].Quantity);
        }, engine: engine);
    }

    [TestMethod]
    public void RowActionPopupUsesMousePlacementAndNonDestructiveActions()
    {
        var fixture = new PartFixtures();
        var engine = fixture.Engine();
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
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(PlacementMode.MousePoint, action.ContextMenu.Placement);
            CollectionAssert.AreEquivalent(
                new[] { "تعديل", "تكرار", "أرشفة" },
                action.ContextMenu.Items.OfType<MenuItem>().Select(item => item.Header).Cast<string>().ToArray());
        }, engine: engine);
    }

    [TestMethod]
    public void KeyboardRowActivationOpensTheExistingPart()
    {
        var fixture = new PartFixtures();
        var engine = fixture.Engine();
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PartsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PartsView>(window).Single();
            var table = WpfTestHost.FindByName<DataGrid>(view, "PartsTable");
            table.SelectedIndex = 0;

            table.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(table), 0, Key.Enter)
            {
                RoutedEvent = Keyboard.PreviewKeyDownEvent
            });
            WpfTestHost.PumpDispatcher();

            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(((PartListItem)table.SelectedItem).Name, view.ViewModel.EditorName);
        }, engine: engine);
    }

    [TestMethod]
    public void PointerRowActivationOpensTheExistingPart()
    {
        var fixture = new PartFixtures();
        var engine = fixture.Engine();
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PartsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PartsView>(window).Single();
            var table = WpfTestHost.FindByName<DataGrid>(view, "PartsTable");
            var row = WpfTestHost.Descendants<DataGridRow>(table).First();
            var part = (PartListItem)row.DataContext;

            row.RaiseEvent(new MouseButtonEventArgs(Mouse.PrimaryDevice, 0, MouseButton.Left)
            {
                RoutedEvent = Mouse.PreviewMouseUpEvent
            });
            WpfTestHost.PumpDispatcher();

            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(part.Name, view.ViewModel.EditorName);
        }, engine: engine);
    }
}
