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
public sealed class WorkflowEnvironmentTests
{
    [TestMethod]
    [TestCategory("WorkflowEnvironment")]
    public void BothAccountsSignInThroughRealWpfAndReadServerConfirmedWork()
    {
        var root = Environment.GetEnvironmentVariable("EITMAD_E2E_ROOT");
        if (string.IsNullOrWhiteSpace(root)) Assert.Inconclusive("Run tools/workflow-environment.ps1 Start -VerifySignIn.");
        var enginePath = Environment.GetEnvironmentVariable("EITMAD_E2E_ENGINE");
        Assert.IsFalse(string.IsNullOrWhiteSpace(enginePath));
        using var secrets = JsonDocument.Parse(File.ReadAllText(Path.Combine(root!, "secrets.json")));
        foreach (var (name, expected) in new[] { ("manager", AuthenticatedSurface.Manager), ("receptionist", AuthenticatedSurface.Receptionist) })
        {
            var directory = Path.Combine(root!, name);
            var credential = File.ReadAllText(Path.Combine(directory, "route.json"));
            foreach (var prefix in new[] { "MEDIA", "PRICING", "QUOTATION" })
                Environment.SetEnvironmentVariable($"EITMAD_{prefix}_CREDENTIAL_ID", credential);
            var bridge = WindowsEngineBridge.Create(["--engine", enginePath!, "--runtime-directory", directory]);
            var sessions = new DesktopSessionController(bridge);
            try
            {
                WpfTestHost.Run(1338, 753, window =>
                {
                    Finish(sessions.StartAsync());
                    Finish(WaitForConnection(bridge));
                    var signIn = WpfTestHost.FindByName<Features.Authentication.SignInView>(window, "SignInSurface");
                    var signedIn = new TaskCompletionSource<AuthenticatedSurface>();
                    signIn.SignedIn += (_, surface) => signedIn.TrySetResult(surface);
                    WpfTestHost.FindByName<TextBox>(window, "UsernameBox").Text = name;
                    WpfTestHost.FindByName<PasswordBox>(window, "PasswordInput").Password = secrets.RootElement.GetProperty($"{name}_password").GetString()!;
                    WpfTestHost.FindByName<Button>(window, "SignInButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    try { Finish(signedIn.Task); }
                    catch (TimeoutException)
                    {
                        Assert.Fail($"{name} sign-in did not complete: {WpfTestHost.FindByName<TextBlock>(window, "SignInError").Text}");
                    }
                    Assert.AreEqual(expected, signedIn.Task.Result);
                    Assert.AreEqual(Visibility.Collapsed, signIn.Visibility);
                    Finish(window.AccountActivation);
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
                    CheckAcceptedWork(root!, orders.Result.Outcome.Payload.AsOrders()!.Items);
                    Finish(sessions.StopAsync());
                }, showSignIn: true, engine: bridge, sessions: sessions);
            }
            finally { sessions.DisposeAsync().AsTask().GetAwaiter().GetResult(); }
        }
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
