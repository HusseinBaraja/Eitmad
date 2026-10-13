using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using System.Windows.Media;
using System.IO;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class CustomersRenderedTests
{
    [TestMethod]
    public void HistoryProjectionUsesRustIdentityAcrossRepublishedQuotation()
    {
        var orders = new OrdersViewModel(true, preview: true).VisibleOrders;
        var sourceOrder = orders.First(order => order.OriginalQuotation is not null);
        var original = sourceOrder.OriginalQuotation!;
        var customer = ContractCustomer(sourceOrder.Customer, sourceOrder.Phone);
        var replacement = new QuotationListItem(original.Id, original.Number, original.Customer,
            original.Date, original.Status, original.Discount, original.Items, phone: sourceOrder.Phone)
        {
            CustomerId = customer.Id,
        };

        var projection = CustomerHistoryProjection.Create(customer, [replacement], orders);

        Assert.AreEqual(customer.Id, projection.Id);
        Assert.IsTrue(projection.Orders.Count > 0);
        Assert.HasCount(1, projection.Quotations.Where(item => item.Number == original.Number));
    }

    [TestMethod]
    public void ReceptionRecordsOpenRustCustomerAndRestoreFocus()
    {
        var record = new Orders.OrderAuthorityTests.Fixture().Order;
        var engine = new FakeEngine { SupportedCapabilities = new HashSet<string> {
            ProtocolIds.Capabilities.EitmadCapabilityCustomerV1, ProtocolIds.Capabilities.EitmadCapabilityOrdersV1,
        } };
        var source = record.Source.Quotation.Evaluation.Customer;
        var customerRecord = ContractCustomer(source.Name, source.Phone, scope: engine.CustomerBranch);
        customerRecord.Id = source.Id; engine.Customers.Add(customerRecord);
        record.Scope = engine.CustomerBranch;
        engine.QueryHandler = query => new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = query.Kind switch {
            Query.OrderListKind => QueryResult.ForOrders(new() { Items = [record], Pending = [], ServerAvailable = true }),
            Query.CustomerGetKind => QueryResult.ForCustomer(engine.Customers.Single(c => c.Id == query.AsCustomerGet()!.CustomerId)),
            Query.CustomerSearchKind => QueryResult.ForCustomers(new() { Items = engine.Customers.ToArray() }),
            _ => throw new InvalidOperationException("Unexpected customer navigation query."),
        } } };
        WpfTestHost.Run(1338, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "OrdersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var orders = WpfTestHost.Descendants<OrdersView>(reception).Single();
            Finish(orders.ViewModel.ActivateAsync());
            var order = orders.ViewModel.VisibleOrders.Single(item => item.Id == record.Id);
            orders.ViewModel.OpenOrder(order);
            WpfTestHost.CompleteLayout(window);
            var open = WpfTestHost.FindByAutomationName<Button>(orders, "فتح تفاصيل عميل الطلب");
            open.Focus();
            open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var detail = WpfTestHost.FindByName<CustomerDetailView>(reception, "CustomerDetail");
            var customer = (CustomerPreview)detail.DataContext;
            Assert.AreEqual(engine.Customers[0].Id, customer.Id);
            Assert.IsTrue(customer.Orders.Any(row => row.Number == order.Number));
            Assert.IsTrue(WpfTestHost.FindByName<Button>(detail, "BackButton").IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, "customers-reception-detail");
            WpfTestHost.FindByName<Button>(detail, "BackButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(open.IsKeyboardFocusWithin);
            open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByAutomationName<Button>(detail, "تعديل بيانات العميل").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var editor = WpfTestHost.FindByName<CustomerEditorView>(detail, "Editor");
            Assert.IsTrue(editor.Dialog.IsOpen);
            reception.ClearHomeSession();
            Assert.IsNull(detail.DataContext);
            Assert.AreEqual(Visibility.Collapsed, detail.Visibility);
            Assert.IsFalse(editor.Dialog.IsOpen);
            Assert.AreEqual("", editor.NameInput.Text);
        }, engine: engine);
    }

    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(15));
        if (!task.IsCompleted) {
            var frame = new DispatcherFrame(); var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default);
            Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ReceptionCustomerSelectionRendersRustSearchResults(int width, int height)
    {
        var engine = new FakeEngine();
        engine.Customers.Add(ContractCustomer("مؤسسة الاعتماد للأثاث", "+967 777 123 456", "صنعاء", "", engine.CustomerBranch));
        WpfTestHost.Run(width, height, window =>
        {
            if (width == 1920) window.WindowState = WindowState.Maximized;
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            var catalog = (SalesCatalogView)reception.FindName("CatalogContent");
            catalog.Visibility = Visibility.Visible;
            var model = (SalesCatalogViewModel)catalog.DataContext;
            model.IsReviewingQuotation = true;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<TextBox>(catalog, "CustomerNameInput").Text = "الاعتماد";
            Finish(model.LastCustomerSearch);
            var review = WpfTestHost.Descendants<CurrentQuotationView>(catalog).Single();
            var section = WpfTestHost.FindByName<Border>(review, "CustomerSection");
            var state = WpfTestHost.FindByName<TextBlock>(review, "CustomerSelectionState");
            var typedState = state.Text;
            var phone = WpfTestHost.FindByName<TextBox>(review, "PhoneInput");
            var scroll = WpfTestHost.Descendants<ScrollViewer>(review).First();
            WpfTestHost.CompleteLayout(window);
            scroll.ScrollToVerticalOffset(scroll.VerticalOffset + section.TranslatePoint(new Point(), scroll).Y);
            WpfTestHost.CompleteLayout(window);
            var match = WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار العميل مؤسسة الاعتماد للأثاث");
            var create = WpfTestHost.FindByAutomationName<Button>(catalog, "عميل جديد");
            foreach (var element in new FrameworkElement[] { phone, match, create })
            {
                var bounds = element.TransformToAncestor(scroll).TransformBounds(new Rect(element.RenderSize));
                Assert.IsTrue(bounds.Left >= 0 && bounds.Right <= scroll.ActualWidth + 1
                    && bounds.Top >= 0 && bounds.Bottom <= scroll.ActualHeight + 1,
                    $"Customer phone, match and new-customer action must fit in the viewport: {bounds}.");
                var local = element.TransformToAncestor(section).TransformBounds(new Rect(element.RenderSize));
                Assert.IsTrue(local.Left >= 0 && local.Right <= section.ActualWidth + 1,
                    "Customer controls must remain within the bounded section.");
            }
            Assert.IsTrue(section.ActualWidth <= 600);
            Assert.AreEqual(0, scroll.ScrollableWidth);
            Assert.AreEqual(FlowDirection.LeftToRight, phone.FlowDirection);
            var dpi = VisualTreeHelper.GetDpi(window);
            var geometry = $"Requested {width}x{height}; actual window {window.ActualWidth}x{window.ActualHeight} DIP; "
                + $"primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX * 100}%; "
                + "WPF RenderTargetBitmap at 96 DPI; synthetic customer search response; customer section scrolled into view.";
            Console.WriteLine(geometry);
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
            {
                Directory.CreateDirectory(directory);
                File.WriteAllText(Path.Combine(directory, $"ui30-{width}x{height}.txt"), geometry);
            }
            WpfTestHost.Capture(window, $"customers-selection-{width}x{height}");
            match.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(engine.Customers[0].Id, model.SelectedCustomer!.Id);
            Assert.AreEqual(engine.Customers[0].Phone, phone.Text);
            Assert.AreNotEqual(typedState, state.Text);
            var selectedState = state.Text;
            WpfTestHost.Capture(window, $"ui30-customer-selected-{width}x{height}");
            phone.Text = "777000001";
            WpfTestHost.CompleteLayout(window);
            Assert.IsNull(model.SelectedCustomer);
            Assert.AreEqual(typedState, state.Text, "Editing a contact must remove the selected-record indication.");
            model.AttachCustomer(PreviewCustomer.FromContract(engine.Customers[0]));
            create.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(model.IsNewCustomer);
            Assert.IsNull(model.SelectedCustomer);
            Assert.AreEqual("", phone.Text);
            Assert.AreNotEqual(typedState, state.Text);
            Assert.AreNotEqual(selectedState, state.Text);
            WpfTestHost.Capture(window, $"ui30-customer-new-{width}x{height}");
            WpfTestHost.FindByAutomationName<Button>(review, "إلغاء العميل الجديد").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(engine.Customers[0].Id, model.SelectedCustomer!.Id);
            Assert.AreEqual(engine.Customers[0].Phone, phone.Text);
            Assert.AreEqual(selectedState, state.Text);
            if (width == 720)
            {
                foreach (var text in WpfTestHost.Descendants<TextBlock>(section)) text.FontSize = Math.Max(text.FontSize, 18);
                foreach (var input in WpfTestHost.Descendants<TextBox>(section)) input.FontSize = 18;
                WpfTestHost.CompleteLayout(window);
                foreach (var element in new FrameworkElement[] { phone, create })
                {
                    var bounds = element.TransformToAncestor(scroll).TransformBounds(new Rect(element.RenderSize));
                    Assert.IsTrue(bounds.Left >= 0 && bounds.Right <= scroll.ActualWidth + 1
                        && bounds.Top >= 0 && bounds.Bottom <= scroll.ActualHeight + 1,
                        "Larger customer text must retain the phone and new-customer action in view.");
                }
                WpfTestHost.Capture(window, "ui30-customer-larger-text-720x560");
            }
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    [DataRow(1536, 864)]
    public void DetailEditKeepsHistoryAndCancelDiscardsDraft(int width, int height)
    {
        var engine = new FakeEngine();
        var contract = ContractCustomer("منزل عائلة الصبري", "000000084",
            "عدن، المنصورة — عنوان تجريبي", "الاتصال قبل توصيل الأثاث.", engine.CustomerBranch);
        engine.Customers.Add(contract);
        WpfTestHost.Run(width, height, window =>
        {
            var customer = new CustomerPreview(contract,
                [new("QUO-2026-0084", new(2026, 9, 12), "سرير وادي ظهر، مرتبة الراحة", "محوّل", "345,000 ر.ي")],
                [new("ORD-2026-0084", new(2026, 9, 13), "سرير وادي ظهر، مرتبة الراحة", "قيد الإنتاج", "345,000 ر.ي")]);
            var view = new CustomerDetailView { DataContext = customer };
            view.Attach(new CustomerClient(engine));
            window.Content = view;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"customers-detail-{width}x{height}");
            foreach (var table in WpfTestHost.Descendants<OperationsTable>(view))
            {
                table.BringIntoView();
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(0, ((ScrollViewer)table.Template.FindName("DG_ScrollViewer", table)).ScrollableWidth, 1);
                WpfTestHost.Capture(window, $"ui04-history-{table.Name}-{width}x{height}");
            }
            var edit = WpfTestHost.FindByName<Button>(view, "EditButton");
            edit.Focus();
            edit.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var name = WpfTestHost.FindByName<TextBox>(view, "NameInput");
            Assert.IsTrue(name.IsKeyboardFocusWithin);
            name.Text = "اسم مؤقت";
            WpfTestHost.FindByAutomationName<Button>(view, "إلغاء تعديل العميل").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("منزل عائلة الصبري", customer.Name);
            Assert.IsTrue(edit.IsKeyboardFocusWithin);
            edit.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            name.Text = "عائلة الصبري";
            WpfTestHost.Capture(window, $"customers-editor-{width}x{height}");
            WpfTestHost.FindByName<Button>(view, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("عائلة الصبري", customer.Name);
            Assert.AreEqual("ORD-2026-0084", customer.Orders.Single().Number);
            Assert.AreEqual("QUO-2026-0084", customer.Quotations.Single().Number);
            foreach (var table in WpfTestHost.Descendants<OperationsTable>(view)) ControlOptions.SetHighContrast(table, true);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"customers-contrast-detail-{width}x{height}");
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void CustomerSyncStateRendersAndBlocksUnresolvedEdits(int width, int height)
    {
        var contract = ContractCustomer("عميل تجريبي له اسم عربي طويل", "+967 777 123 456",
            "عدن، المنصورة — عنوان تجريبي", "الاتصال قبل التسليم");
        WpfTestHost.Run(width, height, window =>
        {
            var preview = new CustomerPreview(contract,
                [new("QUO-2026-0084", new(2026, 9, 12), "سرير وادي ظهر", "محوّل", "345,000 ر.ي")],
                [new("ORD-2026-0084", new(2026, 9, 13), "سرير وادي ظهر", "قيد الإنتاج", "345,000 ر.ي")]);
            var view = new CustomerDetailView { DataContext = preview };
            window.Content = view;
            WpfTestHost.CompleteLayout(window);
            var edit = WpfTestHost.FindByName<Button>(view, "EditButton");
            var status = WpfTestHost.FindByName<StatusBadge>(view, "ConflictStatus");
            var notice = WpfTestHost.FindByName<FeedbackNotice>(view, "ConflictNotice");
            Assert.AreEqual("بانتظار المزامنة", preview.SyncStateLabel);
            Assert.IsTrue(edit.IsEnabled);
            Assert.IsFalse(status.IsVisible);
            Assert.IsFalse(notice.IsVisible);

            contract.SyncState = SyncState.Confirmed;
            preview.Observe(contract);
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("مؤكد من الخادم", preview.SyncStateLabel);
            Assert.IsTrue(edit.IsEnabled);

            contract.SyncState = SyncState.Rejected;
            preview.Observe(contract);
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("رُفضت المزامنة", preview.SyncStateLabel);
            Assert.IsFalse(edit.IsEnabled);
            Assert.IsFalse(status.IsVisible);
            Assert.IsFalse(notice.IsVisible);

            contract.SyncState = SyncState.Conflicted;
            preview.Observe(contract);
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("تعارض يحتاج مراجعة", preview.SyncStateLabel);
            Assert.IsFalse(edit.IsEnabled);
            Assert.IsTrue(status.IsVisible);
            Assert.IsTrue(notice.IsVisible);
            var scroll = WpfTestHost.Ancestor<ScrollViewer>(notice);
            foreach (var element in new FrameworkElement[] { status, notice })
            {
                var bounds = element.TransformToAncestor(scroll).TransformBounds(new Rect(element.RenderSize));
                Assert.IsTrue(bounds.Left >= 0 && bounds.Right <= scroll.ActualWidth + 1
                    && bounds.Top >= 0 && bounds.Bottom <= scroll.ActualHeight + 1,
                    $"The conflict and edit restriction must be visible without scrolling: {bounds}.");
            }
            Assert.AreEqual(contract.Phone, preview.Phone);
            var dpi = VisualTreeHelper.GetDpi(window);
            var geometry = $"Requested {width}x{height}; actual window {window.ActualWidth}x{window.ActualHeight} DIP; "
                + $"primary display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX * 100}%; "
                + "WPF RenderTargetBitmap at 96 DPI; synthetic customer conflict and history; no display settings changed.";
            Console.WriteLine(geometry);
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
            {
                Directory.CreateDirectory(directory);
                File.WriteAllText(Path.Combine(directory, $"ui38-{width}x{height}.txt"), geometry);
            }
            WpfTestHost.Capture(window, $"customers-sync-conflict-{width}x{height}");
            foreach (var table in WpfTestHost.Descendants<OperationsTable>(view))
            {
                table.BringIntoView();
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(1, table.Items.Count, "Conflict must keep the last known history readable.");
                Assert.AreEqual(0, ((ScrollViewer)table.Template.FindName("DG_ScrollViewer", table)).ScrollableWidth, 1);
                WpfTestHost.Capture(window, $"ui38-conflict-{table.Name}-{width}x{height}");
            }
            scroll.ScrollToTop();
            if (width == 720)
            {
                ControlOptions.SetHighContrast(status, true);
                ControlOptions.SetHighContrast(notice, true);
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "ui38-conflict-high-contrast-720x560");
                ControlOptions.SetHighContrast(status, false);
                ControlOptions.SetHighContrast(notice, false);
                status.FontSize = 18;
                notice.FontSize = 18;
                WpfTestHost.CompleteLayout(window);
                var bounds = notice.TransformToAncestor(scroll).TransformBounds(new Rect(notice.RenderSize));
                Assert.IsTrue(bounds.Right <= scroll.ActualWidth + 1 && bounds.Bottom <= scroll.ActualHeight + 1,
                    "Larger conflict text must keep the edit restriction in the viewport.");
                WpfTestHost.Capture(window, "ui38-conflict-larger-text-720x560");
            }
            contract.SyncState = SyncState.Confirmed;
            preview.Observe(contract);
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(status.IsVisible, "A Rust state refresh must remove a resolved conflict warning.");
            Assert.IsFalse(notice.IsVisible);
            Assert.IsTrue(edit.IsEnabled);
        });
    }

    private static Customer ContractCustomer(string name, string phone, string? address = null, string? notes = null, ScopeRef? scope = null) => new()
    {
        Id = Guid.NewGuid(),
        Scope = scope ?? new ScopeRef { Kind = "branch", Id = Guid.NewGuid() },
        Name = name,
        Phone = phone,
        Address = address!,
        Notes = notes!,
        Status = CustomerStatus.Active,
        Revision = 1,
        SyncState = SyncState.Pending,
        UpdatedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
    };
}
