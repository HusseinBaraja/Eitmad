using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Media;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;
namespace Eitmad.WindowsShell.Tests.Rendered;
[TestClass]
public sealed class ProductsRenderedTests
{
    [TestMethod]
    public void InvalidCostCannotSubmitPreviousValueAndDeniedSaveClearsInternalState()
    {
        var data = ProductsPresentationTests.Data(); var engine = new FakeEngine();
        engine.QueryHandler = query => new QueryResponseEnvelope { RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = query.Kind == Query.ProductCategoryListKind ? QueryResult.ForProductCategories(data.Categories) : QueryResult.ForProducts(new ProductPage { Items = data.Products.ToArray(), CanManage = true, CanReadCosts = true }) } };
        engine.CommandHandler = _ => new CommandResponseEnvelope { RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Failed, Payload = new CommandResult { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 } } };
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "ProductsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<ProductsView>(window).Single();
            view.ViewModel.BeginEdit(view.ViewModel.VisibleProducts.Single()); WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<StackPanel>(view, "VariantsPricing").BringIntoView(); WpfTestHost.CompleteLayout(window);
            var cost = WpfTestHost.FindByAutomationName<TextBox>(view, "تكلفة شراء الخيار");
            cost.Text = "invalid"; cost.GetBindingExpression(TextBox.TextProperty)!.UpdateSource();
            var save = WpfTestHost.FindByAutomationName<Button>(view, "حفظ المنتج");
            save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsNull(engine.LastCommand);
            Assert.IsTrue(view.ViewModel.HasEditorError);
            Assert.IsTrue(cost.IsKeyboardFocusWithin);
            cost.Text = "55000"; cost.GetBindingExpression(TextBox.TextProperty)!.UpdateSource();
            save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            Assert.IsNotNull(engine.LastCommand);
            Assert.IsFalse(view.ViewModel.CanManage);
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            Assert.AreEqual("", view.ViewModel.Notes);
            Assert.HasCount(0, view.ViewModel.Variants);
            Assert.HasCount(0, view.ViewModel.VisibleProducts);
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void DurableListAndSupplierEditorRenderAtBaselineSizes(int width, int height)
    {
        var data = ProductsPresentationTests.Data(); var engine = new FakeEngine();
        engine.QueryHandler = query => new QueryResponseEnvelope { RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = query.Kind == Query.ProductCategoryListKind ? QueryResult.ForProductCategories(data.Categories) : QueryResult.ForProducts(new ProductPage { Items = data.Products.ToArray(), CanManage = true, CanReadCosts = true }) } };
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "ProductsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<ProductsView>(window).Single(); Assert.HasCount(1, view.ViewModel.VisibleProducts);
            var filter = WpfTestHost.FindByAutomationName<ComboBox>(view, "تصفية فئة المنتجات");
            Assert.AreEqual(ProductsViewModel.AllCategories, filter.SelectedItem);
            filter.SelectedItem = data.Categories.Items[0].Name;
            view.ViewModel.ApplyDurableData(data);
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(data.Categories.Items[0].Name, filter.SelectedItem);
            filter.SelectedItem = ProductsViewModel.AllCategories;
            WpfTestHost.CompleteLayout(window);
            Console.WriteLine($"Products rendered {window.ActualWidth}x{window.ActualHeight} DIP; scaling {VisualTreeHelper.GetDpi(window).DpiScaleX * 100}%.");
            WpfTestHost.Capture(window, $"products-list-{width}x{height}");
            var action = WpfTestHost.FindByAutomationName<Button>(view, "إجراءات المنتج"); action.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window); Assert.IsTrue(action.ContextMenu!.IsOpen);
            WpfTestHost.Capture((FrameworkElement)action.ContextMenu, $"products-actions-{width}x{height}"); action.ContextMenu.IsOpen = false;
            var add = WpfTestHost.FindByAutomationName<Button>(view, "إضافة منتج"); add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window); Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "ProductNameBox").IsKeyboardFocusWithin); view.ViewModel.CancelEditor();
            view.ViewModel.BeginEdit(view.ViewModel.VisibleProducts.Single()); WpfTestHost.CompleteLayout(window);
            var category = WpfTestHost.FindByAutomationName<ComboBox>(view, "فئة المنتج"); Assert.AreEqual(data.Categories.Items[0].Id, category.SelectedValue);
            Assert.IsNotNull(WpfTestHost.FindByAutomationName<Button>(view, "أرشفة خيار المنتج")); WpfTestHost.Capture(window, $"products-editor-{width}x{height}");
            var variants = WpfTestHost.FindByName<StackPanel>(view, "VariantsPricing"); variants.BringIntoView(); WpfTestHost.CompleteLayout(window);
            var variantName = WpfTestHost.FindByAutomationName<TextBox>(view, "اسم خيار المنتج"); variantName.Focus(); Assert.IsTrue(variantName.IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, $"products-variants-{width}x{height}");
            if (width == 1338) { foreach (var input in WpfTestHost.Descendants<TextBox>(view)) ControlOptions.SetHighContrast(input, true); foreach (var table in WpfTestHost.Descendants<OperationsTable>(view)) ControlOptions.SetHighContrast(table, true); WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window, "products-editor-system-colors"); foreach (var input in WpfTestHost.Descendants<TextBox>(view)) ControlOptions.SetHighContrast(input, false); foreach (var table in WpfTestHost.Descendants<OperationsTable>(view)) ControlOptions.SetHighContrast(table, false); }
            category.BringIntoView(); category.IsDropDownOpen = true; WpfTestHost.CompleteLayout(window);
            var popup = (Popup)category.Template.FindName("PART_Popup", category); Assert.IsNotNull(WpfTestHost.FindByAutomationName<Button>(popup.Child, "إضافة فئة جديدة")); WpfTestHost.Capture((FrameworkElement)popup.Child, $"products-categories-{width}x{height}"); category.IsDropDownOpen = false;
            view.ViewModel.CancelEditor(); view.ViewModel.RequestArchive(view.ViewModel.VisibleProducts.Single()); WpfTestHost.CompleteLayout(window); Assert.IsNotNull(WpfTestHost.FindByAutomationName<Button>(view, "تأكيد أرشفة المنتج")); WpfTestHost.Capture(window, $"products-archive-{width}x{height}");
        }, engine: engine);
    }
}
