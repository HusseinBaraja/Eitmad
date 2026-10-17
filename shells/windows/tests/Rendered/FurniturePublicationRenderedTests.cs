using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Pricing;
using Eitmad.WindowsShell.Tests.Furniture;
using Eitmad.WindowsShell.Tests.Parts;
using Eitmad.WindowsShell.Tests.Pricing;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class FurniturePublicationRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ConfirmedActiveSaveKeepsNextStepVisibleAndRoutesToPricing(int width, int height)
    {
        var fixture = new FurnitureFixtures(); fixture.Seed(); var engine = fixture.Engine();
        var queries = engine.QueryHandler!;
        engine.QueryHandler = query => query.Kind == Query.PricingListKind
            ? PartFixtures.Success(QueryResult.ForPrices(PricingPresentationTests.Data())) : queries(query);
        WpfTestHost.Run(width, height, window =>
        {
            if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
            var view = OpenEditor(window);
            view.ViewModel.EditorName = "خزانة عرض تجريبية بأبواب وأدراج لغرفة النوم";
            view.ViewModel.Variants[0].SellingPrice = 225000;
            Save(view, false);
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            var saved = engine.LastCommand!.AsFurnitureSave()!;
            Assert.AreEqual(FurnitureState.Active, saved.State);
            Assert.AreEqual(225000L, saved.Variants[0].SellingPriceYer);
            // Dismissing the short toast must not remove the required next step.
            view.ViewModel.ClearFeedback(); WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"action-08-active-save-{width}x{height}");
            var notice = WpfTestHost.FindByName<TextBlock>(view, "SaveNextStepText");
            var route = WpfTestHost.FindByName<Button>(view, "OpenPricingButton");
            Assert.IsTrue(notice.IsVisible); StringAssert.Contains(notice.Text, saved.Name);
            Assert.IsTrue(route.IsVisible && route.IsEnabled);
            var viewport = WpfTestHost.Ancestor<ScrollContentPresenter>(route);
            var bounds = route.TransformToAncestor(viewport).TransformBounds(new Rect(route.RenderSize));
            Assert.IsTrue(bounds.Left >= 0 && bounds.Right <= viewport.ActualWidth && bounds.Top >= 0 && bounds.Bottom <= viewport.ActualHeight,
                $"Pricing route {bounds} must fit viewport {viewport.RenderSize}.");
            Assert.AreEqual("فتح التسعير لنشر سعر البيع", AutomationProperties.GetName(route));
            Assert.IsTrue(notice.Focusable == false && route.Focusable);
            WpfTestHost.FindByName<TextBox>(view, "FurnitureSearchBox").Focus();
            // The native tab sequence must provide access to the new action.
            for (var step = 0; step < 6 && !route.IsKeyboardFocusWithin; step++)
                Assert.IsTrue(((UIElement)Keyboard.FocusedElement).MoveFocus(new TraversalRequest(FocusNavigationDirection.Previous)));
            Assert.IsTrue(route.IsKeyboardFocusWithin);
            var pricing = WpfTestHost.Descendants<PricingView>(window).Single();
            pricing.ViewModel.SelectedCategory = "فئة سابقة";
            route.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(pricing.IsVisible);
            Assert.AreEqual(saved.Name, pricing.ViewModel.SearchText);
            Assert.AreEqual(PricingViewModel.AllCategories, pricing.ViewModel.SelectedCategory);
        }, engine: engine);
    }

    [TestMethod]
    public void DraftArchiveUnknownAndEndedSessionDoNotOfferPublication()
    {
        var fixture = new FurnitureFixtures(); fixture.Seed(); var engine = fixture.Engine();
        WpfTestHost.Run(1338, 753, window =>
        {
            var view = OpenEditor(window);
            var accepted = engine.CommandHandler;
            engine.CommandHandler = _ => FurnitureFixtures.Failure(ProtocolIds.ErrorCodes.EitmadErrorFurnitureRevisionConflictV1);
            Save(view, false);
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.IsFalse(WpfTestHost.FindByName<TextBlock>(view, "SaveNextStepText").IsVisible);
            engine.CommandHandler = _ => throw new System.IO.IOException("Synthetic lost save response");
            Save(view, false);
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "OpenPricingButton").IsVisible);
            engine.CommandHandler = accepted;
            Save(view, false);
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "OpenPricingButton").IsVisible);
            view.ViewModel.BeginEdit(view.ViewModel.VisibleFurniture.Single());
            Save(view, true);
            var notice = WpfTestHost.FindByName<TextBlock>(view, "SaveNextStepText");
            StringAssert.Contains(notice.Text, "مسودة");
            Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "OpenPricingButton").IsVisible);
            WpfTestHost.Capture(window, "action-08-draft-save-1338x753");
            fixture.Definitions.Single().State = FurnitureState.Archived;
            view.ViewModel.ApplyDurableData(fixture.Snapshot());
            Assert.IsTrue(view.ViewModel.BeginEdit(view.ViewModel.VisibleFurniture.Single()));
            view.ViewModel.MoveToReview(); Save(view, false);
            Assert.AreEqual(FurnitureState.Archived, engine.LastCommand!.AsFurnitureSave()!.State);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "OpenPricingButton").IsVisible);
            view.ViewModel.ClearSession(); WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(notice.IsVisible);
            Assert.AreEqual("", notice.Text);
        }, engine: engine);
    }

    private static FurnitureView OpenEditor(MainWindow window)
    {
        WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        WpfTestHost.CompleteLayout(window);
        var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
        Assert.IsTrue(view.PrepareEditorAsync(view.ViewModel.VisibleFurniture.Single()).GetAwaiter().GetResult());
        Assert.IsTrue(view.ViewModel.BeginEdit(view.ViewModel.VisibleFurniture.Single()));
        view.ViewModel.MoveToReview(); WpfTestHost.CompleteLayout(window);
        return view;
    }

    private static void Save(FurnitureView view, bool draft)
    {
        WpfTestHost.FindByAutomationName<Button>(view, draft ? "حفظ الأثاث كمسودة" : "حفظ تعريف الأثاث")
            .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        WpfTestHost.CompleteLayout(view);
    }
}
