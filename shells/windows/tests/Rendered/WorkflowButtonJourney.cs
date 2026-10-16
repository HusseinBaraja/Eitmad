using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
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
                RunConnectedSalesButtons(window, bridge, secrets.RootElement, root!);
                Finish(sessions.StopAsync());
            }, showSignIn: true, engine: bridge, sessions: sessions);
        }
        finally { sessions.DisposeAsync().AsTask().GetAwaiter().GetResult(); }
    }

    private static void RunConnectedSalesButtons(MainWindow window, LostSalesReplyBridge bridge, JsonElement secrets, string root)
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
        Login("receptionist");
        ClickVisibleNamed(window.ReceptionistSurface, "ProductsNavButton");
        var catalog = WpfTestHost.FindByName<SalesCatalogView>(window.ReceptionistSurface, "CatalogContent");
        var model = (SalesCatalogViewModel)catalog.DataContext;
        Finish(model.LastCatalogOperation);
        WaitField(() => model.VisibleItems.Count >= 2, "button rehearsal public catalog");
        foreach (var product in new[] { false, true })
        {
            var item = model.VisibleItems.First(item => (item.Entry!.Dimensions is null) == product);
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
        Press(catalog, "عميل جديد");
        var customerName = "عميل أزرار تجريبي " + Guid.NewGuid().ToString("N")[..6];
        SetNamedField(catalog, "CustomerNameInput", customerName);
        SetNamedField(catalog, "PhoneInput", "777123456");
        Press(catalog, "حفظ العميل");
        WaitField(() => !model.IsCustomerSaveBusy && model.SelectedCustomer?.Id is not null, "button customer save");
        var customer = ReadFields(bridge, Query.ForCustomerGet(new() { CustomerId = model.SelectedCustomer!.Id!.Value })).AsCustomer()!;
        Finish(WaitForCustomerConfirmation(bridge, customer));
        SetNamedField(catalog, "DiscountInput", "6");
        Finish(model.LastQuotationEvaluation);
        WaitField(() => model.CanRequestDiscountApproval, "approval request enabled");
        Press(catalog, "طلب موافقة"); Finish(model.LastApprovalRequest);
        for (var retry = 0; !model.IsDiscountPending && model.CanRequestDiscountApproval && retry < 4; ++retry)
        {
            Record("Approval request recovery: " + model.QuotationNotice);
            Finish(Task.Delay(200)); Press(catalog, "طلب موافقة"); Finish(model.LastApprovalRequest);
        }
        WaitField(() => model.IsDiscountPending, "server confirmed pending approval: " + model.QuotationNotice);
        var draft = ReadFields(bridge, Query.ForQuotationDraftList(new() { Limit = 100 })).AsQuotationDrafts()!
            .Items.Single(item => item.Snapshot.Evaluation.Customer.Id == customer.Id);
        var draftId = draft.Snapshot.Id;
        CaptureFields(window, "buttons-approval-pending");

        Change("manager");
        ClickVisibleNamed(window, "QuotationsNavButton");
        var managerQuotes = WpfTestHost.Descendants<QuotationsView>(window).Single(view => view.IsVisible);
        Finish(managerQuotes.ViewModel.ActivateDraftsAsync());
        OpenButtonQuotation(managerQuotes, draftId);
        Press(managerQuotes, "الموافقة على خصم عرض السعر"); Finish(managerQuotes.ViewModel.LastApprovalDecision);
        WaitField(() => managerQuotes.ViewModel.SelectedQuotation?.Approval?.State == DiscountApprovalState.Approved, "approved button reply");
        CaptureFields(window, "buttons-approved");

        Change("receptionist");
        ClickVisibleNamed(window.ReceptionistSurface, "HomeNavButton");
        ClickVisibleNamed(window.ReceptionistSurface, "QuotationsAction");
        var quotes = WpfTestHost.Descendants<QuotationsView>(window.ReceptionistSurface).Single();
        Finish(quotes.ViewModel.ActivateDraftsAsync()); OpenButtonQuotation(quotes, draftId);
        Press(quotes, "إصدار عرض السعر"); Finish(quotes.ViewModel.LastLifecycleAction);
        WaitField(() => quotes.ViewModel.SelectedQuotation?.Lifecycle?.State == QuotationState.Issued, "issued button reply");
        bridge.LoseNext(Command.QuotationAcceptKind);
        Press(quotes, "تأكيد قبول العميل لعرض السعر"); Finish(quotes.ViewModel.LastLifecycleAction);
        WaitField(() => quotes.ViewModel.CanRetryLifecycle, "visible quotation retry after lost accepted reply");
        CaptureFields(window, "buttons-acceptance-retry");
        Press(quotes, "إعادة محاولة عملية عرض السعر السابقة"); Finish(quotes.ViewModel.LastLifecycleAction);
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
        Press(managerOrders, "بدء أمر العمل المؤكد"); Finish(managerOrders.ViewModel.LastAction);
        WaitField(() => managerOrders.ViewModel.SelectedOrder?.Record?.State == OrderState.InProduction, "start work button confirmed");
        Record($"Started work: complete available={managerOrders.ViewModel.SelectedOrder?.CanCompleteWork}; {managerOrders.ViewModel.ListState}; {managerOrders.ViewModel.ActionNotice}");
        Press(managerOrders, "فتح تصنيع الطلب");
        var production = WpfTestHost.Descendants<WorkOrdersView>(window).Single();
        Finish(production.ViewModel.LastLoad);
        WaitField(() => production.ViewModel.SelectedWorkOrder?.Record?.OrderId == orderId, "visible production deep link");
        Press(production, "تغيير حالة أمر العمل"); Finish(production.ViewModel.LastAction);
        WaitField(() => production.ViewModel.SelectedWorkOrder?.Record?.State == WorkState.Completed, "complete work button confirmed");
        CaptureFields(window, "buttons-ready");

        Change("receptionist"); ClickVisibleNamed(window.ReceptionistSurface, "HomeNavButton");
        ClickVisibleNamed(window.ReceptionistSurface, "OrdersAction");
        Finish(orders.ViewModel.ActivateAsync()); OpenButtonOrder(orders, orderId);
        SetField(orders, "Recipient", "مستلم أزرار تجريبي");
        bridge.LoseNext(Command.OrderDeliverKind);
        Press(orders, "تأكيد تسليم الطلب"); Finish(orders.ViewModel.LastAction);
        WaitField(() => orders.ViewModel.CanRetryOrder, "visible delivery retry after lost accepted reply");
        CaptureFields(window, "buttons-delivery-retry");
        Press(orders, "إعادة محاولة عملية الطلب المعلقة"); Finish(orders.ViewModel.LastAction);
        WaitField(() => orders.ViewModel.SelectedOrder?.Record?.State == OrderState.Delivered && !orders.ViewModel.HasOrderRetry, "delivery retry confirmed");
        var confirmed = ReadFields(bridge, Query.ForOrderGet(new() { OrderId = orderId })).AsOrders()!.Items.Single();
        Assert.AreEqual(OrderState.Delivered, confirmed.State);
        Assert.AreEqual(2, bridge.ExactRetries);
        Assert.AreEqual(2, confirmed.Source.Quotation.Evaluation.Lines.Length);
        CaptureFields(window, "buttons-delivered");
        Record("Passed: two lines, approval, issue, acceptance, conversion, production, delivery; two exact retries after real successful commands with withheld replies.");
        File.WriteAllText(Path.Combine(root, "sales-buttons-proof.json"), JsonSerializer.Serialize(new
        {
            orderId, state = confirmed.State.ToString(), totalYer = confirmed.Source.Quotation.Evaluation.Totals.TotalYer,
            lines = confirmed.Source.Quotation.Evaluation.Lines.Length, exactRetries = bridge.ExactRetries, steps,
        }));
        SwitchAccount(window, bridge);
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
        private readonly Dictionary<string, (Guid Key, string Payload)> lost = [];
        public int ExactRetries { get; private set; }
        public int SubmissionCount { get; private set; }
        public string? LastSubmittedKind { get; private set; }
        public void LoseNext(string kind) => loseKind = kind;
        public async Task<CommandResponseEnvelope> SubmitCommandAsync(Command command, Guid idempotencyKey, CancellationToken cancellationToken = default)
        {
            ++SubmissionCount; LastSubmittedKind = command.Kind;
            var payload = JsonSerializer.Serialize(command);
            if (lost.TryGetValue(command.Kind, out var original))
            {
                Assert.AreEqual(original.Key, idempotencyKey, "The visible retry changed the key.");
                Assert.AreEqual(original.Payload, payload, "The visible retry changed the payload.");
            }
            var reply = await inner.SubmitCommandAsync(command, idempotencyKey, cancellationToken);
            if (reply.Outcome.Status != CommandOutcomeStatus.Succeeded) return reply;
            if (loseKind == command.Kind)
            {
                loseKind = null; lost.Add(command.Kind, (idempotencyKey, payload));
                throw new IOException("Synthetic loss after real server acceptance.");
            }
            if (lost.Remove(command.Kind)) ++ExactRetries;
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
        public Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default) => inner.QueryAsync(query, cancellationToken);
        public Task<IEngineSubscription> SubscribeAsync(Subscription subscription, CancellationToken cancellationToken = default) => inner.SubscribeAsync(subscription, cancellationToken);
        public ValueTask DisposeAsync() => inner.DisposeAsync();
    }
}
