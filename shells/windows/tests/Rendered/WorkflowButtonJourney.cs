using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Threading;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.WorkOrders;

namespace Eitmad.WindowsShell.Tests.Rendered;

public sealed partial class WorkflowEnvironmentTests
{
    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public void ConnectedSalesButtonsConfirmWorkAndRetryLostReplies()
    {
        var root = Environment.GetEnvironmentVariable("EITMAD_E2E_ROOT");
        if (string.IsNullOrWhiteSpace(root)) Assert.Inconclusive("Prepare a named tools/workflow-environment.ps1 environment.");
        var enginePath = Environment.GetEnvironmentVariable("EITMAD_E2E_ENGINE");
        Assert.IsFalse(string.IsNullOrWhiteSpace(enginePath));
        var directory = Path.Combine(root!, "receptionist");
        var credential = File.ReadAllText(Path.Combine(directory, "route.json"));
        foreach (var prefix in new[] { "MEDIA", "PRICING", "QUOTATION", "CUSTOMER" })
            Environment.SetEnvironmentVariable($"EITMAD_{prefix}_CREDENTIAL_ID", credential);
        using var secrets = JsonDocument.Parse(File.ReadAllText(Path.Combine(root!, "secrets.json")));
        var bridge = new LostSalesReplyBridge(WindowsEngineBridge.Create(["--engine", enginePath!, "--runtime-directory", directory]));
        var sessions = new DesktopSessionController(bridge);
        try
        {
            WpfTestHost.Run(1338, 753, window =>
            {
                Finish(sessions.StartAsync()); Finish(WaitForConnection(bridge));
                RunConnectedSalesButtons(window, bridge, sessions, secrets.RootElement, root!);
                Finish(sessions.StopAsync());
            }, showSignIn: true, engine: bridge, sessions: sessions);
        }
        finally { sessions.DisposeAsync().AsTask().GetAwaiter().GetResult(); }
    }

