using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Orders;

namespace Eitmad.WindowsShell.Tests.Rendered;

public sealed partial class WorkflowEnvironmentTests
{
    // Real WPF account activation, supervised IPC, TLS and PostgreSQL. No fixture bridge.
    private static void RunSalesSmoke(MainWindow window, IEngineShellBridge bridge, JsonElement secrets, string root)
    {
        void Stage(string step) => File.WriteAllText(Path.Combine(root, "sales-smoke-stage.txt"), step);
        void Login(string name)
        {
            Stage($"Sign in: {name}");
            SignIn(window, name, secrets.GetProperty($"{name}_password").GetString()!,
                name == "manager" ? AuthenticatedSurface.Manager : AuthenticatedSurface.Receptionist, bridge);
        }
        void Change(string name) { SwitchAccount(window, bridge); Login(name); }
        QueryResult Read(Query query)
        {
            var task = bridge.QueryAsync(query); Finish(task);
            Assert.AreEqual(CommandOutcomeStatus.Succeeded, task.Result.Outcome.Status, task.Result.Outcome.Payload.Code);
            return task.Result.Outcome.Payload;
        }
        CommandResult Send(Command command, Guid? key = null)
        {
            Stage(command.Kind);
            var task = SendSmokeCommand(bridge, command, key ?? Guid.NewGuid()); Finish(task);
            return task.Result;
        }

        Login("manager");
        var proofPath = Path.Combine(root, "sales-smoke-proof.json");
        if (File.Exists(proofPath))
        {
            using var proof = JsonDocument.Parse(File.ReadAllText(proofPath));
            var retained = Read(Query.ForOrderGet(new() { OrderId = proof.RootElement.GetProperty("id").GetGuid() })).AsOrders()!.Items.Single();
            Assert.AreEqual(OrderState.Delivered, retained.State, "Restart lost confirmed delivery.");
            Assert.AreEqual(proof.RootElement.GetProperty("number").GetString(), retained.Number);
            Assert.AreEqual(proof.RootElement.GetProperty("totalYer").GetInt64(), retained.Source.Quotation.Evaluation.Totals.TotalYer);
        }
        var catalog = Read(Query.ForSalesCatalogList(new() { Term = "", Limit = 100 })).AsSalesCatalog()!;
        Assert.IsTrue(catalog.ServerAvailable);
        var furniture = catalog.Items.First(item => item.Dimensions is not null);
        var product = catalog.Items.First(item => item.Dimensions is null);
        foreach (var (item, amount) in new[] { (furniture, 12500L), (product, 2000L) })
        {
            var published = Send(Command.ForPricingPublish(new()
            {
                Target = item.Price.Target, ExpectedRevision = item.Price.Revision,
                SellingPriceYer = amount, ConfirmBelowCost = true,
            })).AsPricePublished()!;
            item.Price = published;
            Assert.AreEqual(amount, published.SellingPriceYer);
        }

        Change("receptionist");
        var customer = Send(Command.ForCustomerCreate(new()
        {
            Name = "عميل رحلة الاختبار", Phone = "777123456", Address = "عدن", Notes = null!,
        })).AsCustomerCreated()!.Customer;
        Finish(WaitForCustomerConfirmation(bridge, customer));
        var lines = new[] { furniture, product }.Select(item => new QuotationLineIntent
        {
            Id = Guid.NewGuid(), Configuration = new()
            {
                Dimensions = item.Dimensions, Selection = new()
                {
                    Target = item.Price.Target, PriceRevision = item.Price.Revision, Quantity = 1,
                    ColorId = null, HandleId = null,
                },
            },
        }).ToArray();
        var draft = Send(Command.ForQuotationDraftCreate(new()
        {
            Intent = new() { Customer = new() { Id = customer.Id, Revision = customer.Revision }, Lines = lines, DiscountBasisPoints = 600 },
        })).AsQuotationDraftCreated()!;
        var approval = Send(Command.ForQuotationApprovalRequest(new()
        {
            DraftId = draft.Snapshot.Id, ExpectedRevision = draft.Snapshot.Revision,
        })).AsDiscountApproval()!;
        Assert.AreEqual(DiscountApprovalState.Pending, approval.State);
        Change("manager");
        approval = Send(Command.ForQuotationApprovalDecide(new()
        {
            DraftId = draft.Snapshot.Id, RequestId = approval.RequestId, ExpectedRevision = approval.Revision,
            QuotationRevision = approval.Quotation.Revision, Fingerprint = approval.Fingerprint, Decision = DiscountDecision.Approve,
        })).AsDiscountApproval()!;
        Assert.AreEqual(DiscountApprovalState.Approved, approval.State);

        Change("receptionist");
        var current = Read(Query.ForQuotationList(new() { Limit = 100 })).AsQuotations()!.Items.Single(item => item.Quotation.Id == draft.Snapshot.Id);
        var issued = Send(Command.ForQuotationIssue(new()
        {
            DraftId = draft.Snapshot.Id, ExpectedRevision = current.Revision, ExpectedDraftRevision = draft.Snapshot.Revision,
        })).AsQuotation()!;
        Assert.AreEqual(QuotationState.Issued, issued.State);
        var quotationDocument = Read(Query.ForQuotationCustomerDocument(new() { DraftId = draft.Snapshot.Id })).AsCustomerDocument()!;
        Assert.IsTrue(quotationDocument.CanPrint);
        Assert.AreEqual(13630L, quotationDocument.TotalYer);
        var accepted = Send(Command.ForQuotationAccept(new()
        {
            DraftId = draft.Snapshot.Id, ExpectedRevision = issued.Revision, Method = AcceptanceMethod.InPerson,
        })).AsQuotation()!;
        var conversion = Command.ForOrderConvert(new() { DraftId = draft.Snapshot.Id, ExpectedRevision = accepted.Revision });
        var conversionKey = Guid.NewGuid();
        var order = Send(conversion, conversionKey).AsOrder()!;
        Assert.AreEqual(order.Id, Send(conversion, conversionKey).AsOrder()!.Id);
        Assert.AreEqual(OrderState.Confirmed, order.State);
        Assert.AreEqual(1, order.Work.Length);
        CollectionAssert.AreEqual(new[] { lines[0].Id }, order.Work[0].LineIds);

        Change("manager");
        order = Send(Command.ForOrderWorkStart(new()
        {
            OrderId = order.Id, ExpectedRevision = order.Revision, WorkId = order.Work[0].Id,
            Assignment = "ورشة الاختبار", DueAt = DateTimeOffset.UtcNow.AddDays(1).ToUnixTimeMilliseconds(),
        })).AsOrder()!;
        Assert.AreEqual(OrderState.InProduction, order.State);
        order = Send(Command.ForOrderWorkComplete(new()
        {
            OrderId = order.Id, ExpectedRevision = order.Revision, WorkId = order.Work[0].Id,
        })).AsOrder()!;
        Assert.AreEqual(OrderState.Ready, order.State);
        Change("receptionist");
        order = Read(Query.ForOrderGet(new() { OrderId = order.Id })).AsOrders()!.Items.Single();
        Assert.AreEqual(OrderState.Ready, order.State);
        CollectionAssert.Contains(order.PermittedActions, OrderPermittedAction.Deliver);
        Assert.IsFalse(order.PermittedActions.Contains(OrderPermittedAction.StartWork));
        foreach (var query in new[] { Query.ForOrderCustomerDocument(new() { OrderId = order.Id }), Query.ForOrderQuotationDocument(new() { OrderId = order.Id }) })
        {
            var document = Read(query).AsCustomerDocument()!;
            Assert.IsTrue(document.CanPrint);
            Assert.IsFalse(document.IsDraft);
            Assert.AreEqual(2, document.Lines.Length);
            Assert.AreEqual(quotationDocument.TotalYer, document.TotalYer);
            ShowSmokeDocument(window, bridge, query, document);
        }
        var delivery = Command.ForOrderDeliver(new()
        {
            OrderId = order.Id, ExpectedRevision = order.Revision, Recipient = "مستلم تجريبي", Method = AcceptanceMethod.Written,
        });
        var deliveryKey = Guid.NewGuid();
        order = Send(delivery, deliveryKey).AsOrder()!;
        Assert.AreEqual(OrderState.Delivered, order.State);
        Assert.AreEqual(order.Delivery!.Id, Send(delivery, deliveryKey).AsOrder()!.Delivery!.Id);
        File.WriteAllText(proofPath, JsonSerializer.Serialize(new { id = order.Id, number = order.Number, totalYer = quotationDocument.TotalYer }));
        SwitchAccount(window, bridge);
        Stage("Passed: mixed sales journey and exact conversion/delivery retries");
    }

