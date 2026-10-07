using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
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
                Console.WriteLine($"Quotation requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%");
                Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
                WpfTestHost.Capture(window, $"quotation-evaluation-lines-{width}");
                WpfTestHost.Descendants<ScrollViewer>(view).First().ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(t => t.Text == "959"));
                Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "PrintPreviewButton").IsEnabled);
                WpfTestHost.Capture(window, $"quotation-evaluation-summary-{width}");
                WpfTestHost.FindByName<TextBox>(view, "DiscountInput").Text = "5.001";
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual("—", model.FinalTotalLabel);
                Assert.IsFalse(WpfTestHost.FindByName<Button>(view, "PrintPreviewButton").IsEnabled);
                WpfTestHost.Capture(window, $"quotation-evaluation-invalid-{width}");
            }
            finally { if (model is not null) Finish(model.DeactivateCatalogAsync()); Finish(client.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask()); }
        });
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
