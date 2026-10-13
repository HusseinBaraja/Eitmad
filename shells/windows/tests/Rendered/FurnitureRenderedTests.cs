using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Media;
using System.IO;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Tests.Furniture;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class FurnitureRenderedTests
{
    [TestMethod]
    [DataRow(720, 560, false)]
    [DataRow(720, 560, true)]
    [DataRow(1338, 753, false)]
    [DataRow(1338, 753, true)]
    [DataRow(1920, 1080, false)]
    [DataRow(1920, 1080, true)]
    public void InformationStartsWithIdentifyingFieldsAndKeepsImageReachable(int width, int height, bool edit)
    {
        var fixture = new FurnitureFixtures();
        if (edit) fixture.Seed();
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
            if (edit)
            {
                var item = view.ViewModel.VisibleFurniture.Single();
                Assert.IsTrue(view.PrepareEditorAsync(item).GetAwaiter().GetResult());
                view.ViewModel.BeginEdit(item);
            }
            else Click(view, "إضافة منتج");
            WpfTestHost.CompleteLayout(window);
            var name = WpfTestHost.FindByName<TextBox>(view, "FurnitureNameBox");
            var category = WpfTestHost.FindByAutomationName<ComboBox>(view, "فئة الأثاث");
            var scroll = WpfTestHost.Ancestor<ScrollViewer>(name);
            scroll.ScrollToTop(); WpfTestHost.CompleteLayout(window);
            var viewport = (ScrollContentPresenter)scroll.Template.FindName("PART_ScrollContentPresenter", scroll);
            var dpi = VisualTreeHelper.GetDpi(window);
            var geometry = $"Requested: {width}x{height}; actual window: {window.ActualWidth}x{window.ActualHeight} DIP; "
                + $"scaling: {dpi.DpiScaleX * 100}%; primary display: {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP.";
            Console.WriteLine(geometry);
            var state = edit ? "edit" : "add";
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
            {
                Directory.CreateDirectory(directory);
                File.WriteAllText(Path.Combine(directory, $"ui18-{state}-{width}x{height}.txt"), geometry);
            }
            WpfTestHost.Capture(window, $"ui18-{state}-{width}x{height}");
            foreach (var field in new FrameworkElement[] { name, category })
            {
                var bounds = field.TransformToAncestor(viewport).TransformBounds(new Rect(field.RenderSize));
                Assert.IsTrue(bounds.Top >= -1 && bounds.Bottom <= viewport.ActualHeight + 1,
                    $"Identifying fields must be visible before scrolling: {bounds}; viewport height {viewport.ActualHeight}.");
            }
            var next = WpfTestHost.FindByName<Button>(view, "NextButton");
            var nextBounds = next.TransformToAncestor(view).TransformBounds(new Rect(next.RenderSize));
            Assert.IsTrue(nextBounds.Top >= 0 && nextBounds.Bottom <= view.ActualHeight + 1);
            var choose = WpfTestHost.FindByAutomationName<Button>(view, "اختيار صورة الأثاث");
            if (width > 720)
                Assert.IsTrue(name.PointToScreen(new Point()).X > choose.PointToScreen(new Point()).X,
                    "The wider layout must retain information on the right and image controls on the left.");
            choose.BringIntoView(); WpfTestHost.CompleteLayout(window);
            var chooseBounds = choose.TransformToAncestor(viewport).TransformBounds(new Rect(choose.RenderSize));
            Assert.IsTrue(chooseBounds.Top >= -1 && chooseBounds.Bottom <= viewport.ActualHeight + 1);
            WpfTestHost.Capture(window, $"ui18-image-{state}-{width}x{height}");
            if (width == 720 && !edit)
            {
                name.FontSize *= 1.5; category.FontSize *= 1.5;
                foreach (var field in new FrameworkElement[] { name, category, choose })
                {
                    field.BringIntoView(); WpfTestHost.CompleteLayout(window);
                    var bounds = field.TransformToAncestor(viewport).TransformBounds(new Rect(field.RenderSize));
                    Assert.IsTrue(bounds.Top >= -1 && bounds.Bottom <= viewport.ActualHeight + 1,
                        "Larger text must leave each changed field and the image action reachable.");
                }
                name.BringIntoView(); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "ui18-information-enlarged-text-720x560");
            }
        }, engine: fixture.Engine());
    }

    [TestMethod]
    [DataRow("add")]
    [DataRow("edit")]
    [DataRow("duplicate")]
    [DataRow("remove")]
    public void VariantChangesRejectDelayedReviewMargins(string mutation)
    {
        var f = new FurnitureFixtures(); var saved = f.Seed();
        saved.Variants = [saved.Variants[0], new Eitmad.Contracts.FurnitureVariant
        {
            Id = Guid.NewGuid(), Name = "كبير", Dimensions = saved.Variants[0].Dimensions,
            SellingPriceYer = 300000, ColorIds = [], HandleIds = [],
        }];
        var engine = f.Engine();
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
            Assert.IsTrue(view.PrepareEditorAsync(view.ViewModel.VisibleFurniture.Single()).GetAwaiter().GetResult());
            view.ViewModel.BeginEdit(view.ViewModel.VisibleFurniture.Single());
            var requests = new List<Query>();
            var pending = new[] { new TaskCompletionSource(), new TaskCompletionSource() };
            var handler = engine.QueryHandler!;
            engine.QueryBarrier = query =>
            {
                if (query.Kind != Query.FurnitureReviewKind) return Task.CompletedTask;
                requests.Add(query);
                return pending[requests.Count - 1].Task;
            };
            engine.QueryHandler = query => query.Kind != Query.FurnitureReviewKind ? handler(query)
                : Parts.PartFixtures.Success(QueryResult.ForFurnitureReview(ReferenceEquals(query, requests[0])
                    ? new FurnitureReview { PartsCostYer = 11111, RowCostsYer = [11111], MarginsYer = [111, 222] }
                    : new FurnitureReview { PartsCostYer = 22222, RowCostsYer = [22222], MarginsYer = Enumerable.Repeat(333L, query.AsFurnitureReview()!.Variants.Length).ToArray() }));
            try
            {
                view.ViewModel.Variants[0].SellingPrice = 210000;
                Assert.HasCount(1, requests);
                var first = view.ViewModel.Variants[0];
                switch (mutation)
                {
                    case "add":
                        view.ViewModel.BeginAddVariant(); view.ViewModel.VariantName = "متوسط";
                        Assert.IsTrue(view.ViewModel.SaveVariant()); break;
                    case "edit":
                        view.ViewModel.BeginEditVariant(first); view.ViewModel.VariantWidth = 130;
                        Assert.IsTrue(view.ViewModel.SaveVariant()); break;
                    case "duplicate": view.ViewModel.DuplicateVariant(first); break;
                    case "remove": view.ViewModel.RemoveVariant(first); break;
                }
                Assert.HasCount(2, requests, "Each staged variant change must request a review of the current input.");
                pending[1].SetResult(); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(22222m, view.ViewModel.CurrentPartsCost);
                Assert.IsTrue(view.ViewModel.Variants.All(v => v.Margin == 333m));
                pending[0].SetResult(); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(22222m, view.ViewModel.CurrentPartsCost, "The older review must not replace current costs.");
                Assert.IsTrue(view.ViewModel.Variants.All(v => v.Margin == 333m), "The older review must not assign margins by obsolete positions.");
            }
            finally { foreach (var response in pending) response.TrySetResult(); }
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void CompleteEditorSavesReopensAndKeepsConflictAndRetryFields(int width, int height)
    {
        var f = new FurnitureFixtures(); var engine = f.Engine();
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } capture)
            {
                Directory.CreateDirectory(capture); var dpi = VisualTreeHelper.GetDpi(window);
                File.WriteAllText(Path.Combine(capture, $"geometry-{width}x{height}.txt"), $"Window: {window.ActualWidth} x {window.ActualHeight} DIPs; display scaling: {dpi.DpiScaleX * 100}%.");
            }
            var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
            Assert.IsTrue(view.ViewModel.CanManage); Assert.AreEqual("البحث عن أثاث", AutomationProperties.GetName(WpfTestHost.FindByName<TextBox>(view, "FurnitureSearchBox")));
            Click(view, "إضافة منتج"); var name = WpfTestHost.FindByName<TextBox>(view, "FurnitureNameBox"); Assert.IsTrue(name.IsKeyboardFocusWithin); name.Text = "خزانة اختبار";
            view.ViewModel.ShortDescription = "وصف محفوظ"; view.ViewModel.InternalNotes = "ملاحظة محفوظة";
            view.ViewModel.EditorCategory = "فئة غير محفوظة";
            Next(view); Assert.AreEqual(1, view.ViewModel.CurrentStep);
            StringAssert.Contains(view.ViewModel.EditorError, "احفظ الفئة الجديدة");
            view.ViewModel.EditorCategory = f.Category.Name;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"furniture-information-{width}x{height}"); Next(view); Assert.AreEqual(2, view.ViewModel.CurrentStep);
            Click(view, "إضافة جزء للأثاث"); Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "PartSearchBox").IsKeyboardFocusWithin); Click(view, "اختيار الجزء"); view.ViewModel.SelectedParts[0].Quantity = 2;
            Next(view); Assert.AreEqual(3, view.ViewModel.CurrentStep); Click(view, "إضافة مقاس"); WpfTestHost.FindByName<TextBox>(view, "VariantNameBox").Text = "صغير";
            Click(view, "حفظ المقاس"); Next(view); Assert.AreEqual(4, view.ViewModel.CurrentStep);
            Click(view, "إضافة لون"); WpfTestHost.FindByName<TextBox>(view, "ColorNameBox").Text = "أزرق"; Click(view, "حفظ اللون");
            Click(view, "إضافة مقبض"); WpfTestHost.FindByName<TextBox>(view, "HandleNameBox").Text = "فولاذي"; Click(view, "حفظ المقبض");
            var feedback = WpfTestHost.FindByName<Controls.FeedbackNotice>(view, "Feedback");
            var next = WpfTestHost.FindByName<Button>(view, "NextButton");
            var feedbackBounds = feedback.TransformToAncestor(view).TransformBounds(new Rect(feedback.RenderSize));
            var nextBounds = next.TransformToAncestor(view).TransformBounds(new Rect(next.RenderSize));
            Assert.IsFalse(feedbackBounds.IntersectsWith(nextBounds), "Unsaved-field feedback must leave navigation controls usable.");
            WpfTestHost.Capture(window, $"furniture-options-{width}x{height}"); Next(view); Assert.AreEqual(5, view.ViewModel.CurrentStep); Assert.AreEqual(18900m, view.ViewModel.CurrentPartsCost);
            var price = WpfTestHost.FindByAutomationName<TextBox>(view, "سعر بيع المقاس"); price.Text = "غير صالح"; Next(view); Assert.AreEqual(5, view.ViewModel.CurrentStep); Assert.IsTrue(price.IsKeyboardFocusWithin);
            price.Text = "200000"; Next(view); Assert.AreEqual(6, view.ViewModel.CurrentStep); WpfTestHost.Capture(window, $"furniture-review-{width}x{height}");
            if (width == 1338)
            {
                var accepted = engine.CommandHandler; engine.CommandHandler = _ => FurnitureFixtures.Failure(ProtocolIds.ErrorCodes.EitmadErrorFurnitureRevisionConflictV1);
                Click(view, "حفظ تعريف الأثاث"); Assert.IsTrue(view.ViewModel.IsEditorOpen); Assert.AreEqual("خزانة اختبار", view.ViewModel.EditorName); StringAssert.Contains(view.ViewModel.EditorError, "تغيرت البيانات");
                engine.CommandHandler = _ => throw new System.IO.IOException("synthetic lost response"); Click(view, "حفظ تعريف الأثاث"); Assert.IsFalse(view.ViewModel.CanEditFields); var key = engine.LastIdempotencyKey;
                engine.CommandHandler = accepted; Click(view, "حفظ تعريف الأثاث"); Assert.AreEqual(key, engine.LastIdempotencyKey);
            }
            else Click(view, "حفظ تعريف الأثاث");
            Assert.IsTrue(view.ViewModel.IsListVisible); Assert.HasCount(1, view.ViewModel.VisibleFurniture);
            var saved = engine.LastCommand!.AsFurnitureSave()!; Assert.AreEqual(1200L, saved.Variants[0].Dimensions.WidthMm); Assert.AreEqual(2L, saved.Parts[0].Quantity);
            var rows = WpfTestHost.Descendants<Controls.OperationsTable>(view).Single(t => t.IsVisible); Assert.IsTrue(rows.IsVisible); WpfTestHost.Capture(window, $"furniture-list-{width}x{height}");
            view.ViewModel.BeginEdit(view.ViewModel.VisibleFurniture.Single()); Assert.AreEqual("وصف محفوظ", view.ViewModel.ShortDescription); Assert.AreEqual("ملاحظة محفوظة", view.ViewModel.InternalNotes); Assert.AreEqual(2m, view.ViewModel.SelectedParts[0].Quantity);
            Assert.HasCount(4, view.ViewModel.Colors); Assert.HasCount(4, view.ViewModel.Handles); Assert.AreEqual(saved.Colors[0].Id, view.ViewModel.Colors[0].Id);
            if (width == 720)
            {
                view.Resources[SystemParameters.HighContrastKey] = true;
                view.Resources[SystemColors.WindowBrushKey] = Brushes.Black;
                view.Resources[SystemColors.WindowTextBrushKey] = Brushes.White;
                view.Resources[SystemColors.ControlBrushKey] = Brushes.Black;
                view.Resources[SystemColors.ControlTextBrushKey] = Brushes.White;
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "furniture-system-colors-720x560");
            }
        }, engine: engine);
    }
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void VariantCustomizationDialogRendersNativeAccessibleBoundsAndChoices(int width, int height)
    {
        var f = new FurnitureFixtures(); var furniture = f.Seed();
        var engine = f.Engine();
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
            var prepared = view.PrepareEditorAsync(view.ViewModel.VisibleFurniture.Single()); WpfTestHost.CompleteLayout(window); Assert.IsTrue(prepared.GetAwaiter().GetResult());
            view.ViewModel.BeginEdit(view.ViewModel.VisibleFurniture.Single()); view.ViewModel.BeginEditVariant(view.ViewModel.Variants.Single()); WpfTestHost.CompleteLayout(window);
            var dpi = VisualTreeHelper.GetDpi(window);
            var geometry = $"Requested: {width}x{height}; actual window: {window.ActualWidth}x{window.ActualHeight} DIP; "
                + $"display scaling: {dpi.DpiScaleX * 100}%; primary display: {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; "
                + "capture: WPF RenderTargetBitmap at 96 DPI; synthetic engine responses.";
            Console.WriteLine(geometry);
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
            {
                Directory.CreateDirectory(directory);
                File.WriteAllText(Path.Combine(directory, $"ui20-{width}x{height}.txt"), geometry);
            }
            WpfTestHost.Capture(window, $"furniture-variant-{Math.Round(window.ActualWidth)}x{Math.Round(window.ActualHeight)}-{dpi.DpiScaleX * 100}percent");
            var handle = WpfTestHost.FindByAutomationName<CheckBox>(view, furniture.Handles[0].Name);
            var dialog = WpfTestHost.FindByName<Controls.DialogHost>(view, "VariantDialog");
            var scroll = WpfTestHost.Ancestor<ScrollViewer>(handle);
            var handleBounds = handle.TransformToAncestor(scroll).TransformBounds(new Rect(handle.RenderSize));
            Assert.IsTrue(handleBounds.Top >= -1 && handleBounds.Bottom <= scroll.ActualHeight + 1,
                "Both compatibility lists must be visible without scrolling for the standard fixture.");
            var standardColor = WpfTestHost.FindByAutomationName<CheckBox>(view, furniture.Colors[0].Name);
            var colorBounds = standardColor.TransformToAncestor(scroll).TransformBounds(new Rect(standardColor.RenderSize));
            Assert.IsTrue(colorBounds.Top >= -1 && colorBounds.Bottom <= scroll.ActualHeight + 1);
            Click(view, "حفظ المقاس");
            var unchanged = view.ViewModel.SaveInput(FurnitureState.Draft).Variants[0];
            Assert.IsEmpty(unchanged.ColorIds); Assert.IsEmpty(unchanged.HandleIds);
            Assert.AreEqual(furniture.Variants[0].Dimensions.WidthMm, unchanged.Dimensions.WidthMm);
            Assert.AreEqual(furniture.Variants[0].Dimensions.HeightMm, unchanged.Dimensions.HeightMm);
            Assert.AreEqual(furniture.Variants[0].Dimensions.DepthMm, unchanged.Dimensions.DepthMm);
            Assert.AreEqual(furniture.Variants[0].Name, unchanged.Name);
            view.ViewModel.BeginEditVariant(view.ViewModel.Variants.Single()); WpfTestHost.CompleteLayout(window);
            var colorChoice = new FurnitureOptionChoice(furniture.Colors[0].Id,
                "أبيض عاجي بتشطيب مطفي مناسب لغرف النوم والخزائن الكبيرة والأبواب الداخلية المتناسقة", false);
            view.ViewModel.VariantColorChoices[0] = colorChoice;
            WpfTestHost.CompleteLayout(window);
            var color = WpfTestHost.FindByAutomationName<CheckBox>(view, colorChoice.Name);
            color.BringIntoView(); WpfTestHost.CompleteLayout(window);
            var label = WpfTestHost.Descendants<TextBlock>(color).Single();
            Assert.IsTrue(label.ActualWidth <= color.ActualWidth, "Long option labels must stay inside the checkbox hit area.");
            var checkbox = WpfTestHost.FindByAutomationName<CheckBox>(view, "السماح بتخصيص المقاس");
            Assert.IsTrue(label.ActualHeight > WpfTestHost.Descendants<TextBlock>(checkbox).Single().ActualHeight, "Long option labels must wrap.");
            var hit = (DependencyObject)color.InputHitTest(new Point(1, color.ActualHeight / 2));
            Assert.IsTrue(ReferenceEquals(color, hit) || ReferenceEquals(color, WpfTestHost.Ancestor<CheckBox>(hit)), "The full choice row must remain clickable.");
            var peer = new System.Windows.Automation.Peers.CheckBoxAutomationPeer(color);
            ((System.Windows.Automation.Provider.IToggleProvider)peer.GetPattern(System.Windows.Automation.Peers.PatternInterface.Toggle)).Toggle();
            Assert.IsTrue(view.ViewModel.VariantColorChoices.Single().Selected);
            WpfTestHost.Capture(window, $"furniture-variant-choices-{Math.Round(window.ActualWidth)}x{Math.Round(window.ActualHeight)}-{dpi.DpiScaleX * 100}percent");
            if (width == 720)
            {
                var choicesScroll = WpfTestHost.Ancestor<ScrollViewer>(color);
                choicesScroll.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "furniture-variant-choices-bottom-720x560");
                var choices = WpfTestHost.Descendants<CheckBox>(dialog).ToArray();
                foreach (var choice in choices) choice.FontSize *= 1.5;
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "furniture-variant-enlarged-text-bottom-720x560");
                foreach (var choice in choices) choice.ClearValue(Control.FontSizeProperty);
            }
            checkbox.IsChecked = true; WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"furniture-variant-customization-{width}x{height}");
            var minimum = WpfTestHost.FindByAutomationName<TextBox>(view, "أقل عرض"); Assert.IsTrue(minimum.IsVisible); Assert.IsTrue(minimum.Focus()); minimum.Text = "100";
            WpfTestHost.FindByAutomationName<TextBox>(view, "أكبر عرض").Text = "160"; Click(view, "حفظ المقاس");
            var input = view.ViewModel.SaveInput(FurnitureState.Draft); Assert.AreEqual(1000L, input.Variants[0].Customization.Minimum.WidthMm); Assert.AreEqual(1600L, input.Variants[0].Customization.Maximum.WidthMm);
            CollectionAssert.AreEqual(new[] { furniture.Colors[0].Id }, input.Variants[0].ColorIds);
            view.ViewModel.BeginEditVariant(view.ViewModel.Variants.Single()); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(view.ViewModel.VariantColorChoices.Single().Selected);
            var handleChoice = WpfTestHost.FindByAutomationName<CheckBox>(view, furniture.Handles[0].Name);
            handleChoice.IsChecked = true; Click(view, "حفظ المقاس");
            CollectionAssert.AreEqual(new[] { furniture.Handles[0].Id }, view.ViewModel.SaveInput(FurnitureState.Draft).Variants[0].HandleIds);
        }, engine: engine);
    }
    private static void Click(FurnitureView view, string name) { WpfTestHost.FindByAutomationName<Button>(view, name).RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(view); }
    private static void Next(FurnitureView view) { WpfTestHost.FindByName<Button>(view, "NextButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(view); }
}