    private static void RunConnectedSalesButtons(MainWindow window, LostSalesReplyBridge bridge, DesktopSessionController sessions, JsonElement secrets, string root)
    {
        var steps = new List<string>();
        void Record(string step)
        {
            steps.Add(step); Console.WriteLine(step);
            File.WriteAllText(Path.Combine(root, "sales-buttons-steps.json"), JsonSerializer.Serialize(steps));
        }
        void Login(string name)
        {
            SignIn(window, name, secrets.GetProperty($"{name}_password").GetString()!,
                name == "manager" ? AuthenticatedSurface.Manager : AuthenticatedSurface.Receptionist, bridge);
            Record($"Signed in: {name}");
        }
        void Change(string name) { SwitchAccount(window, bridge); Login(name); }
        void Press(DependencyObject view, string name)
        {
            WaitField(() => WpfTestHost.Descendants<Button>(view).Any(button => button.IsVisible && button.IsEnabled
                && System.Windows.Automation.AutomationProperties.GetName(button) == name), "visible enabled button: " + name);
            var button = WpfTestHost.Descendants<Button>(view).Single(button => button.IsVisible
                && System.Windows.Automation.AutomationProperties.GetName(button) == name);
            button.BringIntoView();
            ClickAutomation(view, name); Record($"Visible enabled button: {name}");
        }
        void Submit(DependencyObject view, string name, string kind, Func<Task> operation)
        {
            var previous = operation(); var count = bridge.SubmissionCount;
            Press(view, name);
            WaitField(() => !ReferenceEquals(previous, operation()) && bridge.SubmissionCount == count + 1
                && bridge.LastSubmittedKind == kind, "handler submitted: " + kind);
            Finish(operation());
            if (view is QuotationsView quotations)
            {
                WaitField(() => quotations.ViewModel.LastDraftLoad.IsCompleted && quotations.ViewModel.LifecycleAvailable, "current quotation refresh");
                Finish(quotations.ViewModel.LastDraftLoad);
            }
            if (view is OrdersView orders)
            {
                WaitField(() => orders.ViewModel.LastLoad.IsCompleted && orders.ViewModel.ActionsAvailable, "current Orders refresh");
                Finish(orders.ViewModel.LastLoad);
            }
        }
        Login("receptionist");
        var existingDrafts = ReadFields(bridge, Query.ForQuotationDraftList(new() { Limit = 100 })).AsQuotationDrafts()!.Items.Select(item => item.Snapshot.Id).ToHashSet();
        var originalOrders = ReadFields(bridge, Query.ForOrderList(new() { Limit = 100 })).AsOrders()!.Items.ToDictionary(item => item.Id, item => JsonSerializer.Serialize(item));
        ClickVisibleNamed(window.ReceptionistSurface, "ProductsNavButton");
        var catalog = WpfTestHost.FindByName<SalesCatalogView>(window.ReceptionistSurface, "CatalogContent");
        var model = (SalesCatalogViewModel)catalog.DataContext;
        Finish(model.LastCatalogOperation);
        WaitField(() => model.VisibleItems.Count >= 2, "button rehearsal public catalog");
        foreach (var product in new[] { false, true })
        {
            var item = model.VisibleItems.Single(item => item.Entry!.Name == (product ? "مرتبة الراحة المزدوجة" : "خزانة السكينة ببابين"));
            Press(catalog, item.SelectionName); Finish(model.LastCatalogOperation); WpfTestHost.CompleteLayout(window);
            if (product)
            {
                var selection = WpfTestHost.Descendants<ProductSelectionView>(catalog).Single();
                WpfTestHost.FindByName<ListBox>(selection, "VariantsList").SelectedIndex = 0;
                WaitField(() => model.ProductSelection?.CanAdd == true, "checked Product choice");
                ClickVisibleNamed(selection, "AddButton");
            }
            else
            {
                var selection = WpfTestHost.Descendants<FurnitureSelectionView>(catalog).Single();
                WpfTestHost.FindByName<ListBox>(selection, "SizesList").SelectedIndex = 0;
                WaitField(() => model.Selection?.CanAdd == true, "checked Furniture choice");
                ClickVisibleNamed(selection, "AddButton");
            }
            WaitField(() => model.QuotationLines.Count == (product ? 2 : 1), "add checked line button");
            Record(product ? "Product added with visible button" : "Furniture added with visible button");
            Press(catalog, "العودة إلى المنتجات");
        }
        var titleBar = WpfTestHost.FindByName<ShellTitleBar>(window.ReceptionistSurface, "ReceptionistTitleBar");
        ClickVisibleNamed(titleBar, "PrimaryActionButton");
        WaitField(() => model.IsReviewingQuotation, "quotation button");
        SetNamedField(catalog, "CustomerNameInput", "أسرة سامر السعدي"); Finish(model.LastCustomerSearch);
        Press(catalog, "اختيار العميل أسرة سامر السعدي");
        WaitField(() => model.SelectedCustomer?.Id is not null, "prepared family selected");
        var customer = ReadFields(bridge, Query.ForCustomerGet(new() { CustomerId = model.SelectedCustomer!.Id!.Value })).AsCustomer()!;
        Finish(WaitForCustomerConfirmation(bridge, customer));
        Finish(model.LastQuotationEvaluation);
        Assert.AreEqual(280_000m, model.FinalTotal);
        Assert.AreEqual(2, model.QuotationLines.Count);
        CollectionAssert.AreEquivalent(new[] { 185_000m, 95_000m }, model.QuotationLines.Select(line => line.UnitPrice).ToArray());
        Assert.IsTrue(model.QuotationLines.All(line => line.Quantity == 1));
        Press(catalog, "حفظ كمسودة");
        WaitField(() => !model.IsDraftBusy && !model.HasUnsavedDraftChanges, "draft save confirmed");
        var saved = ReadFields(bridge, Query.ForQuotationDraftList(new() { Limit = 100 })).AsQuotationDrafts()!.Items
            .Single(item => item.Snapshot.Evaluation.Customer.Id == customer.Id && item.Snapshot.Intent.DiscountBasisPoints == 0
                && !existingDrafts.Contains(item.Snapshot.Id));
        var draftId = saved.Snapshot.Id;
        Assert.AreEqual(280_000L, saved.Snapshot.Evaluation.Totals.SubtotalYer);
        Finish(model.OpenDraftAsync(draftId));
        Assert.IsFalse(model.HasUnsavedDraftChanges);
        Record("Saved and reopened actual family draft: " + draftId);
        CaptureFields(window, "buttons-draft-reopened");
        using (var guard = new ConnectedEditorGuard(model, window))
        {
            SetNamedField(guard.View, "DiscountInput", "6");
            guard.Window.Close(); WpfTestHost.CompleteLayout(guard.Window);
            Assert.IsTrue(guard.Window.UnsavedDialog.IsOpen);
            ClickVisibleNamed(guard.Window, "ContinueEditingButton");
            Assert.IsTrue(guard.Window.IsVisible); Assert.IsFalse(guard.Window.UnsavedDialog.IsOpen);
            Assert.AreEqual("6", model.DiscountInput);
            WpfTestHost.Capture(guard.Window, "sales-editor-continue");
            Finish(model.LastQuotationEvaluation);
            Press(guard.View, "حفظ كمسودة");
            WaitField(() => !model.IsDraftBusy && !model.HasUnsavedDraftChanges, "guard draft save confirmed");
        }
        Finish(model.LastQuotationEvaluation);
        WaitField(() => model.CanRequestDiscountApproval, "approval request enabled");
        Submit(catalog, "طلب موافقة", Command.QuotationApprovalRequestKind, () => model.LastApprovalRequest);
        Record($"Request handler finished: pending={model.IsDiscountPending}; dirty={model.HasUnsavedDraftChanges}; {model.QuotationNotice}; {model.DraftState}");
        WaitField(() => model.IsDiscountPending, "server confirmed pending approval: " + model.QuotationNotice);
        var draft = ReadFields(bridge, Query.ForQuotationDraftGet(new() { DraftId = draftId })).AsQuotationDraft()!;
        Assert.AreEqual(16_800L, draft.Snapshot.Evaluation.Totals.DiscountYer);
        Assert.AreEqual(263_200L, draft.Snapshot.Evaluation.Totals.TotalYer);
        CaptureFields(window, "buttons-approval-pending");

        Change("manager");
        ClickVisibleNamed(window, "QuotationsNavButton");
        var managerQuotes = WpfTestHost.Descendants<QuotationsView>(window).Single(view => view.IsVisible);
        Finish(managerQuotes.ViewModel.ActivateDraftsAsync());
        OpenButtonQuotation(managerQuotes, draftId);
        var requestId = managerQuotes.ViewModel.SelectedQuotation!.Approval!.RequestId;
        Submit(managerQuotes, "الموافقة على خصم عرض السعر", Command.QuotationApprovalDecideKind, () => managerQuotes.ViewModel.LastApprovalDecision);
        Record($"Approval handler finished: {managerQuotes.ViewModel.DecisionNotice}; {managerQuotes.ViewModel.ListState}; state={managerQuotes.ViewModel.SelectedQuotation?.Approval?.State}");
        WaitField(() => managerQuotes.ViewModel.SelectedQuotation?.Approval?.State == DiscountApprovalState.Approved, "approved button reply");
        Assert.AreEqual(requestId, managerQuotes.ViewModel.SelectedQuotation!.Approval!.RequestId);
        ScrollSaleToTop(managerQuotes);
        CaptureFields(window, "buttons-approved");

        Change("receptionist");
        ClickVisibleNamed(window.ReceptionistSurface, "HomeNavButton");
        ClickVisibleNamed(window.ReceptionistSurface, "QuotationsAction");
        var quotes = WpfTestHost.Descendants<QuotationsView>(window.ReceptionistSurface).Single();
        Finish(quotes.ViewModel.ActivateDraftsAsync()); OpenButtonQuotation(quotes, draftId);
        Submit(quotes, "إصدار عرض السعر", Command.QuotationIssueKind, () => quotes.ViewModel.LastLifecycleAction);
        WaitField(() => quotes.ViewModel.SelectedQuotation?.Lifecycle?.State == QuotationState.Issued, "issued button reply");
        var quotationNumber = quotes.ViewModel.SelectedQuotation!.Lifecycle!.Number;
        Record("Issued: " + quotationNumber);
        ShowSavedButtonDocument(window, quotes, bridge, draftId);
        bridge.LoseNext(Command.QuotationAcceptKind);
        Submit(quotes, "تأكيد قبول العميل لعرض السعر", Command.QuotationAcceptKind, () => quotes.ViewModel.LastLifecycleAction);
        WaitField(() => quotes.ViewModel.CanRetryLifecycle, "visible quotation retry after lost accepted reply");
        CaptureFields(window, "buttons-acceptance-retry");
        Submit(quotes, "إعادة محاولة عملية عرض السعر السابقة", Command.QuotationAcceptKind, () => quotes.ViewModel.LastLifecycleAction);
        WaitField(() => quotes.ViewModel.SelectedQuotation?.Lifecycle?.State == QuotationState.Accepted, "acceptance retry confirmed");
        Press(quotes, "تحويل عرض السعر إلى طلب"); Press(quotes, "تأكيد وإنشاء الطلب");
        var orders = window.ReceptionistSurface.PreviewOrders;
        WaitField(() => orders.IsVisible && orders.ViewModel.SelectedOrder?.Record?.Source.Quotation.Id == draftId, "button conversion confirmed");
        var orderId = orders.ViewModel.SelectedOrder!.Id;
        CaptureFields(window, "buttons-converted");

        Change("manager"); ClickVisibleNamed(window, "OrdersNavButton");
        var managerOrders = WpfTestHost.Descendants<OrdersView>(window).Single(view => view.IsVisible);
        Finish(managerOrders.ViewModel.ActivateAsync()); OpenButtonOrder(managerOrders, orderId);
        SetField(managerOrders, "Assignment", "ورشة أزرار تجريبية");
        var due = WpfTestHost.Descendants<DatePicker>(managerOrders).Single(picker => picker.IsVisible);
        due.SelectedDate = DateTime.Today.AddDays(7); due.GetBindingExpression(DatePicker.SelectedDateProperty)?.UpdateSource();
        Submit(managerOrders, "بدء أمر العمل المؤكد", Command.OrderWorkStartKind, () => managerOrders.ViewModel.LastAction);
        WaitField(() => managerOrders.ViewModel.SelectedOrder?.Record?.State == OrderState.InProduction, "start work button confirmed");
        Record($"Started work: complete available={managerOrders.ViewModel.SelectedOrder?.CanCompleteWork}; {managerOrders.ViewModel.ListState}; {managerOrders.ViewModel.ActionNotice}");
        Submit(managerOrders, "تأكيد اكتمال أمر العمل", Command.OrderWorkCompleteKind, () => managerOrders.ViewModel.LastAction);
        WaitField(() => managerOrders.ViewModel.LastLoad.IsCompleted && managerOrders.ViewModel.SelectedOrder?.Record?.State == OrderState.Ready,
            "direct Orders Complete confirmed Ready");
        Assert.AreEqual(WorkState.Completed, managerOrders.ViewModel.SelectedOrder!.Record!.Work.Single().State);
        Assert.IsTrue(managerOrders.IsVisible);
        ScrollSaleToTop(managerOrders);
        CaptureFields(window, "buttons-ready");

        Change("receptionist"); ClickVisibleNamed(window.ReceptionistSurface, "HomeNavButton");
        ClickVisibleNamed(window.ReceptionistSurface, "OrdersAction");
        Finish(orders.ViewModel.ActivateAsync()); OpenButtonOrder(orders, orderId);
        SetField(orders, "Recipient", "مستلم أزرار تجريبي");
        bridge.LoseNext(Command.OrderDeliverKind);
        Submit(orders, "تأكيد تسليم الطلب", Command.OrderDeliverKind, () => orders.ViewModel.LastAction);
        WaitField(() => orders.ViewModel.CanRetryOrder, "visible delivery retry after lost accepted reply");
        CaptureFields(window, "buttons-delivery-retry");
        Submit(orders, "إعادة محاولة عملية الطلب المعلقة", Command.OrderDeliverKind, () => orders.ViewModel.LastAction);
        WaitField(() => orders.ViewModel.SelectedOrder?.Record?.State == OrderState.Delivered && !orders.ViewModel.HasOrderRetry, "delivery retry confirmed");
        var confirmed = ReadFields(bridge, Query.ForOrderGet(new() { OrderId = orderId })).AsOrders()!.Items.Single();
        Assert.AreEqual(OrderState.Delivered, confirmed.State);
        Assert.AreEqual(2, bridge.ExactRetries);
        Assert.AreEqual(2, confirmed.Source.Quotation.Evaluation.Lines.Length);
        Assert.AreEqual(263_200L, confirmed.Source.Quotation.Evaluation.Totals.TotalYer);
        Assert.IsNotNull(confirmed.Delivery);
        Assert.AreEqual(1, bridge.Count(Command.OrderConvertKind));
        VerifySaleReadback(bridge, confirmed, originalOrders);
        CaptureFields(window, "buttons-delivered");
        Record("Passed: two lines, approval, issue, acceptance, conversion, production, delivery; two exact retries after real successful commands with withheld replies.");
        File.WriteAllText(Path.Combine(root, "sales-buttons-proof.json"), JsonSerializer.Serialize(new
        {
            draftId, quotationNumber, orderId, orderNumber = confirmed.Number, workNumber = confirmed.Work.Single().Number,
            state = confirmed.State.ToString(), totalYer = confirmed.Source.Quotation.Evaluation.Totals.TotalYer,
            lines = confirmed.Source.Quotation.Evaluation.Lines.Length, exactRetries = bridge.ExactRetries,
            retryIdentity = bridge.RetryEvidence, deliveryId = confirmed.Delivery.Id, steps,
        }));
        SwitchAccount(window, bridge);
        var generation = bridge.Snapshot.Generation;
        Finish(sessions.StopAsync()); Finish(sessions.StartAsync()); Finish(WaitForConnection(bridge));
        Assert.IsTrue(bridge.Snapshot.Generation > generation);
        Login("receptionist");
        VerifySaleReadback(bridge, confirmed, originalOrders);
        ClickVisibleNamed(window.ReceptionistSurface, "HomeNavButton"); ClickVisibleNamed(window.ReceptionistSurface, "OrdersAction");
        Finish(orders.ViewModel.ActivateAsync()); OpenButtonOrder(orders, orderId);
        ScrollSaleToTop(orders);
        CaptureFieldSizes(window, "buttons-delivered-restarted");
        Record($"Restart readback passed: {quotationNumber} > {confirmed.Number} > {confirmed.Work.Single().Number}; Delivered; 263200 YER.");
        File.WriteAllText(Path.Combine(root, "sales-restart-proof.json"), JsonSerializer.Serialize(new { quotationNumber, confirmed.Number,
            orderId, workNumber = confirmed.Work.Single().Number, state = "Delivered", totalYer = 263_200L, engineRestarted = true,
            retainedOriginalOrders = originalOrders.Count, exactRetries = bridge.ExactRetries }));
        SwitchAccount(window, bridge);
    }

