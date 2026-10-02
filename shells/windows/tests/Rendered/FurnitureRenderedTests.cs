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
    /// <summary>Rejects delayed review margins after each staged variant mutation, even when IPC ignores cancellation.</summary>
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

    /// <summary>Verifies complete editor saves reopens and keeps conflict and retry fields.</summary>
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
    /// <summary>Verifies variant customization dialog renders native accessible bounds and choices.</summary>
    [TestMethod]
    public void VariantCustomizationDialogRendersNativeAccessibleBoundsAndChoices()
    {
        var f = new FurnitureFixtures(); f.Seed(); var engine = f.Engine();
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
            var prepared = view.PrepareEditorAsync(view.ViewModel.VisibleFurniture.Single()); WpfTestHost.CompleteLayout(window); Assert.IsTrue(prepared.GetAwaiter().GetResult());
            view.ViewModel.BeginEdit(view.ViewModel.VisibleFurniture.Single()); view.ViewModel.BeginEditVariant(view.ViewModel.Variants.Single()); WpfTestHost.CompleteLayout(window);
            var checkbox = WpfTestHost.FindByAutomationName<CheckBox>(view, "السماح بتخصيص المقاس"); checkbox.IsChecked = true; WpfTestHost.CompleteLayout(window);
            var minimum = WpfTestHost.FindByAutomationName<TextBox>(view, "أقل عرض"); Assert.IsTrue(minimum.IsVisible); Assert.IsTrue(minimum.Focus()); minimum.Text = "100";
            WpfTestHost.FindByAutomationName<TextBox>(view, "أكبر عرض").Text = "160"; Click(view, "حفظ المقاس");
            var input = view.ViewModel.SaveInput(FurnitureState.Draft); Assert.AreEqual(1000L, input.Variants[0].Customization.Minimum.WidthMm); Assert.AreEqual(1600L, input.Variants[0].Customization.Maximum.WidthMm);
        }, engine: engine);
    }
    /// <summary>Invokes an accessible Furniture action and pumps native layout work.</summary>
    private static void Click(FurnitureView view, string name) { WpfTestHost.FindByAutomationName<Button>(view, name).RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(view); }
    /// <summary>Invokes wizard navigation and pumps pending native layout work.</summary>
    private static void Next(FurnitureView view) { WpfTestHost.FindByName<Button>(view, "NextButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(view); }
}
