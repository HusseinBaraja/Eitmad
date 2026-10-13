using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using System.IO;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.Products;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class QuotationEvaluationRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void LiveQuotationShowsReturnedSummaryAndClearsInvalidInputTotals(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            var entry = SalesCatalogAuthorityTests.Entry();
            var engine = QuotationEvaluationTests.Engine(entry);
            engine.QueryHandler = q => {
                if (q.AsQuotationEvaluate() is { } input) return QuotationEvaluationTests.Evaluation(input);
                var response = SalesCatalogAuthorityTests.Handle(q, entry);
                if (response.Outcome.Payload.AsSalesConfiguration() is { } configuration) {
                    configuration.Price.UnitPriceYer = 1010; configuration.Price.TotalYer = 1010;
                }
                return response;
            };
            var client = new SalesCatalogClient(engine);
            SalesCatalogViewModel? model = null;
            try
            {
                var preparation = QuotationEvaluationTests.Model(client); Finish(preparation); model = preparation.Result;
                model.DiscountInput = "5"; Finish(model.LastQuotationEvaluation);
                var view = new CurrentQuotationView { DataContext = model }; window.Content = view;
                if (width == 1920) window.WindowState = WindowState.Maximized;
                WpfTestHost.CompleteLayout(window);
                var dpi = VisualTreeHelper.GetDpi(window);
                var geometry = $"Quotation requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; display {SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX:0}x{SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY:0} px; scale {dpi.DpiScaleX * 100:0}%; WPF RenderTargetBitmap at 96 DPI; synthetic Rust responses.";
                Console.WriteLine(geometry);
                if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory) {
                    Directory.CreateDirectory(directory);
                    File.WriteAllText(Path.Combine(directory, $"ui31-{width}.txt"), geometry);
                }
                Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
                WpfTestHost.Capture(window, $"quotation-evaluation-lines-{width}");
                WpfTestHost.Descendants<ScrollViewer>(view).First().ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(t => t.Text == "959"));
                Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "PrintPreviewButton").IsEnabled);
                AssertSummaryFits(view);
                WpfTestHost.Capture(window, $"quotation-evaluation-summary-{width}");
                if (width == 720) {
                    engine.QueryHandler = q => q.AsQuotationEvaluate() is { } input
                        ? QuotationEvaluationTests.Evaluation(input, approval: true) : SalesCatalogAuthorityTests.Handle(q, entry);
                    model.DiscountInput = "6"; Finish(model.LastQuotationEvaluation);
                    WpfTestHost.CompleteLayout(window);
                    WpfTestHost.Descendants<ScrollViewer>(view).First().ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                    AssertSummaryFits(view);
                    WpfTestHost.Capture(window, "quotation-evaluation-approval-720");
                }
                WpfTestHost.FindByName<TextBox>(view, "DiscountInput").Text = "5.001";
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual("—", model.FinalTotalLabel);
                Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "PrintPreviewButton").IsEnabled);
                WpfTestHost.Capture(window, $"quotation-evaluation-invalid-{width}");
                if (width == 720) {
                    view.Resources[SystemParameters.HighContrastKey] = true;
                    WpfTestHost.CompleteLayout(window);
                    WpfTestHost.Capture(window, "quotation-evaluation-system-colors-720");
                }
            }
            finally { if (model is not null) Finish(model.DeactivateCatalogAsync()); Finish(client.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask()); }
        });
    }

    private static void AssertSummaryFits(CurrentQuotationView view)
    {
        var summary = WpfTestHost.FindByName<Border>(view, "FinancialSummary");
        Assert.IsTrue(summary.ActualWidth <= 480);
        var amounts = WpfTestHost.Descendants<AmountDisplay>(summary).ToArray();
        Assert.HasCount(3, amounts);
        var bounds = amounts.Select(amount => amount.TransformToAncestor(summary).TransformBounds(new Rect(amount.RenderSize))).ToArray();
        foreach (var bound in bounds) {
            Assert.IsTrue(new Rect(summary.RenderSize).Contains(bound), "Evaluated amounts must fit inside the summary.");
        }
        var grid = WpfTestHost.Ancestor<Grid>(amounts[0]);
        var valueEdges = amounts.Select(amount => amount.TransformToAncestor(grid).TransformBounds(new Rect(amount.RenderSize)).Right).ToArray();
        foreach (var edge in valueEdges) Assert.AreEqual(valueEdges[0], edge, 1, "Amounts must share one value column.");
        var input = WpfTestHost.FindByName<TextBox>(summary, "DiscountInput");
        Assert.IsTrue(new Rect(summary.RenderSize).Contains(input.TransformToAncestor(summary).TransformBounds(new Rect(input.RenderSize))));
    }

    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(10));
        if (!task.IsCompleted) {
            var frame = new DispatcherFrame(); var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default);
            Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }
}