    private static void ScrollSaleToTop(FrameworkElement view)
    {
        foreach (var scroll in WpfTestHost.Descendants<ScrollViewer>(view).Where(scroll => scroll.IsVisible)) scroll.ScrollToTop();
        WpfTestHost.CompleteLayout(view);
    }

    private sealed class ConnectedEditorGuard : IDisposable
    {
        public SalesCatalogView View { get; }
        public QuotationEditorWindow Window { get; }
        public ConnectedEditorGuard(SalesCatalogViewModel model, MainWindow owner)
        {
            View = new() { DataContext = model, ShowQuotationHeader = true };
            Window = new(model, View) { Owner = owner, Width = 1050, Height = 780, Left = -10000, Top = 0, ShowInTaskbar = false };
            Window.Show(); WpfTestHost.CompleteLayout(Window);
        }
        public void Dispose() => Window.CloseInvalidated();
    }

    private static void VerifySaleReadback(IEngineShellBridge bridge, OrderRecord expected, Dictionary<Guid, string> originalOrders)
    {
        var task = ReadServerConfirmed(bridge, Query.ForOrderList(new() { Limit = 100 }), result => result.AsOrders()!.ServerAvailable); Finish(task);
        var orders = task.Result.Outcome.Payload.AsOrders()!.Items;
        Assert.AreEqual(originalOrders.Count + 1, orders.Length);
        var actual = orders.Single(order => order.Source.Quotation.Id == expected.Source.Quotation.Id);
        Assert.AreEqual(JsonSerializer.Serialize(expected), JsonSerializer.Serialize(actual));
        foreach (var (id, value) in originalOrders) Assert.AreEqual(value, JsonSerializer.Serialize(orders.Single(order => order.Id == id)));
        var quotation = ReadFields(bridge, Query.ForQuotationList(new() { Limit = 100 })).AsQuotations()!.Items.Single(item => item.Quotation.Id == expected.Source.Quotation.Id);
        Assert.AreEqual(QuotationState.Converted, quotation.State);
        foreach (var query in new[] { Query.ForOrderCustomerDocument(new() { OrderId = expected.Id }), Query.ForOrderQuotationDocument(new() { OrderId = expected.Id }) })
        {
            var document = ReadFields(bridge, query).AsCustomerDocument()!;
            Assert.AreEqual(263_200L, document.TotalYer); Assert.AreEqual(16_800L, document.DiscountYer);
            Assert.AreEqual(2, document.Lines.Length); Assert.AreEqual(expected.Source.Quotation.Evaluation.Customer.Id, document.Customer.Id);
        }
    }

