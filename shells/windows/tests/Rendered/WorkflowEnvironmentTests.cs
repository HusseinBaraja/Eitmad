using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.WindowsShell.Features.Authentication;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
[DoNotParallelize]
public sealed partial class WorkflowEnvironmentTests
{
    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public void OneDesktopSwitchesAccountsAndRetainsServerConfirmedWork()
    {
        var root = Environment.GetEnvironmentVariable("EITMAD_E2E_ROOT");
        if (string.IsNullOrWhiteSpace(root)) Assert.Inconclusive("Run tools/workflow-environment.ps1 Start -VerifySignIn.");
        var enginePath = Environment.GetEnvironmentVariable("EITMAD_E2E_ENGINE");
        Assert.IsFalse(string.IsNullOrWhiteSpace(enginePath));
        using var secrets = JsonDocument.Parse(File.ReadAllText(Path.Combine(root!, "secrets.json")));
        var directory = Path.Combine(root!, "receptionist");
        var credential = File.ReadAllText(Path.Combine(directory, "route.json"));
        foreach (var prefix in new[] { "MEDIA", "PRICING", "QUOTATION", "CUSTOMER" })
            Environment.SetEnvironmentVariable($"EITMAD_{prefix}_CREDENTIAL_ID", credential);
        var bridge = WindowsEngineBridge.Create(["--engine", enginePath!, "--runtime-directory", directory]);
        var sessions = new DesktopSessionController(bridge);
        try
        {
            WpfTestHost.Run(1338, 753, window =>
            {
                Finish(sessions.StartAsync());
                Finish(WaitForConnection(bridge));
                var generation = bridge.Snapshot.Generation;
                RunSalesSmoke(window, bridge, secrets.RootElement, root!);
                Dictionary<Guid, string>? retainedDrafts = null;
                var customerVerified = false;
                foreach (var (name, expected) in Enumerable.Range(0, 3).SelectMany(_ => new[] { ("manager", AuthenticatedSurface.Manager), ("receptionist", AuthenticatedSurface.Receptionist) }))
                {
                    SignIn(window, name, secrets.RootElement.GetProperty($"{name}_password").GetString()!, expected, bridge);
                    ReadConfirmedWork(bridge, root!);
                    if (name == "receptionist" && !customerVerified)
                    {
                        Finish(CreateEditAndUseCustomer(bridge));
                        customerVerified = true;
                    }
                    var drafts = bridge.QueryAsync(Query.ForQuotationDraftList(new() { Limit = 100 }));
                    Finish(drafts);
                    Assert.AreEqual(CommandOutcomeStatus.Succeeded, drafts.Result.Outcome.Status);
                    var current = drafts.Result.Outcome.Payload.AsQuotationDrafts()!.Items.ToDictionary(
                        draft => draft.Snapshot.Id, draft => JsonSerializer.Serialize(draft.Snapshot));
                    Assert.IsTrue(current.Count >= 5);
                    retainedDrafts ??= current;
                    foreach (var (id, original) in retainedDrafts)
                        Assert.IsTrue(current.TryGetValue(id, out var retained) && retained == original, "A saved draft changed during account switching.");
                    SwitchAccount(window, bridge);
                    Assert.AreEqual(generation, bridge.Snapshot.Generation, "Account switching restarted the engine.");
                }
                Finish(sessions.StopAsync());
            }, showSignIn: true, engine: bridge, sessions: sessions);
        }
        finally { sessions.DisposeAsync().AsTask().GetAwaiter().GetResult(); }
    }

