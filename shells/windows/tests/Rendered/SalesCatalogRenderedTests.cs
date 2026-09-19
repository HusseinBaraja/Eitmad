using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class SalesCatalogRenderedTests
{
    [TestMethod]
    public void HomeStartsWithCatalogAndReviewRestoresSelectionFocus()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "catalog-home");
            WpfTestHost.FindByName<Button>(reception, "NewQuotationAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(reception, "CatalogContent");
            var model = (SalesCatalogViewModel)catalog.DataContext;
            Assert.IsFalse(model.IsReviewingQuotation);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(catalog, "CatalogSearch").IsKeyboardFocusWithin);
            Assert.AreEqual("", model.CustomerName);
            Assert.AreEqual("", model.Phone);
            WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار وسادة فندقية").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var detail = WpfTestHost.FindByName<ProductSelectionView>(catalog, "ProductSelectionView");
            WpfTestHost.FindByName<Button>(detail, "AddButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsFalse(model.IsReviewingQuotation);
            WpfTestHost.FindByAutomationName<Button>(reception, "فتح عرض السعر").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var continueButton = WpfTestHost.FindByAutomationName<Button>(catalog, "متابعة اختيار المنتجات");
            Assert.IsTrue(continueButton.IsKeyboardFocusWithin);
            continueButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(detail, "BackButton").IsKeyboardFocusWithin);
            WpfTestHost.FindByName<Button>(reception, "HomeNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.FindByName<Button>(reception, "NewQuotationAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(model.IsSelecting);
            Assert.HasCount(1, model.QuotationLines);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(catalog, "CatalogSearch").IsKeyboardFocusWithin);
        });
    }

    [TestMethod]
    public void CurrentQuotationRendersAndReusesSelectionEditor()
    {
        WpfTestHost.Run(1338, 1000, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "ProductsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(reception, "CatalogContent");
            var model = (SalesCatalogViewModel)catalog.DataContext;
            model.Select(model.VisibleItems.First());
            model.Selection!.SelectedSize = model.Selection.Sizes[0];
            model.Selection.SelectedColor = model.Selection.Colors[0];
            model.Selection.SelectedHandle = model.Selection.Handles[0];
            model.AddSelection(); model.CloseSelection();
            model.IsReviewingQuotation = true;
            WpfTestHost.CompleteLayout(window);
            var quotation = WpfTestHost.FindByName<CurrentQuotationView>(catalog, "QuotationView");
            Assert.IsTrue(quotation.IsVisible);
            WpfTestHost.Capture(window, "catalog-current-quotation");
            var edit = WpfTestHost.FindByAutomationName<Button>(quotation, "تعديل " + model.QuotationLines[0].Name);
            edit.Focus(); Assert.IsTrue(edit.IsKeyboardFocusWithin);
            edit.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(quotation.IsVisible);
            var selection = WpfTestHost.FindByName<FurnitureSelectionView>(catalog, "SelectionView");
            Assert.IsTrue(selection.IsVisible);
            WpfTestHost.FindByAutomationName<Button>(selection, "حفظ التعديلات").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(quotation.IsVisible);
            WpfTestHost.FindByAutomationName<Button>(quotation, "عميل جديد").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(quotation, "CustomerNameInput").IsKeyboardFocusWithin);
            model.CustomerName = "عميل تجريبي";
            model.Phone = "000000000";
            var scroll = WpfTestHost.Descendants<ScrollViewer>(quotation).First();
            scroll.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
            var discount = WpfTestHost.FindByAutomationName<TextBox>(quotation, "نسبة الخصم");
            discount.Focus(); Assert.IsTrue(discount.IsKeyboardFocusWithin);
            discount.Text = "5";
            WpfTestHost.CompleteLayout(window);
            var save = WpfTestHost.FindByAutomationName<Button>(quotation, "حفظ عرض السعر");
            Assert.IsTrue(save.IsEnabled);
            WpfTestHost.Capture(window, "catalog-discount-allowed");
            discount.Text = "10";
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(save.IsEnabled);
            var request = WpfTestHost.FindByAutomationName<Button>(quotation, "طلب موافقة");
            Assert.IsTrue(request.IsVisible);
            request.Focus(); Assert.IsTrue(request.IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, "catalog-discount-request");
            request.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var draft = WpfTestHost.FindByAutomationName<Button>(quotation, "حفظ كمسودة");
            Assert.IsTrue(draft.IsKeyboardFocusWithin);
            Assert.IsTrue(draft.IsEnabled);
            Assert.IsFalse(request.IsVisible);
            Assert.IsFalse(save.IsEnabled);
            draft.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsTrue(model.IsDiscountPending);
            scroll.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "catalog-discount-pending");
        });
    }

    [TestMethod]
    public void ReadyMadeDetailsRenderAndAddBothProductTypes()
    {
        WpfTestHost.Run(1338, 1000, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "ProductsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(reception, "CatalogContent");
            var model = (SalesCatalogViewModel)catalog.DataContext;
            foreach (var name in new[] { "مرتبة طبية", "وسادة فندقية" })
            {
                var select = WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار " + name);
                select.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                var detail = WpfTestHost.FindByName<ProductSelectionView>(catalog, "ProductSelectionView");
                Assert.IsTrue(detail.IsVisible);
                Assert.IsTrue(WpfTestHost.FindByName<Button>(detail, "BackButton").IsKeyboardFocusWithin);
                var variants = WpfTestHost.FindByName<ListBox>(detail, "VariantsList");
                var add = WpfTestHost.FindByName<Button>(detail, "AddButton");
                Assert.AreEqual(model.ProductSelection!.HasVariants, variants.IsVisible);
                if (model.ProductSelection.HasVariants)
                {
                    Assert.IsFalse(add.IsEnabled);
                    variants.SelectedIndex = 1;
                    WpfTestHost.CompleteLayout(window);
                    var card = (ListBoxItem)variants.ItemContainerGenerator.ContainerFromIndex(1);
                    card.Focus();
                    Assert.IsTrue(card.IsKeyboardFocusWithin);
                    Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(card).Any(text => text.Text == "✓" && text.IsVisible));
                }
                WpfTestHost.FindByAutomationName<Button>(detail, "زيادة الكمية").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(add.IsEnabled);
                WpfTestHost.Capture(window, "catalog-" + (model.ProductSelection.HasVariants ? "product-variants" : "product-simple"));
                add.BringIntoView();
                WpfTestHost.CompleteLayout(window);
                add.Focus();
                Assert.IsTrue(add.IsKeyboardFocusWithin);
                add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.AreEqual(2, model.QuotationLines.Last().Quantity);
                WpfTestHost.FindByName<Button>(detail, "BackButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(select.IsKeyboardFocusWithin);
            }
            Assert.HasCount(2, model.QuotationLines);
        });
    }

    [TestMethod]
    public void FurnitureOptionRowsScrollThePageOverCardsAndEmptySpace()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "ProductsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(reception, "CatalogContent");
            WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار خزانة السكينة")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var detail = WpfTestHost.FindByName<FurnitureSelectionView>(catalog, "SelectionView");
            var page = WpfTestHost.Descendants<ScrollViewer>(detail).First();

            foreach (var name in new[] { "SizesList", "ColorsList", "HandlesList" })
            {
                var list = WpfTestHost.FindByName<ListBox>(detail, name);
                var card = (ListBoxItem)list.ItemContainerGenerator.ContainerFromIndex(0);
                list.SelectedIndex = 0;
                card.BringIntoView();
                WpfTestHost.CompleteLayout(window);
                var cardPoint = card.TranslatePoint(new Point(card.ActualWidth / 2, card.ActualHeight / 2), list);
                var emptyPoint = new Point(list.ActualWidth - 2, cardPoint.Y);
                foreach (var point in new[] { cardPoint, emptyPoint })
                {
                    var target = list.InputHitTest(point) as DependencyObject;
                    Assert.IsNotNull(target);
                    if (point == emptyPoint)
                        Assert.IsFalse(WpfTestHost.Descendants<ListBoxItem>(list).Any(item => item.IsAncestorOf(target) || item == target), "The regression must cover empty row space.");
                    var source = target as UIElement ?? WpfTestHost.Ancestor<UIElement>(target);
                    foreach (var delta in new[] { -120, 120 })
                    {
                        page.ScrollToVerticalOffset(page.ScrollableHeight / 2);
                        WpfTestHost.CompleteLayout(window);
                        var before = page.VerticalOffset;
                        source.RaiseEvent(new MouseWheelEventArgs(Mouse.PrimaryDevice, Environment.TickCount, delta)
                        {
                            RoutedEvent = Mouse.MouseWheelEvent,
                        });
                        WpfTestHost.CompleteLayout(window);
                        Assert.IsTrue(delta < 0 ? page.VerticalOffset > before : page.VerticalOffset < before,
                            $"{name}: wheel {delta} at {point} must scroll the page.");
                        Assert.AreEqual(0, list.SelectedIndex, "Scrolling must preserve the selected option.");
                    }
                }
            }
            WpfTestHost.Capture(window, "catalog-selection-scroll");
        });
    }

    [TestMethod]
    public void ReceptionCatalogSupportsNavigationSearchClearAndKeyboardSelection()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "ProductsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(reception, "CatalogContent");
            Assert.IsTrue(catalog.IsVisible);
            Assert.AreEqual(FlowDirection.RightToLeft, catalog.FlowDirection);
            Assert.AreEqual("الكل", WpfTestHost.FindByAutomationName<ListBox>(catalog, "فئات المنتجات").SelectedItem);
            WpfTestHost.Capture(window, "catalog-normal");
            var search = WpfTestHost.FindByName<TextBox>(catalog, "CatalogSearch");
            search.Focus();
            Assert.IsTrue(search.IsKeyboardFocusWithin);
            search.Text = "لا يوجد";
            WpfTestHost.CompleteLayout(window);
            var clear = WpfTestHost.FindByAutomationName<Button>(catalog, "مسح الفلاتر");
            Assert.IsTrue(clear.IsVisible);
            clear.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(search.IsKeyboardFocusWithin);
            Assert.AreEqual(string.Empty, search.Text);
            var categories = WpfTestHost.FindByAutomationName<ListBox>(catalog, "فئات المنتجات");
            categories.SelectedItem = "المراتب";
            WpfTestHost.CompleteLayout(window);
            Assert.HasCount(1, ((SalesCatalogViewModel)catalog.DataContext).VisibleItems);
            WpfTestHost.Capture(window, "catalog-retail");
            var select = WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار مرتبة طبية");
            select.Focus();
            Assert.IsTrue(select.IsKeyboardFocusWithin);
            select.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsTrue(((SalesCatalogViewModel)catalog.DataContext).IsSelectingProduct);
            ((SalesCatalogViewModel)catalog.DataContext).CloseSelection();
            ((SalesCatalogViewModel)catalog.DataContext).ClearFilters();
            window.Width = 1000;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "catalog-compact");
            WpfTestHost.FindByName<Button>(reception, "HomeNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(catalog.IsVisible);
            var catalogNav = WpfTestHost.FindByName<Button>(reception, "ProductsNavButton");
            Assert.IsTrue(catalogNav.IsVisible);
            catalogNav.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(catalog.IsVisible);
            catalog.Resources[SystemParameters.HighContrastKey] = true;
            catalog.Resources[SystemColors.WindowBrushKey] = Brushes.Black;
            catalog.Resources[SystemColors.WindowTextBrushKey] = Brushes.White;
            catalog.Resources[SystemColors.ControlBrushKey] = Brushes.Black;
            catalog.Resources[SystemColors.ControlTextBrushKey] = Brushes.White;
            catalog.Resources[SystemColors.GrayTextBrushKey] = Brushes.White;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "catalog-system-colors");
        });
    }

    [TestMethod]
    public void FurnitureSelectionRendersOptionsTotalsAndKeepsCatalogFlow()
    {
        WpfTestHost.Run(1338, 1000, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "ProductsAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(reception, "CatalogContent");
            var select = WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار خزانة السكينة");
            select.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var detail = WpfTestHost.FindByName<FurnitureSelectionView>(catalog, "SelectionView");
            Assert.IsTrue(detail.IsVisible);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(detail, "BackButton").IsKeyboardFocusWithin);
            var sizes = WpfTestHost.FindByName<ListBox>(detail, "SizesList");
            var colors = WpfTestHost.FindByName<ListBox>(detail, "ColorsList");
            var handles = WpfTestHost.FindByName<ListBox>(detail, "HandlesList");
            var add = WpfTestHost.FindByName<Button>(detail, "AddButton");
            Assert.IsFalse(add.IsEnabled);
            sizes.SelectedIndex = 2; colors.SelectedIndex = 1; handles.SelectedIndex = 1;
            WpfTestHost.CompleteLayout(window);
            var card = (ListBoxItem)sizes.ItemContainerGenerator.ContainerFromIndex(2);
            card.Focus();
            Assert.IsTrue(card.IsKeyboardFocusWithin);
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(card).Any(t => t.Text == "✓" && t.IsVisible));
            Assert.IsFalse(WpfTestHost.Descendants<TextBox>(detail).Any());
            WpfTestHost.Capture(window, "catalog-selection");
            var increase = WpfTestHost.FindByAutomationName<Button>(detail, "زيادة الكمية");
            increase.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("626,000 YER", ((FurnitureSelectionViewModel)detail.DataContext).LineTotalLabel);
            add.BringIntoView();
            WpfTestHost.CompleteLayout(window);
            add.Focus();
            Assert.IsTrue(add.IsKeyboardFocusWithin);
            add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(detail.IsVisible);
            var model = (SalesCatalogViewModel)catalog.DataContext;
            Assert.HasCount(1, model.QuotationLines);
            var title = WpfTestHost.FindByName<Eitmad.WindowsShell.Controls.ShellTitleBar>(reception, "ReceptionistTitleBar");
            Assert.AreEqual(model.QuotationLabel, title.PrimaryActionLabel);
            WpfTestHost.Capture(window, "catalog-selection-total");
            WpfTestHost.FindByAutomationName<Button>(reception, "فتح عرض السعر").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(model.IsReviewingQuotation);
            var continueButton = WpfTestHost.FindByAutomationName<Button>(catalog, "متابعة اختيار المنتجات");
            continueButton.Focus();
            Assert.IsTrue(continueButton.IsKeyboardFocusWithin);
            continueButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(model.IsReviewingQuotation);
            Assert.AreEqual(2, model.Selection!.Quantity);
            window.Width = 1000;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "catalog-selection-compact");
            detail.Resources[SystemParameters.HighContrastKey] = true;
            detail.Resources[SystemColors.WindowBrushKey] = Brushes.Black;
            detail.Resources[SystemColors.WindowTextBrushKey] = Brushes.White;
            detail.Resources[SystemColors.ControlBrushKey] = Brushes.Black;
            detail.Resources[SystemColors.ControlTextBrushKey] = Brushes.White;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "catalog-selection-system-colors");
            WpfTestHost.FindByName<Button>(detail, "BackButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(detail.IsVisible);
            Assert.IsTrue(select.IsKeyboardFocusWithin);
        });
    }
}
