using System.Globalization;
using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.Quotations;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class QuotationDraftResolutionRenderedTests
{
    [TestMethod]
    [DataRow(1920,1080)]
    [DataRow(1338,753)]
    [DataRow(720,560)]
    public void ReviewRequiresExplicitChoiceKeepsRetryAndRejectsStaleDecision(int width,int height)
    {
        var page = Comparison();
        var engine = new FakeEngine {
            SupportedCapabilities = new HashSet<string> { ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftResolutionV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1 },
            QueryHandler = _ => SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationDraftReview(page)),
            CommandHandler = _ => throw new IOException("Synthetic lost response"),
        };
        var client = new QuotationDraftClient(engine);
        WpfTestHost.Run(width,height,window => {
            Console.WriteLine($"Available display before capture: {System.Windows.Forms.Screen.PrimaryScreen!.Bounds}; WPF display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight}");
            var view = new QuotationDraftResolutionView(client,page.Current.Id,page.Current.Evaluation.Scope.Id); window.Content = view;
            if (width == 1920) window.WindowState = WindowState.Maximized;
            Finish(view.LoadAsync()); WpfTestHost.CompleteLayout(window);
            var dpi = VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"Requested {width}x{height}; actual window {window.ActualWidth}x{window.ActualHeight}; DPI {dpi.PixelsPerInchX}; scaling {dpi.DpiScaleX*100}%");
            Assert.AreEqual(System.Windows.FlowDirection.RightToLeft, view.FlowDirection);
            Assert.AreEqual(-1,view.Choice.SelectedIndex);
            Finish(view.ConfirmAsync()); Assert.IsNull(engine.LastCommand);
            view.Sources.Focus(); Assert.IsTrue(view.Sources.IsKeyboardFocusWithin);
            Assert.AreEqual("اختيار تعارض المسودة أو تعديل محفوظ",AutomationProperties.GetName(view.Sources));
            view.Sources.IsDropDownOpen=true; WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window,$"draft-resolution-source-popup-{width}");
            view.Sources.IsDropDownOpen=false;
            Assert.AreEqual(2,view.Sources.Items.Count);
            view.Choice.SelectedIndex=1; view.Choice.BringIntoView(); view.Choice.Focus(); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(view.Choice.IsKeyboardFocusWithin);
            Assert.AreEqual("اختيار النسخة المطلوبة للمسودة",AutomationProperties.GetName(view.Choice));
            WpfTestHost.Capture(window,$"draft-resolution-{width}x{height}");
            Assert.IsTrue(view.ConfirmButton.IsVisible);
            Finish(view.ConfirmAsync());
            var key=engine.LastIdempotencyKey; var command=JsonSerializer.Serialize(engine.LastCommand);
            Assert.IsFalse(view.Choice.IsEnabled); Assert.IsFalse(view.ReloadButton.IsEnabled);
            Finish(view.ConfirmAsync());
            Assert.AreEqual(key,engine.LastIdempotencyKey); Assert.AreEqual(command,JsonSerializer.Serialize(engine.LastCommand));
            engine.CommandHandler = _ => Failure(ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftConflictV1);
            Finish(view.ConfirmAsync()); Assert.IsFalse(view.ConfirmButton.IsEnabled); Assert.IsTrue(view.ReloadButton.IsEnabled);
            Finish(view.LoadAsync()); Assert.AreEqual(-1,view.Choice.SelectedIndex);
            if (width == 1338) {
                foreach(var control in WpfTestHost.Descendants<Control>(view)) Controls.ControlOptions.SetHighContrast(control,true);
                WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window,"draft-resolution-high-contrast");
            }
            view.Detach();
        },engine:engine);
        Finish(client.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask());
    }
    [TestMethod]
    public void ManagerOpensHeldDraftWithConfirmedServerBranch()
    {
        var page = Comparison();
        var local = new QuotationDraft { Scope = page.Current.Evaluation.Scope, Snapshot = page.Items[0].Local, SyncState = SyncState.Conflicted };
        var serverScope = new ScopeRef { Kind = "branch", Id = Guid.NewGuid() };
        var record = new QuotationRecord { Scope = serverScope, Quotation = page.Current, State = QuotationState.Draft };
        var engine = new FakeEngine { SupportedCapabilities = new HashSet<string> {
            ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftResolutionV1,
            ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationApprovalV1,
        } };
        var reviewedBranch = Guid.Empty;
        engine.QueryHandler = q => {
            if (q.AsQuotationDraftReview() is { } review) reviewedBranch = review.BranchId;
            return q.Kind switch {
            Query.QuotationDraftListKind => SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationDrafts(new() { Items = [local] })),
            Query.QuotationListKind => SalesCatalogAuthorityTests.Response(QueryResult.ForQuotations(new() { Items = [record], ServerAvailable = true })),
            Query.QuotationApprovalListKind => SalesCatalogAuthorityTests.Response(QueryResult.ForDiscountApprovals(new() { Items = [new() { Scope = serverScope, Quotation = page.Current, State = DiscountApprovalState.Pending }] })),
            Query.QuotationDraftReviewKind => SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationDraftReview(page)),
            _ => SalesCatalogAuthorityTests.Failure(ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1),
            };
        };
        var client = new QuotationDraftClient(engine);
        WpfTestHost.Run(720,560,window => {
            var model = new QuotationsViewModel(); model.AttachDraftClient(client); Finish(model.ActivateDraftsAsync());
            model.OpenQuotation(model.VisibleQuotations.Single());
            Assert.AreSame(local, model.SelectedQuotation!.Draft);
            Assert.IsTrue(model.CanReviewDraft);
            var view = model.CreateDraftReview()!; window.Content = view; Finish(view.LoadAsync());
            Assert.AreEqual(serverScope.Id, reviewedBranch);
            view.Detach(); Finish(model.DeactivateDraftsAsync());
        });
        Finish(client.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask());
    }
    [TestMethod]
    public void ReopenedUncertainDecisionRetainsServerKeyAndFencesLateResponse()
    {
        var page=Comparison(); page.CanResolve=false; // The document became final after an uncertain decision.
        var source=QuotationDraftResolutionSource.ForConflict(new() { ConflictId=page.Items[0].Conflict.ConflictId });
        var key=Guid.NewGuid(); page.PendingResolution=new() { Scope=page.Current.Evaluation.Scope,IdempotencyKey=key,
            Command=new() { DraftId=page.Current.Id,BranchId=page.Current.Evaluation.Scope.Id,ExpectedServerRevision=2,
                Source=JsonSerializer.Deserialize<Dictionary<string,object>>(JsonSerializer.Serialize(source))!,Choice=QuotationDraftResolutionChoice.KeepCurrent } };
        var engine=new FakeEngine {SupportedCapabilities = new HashSet<string> { ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftResolutionV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1 }, QueryHandler=_=>SalesCatalogAuthorityTests.Response(QueryResult.ForQuotationDraftReview(page)),
            CommandHandler=_=>Failure(ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftUnavailableV1)};
        var client=new QuotationDraftClient(engine);
        WpfTestHost.Run(720,560,window=> {
            var view=new QuotationDraftResolutionView(client,page.Current.Id,page.Current.Evaluation.Scope.Id);window.Content=view;
            Finish(view.LoadAsync());Assert.IsFalse(view.Choice.IsEnabled);Assert.IsTrue(view.ConfirmButton.IsEnabled);
            Finish(view.ConfirmAsync());Assert.AreEqual(key,engine.LastIdempotencyKey);
            var barrier=new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            engine.QueryBarrier=_=>barrier.Task;
            var loading=view.LoadAsync();view.Detach();barrier.SetResult();Finish(loading);
            Assert.IsFalse(view.Choice.IsEnabled);
        },engine:engine);
        Finish(client.DisposeAsync().AsTask());Finish(engine.DisposeAsync().AsTask());
    }
    private static QuotationDraftReview Comparison()
    {
        var authority=new QuotationDraftTests.Authority();
        var intent=new EvaluateQuotation { Customer=new() { Id=Guid.NewGuid(),Revision=1 },DiscountBasisPoints=300,
            Lines=[new() { Id=Guid.NewGuid(),Configuration=new() { Selection=new() { Target=authority.Furniture.Price.Target,PriceRevision=1,Quantity=2 },
                Dimensions=new() { WidthMm=1800,HeightMm=2200,DepthMm=600 } } }] };
        var current=new QuotationDraftSnapshot { Id=Guid.NewGuid(),Revision=2,Intent=intent,Evaluation=authority.Evaluate(intent) };
        var local=QuotationDraftTests.Copy(current); local.Intent.DiscountBasisPoints=400;local.Evaluation=authority.Evaluate(local.Intent);
        var change=new ChangeRecord { ChangeId=Guid.NewGuid(),RecordId=current.Id,Scope=current.Evaluation.Scope,BaseRevision=1,Revision=2,IdempotencyKey=Guid.NewGuid() };
        return new() { Current=current,CanResolve=true,Items=[new() { Local=local,Remote=current,Conflict=new() {ConflictId=Guid.NewGuid(),Scope=current.Evaluation.Scope,Local=change,Remote=change} }],
            PendingReviews=[new() {Change=change,Snapshot=local}] };
    }
    private static CommandResponseEnvelope Failure(string code)=>new() {Outcome=new() {Status=CommandOutcomeStatus.Failed,Payload=new() {Code=code}}};
    private static void Finish(Task task)
    {
        task=task.WaitAsync(TimeSpan.FromSeconds(10));
        if(!task.IsCompleted) {
            var frame=new DispatcherFrame();var dispatcher=Dispatcher.CurrentDispatcher;
            _=task.ContinueWith(_=>dispatcher.BeginInvoke(new Action(()=>frame.Continue=false)),TaskScheduler.Default);Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }
}