    private static void ShowSavedButtonDocument(MainWindow owner, QuotationsView quotes, IEngineShellBridge bridge, Guid draftId)
    {
        var saved = ReadFields(bridge, Query.ForQuotationCustomerDocument(new() { DraftId = draftId })).AsCustomerDocument()!;
        Assert.AreEqual("أسرة سامر السعدي", saved.Customer.Name); Assert.AreEqual(280_000L, saved.SubtotalYer);
        Assert.AreEqual(16_800L, saved.DiscountYer); Assert.AreEqual(263_200L, saved.TotalYer); Assert.AreEqual(600L, saved.DiscountBasisPoints);
        Assert.AreEqual(2, saved.Lines.Length); Assert.IsTrue(saved.Lines.All(line => line.Quantity == 1));
        var pages = new CustomerDocumentPages(QuotationCustomerDocument.CreateSaved(saved)); pages.ComputePageCount();
        var page = pages.GetPage(0);
        var bitmap = new RenderTargetBitmap((int)Math.Ceiling(page.Size.Width), (int)Math.Ceiling(page.Size.Height), 96, 96, PixelFormats.Pbgra32);
        bitmap.Render(page.Visual);
        var encoder = new PngBitmapEncoder(); encoder.Frames.Add(BitmapFrame.Create(bitmap));
        using (var stream = File.Create(Path.Combine(Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR")!, "sales-saved-document-page.png"))) encoder.Save(stream);
        var source = quotes.ViewModel.SelectedQuotation!.Lifecycle!.Quotation.Evaluation.Lines;
        foreach (var line in source)
        {
            var printed = saved.Lines.Single(item => item.Name == line.Name);
            Assert.AreEqual(line.Description, printed.Description); Assert.AreEqual(line.VariantName, printed.VariantName);
            Assert.AreEqual(JsonSerializer.Serialize(line.Dimensions), JsonSerializer.Serialize(printed.Dimensions));
            Assert.AreEqual(line.Price.UnitPriceYer, printed.UnitPriceYer);
        }
        async Task Inspect()
        {
            await Dispatcher.Yield(DispatcherPriority.Background);
            using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(25));
            Window? previewWindow = null;
            try
            {
                while ((previewWindow = Application.Current.Windows.OfType<Window>().FirstOrDefault(window => window.Content is PrintPreview)) is null)
                    await Task.Delay(50, deadline.Token);
                var preview = (PrintPreview)previewWindow.Content;
                previewWindow.WindowState = WindowState.Maximized; WpfTestHost.CompleteLayout(previewWindow);
                var text = new TextRange(preview.Document.ContentStart, preview.Document.ContentEnd).Text;
                foreach (var value in new[] { saved.Number, saved.Customer.Name, "خزانة السكينة ببابين", "مرتبة الراحة المزدوجة", "280,000", "16,800", "263,200" })
                    Assert.IsTrue(text.Contains(value), "Saved rendered document missing " + value);
                DocumentViewer.FitToWidthCommand.Execute(null, preview.Pages); WpfTestHost.CompleteLayout(previewWindow);
                Assert.IsNotNull(preview.Pages.Document); Assert.IsTrue(preview.Pages.Document.DocumentPaginator.PageCount > 0);
                WpfTestHost.Capture(previewWindow, "sales-saved-document-page-width");
            }
            finally { previewWindow?.Close(); }
        }
        var inspection = Inspect(); ClickAutomation(quotes, "طباعة عرض السعر المؤكد"); Finish(inspection);
    }

    private static void ClickVisibleNamed(DependencyObject root, string name)
    {
        if (root is FrameworkElement element) WpfTestHost.CompleteLayout(element);
        var button = WpfTestHost.FindByName<Button>(root, name);
        button.BringIntoView();
        if (root is FrameworkElement parent) WpfTestHost.CompleteLayout(parent);
        Assert.IsTrue(button.IsVisible, $"Expected visible button: {name}");
        ClickField(button);
    }

    private static void OpenButtonQuotation(QuotationsView view, Guid id)
    {
        WaitField(() => view.ViewModel.VisibleQuotations.Any(row => row.Id == id), "quotation row loaded");
        var table = WpfTestHost.FindByName<OperationsTable>(view, "QuotationTable");
        table.RaiseEvent(new RowInvokedEventArgs(OperationsTable.RowInvokedEvent, table, view.ViewModel.VisibleQuotations.Single(row => row.Id == id)));
        WpfTestHost.CompleteLayout(view);
    }

    private static void OpenButtonOrder(OrdersView view, Guid id)
    {
        WaitField(() => view.ViewModel.VisibleOrders.Any(row => row.Id == id), "order row loaded");
        var table = WpfTestHost.Descendants<OperationsTable>(view).Single(table => table.IsVisible && table.IsRowInvocationEnabled);
        table.RaiseEvent(new RowInvokedEventArgs(OperationsTable.RowInvokedEvent, table, view.ViewModel.VisibleOrders.Single(row => row.Id == id)));
        WpfTestHost.CompleteLayout(view);
    }

    // Only the response is withheld. Every command, query and subscription uses the real engine.
    private sealed class LostSalesReplyBridge(IEngineShellBridge inner) : IEngineShellBridge
    {
        private string? loseKind;
        private readonly Dictionary<string, (Guid Key, string Payload, string Result)> lost = [];
        public List<object> RetryEvidence { get; } = [];
        public int ExactRetries { get; private set; }
        public int SubmissionCount { get; private set; }
        public string? LastSubmittedKind { get; private set; }
        public Command? LastSubmittedCommand { get; private set; }
        public CommandResponseEnvelope? LastReply { get; private set; }
        public CheckSalesConfiguration? LastConfigurationInput { get; private set; }
        public QueryResponseEnvelope? LastConfigurationReply { get; private set; }
        private readonly Dictionary<string, int> submittedKinds = [];
        public int Count(string kind) => submittedKinds.GetValueOrDefault(kind);
        public void LoseNext(string kind) => loseKind = kind;
        public async Task<CommandResponseEnvelope> SubmitCommandAsync(Command command, Guid idempotencyKey, CancellationToken cancellationToken = default)
        {
            ++SubmissionCount; LastSubmittedKind = command.Kind;
            LastSubmittedCommand = command;
            submittedKinds[command.Kind] = Count(command.Kind) + 1;
            var payload = JsonSerializer.Serialize(command);
            if (lost.TryGetValue(command.Kind, out var original))
            {
                Assert.AreEqual(original.Key, idempotencyKey, "The visible retry changed the key.");
                Assert.AreEqual(original.Payload, payload, "The visible retry changed the payload.");
            }
            var reply = await inner.SubmitCommandAsync(command, idempotencyKey, cancellationToken);
            LastReply = reply;
            Console.WriteLine($"Command reply {command.Kind}: {reply.Outcome.Status}; {reply.Outcome.Payload.Code}; approval={reply.Outcome.Payload.AsDiscountApproval()?.State}");
            if (reply.Outcome.Status != CommandOutcomeStatus.Succeeded) return reply;
            if (loseKind == command.Kind)
            {
                loseKind = null; lost.Add(command.Kind, (idempotencyKey, payload, JsonSerializer.Serialize(reply.Outcome.Payload)));
                throw new IOException("Synthetic loss after real server acceptance.");
            }
            if (lost.Remove(command.Kind, out var accepted))
            {
                Assert.AreEqual(accepted.Result, JsonSerializer.Serialize(reply.Outcome.Payload), "The retry must return the same accepted server record.");
                ++ExactRetries;
                RetryEvidence.Add(new { kind = command.Kind, key = idempotencyKey,
                    payloadSha256 = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(System.Text.Encoding.UTF8.GetBytes(payload))),
                    exactKeyAndPayload = true, identicalServerRecord = true });
            }
            return reply;
        }
        public event Action<EngineSupervisionSnapshot>? StateChanged { add => inner.StateChanged += value; remove => inner.StateChanged -= value; }
        public EngineSupervisionSnapshot Snapshot => inner.Snapshot;
        public bool SupportsCapability(string capability) => inner.SupportsCapability(capability);
        public Task StartAsync(CancellationToken cancellationToken = default) => inner.StartAsync(cancellationToken);
        public Task StopAsync(CancellationToken cancellationToken = default) => inner.StopAsync(cancellationToken);
        public Task<DesktopSessionState> SignInAsync(string username, string password, CancellationToken cancellationToken = default) => inner.SignInAsync(username, password, cancellationToken);
        public Task<DesktopSessionState?> GetSessionStateAsync(CancellationToken cancellationToken = default) => inner.GetSessionStateAsync(cancellationToken);
        public Task SignOutAsync(CancellationToken cancellationToken = default) => inner.SignOutAsync(cancellationToken);
        public async Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default)
        {
            var reply = await inner.QueryAsync(query, cancellationToken);
            if (query.AsSalesCatalogCheck() is { } input)
            {
                LastConfigurationInput = input;
                LastConfigurationReply = reply;
            }
            return reply;
        }
        public Task<IEngineSubscription> SubscribeAsync(Subscription subscription, CancellationToken cancellationToken = default) => inner.SubscribeAsync(subscription, cancellationToken);
        public ValueTask DisposeAsync() => inner.DisposeAsync();
    }
}
