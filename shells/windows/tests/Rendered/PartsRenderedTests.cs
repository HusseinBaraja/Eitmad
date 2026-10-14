using System.Windows;
using System.Windows.Automation;
using System.Windows.Automation.Peers;
using System.Windows.Automation.Provider;
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
    [DataRow(1920, 1080)]
    [DataRow(1536, 864)]
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
            AssertFooterVisible(view);
            var geometry = $"Requested: {width}x{height}; actual window: {window.ActualWidth}x{window.ActualHeight} DIP; "
                + $"display scaling {VisualTreeHelper.GetDpi(window).DpiScaleX * 100}%; "
                + $"available display: {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP.";
            Console.WriteLine(geometry);
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
            {
                System.IO.Directory.CreateDirectory(directory);
                System.IO.File.WriteAllText(System.IO.Path.Combine(directory, $"part-geometry-{width}x{height}.txt"), geometry);
            }
            WpfTestHost.Capture(window,$"part-information-{width}x{height}");

            editorName.Text = "جانب خزانة";
            WpfTestHost.FindByAutomationName<Button>(view, "التالي إلى المواد الخام")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(2, view.ViewModel.CurrentStep);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Border>(view, "MaterialsStep").Visibility);
            AssertFooterVisible(view);

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
            var materialsStep = WpfTestHost.FindByName<Border>(view, "MaterialsStep");
            AssertCostPresentation(materialsStep, pending: true);
            WpfTestHost.Capture(window,$"part-materials-{width}x{height}");
            var materialRows = WpfTestHost.FindByName<Controls.OperationsTable>(view, "MaterialRows");
            var unitCost = WpfTestHost.Descendants<Controls.AmountDisplay>(materialRows)
                .First(amount => amount.AmountText == view.ViewModel.SelectedMaterials[0].UnitCostLabel);
            unitCost.BringIntoView();
            WpfTestHost.CompleteLayout(window);
            var costPoint = unitCost.TranslatePoint(new Point(), materialRows);
            Assert.IsTrue(costPoint.X >= 0 && costPoint.X + unitCost.ActualWidth <= materialRows.ActualWidth);
            Assert.AreEqual(0, ((ScrollViewer)materialRows.Template.FindName("DG_ScrollViewer", materialRows)).ScrollableWidth, 1);
            WpfTestHost.Capture(window, $"ui04-material-costs-{width}x{height}");

            if (width == 720)
            {
                WpfTestHost.Descendants<TextBlock>(materialsStep)
                    .Single(text => text.Text == "تكلفة الجزء").BringIntoView();
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, $"part-materials-total-{width}x{height}");
            }
            if (width == 1338)
            {
                var costText = WpfTestHost.Descendants<TextBlock>(materialsStep)
                    .Where(text => text.Text == "بانتظار المراجعة")
                    .Select(text => (Text: text, Size: text.FontSize)).ToArray();
                foreach (var item in costText) item.Text.FontSize = item.Size * 1.25;
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "part-materials-enlarged-cost-text-1338x753");
                foreach (var item in costText) item.Text.FontSize = item.Size;
                WpfTestHost.CompleteLayout(window);
            }

            WpfTestHost.FindByAutomationName<Button>(view, "التالي إلى المراجعة")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(3, view.ViewModel.CurrentStep);
            AssertFooterVisible(view);
            Assert.AreEqual("9,450",view.ViewModel.TotalPartCostLabel);
            AssertCostPresentation(WpfTestHost.FindByName<Border>(view, "ReviewStep"), pending: false);
            WpfTestHost.Capture(window,$"part-review-{width}x{height}");
            if (width == 720)
            {
                WpfTestHost.Descendants<Controls.AmountDisplay>(view)
                    .First(amount => amount.IsVisible && amount.AmountText == "9,450").BringIntoView();
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, $"part-review-total-{width}x{height}");
            }
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Border>(view, "ReviewStep").Visibility);

            view.ViewModel.MoveToPreviousStep();
            WpfTestHost.CompleteLayout(window);
            AssertCostPresentation(materialsStep, pending: false);
            view.ViewModel.SelectedMaterials[0].Quantity = "1.200";
            WpfTestHost.CompleteLayout(window);
            AssertCostPresentation(materialsStep, pending: true);
            view.ViewModel.SelectedMaterials[0].Quantity = "1.2";
            WpfTestHost.FindByAutomationName<Button>(view, "التالي إلى المراجعة")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(3, view.ViewModel.CurrentStep);

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
    public void CategoryDisclosurePreservesDraftAndFixedNavigation()
    {
        var fixture = new PartFixtures();
        WpfTestHost.Run(1920, 1080, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PartsNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PartsView>(window).Single();
            WpfTestHost.FindByAutomationName<Button>(view, "إضافة جزء")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var name = WpfTestHost.FindByName<TextBox>(view, "EditorNameBox");
            Assert.IsTrue(name.IsKeyboardFocusWithin);
            var next = WpfTestHost.FindByAutomationName<Button>(view, "التالي إلى المواد الخام");

            name.Text = "جانب خزانة";
            var notes = WpfTestHost.FindByAutomationName<TextBox>(view, "وصف الجزء أو ملاحظاته");
            notes.Text = "جانب خزانة تجريبي";
            var category = view.ViewModel.EditorCategory;
            var disclosure = WpfTestHost.FindByName<Expander>(view, "CategoryCreation");
            var peer = new ExpanderAutomationPeer(disclosure);
            Assert.AreEqual("إظهار فئة جزء جديدة", peer.GetName());
            var provider = (IExpandCollapseProvider)peer.GetPattern(PatternInterface.ExpandCollapse);
            Assert.AreEqual(ExpandCollapseState.Collapsed, provider.ExpandCollapseState);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "part-information-filled");
            var footer = WpfTestHost.FindByName<Border>(view, "EditorFooter");
            var footerY = footer.TranslatePoint(new Point(), view).Y;

            provider.Expand();
            WpfTestHost.CompleteLayout(window);
            var header = WpfTestHost.Descendants<ToggleButton>(disclosure).Single();
            Assert.IsTrue(header.Focus());
            Assert.IsNotNull(header.FocusVisualStyle);
            Assert.IsTrue(header.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
            var newCategory = WpfTestHost.FindByName<TextBox>(view, "NewCategoryNameBox");
            Assert.IsTrue(newCategory.IsKeyboardFocusWithin, $"Focused after category header: {Keyboard.FocusedElement}");
            newCategory.Text = "أجزاء خزانة";
            provider.Collapse();
            provider.Expand();
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("أجزاء خزانة", newCategory.Text);
            Assert.AreEqual("جانب خزانة", name.Text);
            Assert.AreEqual("جانب خزانة تجريبي", notes.Text);
            Assert.AreSame(category, view.ViewModel.EditorCategory);
            AssertFooterVisible(view);
            WpfTestHost.Capture(window, "part-information-category-expanded");

            var scroll = WpfTestHost.FindByName<ScrollViewer>(view, "EditorScroll");
            scroll.ScrollToEnd();
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(footerY, footer.TranslatePoint(new Point(), view).Y, 0.1);
            AssertFooterVisible(view);
            next.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByAutomationName<Button>(view, "السابق إلى المعلومات")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("أجزاء خزانة", view.ViewModel.NewCategoryName);
            Assert.AreEqual("جانب خزانة تجريبي", view.ViewModel.EditorDescription);
            AssertFooterVisible(view);
            name.Text = "";
            next.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.AreEqual(1, view.ViewModel.CurrentStep);
            Assert.IsTrue(name.IsKeyboardFocusWithin);
        }, engine: fixture.Engine());
    }

    private static void AssertFooterVisible(PartsView view)
    {
        var footer = WpfTestHost.FindByName<Border>(view, "EditorFooter");
        foreach (var button in WpfTestHost.Descendants<Button>(footer).Where(button => button.IsVisible))
        {
            var bounds = button.TransformToAncestor(view).TransformBounds(new Rect(button.RenderSize));
            Assert.IsTrue(bounds.Top >= 0 && bounds.Bottom <= view.ActualHeight + 0.1);
            Assert.IsTrue(bounds.Left >= 0 && bounds.Right <= view.ActualWidth + 0.1);
        }
    }

    private static void AssertCostPresentation(Border step, bool pending)
    {
        var pendingLabels = WpfTestHost.Descendants<TextBlock>(step)
            .Where(text => text.Text == "بانتظار المراجعة" && text.IsVisible).ToArray();
        Assert.HasCount(pending ? 3 : 0, pendingLabels);
        var amounts = WpfTestHost.Descendants<Controls.AmountDisplay>(step).Where(amount => amount.IsVisible).ToArray();
        Assert.IsFalse(amounts.Any(amount => amount.AmountText == "—"));
        if (!pending)
        {
            Assert.IsTrue(amounts.Any(amount => amount.AmountText == "9,450"));
            Assert.IsTrue(amounts.Any(amount => amount.AmountText == "8,700"));
            Assert.IsTrue(amounts.Any(amount => amount.AmountText == "750"));
        }
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
