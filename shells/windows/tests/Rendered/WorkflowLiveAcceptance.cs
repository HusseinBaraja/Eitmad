using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.WorkOrders;

namespace Eitmad.WindowsShell.Tests.Rendered;

public sealed partial class WorkflowEnvironmentTests
{
    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public void ConnectedCustomizationFieldsRejectBoundsAndRetainEvaluatedDraft() =>
        RunLiveAcceptance((window, bridge, sessions, secrets, root) =>
        {
            SignIn(window, "receptionist", secrets.GetProperty("receptionist_password").GetString()!, AuthenticatedSurface.Receptionist, bridge);
            var retained = LiveRetainedRecords(bridge);
            var drafts = ReadFields(bridge, Query.ForQuotationDraftList(new() { Limit = 100 })).AsQuotationDrafts()!.Items
                .Select(item => item.Snapshot.Id).ToHashSet();
            ClickVisibleNamed(window.ReceptionistSurface, "ProductsNavButton");
            var catalog = WpfTestHost.FindByName<SalesCatalogView>(window.ReceptionistSurface, "CatalogContent");
            var model = (SalesCatalogViewModel)catalog.DataContext;
            Finish(model.LastCatalogOperation);
            var item = model.VisibleItems.Single(item => item.Entry!.Name == "خزانة السكينة حسب المقاس");
            ClickAutomation(catalog, item.SelectionName); Finish(model.LastCatalogOperation);
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<FurnitureSelectionView>(catalog).Single();
            var selection = model.Selection!;
            WpfTestHost.FindByName<ListBox>(view, "SizesList").SelectedItem = selection.Sizes.Single(size => size.Price == 185_000m);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<ListBox>(view, "ColorsList").SelectedItem = selection.Colors.Single(option => option.Name.Contains("جوز"));
            WpfTestHost.FindByName<ListBox>(view, "HandlesList").SelectedItem = selection.Handles.Single(option => option.Name.Contains("نحاس"));
            SetField(view, "WidthCm", "140"); SetField(view, "HeightCm", "210"); SetField(view, "DepthCm", "60");
            Finish(model.LastCatalogOperation);
            Assert.IsTrue(selection.CanAdd, selection.Guidance);
            Assert.IsTrue(selection.Configuration!.ServerAvailable);
            Assert.AreEqual(CommandOutcomeStatus.Succeeded, bridge.LastConfigurationReply!.Outcome.Status);
            Assert.AreEqual(15_000L, selection.Configuration.AdditionsYer);
            Assert.AreEqual(200_000L, selection.Configuration.Price.TotalYer);
            AssertDimensions(selection.Configuration.Dimensions!);
            Assert.AreEqual("200,000 ر.ي", WpfTestHost.FindByName<TextBlock>(view, "LineTotal").Text);
            CaptureFieldSizes(window, "custom-valid");

            SetField(view, "WidthCm", "180.1"); Finish(model.LastCatalogOperation);
            Assert.AreEqual(1801L, bridge.LastConfigurationInput!.Dimensions!.WidthMm);
            Assert.AreEqual(CommandOutcomeStatus.Failed, bridge.LastConfigurationReply!.Outcome.Status);
            Assert.AreEqual(ProtocolIds.ErrorCodes.EitmadErrorPricingInvalidV1, bridge.LastConfigurationReply.Outcome.Payload.Code);
            Assert.IsFalse(selection.IsEvaluating);
            Assert.IsNull(selection.Configuration);
            Assert.IsFalse(selection.CanAdd);
            Assert.AreEqual("—", WpfTestHost.FindByName<TextBlock>(view, "LineTotal").Text);
            Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "AddButton").IsEnabled);
            Assert.AreEqual(0, model.QuotationLines.Count);
            StringAssert.Contains(selection.Guidance, "مقاسات");
            var rejectedMessage = selection.Guidance;
            CaptureFieldSizes(window, "custom-invalid");

