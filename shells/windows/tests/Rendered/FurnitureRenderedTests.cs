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
    public void ManagerListAndThreeStepEditorRenderAccessibleInteractions()
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
}
