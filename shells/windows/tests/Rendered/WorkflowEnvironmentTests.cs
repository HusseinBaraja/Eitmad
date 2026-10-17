using System.IO;
using Eitmad.Platform.Windows.LocalIpc;
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
        var bridge = WindowsEngineBridge.Create(["--engine", enginePath!, "--runtime-directory", directory]);
        var sessions = new DesktopSessionController(bridge);
        RunWithCleanupAsync(() =>
        {
            WpfTestHost.Run(1338, 753, window =>
            {
                Finish(sessions.StartAsync());
                Finish(WaitForConnection(bridge));
                var generation = bridge.Snapshot.Generation;
                Dictionary<Guid, string>? retainedDrafts = null;
                var customerVerified = false;
                foreach (var (name, expected) in Enumerable.Range(0, 2).SelectMany(_ => new[] { ("manager", AuthenticatedSurface.Manager), ("receptionist", AuthenticatedSurface.Receptionist) }))
                {
                    SignIn(window, name, secrets.RootElement.GetProperty($"{name}_password").GetString()!, expected, bridge);
                    ReadConfirmedWork(bridge);
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
                    Assert.IsTrue(current.Count >= 1);
                    retainedDrafts ??= current;
                    foreach (var (id, original) in retainedDrafts)
                        Assert.IsTrue(current.TryGetValue(id, out var retained) && retained == original, "A saved draft changed during account switching.");
                    SwitchAccount(window, bridge);
                    Assert.AreEqual(generation, bridge.Snapshot.Generation, "Account switching restarted the engine.");
                }
                Finish(sessions.StopAsync());
            }, showSignIn: true, engine: bridge, sessions: sessions);
            return Task.CompletedTask;
        }, () => sessions.DisposeAsync()).GetAwaiter().GetResult();
    }

    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public void DiagnosticAccountSwitchAndConfirmation()
    {
        var root = Environment.GetEnvironmentVariable("EITMAD_E2E_ROOT");
        if (string.IsNullOrWhiteSpace(root)) Assert.Inconclusive("Run tools/workflow-environment.ps1 Start -DiagnosticActivation.");
        var engine = Environment.GetEnvironmentVariable("EITMAD_E2E_ENGINE");
        Assert.IsFalse(string.IsNullOrWhiteSpace(engine));
        using var secrets = JsonDocument.Parse(File.ReadAllText(Path.Combine(root!, "secrets.json")));
        var bridge = WindowsEngineBridge.Create(["--engine", Path.GetFullPath(engine!), "--runtime-directory", Path.Combine(root!, "receptionist")]);
        var sessions = new DesktopSessionController(bridge);
        RunWithCleanupAsync(() =>
        {
            WpfTestHost.Run(1338, 753, window =>
            {
                Finish(sessions.StartAsync());
                Finish(WaitForConnection(bridge));
                var generation = bridge.Snapshot.Generation;
                foreach (var (name, surface) in new[] { ("manager", AuthenticatedSurface.Manager), ("receptionist", AuthenticatedSurface.Receptionist) })
                {
                    using var diagnostic = new OperationDiagnostics(DiagnosticPhase.Activation);
                    SignIn(window, name, secrets.RootElement.GetProperty($"{name}_password").GetString()!, surface, bridge);
                    // A nonempty synthetic term exercises Home's catalog source too.
                    var searchedHome = bridge.QueryAsync(Query.ForHomeRead(new() { Term = "\u062a\u0634\u062e\u064a\u0635" }));
                    Finish(searchedHome);
                    Assert.AreEqual(CommandOutcomeStatus.Succeeded, searchedHome.Result.Outcome.Status);
                    var home = searchedHome.Result.Outcome.Payload.AsHome()!;
                    foreach (var section in new[] { home.Quotations, home.Orders, home.Approvals, home.Catalog })
                    {
                        Assert.AreEqual(HomeAvailability.Available, section.Availability);
                        Assert.IsTrue(section.ServerAvailable, "The diagnostic Home source must report actual server confirmation.");
                    }
                    Finish(ReadServerConfirmed(bridge, Query.ForOrderList(new() { Limit = 100 }), result => result.AsOrders()!.ServerAvailable));
                    SwitchAccount(window, bridge);
                    Assert.AreEqual(generation, bridge.Snapshot.Generation, "Diagnostic account switching restarted the engine.");
                    diagnostic.Finish();
                }
                Finish(sessions.StopAsync());
            }, showSignIn: true, engine: bridge, sessions: sessions);
            return Task.CompletedTask;
        }, () => sessions.DisposeAsync()).GetAwaiter().GetResult();
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
            // Activation loads several independent capabilities, each with its own IPC deadline.
            Finish(window.AccountActivation, TimeSpan.FromMinutes(2));
            Assert.AreEqual(expected == AuthenticatedSurface.Manager ? Visibility.Visible : Visibility.Collapsed, window.ResponsiveRoot.Visibility,
                $"{name} activation: {bridge.Snapshot.State}, {bridge.Snapshot.IpcHealth}, exit {bridge.Snapshot.LastExit?.ExitCode}.");
            Assert.AreEqual(expected == AuthenticatedSurface.Receptionist ? Visibility.Visible : Visibility.Collapsed, window.ReceptionistSurface.Visibility);
        }
        finally { signIn.SignedIn -= Observe; }
    }

    private static void ReadConfirmedWork(IEngineShellBridge bridge)
    {
        var quotations = ReadServerConfirmed(bridge, Query.ForQuotationList(new() { Limit = 100 }),
            result => result.AsQuotations()!.ServerAvailable);
        Finish(quotations);
        Assert.IsTrue(quotations.Result.Outcome.Payload.AsQuotations()!.Items.Length >= 1);
        var orders = ReadServerConfirmed(bridge, Query.ForOrderList(new() { Limit = 100 }),
            result => result.AsOrders()!.ServerAvailable);
        Finish(orders);
        Assert.IsTrue(orders.Result.Outcome.Payload.AsOrders()!.Items.Length >= 1);
    }

    private static async Task<QueryResponseEnvelope> ReadServerConfirmed(
        IEngineShellBridge bridge, Query query, Func<QueryResult, bool> confirmed)
    {
        var response = await bridge.QueryAsync(query);
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, response.Outcome.Status);
        Assert.IsTrue(confirmed(response.Outcome.Payload), "The first response must be server-confirmed.");
        return response;
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

    // Retain the test failure if session disposal also fails.
    private static async Task RunWithCleanupAsync(Func<Task> action, Func<ValueTask> cleanup)
    {
        Exception? primary = null;
        try { await action().ConfigureAwait(false); }
        catch (Exception error) { primary = error; }
        try { await cleanup().ConfigureAwait(false); }
        catch (Exception secondary) when (primary is not null) { throw new AggregateException(primary, secondary); }
        if (primary is not null) System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(primary).Throw();
    }

    private static void Finish(Task task, TimeSpan? timeout = null)
    {
        task = task.WaitAsync(timeout ?? TimeSpan.FromSeconds(30));
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
