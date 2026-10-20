using System.Text.Json;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Tests.Customers;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class CustomerResolutionRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ExplicitFieldSelectionRetriesFrozenDecisionAndRequiresNewReviewAfterStale(int width, int height)
    {
        var page = Comparison();
        var engine = new FakeEngine {
            QueryHandler = _ => CustomerDirectoryTests.Result(QueryResult.ForCustomerConflicts(page)),
            CommandHandler = _ => Failure(ProtocolIds.ErrorCodes.EitmadErrorCustomerUnavailableV1),
        };
        var client = new CustomerClient(engine);
        WpfTestHost.Run(width, height, window => {
            var view = new CustomerResolutionView(); window.Content = view;
            Finish(view.OpenAsync(page.Current, client));
            view.Sources.SelectedIndex = 0; WpfTestHost.CompleteLayout(window);
            var fields = (ResolutionField[])view.Fields.ItemsSource;
            Assert.IsTrue(fields.All(f => f.Choice == -1));
            Finish(view.ConfirmAsync()); Assert.IsNull(engine.LastCommand);
            var choices = WpfTestHost.Descendants<ComboBox>(view.Fields).ToArray();
            Assert.HasCount(4, choices);
            choices[0].Focus(); Assert.IsTrue(choices[0].IsKeyboardFocusWithin);
            Assert.IsFalse(string.IsNullOrEmpty(AutomationProperties.GetName(choices[0])));
            choices[0].SelectedIndex = 0; choices[1].SelectedIndex = 2;
            choices[2].SelectedIndex = 1; choices[3].SelectedIndex = 3;
            fields[3].Value = "اتصل قبل التسليم — نموذج تجريبي";
            WpfTestHost.CompleteLayout(window);
            var dpi = VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"Window {window.ActualWidth}x{window.ActualHeight}; DPI {dpi.PixelsPerInchX}; display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight}");
            WpfTestHost.Capture(window, $"customer-resolution-{width}x{height}");
            Assert.IsTrue(view.ConfirmButton.IsVisible);
            Finish(view.ConfirmAsync());
            var first = JsonSerializer.Serialize(engine.LastCommand); var key = engine.LastIdempotencyKey;
            Assert.IsFalse(view.Fields.IsEnabled); Assert.IsFalse(view.ReloadButton.IsEnabled);
            Finish(view.ConfirmAsync());
            Assert.AreEqual(first, JsonSerializer.Serialize(engine.LastCommand)); Assert.AreEqual(key, engine.LastIdempotencyKey);
            engine.CommandHandler = _ => Failure(ProtocolIds.ErrorCodes.EitmadErrorCustomerRevisionConflictV1);
            Finish(view.ConfirmAsync());
            Assert.IsNull(view.Fields.ItemsSource); Assert.IsFalse(view.ConfirmButton.IsEnabled);
            Assert.IsTrue(view.ReloadButton.IsEnabled);
            view.ReloadButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(view.LastOperation);
            view.Sources.SelectedIndex = 0; WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(((ResolutionField[])view.Fields.ItemsSource).All(f => f.Choice == -1));
            if (width == 1338) {
                foreach (var control in WpfTestHost.Descendants<Control>(view)) Controls.ControlOptions.SetHighContrast(control, true);
                WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window, "customer-resolution-high-contrast");
            }
            view.ClearSession(); Assert.IsNull(view.Fields.ItemsSource); Assert.IsNull(view.Sources.ItemsSource);
        }, engine: engine);
        Finish(client.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask());
    }

    [TestMethod]
    public void ReopenedUncertainDecisionUsesPersistedKeyAndClearsLateComparison()
    {
        var page = Comparison(); var key = Guid.NewGuid();
        page.Items = [];
        page.PendingResolution = new() { IdempotencyKey = key, Scope = page.Current.Scope,
            Command = new() { Customer = new() { CustomerId = page.Current.Id, BranchId = page.Current.Scope.Id },
                ExpectedServerRevision = 2, Contact = new() { Name = page.Current.Name, Phone = page.Current.Phone },
                Source = new() { ["kind"] = "conflict", ["payload"] = new { conflictId = Guid.NewGuid() } } } };
        var engine = new FakeEngine { QueryHandler = _ => CustomerDirectoryTests.Result(QueryResult.ForCustomerConflicts(page)),
            CommandHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded,
                Payload = CommandResult.ForCustomerConflictResolved(new() { Revision = 3 }) } } };
        var client = new CustomerClient(engine);
        WpfTestHost.Run(1338, 753, window => {
            var view = new CustomerResolutionView(); window.Content = view;
            Finish(view.OpenAsync(page.Current, client)); Assert.IsFalse(view.Fields.IsEnabled);
            Finish(view.ConfirmAsync()); Assert.AreEqual(key, engine.LastIdempotencyKey);
            Assert.IsNull(view.Fields.ItemsSource); Assert.IsFalse(view.ConfirmButton.IsEnabled);
            var gate = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            engine.QueryBarrier = _ => gate.Task;
            var load = view.OpenAsync(page.Current, client); view.ClearSession(); gate.SetResult(); Finish(load);
            Assert.IsNull(view.Fields.ItemsSource); Assert.IsNull(view.Sources.ItemsSource);
        }, engine: engine);
        Finish(client.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask());
    }

    private static CommandResponseEnvelope Failure(string code) => new() {
        Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = code } },
    };
    private static CustomerConflictPage Comparison()
    {
        var customer = CustomerDirectoryTests.Contact(Guid.NewGuid(), "منزل الصبري — بيانات تجريبية"); customer.Revision = 2;
        var local = new CustomerSyncPayload { CustomerId = customer.Id, Revision = 2, Name = "منزل الصبري الجديد", Phone = "777123456", Address = "عدن، المنصورة" };
        var remote = new CustomerSyncPayload { CustomerId = customer.Id, Revision = 2, Name = customer.Name, Phone = customer.Phone, Address = "عدن، كريتر" };
        return new() { Current = customer, PendingReviews = [], Items = [new() {
            Local = local, Remote = remote, Conflict = new() { ConflictId = Guid.NewGuid(), Scope = customer.Scope, RecordId = customer.Id },
        }] };
    }
    private static void Finish(Task task) { while (!task.IsCompleted) WpfTestHost.PumpDispatcher(); task.GetAwaiter().GetResult(); }
}