    private static void ShowSmokeDocument(MainWindow owner, IEngineShellBridge bridge, Query query, CustomerDocument document)
    {
        var preview = new PrintPreview { Document = OrderCustomerDocument.CreateSaved(document), CanPrint = document.CanPrint };
        preview.AuthorizePrint = async () =>
        {
            var response = await bridge.QueryAsync(query);
            return response.Outcome.Status == CommandOutcomeStatus.Succeeded
                && response.Outcome.Payload.AsCustomerDocument()?.CanPrint == true;
        };
        var window = new Window { Owner = owner, Content = preview, Width = 1000, Height = 780,
            Left = -10000, ShowInTaskbar = false, FlowDirection = FlowDirection.RightToLeft };
        preview.BackRequested += (_, _) => window.Close();
        try
        {
            window.Show();
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(WpfTestHost.Descendants<DocumentViewer>(preview).Single().PageCount > 0);
            var authorized = preview.AuthorizePrint(); Finish(authorized);
            Assert.IsTrue(authorized.Result, "Printing must be reauthorized by real Rust state.");
            preview.BackButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsFalse(window.IsVisible);
        }
        finally { window.Close(); }
    }

    private static async Task<CommandResult> SendSmokeCommand(IEngineShellBridge bridge, Command command, Guid key)
    {
        var until = DateTime.UtcNow.AddSeconds(15);
        while (true)
        {
            var response = await bridge.SubmitCommandAsync(command, key);
            if (response.Outcome.Status == CommandOutcomeStatus.Succeeded) return response.Outcome.Payload;
            // A live read can occupy the route. Pricing also retains unknown outcomes;
            // Rust queries the retained receipt before resubmitting the same intent/key.
            var code = response.Outcome.Payload.Code;
            if (code is not (ProtocolIds.ErrorCodes.EitmadErrorQuotationApprovalUnavailableV1
                or ProtocolIds.ErrorCodes.EitmadErrorQuotationUnavailableV1 or ProtocolIds.ErrorCodes.EitmadErrorOrderUnavailableV1
                or ProtocolIds.ErrorCodes.EitmadErrorPricingUnconfirmedV1)
                || DateTime.UtcNow >= until)
                Assert.Fail($"Smoke command {command.Kind} failed: {code}");
            await Task.Delay(100);
        }
    }
}
