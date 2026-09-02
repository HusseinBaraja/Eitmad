using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using Eitmad.WindowsShell.Features.Furniture;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class FurnitureRenderedTests
{
    [TestMethod]
    public void ManagerListAndSixStepEditorRenderAccessibleInteractions()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
            Assert.AreEqual(Visibility.Visible, view.Visibility);
            Assert.AreEqual("البحث عن أثاث", AutomationProperties.GetName(WpfTestHost.FindByName<TextBox>(view, "FurnitureSearchBox")));
            Assert.IsGreaterThan(0, WpfTestHost.Descendants<Canvas>(view).Count(canvas => canvas.Visibility == Visibility.Visible));

            WpfTestHost.FindByAutomationName<Button>(view, "إضافة منتج")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(1, view.ViewModel.CurrentStep);
            var name = WpfTestHost.FindByName<TextBox>(view, "FurnitureNameBox");
            Assert.IsTrue(name.IsKeyboardFocusWithin);
            Assert.IsTrue(WpfTestHost.FindByAutomationName<Button>(view, "اختيار صورة المنتج").IsEnabled);

            name.Text = "خزانة اختبار";
            WpfTestHost.FindByName<Button>(view, "NextButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(2, view.ViewModel.CurrentStep);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<StackPanel>(view, "PartsStep").Visibility);

            WpfTestHost.FindByAutomationName<Button>(view, "إضافة جزء للأثاث")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "PartSearchBox").IsKeyboardFocusWithin);
            WpfTestHost.FindByAutomationName<Button>(view, "اختيار الجزء")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.HasCount(1, view.ViewModel.SelectedParts);

            WpfTestHost.FindByName<Button>(view, "NextButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(3, view.ViewModel.CurrentStep);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<StackPanel>(view, "VariantsStep").Visibility);

            WpfTestHost.FindByAutomationName<Button>(view, "إضافة مقاس")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            var variantName = WpfTestHost.FindByName<TextBox>(view, "VariantNameBox");
            Assert.IsTrue(variantName.IsKeyboardFocusWithin);
            variantName.Text = "صغير";
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ المقاس")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.HasCount(1, view.ViewModel.Variants);

            WpfTestHost.FindByName<Button>(view, "NextButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(4, view.ViewModel.CurrentStep);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<StackPanel>(view, "OptionsStep").Visibility);
            Assert.HasCount(3, view.ViewModel.Colors);
            Assert.HasCount(3, view.ViewModel.Handles);
            Assert.AreEqual("مشمول", view.ViewModel.Colors[0].PriceAdjustmentLabel);
            Assert.AreEqual("+10,000 YER", view.ViewModel.Colors[2].PriceAdjustmentLabel);
            Assert.IsTrue(WpfTestHost.FindByAutomationName<Button>(view, "إضافة لون").IsEnabled);
            Assert.IsTrue(WpfTestHost.FindByAutomationName<Button>(view, "إضافة مقبض").IsEnabled);

            WpfTestHost.FindByAutomationName<Button>(view, "إضافة لون")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            var colorName = WpfTestHost.FindByName<TextBox>(view, "ColorNameBox");
            Assert.IsTrue(colorName.IsKeyboardFocusWithin);
            colorName.Text = "أزرق";
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ اللون")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.HasCount(4, view.ViewModel.Colors);

            WpfTestHost.FindByAutomationName<Button>(view, "إضافة مقبض")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            var handleName = WpfTestHost.FindByName<TextBox>(view, "HandleNameBox");
            Assert.IsTrue(handleName.IsKeyboardFocusWithin);
            handleName.Text = "فولاذي";
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ المقبض")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.HasCount(4, view.ViewModel.Handles);

            WpfTestHost.FindByName<Button>(view, "NextButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(5, view.ViewModel.CurrentStep);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<StackPanel>(view, "PricingStep").Visibility);
            var sellingPrice = WpfTestHost.Descendants<TextBox>(view)
                .Single(box => AutomationProperties.GetName(box) == "سعر بيع المقاس");
            sellingPrice.Text = "غير صالح";

            WpfTestHost.FindByName<Button>(view, "NextButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(5, view.ViewModel.CurrentStep);
            Assert.IsTrue(sellingPrice.IsKeyboardFocusWithin);
            StringAssert.Contains(view.ViewModel.EditorError, "صحّح سعر البيع");

            sellingPrice.Text = "200000";

            WpfTestHost.FindByName<Button>(view, "NextButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(6, view.ViewModel.CurrentStep);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<StackPanel>(view, "ReviewStep").Visibility);
            Assert.IsTrue(WpfTestHost.FindByAutomationName<Button>(view, "حفظ الأثاث كمسودة").IsEnabled);
            Assert.IsTrue(WpfTestHost.FindByAutomationName<Button>(view, "حفظ الأثاث ونشره").IsEnabled);

            WpfTestHost.FindByAutomationName<Button>(view, "حفظ الأثاث ونشره")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(view.ViewModel.IsListVisible);
            StringAssert.Contains(view.ViewModel.FeedbackMessage, "المعاينة المحلية");
        });
    }

    [TestMethod]
    public void FurnitureActionPopupUsesPhysicalMousePlacement()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
            var action = WpfTestHost.FindByAutomationName<Button>(view, "إجراءات الأثاث");

            action.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();

            Assert.IsNotNull(action.ContextMenu);
            Assert.IsTrue(action.ContextMenu.IsOpen);
            Assert.AreEqual(PlacementMode.MousePoint, action.ContextMenu.Placement);
            CollectionAssert.AreEquivalent(
                new[] { "تعديل", "تكرار", "أرشفة" },
                action.ContextMenu.Items.OfType<MenuItem>().Select(item => item.Header).Cast<string>().ToArray());
        });
    }

    [TestMethod]
    public void ReviewDraftActionReturnsToListWithDraftFeedback()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
            var furniture = view.ViewModel.VisibleFurniture.First(item => !item.IsArchived);
            view.ViewModel.BeginEdit(furniture);
            Assert.IsTrue(view.ViewModel.MoveToParts());
            Assert.IsTrue(view.ViewModel.MoveToVariants());
            Assert.IsTrue(view.ViewModel.MoveToOptions());
            Assert.IsTrue(view.ViewModel.MoveToPricing());
            Assert.IsTrue(view.ViewModel.MoveToReview());
            WpfTestHost.CompleteLayout(view);

            WpfTestHost.FindByAutomationName<Button>(view, "حفظ الأثاث كمسودة")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);

            Assert.IsTrue(view.ViewModel.IsListVisible);
            Assert.IsTrue(furniture.IsDraft);
            StringAssert.Contains(view.ViewModel.FeedbackMessage, "حُفظت مسودة");
        });
    }
}
