using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.Quotations;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class QuotationEditorClosingTests
{
    [TestMethod]
    [DataRow(false, 1920, 1080)]
    [DataRow(false, 1338, 753)]
    [DataRow(false, 720, 560)]
    [DataRow(true, 1338, 753)]
    public void ContinuePreservesInputThenSaveClosesAndReopens(bool existing, int width, int height)
    {
        Run(width, height, (window, model, authority, _) => {
            if (existing) { Finish(model.SaveDraftAsync()); Finish(model.OpenDraftAsync(authority.Draft!.Snapshot.Id)); }
            model.DiscountInput = "6";
            model.AttachCustomer(new("عميل آخر تجريبي", "777000222", "عدن", "", Guid.NewGuid(), 1));
            model.DuplicateLine(model.QuotationLines[0]);
            Finish(model.LastQuotationEvaluation);
            var customer = model.SelectedCustomer;
            var lines = model.QuotationLines.ToArray();
            window.Close(); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(window.IsVisible); Assert.IsTrue(window.UnsavedDialog.IsOpen);
            Assert.IsTrue(window.ContinueEditingButton.IsKeyboardFocused);
            foreach (var button in new[] { window.SaveAndCloseButton, window.DiscardButton, window.ContinueEditingButton })
            {
                Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(button)));
                Assert.IsTrue(button.ActualWidth > 0 && button.ActualHeight > 0);
                var bounds = button.TransformToAncestor(window).TransformBounds(new Rect(button.RenderSize));
                Assert.IsTrue(new Rect(window.RenderSize).Contains(bounds), "Close choices must fit inside the editor.");
            }
            if (!existing) WpfTestHost.Capture(window, $"action-06-close-prompt-{width}");
            Click(window.ContinueEditingButton);
            Assert.IsTrue(window.IsVisible); Assert.IsFalse(window.UnsavedDialog.IsOpen);
            Assert.AreEqual("6", model.DiscountInput); Assert.AreSame(customer, model.SelectedCustomer);
            Assert.AreEqual("عميل آخر تجريبي", model.CustomerName); Assert.AreEqual("777000222", model.Phone); Assert.AreEqual("عدن", model.Address);
            CollectionAssert.AreEqual(lines, model.QuotationLines.ToArray());
            window.Close(); Click(window.SaveAndCloseButton);
            Finish(QuotationDraftTests.WaitFor(() => !window.IsVisible));
            Assert.IsFalse(model.HasUnsavedDraftChanges);
            Assert.AreEqual(600, authority.Draft!.Snapshot.Intent.DiscountBasisPoints);
            Assert.HasCount(3, authority.Draft.Snapshot.Intent.Lines);
            model.DiscountInput = "7";
            Finish(model.OpenDraftAsync(authority.Draft.Snapshot.Id));
            Assert.AreEqual("6", model.DiscountInput); Assert.HasCount(3, model.QuotationLines);
            CollectionAssert.AreEqual(lines.Select(line => line.Id).ToArray(), model.QuotationLines.Select(line => line.Id).ToArray());
            Assert.AreEqual(customer!.Id, model.SelectedCustomer!.Id);
            var reopened = new QuotationEditorWindow(model, new SalesCatalogView { DataContext = model });
            reopened.Show(); reopened.Close();
            Assert.IsFalse(reopened.IsVisible); Assert.IsFalse(reopened.UnsavedDialog.IsOpen);
        });
    }

    [TestMethod]
    [DataRow(ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftInvalidV1)]
    [DataRow(ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftUnavailableV1)]
    [DataRow(ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftConflictV1)]
    public void FailedSaveKeepsWindowAndInput(string code)
    {
        Run(1338, 753, (window, model, authority, engine) => {
            engine.CommandHandler = _ => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = code } } };
            var lines = model.QuotationLines.ToArray();
            window.Close(); Click(window.SaveAndCloseButton);
            Finish(QuotationDraftTests.WaitFor(() => !model.IsDraftBusy));
            Assert.IsTrue(window.IsVisible); Assert.IsTrue(model.HasUnsavedDraftChanges); Assert.IsTrue(model.HasDraftSaveFailure);
            Assert.IsFalse(window.UnsavedDialog.IsOpen); Assert.IsTrue(model.IsReviewingQuotation);
            CollectionAssert.AreEqual(lines, model.QuotationLines.ToArray()); Assert.AreEqual("5", model.DiscountInput);
            Assert.IsNull(authority.Draft);
            window.Close(); Assert.IsTrue(window.UnsavedDialog.IsOpen);
        });
    }

    [TestMethod]
    public void InvalidFieldsKeepEditorOpenWithoutSaving()
    {
        Run(1338, 753, (window, model, authority, _) => {
            model.DiscountInput = "invalid";
            window.Close(); Click(window.SaveAndCloseButton);
            Assert.IsTrue(window.IsVisible); Assert.IsTrue(model.HasUnsavedDraftChanges);
            Assert.IsFalse(window.UnsavedDialog.IsOpen); Assert.AreEqual("invalid", model.DiscountInput);
            Assert.IsNull(authority.Draft);
        });
    }

    [TestMethod]
    public void LostReplyOnUnchangedDraftPromptsAndConfirmedRetryCloses()
    {
        Run(1338, 753, (window, model, authority, engine) => {
            Finish(model.SaveDraftAsync()); Assert.IsFalse(model.HasUnsavedDraftChanges);
            var handler = engine.CommandHandler!;
            engine.CommandHandler = command => { handler(command); throw new System.IO.IOException("Synthetic lost reply"); };
            Finish(model.SaveDraftAsync());
            Assert.IsTrue(model.HasUnsavedDraftChanges);
            var key = engine.LastIdempotencyKey;
            window.Close(); Assert.IsTrue(window.UnsavedDialog.IsOpen);
            engine.CommandHandler = handler; Click(window.SaveAndCloseButton);
            Finish(QuotationDraftTests.WaitFor(() => !window.IsVisible));
            Assert.AreEqual(key, engine.LastIdempotencyKey); Assert.AreEqual(2, authority.Draft!.Snapshot.Revision);
            Assert.HasCount(2, authority.Replays);
        });
    }

    [TestMethod]
    public void CloseDuringSaveAndNewerEditsCannotDiscardWork()
    {
        Run(1338, 753, (window, model, authority, engine) => {
            var entered = new TaskCompletionSource(); var release = new TaskCompletionSource();
            engine.CommandBarrier = _ => { entered.TrySetResult(); return release.Task; };
            window.Close(); Click(window.SaveAndCloseButton); Finish(entered.Task);
            window.Close(); Assert.IsTrue(window.IsVisible); Assert.IsFalse(window.DiscardButton.IsEnabled);
            model.DiscountInput = "6"; Finish(model.LastQuotationEvaluation);
            release.SetResult(); Finish(QuotationDraftTests.WaitFor(() => !model.IsDraftBusy));
            Assert.IsTrue(window.IsVisible); Assert.IsTrue(model.HasUnsavedDraftChanges); Assert.AreEqual("6", model.DiscountInput);
            Assert.AreEqual(500, authority.Draft!.Snapshot.Intent.DiscountBasisPoints);
            window.Close(); Click(window.SaveAndCloseButton);
            Finish(QuotationDraftTests.WaitFor(() => !window.IsVisible));
            Assert.AreEqual(600, authority.Draft.Snapshot.Intent.DiscountBasisPoints);
            Assert.HasCount(2, authority.Replays);
        });
    }

    [TestMethod]
    public void DiscardAndInvalidationCloseWithoutSaving()
    {
        Run(1338, 753, (window, model, authority, _) => {
            window.Close(); Click(window.DiscardButton);
            Assert.IsFalse(window.IsVisible); Assert.IsNull(authority.Draft);
            var invalidated = new QuotationEditorWindow(model, new SalesCatalogView { DataContext = model });
            invalidated.Show(); invalidated.CloseInvalidated();
            Assert.IsFalse(invalidated.IsVisible); Assert.IsFalse(invalidated.UnsavedDialog.IsOpen); Assert.IsNull(authority.Draft);
        });
    }

    private static void Run(int width, int height, Action<QuotationEditorWindow, SalesCatalogViewModel, QuotationDraftTests.Authority, FakeEngine> test)
    {
        WpfTestHost.Run(width, height, owner => {
            var authority = new QuotationDraftTests.Authority(); var engine = authority.Engine();
            var catalog = new SalesCatalogClient(engine); var drafts = new QuotationDraftClient(engine);
            var preparation = QuotationDraftTests.Editor(catalog, drafts); Finish(preparation); var model = preparation.Result;
            var window = new QuotationEditorWindow(model, new SalesCatalogView { DataContext = model, ShowQuotationHeader = true }) {
                Owner = owner, Width = width, Height = height, Left = -10000, Top = 0, ShowInTaskbar = false,
            };
            if (width == 1920) window.WindowState = WindowState.Maximized;
            try { window.Show(); WpfTestHost.CompleteLayout(window); test(window, model, authority, engine); }
            finally
            {
                if (window.IsVisible) window.CloseInvalidated();
                Finish(model.DisposeEditorAsync()); Finish(drafts.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask());
            }
        });
    }

    private static void Click(Button button) => button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(10));
        if (!task.IsCompleted)
        {
            var frame = new DispatcherFrame(); var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default);
            Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }
}