            SetField(view, "WidthCm", "140"); Finish(model.LastCatalogOperation);
            Assert.IsTrue(selection.CanAdd, selection.Guidance);
            ClickVisibleNamed(view, "AddButton");
            WaitField(() => model.QuotationLines.Count == 1, "restored valid customization Add handler");
            ClickAutomation(catalog, "العودة إلى المنتجات");
            var titleBar = WpfTestHost.FindByName<ShellTitleBar>(window.ReceptionistSurface, "ReceptionistTitleBar");
            ClickVisibleNamed(titleBar, "PrimaryActionButton");
            WaitField(() => model.IsReviewingQuotation, "custom draft review");
            SetNamedField(catalog, "CustomerNameInput", "أسرة سامر السعدي"); Finish(model.LastCustomerSearch);
            ClickAutomation(catalog, "اختيار العميل أسرة سامر السعدي"); Finish(model.LastQuotationEvaluation);
            Assert.AreEqual(200_000m, model.FinalTotal);
            ClickAutomation(catalog, "حفظ كمسودة");
            WaitField(() => !model.IsDraftBusy && !model.HasUnsavedDraftChanges, "new customization saved");
            var saved = ReadFields(bridge, Query.ForQuotationDraftList(new() { Limit = 100 })).AsQuotationDrafts()!.Items
                .Single(item => !drafts.Contains(item.Snapshot.Id));
            // Draft snapshots are local-first; prove live authority independently of that flag.
            var evaluated = ReadServerConfirmed(bridge, Query.ForQuotationEvaluate(saved.Snapshot.Intent), result => result.AsQuotationEvaluation()!.ServerAvailable);
            Finish(evaluated);
            var live = evaluated.Result.Outcome.Payload.AsQuotationEvaluation()!;
            Assert.AreEqual(0, live.Errors.Length);
            Assert.AreEqual(200_000L, live.Totals.TotalYer);
            Assert.AreEqual(JsonSerializer.Serialize(live.Lines), JsonSerializer.Serialize(saved.Snapshot.Evaluation.Lines));
            Assert.AreEqual(200_000L, saved.Snapshot.Evaluation.Totals.TotalYer);
            var line = saved.Snapshot.Evaluation.Lines.Single();
            AssertDimensions(line.Dimensions!);
            Assert.AreEqual("خزانة السكينة حسب المقاس", line.Name);
            Assert.AreEqual(selection.SelectedSize!.Name, line.VariantName);
            Assert.AreEqual(selection.SelectedColor!.Name, line.ColorName);
            Assert.AreEqual(selection.SelectedHandle!.Name, line.HandleName);
            Assert.AreEqual(1L, Convert.ToInt64(line.Quantity));
            Assert.AreEqual(200_000L, line.Price.UnitPriceYer);
            Finish(model.OpenDraftAsync(saved.Snapshot.Id));
            Assert.AreEqual(200_000m, model.FinalTotal);
            Assert.IsFalse(model.HasUnsavedDraftChanges);
            AssertLiveRetainedRecords(bridge, retained);
            File.WriteAllText(Path.Combine(root, "live-customization-proof.json"), JsonSerializer.Serialize(new
            {
                draftId = saved.Snapshot.Id, totalYer = saved.Snapshot.Evaluation.Totals.TotalYer,
                baseYer = selection.SelectedSize.Price, additionsYer = 15_000, line,
                invalidWidthMm = 1801, rejectedMessage, invalidTotal = "—", invalidAddEnabled = false,
                retainedRecords = retained.Count, savedAndReopened = true, liveServerConfirmed = live.ServerAvailable,
            }));
        });

    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public void ConnectedOrdersDueDateFieldsRejectYesterdayAndAcceptToday() => RunConnectedDueDateFields(true);

    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public void ConnectedWorkOrdersDueDateFieldsRejectYesterdayAndAcceptToday() => RunConnectedDueDateFields(false);

    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public void ConnectedAcceptedDueDatesReadBackWithRenderedLabels() =>
        RunLiveAcceptance((window, bridge, sessions, secrets, root) =>
        {
            var ids = Environment.GetEnvironmentVariable("EITMAD_E2E_DUE_ORDER_IDS");
            if (string.IsNullOrWhiteSpace(ids)) Assert.Inconclusive("Supply the two synthetic deadline order IDs for readback.");
            var orderIds = ids!.Split(',').Select(Guid.Parse).ToArray();
            Assert.AreEqual(2, orderIds.Length);
            SignIn(window, "manager", secrets.GetProperty("manager_password").GetString()!, AuthenticatedSurface.Manager, bridge);
            var retained = LiveRetainedRecords(bridge);
            ClickVisibleNamed(window, "WorkOrdersNavButton");
            var view = WpfTestHost.Descendants<WorkOrdersView>(window).Single();
            Finish(view.ViewModel.ActivateAsync());
            var today = DateTimeOffset.UtcNow.ToOffset(TimeSpan.FromHours(3)).Date;
            var next = DateOnly.FromDateTime(today).AddDays(1);
            var expected = new DateTimeOffset(next.Year, next.Month, next.Day, 0, 0, 0, TimeSpan.Zero)
                .ToUnixTimeMilliseconds() - 10_800_001;
            var proof = new List<object>();
            foreach (var id in orderIds)
            {
                var order = ReadLiveOrder(bridge, id);
                Assert.AreEqual(OrderState.InProduction, order.State);
                Assert.AreEqual(expected, order.Work.Single().DueAt);
                Assert.AreEqual(WorkState.InProgress, order.Work.Single().State);
                var row = view.ViewModel.VisibleWorkOrders.Single(item => item.Record!.OrderId == id);
                Assert.AreEqual(DateOnly.FromDateTime(today), row.DueDate);
                Assert.AreEqual(WorkOrderStatus.InProgress, row.Status);
                if (view.ViewModel.IsDetailVisible) ClickVisibleNamed(view, "BackToWorkOrdersButton");
                var table = WpfTestHost.Descendants<OperationsTable>(view).Single(table => table.IsVisible && table.IsRowInvocationEnabled);
                table.RaiseEvent(new RowInvokedEventArgs(OperationsTable.RowInvokedEvent, table, row));
                WpfTestHost.CompleteLayout(window);
                var date = WpfTestHost.Descendants<TextBlock>(view).Single(text => text.IsVisible
                    && text.GetBindingExpression(TextBlock.TextProperty)?.ParentBinding.Path?.Path == "SelectedWorkOrder.DueDateLabel");
                Assert.IsFalse(string.IsNullOrWhiteSpace(date.Text));
                Assert.AreEqual(row.DueDateLabel, date.Text);
                date.BringIntoView();
                CaptureFields(window, "due-readback-" + order.Number);
                proof.Add(new { id, order.Number, workId = row.Id, workNumber = row.Number, dueAt = expected,
                    selectedDate = today.ToString("yyyy-MM-dd"), displayedDate = date.Text, displayedStatus = row.StatusLabel,
                    assignment = row.Record!.Assignment, totalYer = order.Source.Quotation.Evaluation.Totals.TotalYer,
                    sourceQuotationId = order.Source.Quotation.Id, sourceNumber = order.Source.Number,
                    state = order.State.ToString(), workState = row.Record.State.ToString() });
            }
            AssertLiveRetainedRecords(bridge, retained);
            File.WriteAllText(Path.Combine(root, "live-due-date-readback-proof.json"), JsonSerializer.Serialize(proof));
        });

    private static void RunConnectedDueDateFields(bool fromOrders) =>
        RunLiveAcceptance((window, bridge, sessions, secrets, root) =>
        {
            // Reuse the accepted-sale setup, without repeating the complete sales journey.
            OrderRecord order;
            if (Environment.GetEnvironmentVariable("EITMAD_E2E_PLANNED_ORDER_ID") is { Length: > 0 } existing)
            {
                Finish(sessions.SignInAsync("receptionist", secrets.GetProperty("receptionist_password").GetString()!));
                order = ReadLiveOrder(bridge, Guid.Parse(existing));
                Assert.AreEqual(OrderState.Confirmed, order.State, "Only a still-planned synthetic order can be reused.");
                Assert.IsNull(order.Delivery);
                Finish(sessions.SignOutAsync());
            }
            else { var setup = PrepareDirectOrder(sessions, bridge, secrets); Finish(setup); order = setup.Result; }
            SignIn(window, "manager", secrets.GetProperty("manager_password").GetString()!, AuthenticatedSurface.Manager, bridge);
            var retained = LiveRetainedRecords(bridge);
            retained.Remove("order:" + order.Id);
            var proof = new List<object>();
            {
                Assert.AreEqual(WorkState.Planned, order.Work.Single().State);
                ClickVisibleNamed(window, fromOrders ? "OrdersNavButton" : "WorkOrdersNavButton");
                var orders = WpfTestHost.Descendants<OrdersView>(window).Single();
                var work = WpfTestHost.Descendants<WorkOrdersView>(window).Single();
                if (fromOrders) { Finish(orders.ViewModel.ActivateAsync()); OpenButtonOrder(orders, order.Id); }
                else
                {
                    Finish(work.ViewModel.ActivateAsync());
                    if (work.ViewModel.IsDetailVisible) ClickVisibleNamed(work, "BackToWorkOrdersButton");
                    var row = work.ViewModel.VisibleWorkOrders.Single(item => item.Record!.OrderId == order.Id);
                    var table = WpfTestHost.Descendants<OperationsTable>(work).Single(table => table.IsVisible && table.IsRowInvocationEnabled);
                    table.RaiseEvent(new RowInvokedEventArgs(OperationsTable.RowInvokedEvent, table, row));
                    WpfTestHost.CompleteLayout(window);
                }
                FrameworkElement view = fromOrders ? orders : work;
                SetField(view, "Assignment", "ورشة تحقق موعد اليوم");
                var clock = DateTimeOffset.UtcNow;
                var today = clock.ToOffset(TimeSpan.FromHours(3)).Date;
                var before = JsonSerializer.Serialize(ReadLiveOrder(bridge, order.Id));
                foreach (var valid in new[] { false, true })
                {
                    Assert.AreEqual(today, DateTimeOffset.UtcNow.ToOffset(TimeSpan.FromHours(3)).Date, "Aden day changed; start a new bounded check.");
                    var selected = valid ? today : today.AddDays(-1);
                    var picker = WpfTestHost.Descendants<DatePicker>(view).Single(picker => picker.IsVisible);
                    picker.SelectedDate = selected;
                    picker.GetBindingExpression(DatePicker.SelectedDateProperty)!.UpdateSource();
                    WpfTestHost.CompleteLayout(window);
                    Task Operation() => fromOrders ? orders.ViewModel.LastAction : work.ViewModel.LastAction;
                    var previous = Operation(); var count = bridge.SubmissionCount;
                    ClickAutomation(view, fromOrders ? "بدء أمر العمل المؤكد" : "تغيير حالة أمر العمل");
                    WaitField(() => !ReferenceEquals(previous, Operation()) && bridge.SubmissionCount == count + 1
                        && bridge.LastSubmittedKind == Command.OrderWorkStartKind, "due date handler submission");
                    Finish(Operation());
                    Finish(fromOrders ? orders.ViewModel.LastLoad : work.ViewModel.LastLoad);
                    var input = bridge.LastSubmittedCommand!.AsOrderWorkStart()!;
                    // Independent calendar oracle: next Aden midnight is 21:00 UTC, minus one ms.
                    var next = DateOnly.FromDateTime(selected).AddDays(1);
                    var expected = new DateTimeOffset(next.Year, next.Month, next.Day, 0, 0, 0, TimeSpan.Zero)
                        .ToUnixTimeMilliseconds() - 3 * 60 * 60 * 1000 - 1;
                    Assert.AreEqual(expected, input.DueAt);
                    Assert.AreEqual(order.Id, input.OrderId);
                    Assert.AreEqual("ورشة تحقق موعد اليوم", input.Assignment);
                    var notice = fromOrders ? orders.ViewModel.ActionNotice : work.ViewModel.ActionNotice;
                    var current = ReadLiveOrder(bridge, order.Id);
                    Assert.AreEqual(JsonSerializer.Serialize(order.Source), JsonSerializer.Serialize(current.Source));
                    if (!valid)
                    {
                        Assert.AreEqual(CommandOutcomeStatus.Failed, bridge.LastReply!.Outcome.Status);
                        Assert.AreEqual(ProtocolIds.ErrorCodes.EitmadErrorOrderInvalidV1, bridge.LastReply.Outcome.Payload.Code);
                        Assert.AreEqual(before, JsonSerializer.Serialize(current), "Rejected due date mutated the still-planned order.");
                        StringAssert.Contains(notice, "رفض"); StringAssert.Contains(notice, "موعد");
                        Assert.IsFalse(fromOrders ? orders.ViewModel.CanRetryOrder : work.ViewModel.CanRetry);
                        Assert.IsTrue(fromOrders ? orders.ViewModel.SelectedOrder!.CanStartWork : work.ViewModel.SelectedWorkOrder!.Record!.CanStart);
                    }
                    else
                    {
                        Assert.AreEqual(CommandOutcomeStatus.Succeeded, bridge.LastReply!.Outcome.Status);
                        Assert.AreEqual(OrderState.InProduction, current.State);
                        Assert.AreEqual(WorkState.InProgress, current.Work.Single().State);
                        Assert.AreEqual(expected, current.Work.Single().DueAt);
                        StringAssert.Contains(notice, "أكد الخادم");
                        Finish(work.ViewModel.ActivateAsync());
                        var row = work.ViewModel.VisibleWorkOrders.Single(item => item.Record!.OrderId == order.Id);
                        Assert.AreEqual(DateOnly.FromDateTime(today), row.DueDate);
                        Assert.AreEqual(WorkOrderStatus.InProgress, row.Status);
                        work.ViewModel.OpenWorkOrder(row);
                        Assert.AreEqual(today, work.ViewModel.DueDate);
                    }
                    CaptureFields(window, $"due-{(fromOrders ? "orders" : "work-orders")}-{(valid ? "today" : "yesterday")}");
                    proof.Add(new { entry = fromOrders ? "Orders" : "WorkOrders", order.Id, order.Number,
                        workId = current.Work.Single().Id, clockUtc = clock, selectedDate = selected.ToString("yyyy-MM-dd"),
                        expectedWire = expected, actualWire = input.DueAt, outcome = bridge.LastReply!.Outcome.Status.ToString(),
                        error = bridge.LastReply.Outcome.Payload.Code, notice, state = current.State.ToString(),
                        workState = current.Work.Single().State.ToString(), sameSource = true, unchangedOnRejection = !valid });
                    Console.WriteLine($"Live deadline {order.Number}, {(fromOrders ? "Orders" : "WorkOrders")}: date={selected:yyyy-MM-dd}, wire={input.DueAt}, expected={expected}, outcome={bridge.LastReply.Outcome.Status}, state={current.State}, work={current.Work.Single().State}; {notice}");
                }
            }
            AssertLiveRetainedRecords(bridge, retained);
            File.WriteAllText(Path.Combine(root, $"live-due-date-{(fromOrders ? "orders" : "work-orders")}-proof.json"), JsonSerializer.Serialize(proof));
        });

    private static void RunLiveAcceptance(Action<MainWindow, LostSalesReplyBridge, DesktopSessionController, JsonElement, string> action)
    {
        var root = Environment.GetEnvironmentVariable("EITMAD_E2E_ROOT");
        if (string.IsNullOrWhiteSpace(root)) Assert.Inconclusive("Start a retained named workflow environment.");
        var engine = Environment.GetEnvironmentVariable("EITMAD_E2E_ENGINE");
        Assert.IsFalse(string.IsNullOrWhiteSpace(engine));
        var directory = Path.Combine(root!, "receptionist");
        var credential = File.ReadAllText(Path.Combine(directory, "route.json"));
        foreach (var prefix in new[] { "MEDIA", "PRICING", "QUOTATION", "CUSTOMER" })
            Environment.SetEnvironmentVariable($"EITMAD_{prefix}_CREDENTIAL_ID", credential);
        using var secrets = JsonDocument.Parse(File.ReadAllText(Path.Combine(root!, "secrets.json")));
        var bridge = new LostSalesReplyBridge(WindowsEngineBridge.Create(["--engine", engine!, "--runtime-directory", directory]));
        var sessions = new DesktopSessionController(bridge);
        try
        {
            Finish(sessions.StartAsync()); Finish(WaitForConnection(bridge));
            WpfTestHost.Run(1338, 753, window =>
            {
                action(window, bridge, sessions, secrets.RootElement, root!);
                SwitchAccount(window, bridge); Finish(sessions.StopAsync());
            }, showSignIn: true, engine: bridge, sessions: sessions);
        }
        finally { sessions.DisposeAsync().AsTask().GetAwaiter().GetResult(); }
    }

    private static OrderRecord ReadLiveOrder(IEngineShellBridge bridge, Guid id)
    {
        var read = ReadServerConfirmed(bridge, Query.ForOrderList(new() { Limit = 100 }), result => result.AsOrders()!.ServerAvailable);
        Finish(read);
        return read.Result.Outcome.Payload.AsOrders()!.Items.Single(item => item.Id == id);
    }

    private static Dictionary<string, string> LiveRetainedRecords(IEngineShellBridge bridge)
    {
        var records = new Dictionary<string, string>();
        var orders = ReadServerConfirmed(bridge, Query.ForOrderList(new() { Limit = 100 }), result => result.AsOrders()!.ServerAvailable); Finish(orders);
        foreach (var order in orders.Result.Outcome.Payload.AsOrders()!.Items) records.Add("order:" + order.Id, JsonSerializer.Serialize(order));
        var quotes = ReadServerConfirmed(bridge, Query.ForQuotationList(new() { Limit = 100 }), result => result.AsQuotations()!.ServerAvailable); Finish(quotes);
        foreach (var quote in quotes.Result.Outcome.Payload.AsQuotations()!.Items) records.Add("quotation:" + quote.Quotation.Id, JsonSerializer.Serialize(quote));
        var catalog = ReadServerConfirmed(bridge, Query.ForSalesCatalogList(new() { Term = "", Limit = 100 }), result => result.AsSalesCatalog()!.ServerAvailable); Finish(catalog);
        records.Add("catalog", JsonSerializer.Serialize(catalog.Result.Outcome.Payload.AsSalesCatalog()!.Items));
        return records;
    }

    private static void AssertLiveRetainedRecords(IEngineShellBridge bridge, Dictionary<string, string> before)
    {
        var after = LiveRetainedRecords(bridge);
        foreach (var (key, value) in before) Assert.AreEqual(value, after[key], "Changed retained record: " + key);
    }

    private static void AssertDimensions(FurnitureDimensions dimensions)
    {
        Assert.AreEqual(1400L, dimensions.WidthMm);
        Assert.AreEqual(2100L, dimensions.HeightMm);
        Assert.AreEqual(600L, dimensions.DepthMm);
    }
}
