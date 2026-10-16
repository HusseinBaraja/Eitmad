using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Automation;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Features.Orders;

namespace Eitmad.WindowsShell.Tests.Rendered;

public sealed partial class WorkflowEnvironmentTests
{
    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public void ConnectedOrdersDirectButtonsConfirmProductionAndRetryExactCompletion()
    {
        var root = Environment.GetEnvironmentVariable("EITMAD_E2E_ROOT");
        if (string.IsNullOrWhiteSpace(root)) Assert.Inconclusive("Prepare a separate named workflow environment.");
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
            Finish(sessions.StartAsync()); Finish(WaitForConnection(bridge));
            var setup = PrepareDirectOrder(sessions, bridge, secrets.RootElement); Finish(setup);
            WpfTestHost.Run(1338, 753, window =>
            {
                SignIn(window, "manager", secrets.RootElement.GetProperty("manager_password").GetString()!, AuthenticatedSurface.Manager, bridge);
                ClickVisibleNamed(window, "OrdersNavButton");
                var view = WpfTestHost.Descendants<OrdersView>(window).Single(view => view.IsVisible);
                var model = view.ViewModel;
                Finish(model.ActivateAsync()); OpenButtonOrder(view, setup.Result.Id);
                void Press(string name, string kind)
                {
                    WaitField(() => WpfTestHost.Descendants<Button>(view).Any(button => button.IsVisible && button.IsEnabled
                        && AutomationProperties.GetName(button) == name), "direct Orders button: " + name);
                    var previous = model.LastAction;
                    var submitted = bridge.SubmissionCount;
                    ClickAutomation(view, name);
                    WaitField(() => !ReferenceEquals(previous, model.LastAction) && bridge.SubmissionCount == submitted + 1
                        && bridge.LastSubmittedKind == kind, "direct Orders handler submitted: " + kind);
                    Finish(model.LastAction);
                    WaitField(() => model.LastLoad.IsCompleted && model.ActionsAvailable, "current Orders operation and refresh");
                    Finish(model.LastLoad);
                }
                SetField(view, "Assignment", "ورشة تحقق الإكمال المباشر");
                Press("بدء أمر العمل المؤكد", Command.OrderWorkStartKind);
                WaitField(() => model.SelectedOrder?.Record?.State == OrderState.InProduction, "direct Start confirmation");
                Console.WriteLine($"After Start: Complete={model.SelectedOrder!.CanCompleteWork}; {model.ListState}; {model.ActionNotice}");
                CaptureFields(window, "direct-started");
                bridge.LoseNext(Command.OrderWorkCompleteKind);
                Press("تأكيد اكتمال أمر العمل", Command.OrderWorkCompleteKind);
                Console.WriteLine($"After Complete: retry={model.CanRetryOrder}; state={model.SelectedOrder?.Record?.State}; {model.ListState}; {model.ActionNotice}");
                Assert.IsTrue(model.CanRetryOrder, "A withheld successful Complete reply must remain uncertain.");
                CaptureFields(window, "direct-complete-retry");
                Press("إعادة محاولة عملية الطلب المعلقة", Command.OrderWorkCompleteKind);
                WaitField(() => model.SelectedOrder?.Record?.State == OrderState.Ready && !model.HasOrderPending, "direct Complete receipt confirmation");
                Assert.AreEqual(1, bridge.ExactRetries);
                Assert.AreEqual("", model.ListState, "The final read must be server-confirmed.");
                Assert.AreEqual(WorkState.Completed, model.SelectedOrder!.Record!.Work.Single().State);
                CaptureFieldSizes(window, "direct-ready");
                File.WriteAllText(Path.Combine(root!, "direct-orders-proof.json"), JsonSerializer.Serialize(new
                {
                    orderId = setup.Result.Id, state = model.SelectedOrder.Record.State.ToString(),
                    workState = model.SelectedOrder.Record.Work.Single().State.ToString(),
                    exactRetries = bridge.ExactRetries, serverConfirmed = model.ListState == "", route = "Orders Start > Orders Complete > Orders exact retry",
                }));
                SwitchAccount(window, bridge); Finish(sessions.StopAsync());
            }, showSignIn: true, engine: bridge, sessions: sessions);
        }
        finally { sessions.DisposeAsync().AsTask().GetAwaiter().GetResult(); }
    }

    private static async Task<OrderRecord> PrepareDirectOrder(DesktopSessionController sessions, IEngineShellBridge bridge, JsonElement secrets)
    {
        Assert.AreEqual(AuthenticatedSurface.Receptionist, await sessions.SignInAsync("receptionist", secrets.GetProperty("receptionist_password").GetString()!));
        var response = await bridge.QueryAsync(Query.ForOrderList(new() { Limit = 100 }));
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, response.Outcome.Status);
        var page = response.Outcome.Payload.AsOrders()!;
        Assert.IsTrue(page.ServerAvailable);
        var source = page.Items.First(item => item.Work.Length > 0).Source.Quotation;
        async Task<CommandResult> Send(Command command)
        {
            var reply = await bridge.SubmitCommandAsync(command, Guid.NewGuid());
            Assert.AreEqual(CommandOutcomeStatus.Succeeded, reply.Outcome.Status, reply.Outcome.Payload.Code);
            return reply.Outcome.Payload;
        }
        var draft = (await Send(Command.ForQuotationDraftCreate(new() { Intent = source.Intent }))).AsQuotationDraftCreated()!;
        var quotations = await bridge.QueryAsync(Query.ForQuotationList(new() { Limit = 100 }));
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, quotations.Outcome.Status);
        var current = quotations.Outcome.Payload.AsQuotations()!.Items.Single(item => item.Quotation.Id == draft.Snapshot.Id);
        var issued = (await Send(Command.ForQuotationIssue(new() { DraftId = draft.Snapshot.Id,
            ExpectedRevision = current.Revision, ExpectedDraftRevision = draft.Snapshot.Revision }))).AsQuotation()!;
        var accepted = (await Send(Command.ForQuotationAccept(new() { DraftId = draft.Snapshot.Id,
            ExpectedRevision = issued.Revision, Method = AcceptanceMethod.InPerson }))).AsQuotation()!;
        var order = (await Send(Command.ForOrderConvert(new() { DraftId = draft.Snapshot.Id, ExpectedRevision = accepted.Revision }))).AsOrder()!;
        Assert.AreEqual(OrderState.Confirmed, order.State);
        await sessions.SignOutAsync();
        return order;
    }
}