    private static async Task CreateEditAndUseCustomer(IEngineShellBridge bridge)
    {
        var created = await bridge.SubmitCommandAsync(Command.ForCustomerCreate(new()
        {
            Name = "عميل تحقق المزامنة", Phone = "777123456", Address = null!, Notes = null!,
        }), Guid.NewGuid());
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, created.Outcome.Status);
        var customer = created.Outcome.Payload.AsCustomerCreated()!.Customer;
        await WaitForCustomerConfirmation(bridge, customer);
        var updated = await bridge.SubmitCommandAsync(Command.ForCustomerUpdate(new()
        {
            CustomerId = customer.Id, ExpectedRevision = customer.Revision,
            Name = "عميل تحقق محدث", Phone = customer.Phone, Address = "عدن", Notes = null!,
        }), Guid.NewGuid());
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, updated.Outcome.Status);
        customer = updated.Outcome.Payload.AsCustomerUpdated()!.Customer;
        await WaitForCustomerConfirmation(bridge, customer);

        var catalog = await bridge.QueryAsync(Query.ForSalesCatalogList(new() { Term = "", Limit = 100 }));
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, catalog.Outcome.Status);
        var item = catalog.Outcome.Payload.AsSalesCatalog()!.Items.First(entry => entry.Dimensions is null);
        QuotationLineIntent[] lines = [new()
        {
            Id = Guid.NewGuid(), Configuration = new()
            {
                Dimensions = item.Dimensions,
                Selection = new() { Target = item.Price.Target, PriceRevision = item.Price.Revision, Quantity = 1 },
            },
        }];
        foreach (var discount in new[] { 0L, 600L })
        {
            var saved = await bridge.SubmitCommandAsync(Command.ForQuotationDraftCreate(new()
            {
                Intent = new() { Customer = new() { Id = customer.Id, Revision = customer.Revision },
                    Lines = lines, DiscountBasisPoints = discount },
            }), Guid.NewGuid());
            Assert.AreEqual(CommandOutcomeStatus.Succeeded, saved.Outcome.Status, saved.Outcome.Payload.Code);
            var draft = saved.Outcome.Payload.AsQuotationDraftCreated()!;
            var command = discount == 0
                ? Command.ForQuotationIssue(new() { DraftId = draft.Snapshot.Id, ExpectedRevision = 1,
                    ExpectedDraftRevision = draft.Snapshot.Revision })
                : Command.ForQuotationApprovalRequest(new() { DraftId = draft.Snapshot.Id, ExpectedRevision = draft.Snapshot.Revision });
            var key = Guid.NewGuid();
            var confirmed = await bridge.SubmitCommandAsync(command, key);
            // Live screen queries share the approval route's fail-fast admission. A busy
            // route is retryable; retain the key so an accepted request cannot be duplicated.
            var retryUntil = DateTime.UtcNow.AddSeconds(15);
            while (confirmed.Outcome.Status == CommandOutcomeStatus.Failed
                && confirmed.Outcome.Payload.Code == ProtocolIds.ErrorCodes.EitmadErrorQuotationApprovalUnavailableV1
                && DateTime.UtcNow < retryUntil)
            {
                await Task.Delay(100);
                confirmed = await bridge.SubmitCommandAsync(command, key);
            }
            Assert.AreEqual(CommandOutcomeStatus.Succeeded, confirmed.Outcome.Status,
                $"The server must accept the customer's current revision without an engine restart (discount {discount}, error {confirmed.Outcome.Payload.Code}).");
        }
    }

    private static async Task WaitForCustomerConfirmation(IEngineShellBridge bridge, Customer customer)
    {
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(15));
        while (true)
        {
            var response = await bridge.QueryAsync(Query.ForCustomerGet(new() { CustomerId = customer.Id }), timeout.Token);
            Assert.AreEqual(CommandOutcomeStatus.Succeeded, response.Outcome.Status);
            var current = response.Outcome.Payload.AsCustomer()!;
            if (current.Revision == customer.Revision && current.SyncState == SyncState.Confirmed) return;
            await Task.Delay(100, timeout.Token);
        }
    }

    private static void SignIn(MainWindow window, string name, string password, AuthenticatedSurface expected, IEngineShellBridge bridge)
    {
        var signIn = window.SignInSurface;
        var signedIn = new TaskCompletionSource<AuthenticatedSurface>();
        void Observe(object? sender, AuthenticatedSurface surface) => signedIn.TrySetResult(surface);
        signIn.SignedIn += Observe;
        try
        {
            WpfTestHost.FindByName<TextBox>(window, "UsernameBox").Text = name;
            WpfTestHost.FindByName<PasswordBox>(window, "PasswordInput").Password = password;
            WpfTestHost.FindByName<Button>(window, "SignInButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            try { Finish(signedIn.Task); }
            catch (TimeoutException)
            {
                Assert.Fail($"{name} sign-in did not complete: {WpfTestHost.FindByName<TextBlock>(window, "SignInError").Text}");
            }
            Assert.AreEqual(expected, signedIn.Task.Result);
            Assert.AreEqual(Visibility.Collapsed, signIn.Visibility);
            Finish(window.AccountActivation);
            Assert.AreEqual(expected == AuthenticatedSurface.Manager ? Visibility.Visible : Visibility.Collapsed, window.ResponsiveRoot.Visibility,
                $"{name} activation: {bridge.Snapshot.State}, {bridge.Snapshot.IpcHealth}, exit {bridge.Snapshot.LastExit?.ExitCode}.");
            Assert.AreEqual(expected == AuthenticatedSurface.Receptionist ? Visibility.Visible : Visibility.Collapsed, window.ReceptionistSurface.Visibility);
        }
        finally { signIn.SignedIn -= Observe; }
    }

    private static void ReadConfirmedWork(IEngineShellBridge bridge, string root)
    {
        var quotations = bridge.QueryAsync(Query.ForQuotationList(new() { Limit = 100 }));
        Finish(quotations);
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, quotations.Result.Outcome.Status);
        Assert.IsTrue(quotations.Result.Outcome.Payload.AsQuotations()!.ServerAvailable);
        Assert.IsTrue(quotations.Result.Outcome.Payload.AsQuotations()!.Items.Length >= 5);
        var orders = bridge.QueryAsync(Query.ForOrderList(new() { Limit = 100 }));
        Finish(orders);
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, orders.Result.Outcome.Status);
        Assert.IsTrue(orders.Result.Outcome.Payload.AsOrders()!.ServerAvailable);
        Assert.IsTrue(orders.Result.Outcome.Payload.AsOrders()!.Items.Length >= 1);
        CheckAcceptedWork(root, orders.Result.Outcome.Payload.AsOrders()!.Items);
    }

    private static void SwitchAccount(MainWindow window, IEngineShellBridge bridge)
    {
        MainWindow.SwitchAccountCommand.Execute(null, window);
        WpfTestHost.CompleteLayout(window);
        Assert.IsTrue(window.AccountSwitchDialog.IsOpen);
        window.CancelAccountSwitchButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.IsFalse(window.AccountSwitchDialog.IsOpen);
        Assert.AreEqual(Visibility.Collapsed, window.SignInSurface.Visibility);
        MainWindow.SwitchAccountCommand.Execute(null, window);
        WpfTestHost.CompleteLayout(window);
        window.ConfirmAccountSwitchButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Finish(window.AccountTransition);
        Assert.AreEqual(Visibility.Visible, window.SignInSurface.Visibility);
        Assert.AreEqual("", WpfTestHost.FindByName<PasswordBox>(window, "PasswordInput").Password);
        Assert.AreEqual("", WpfTestHost.FindByName<TextBox>(window, "UsernameBox").Text);
        Assert.AreEqual(Visibility.Collapsed, window.ReceptionistSurface.Visibility);
        Assert.AreEqual(Visibility.Collapsed, window.ResponsiveRoot.Visibility);
        var signedOut = bridge.QueryAsync(Query.ForOrderList(new() { Limit = 1 }));
        Finish(signedOut);
        Assert.AreEqual(CommandOutcomeStatus.Failed, signedOut.Result.Outcome.Status);
    }

    private static void CheckAcceptedWork(string root, OrderRecord[] orders)
    {
        // The server owns this immutable accepted source. A restart must retain it exactly.
        // Later orders are allowed; no fixture or business record is reset by this proof.
        var current = orders.ToDictionary(order => order.Id.ToString(), order => JsonSerializer.Serialize(new
        {
            order.Number, order.CreatedAt,
            order.Source.Quotation, order.Source.Acceptance,
        }));
        var path = Path.Combine(root, "accepted-work-proof.json");
        if (File.Exists(path))
        {
            var original = JsonSerializer.Deserialize<Dictionary<string, string>>(File.ReadAllText(path))!;
            foreach (var (id, accepted) in original)
            {
                Assert.IsTrue(current.TryGetValue(id, out var retained), "An accepted order was lost.");
                Assert.AreEqual(accepted, retained, "An accepted order source changed after restart.");
            }
        }
        else File.WriteAllText(path, JsonSerializer.Serialize(current));
    }

    private static async Task WaitForConnection(IEngineShellBridge bridge)
    {
        var connected = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        void Changed(EngineSupervisionSnapshot snapshot)
        {
            if (snapshot.IpcHealth == EngineIpcHealthState.Connected) connected.TrySetResult();
        }
        bridge.StateChanged += Changed;
        try
        {
            Changed(bridge.Snapshot);
            try { await connected.Task.WaitAsync(TimeSpan.FromSeconds(20)); }
            catch (TimeoutException)
            {
                Assert.Fail($"Engine connection failed: {bridge.Snapshot.State}, {bridge.Snapshot.IpcHealth}, exit {bridge.Snapshot.LastExit?.ExitCode}.");
            }
        }
        finally { bridge.StateChanged -= Changed; }
    }

    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(30));
        if (!task.IsCompleted)
        {
            var frame = new DispatcherFrame();
            var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default);
            Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }
}
