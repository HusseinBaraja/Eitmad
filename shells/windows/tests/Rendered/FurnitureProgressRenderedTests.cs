using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Media;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Layout;
using Eitmad.WindowsShell.Tests.Furniture;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class FurnitureProgressRenderedTests
{
    [TestMethod]
    [DataRow(720, 560)]
    [DataRow(1338, 753)]
    [DataRow(1920, 1080)]
    public void ProgressTracksEveryStepAndSwitchesPresentationOnResize(int width, int height)
    {
        var fixture = new FurnitureFixtures();
        fixture.Seed();
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
            var item = view.ViewModel.VisibleFurniture.Single();
            Assert.IsTrue(view.PrepareEditorAsync(item).GetAwaiter().GetResult());
            view.ViewModel.BeginEdit(item);
            WpfTestHost.CompleteLayout(window);
            var indicator = WpfTestHost.FindByName<StepIndicator>(view, "FurnitureProgress");
            var dpi = VisualTreeHelper.GetDpi(window);
            var geometry = $"Requested: {width}x{height}; actual window: {window.ActualWidth}x{window.ActualHeight} DIP; "
                + $"scaling: {dpi.DpiScaleX * 100}%; primary display: {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; "
                + "capture: WPF RenderTargetBitmap at 96 DPI; synthetic engine responses.";
            Console.WriteLine(geometry);
            if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory)
            {
                Directory.CreateDirectory(directory);
                File.WriteAllText(Path.Combine(directory, $"ui19-{width}x{height}.txt"), geometry);
            }

            for (var step = 1; step <= indicator.Items.Count; step++)
            {
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(step, view.ViewModel.CurrentStep);
                if (step is 1 or 5) WpfTestHost.Capture(window, $"ui19-step-{step}-{width}x{height}");
                AssertProgress(indicator, step);
                if (step < indicator.Items.Count)
                    WpfTestHost.FindByName<Button>(view, "NextButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            }

            if (width == 1338)
            {
                view.ViewModel.MoveToPreviousStep();
                window.Width = 720;
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(ResponsiveLayoutMode.Compact, ResponsiveLayout.GetMode(indicator));
                AssertProgress(indicator, 5);
                view.ViewModel.MoveToPreviousStep();
                WpfTestHost.CompleteLayout(window);
                AssertProgress(indicator, 4);
                window.Width = width;
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(ResponsiveLayoutMode.Standard, ResponsiveLayout.GetMode(indicator));
                AssertProgress(indicator, 4);
            }

            if (width == 720)
            {
                view.ViewModel.MoveToPreviousStep();
                indicator.FontSize *= 1.5;
                view.Resources[SystemParameters.HighContrastKey] = true;
                view.Resources[SystemColors.WindowTextBrushKey] = Brushes.White;
                view.Resources[SystemColors.WindowBrushKey] = Brushes.Black;
                view.Resources[SystemColors.ControlTextBrushKey] = Brushes.White;
                view.Resources[SystemColors.ControlBrushKey] = Brushes.Black;
                WpfTestHost.CompleteLayout(window);
                AssertProgress(indicator, 5);
                Assert.AreEqual(Brushes.White, indicator.Foreground);
                Assert.AreEqual(Brushes.Black, WpfTestHost.Descendants<Grid>(indicator).First().Background);
                WpfTestHost.Capture(window, "ui19-step-5-large-text-system-colors-720x560");
            }
        }, engine: fixture.Engine());
    }

    private static void AssertProgress(StepIndicator indicator, int currentStep)
    {
        var containers = Enumerable.Range(0, indicator.Items.Count)
            .Select(index => (FrameworkElement)indicator.ItemContainerGenerator.ContainerFromIndex(index)).ToArray();
        var compact = ResponsiveLayout.GetMode(indicator) == ResponsiveLayoutMode.Compact;
        Assert.AreEqual(compact ? 1 : indicator.Items.Count, containers.Count(container => container.IsVisible));
        var current = containers[currentStep - 1];
        Assert.IsTrue(current.IsVisible);
        var label = WpfTestHost.Descendants<TextBlock>(current)
            .Single(text => text.Text == (string)indicator.Items[currentStep - 1]);
        var labelBounds = label.TransformToAncestor(indicator).TransformBounds(new Rect(label.RenderSize));
        Assert.IsTrue(labelBounds.Left >= -1 && labelBounds.Right <= indicator.ActualWidth + 1,
            "The current step name must fit inside the progress area.");
        if (!compact) return;
        var position = WpfTestHost.Descendants<TextBlock>(indicator).Single(text => text.Name == "StepPosition");
        Assert.AreEqual($"الخطوة {currentStep} من {indicator.Items.Count}",
            string.Concat(position.Inlines.OfType<Run>().Select(run => run.Text)));
        var positionBounds = position.TransformToAncestor(indicator).TransformBounds(new Rect(position.RenderSize));
        Assert.IsTrue(positionBounds.Left >= -1 && positionBounds.Right <= indicator.ActualWidth + 1);
        Assert.IsFalse(labelBounds.IntersectsWith(positionBounds), "The current step and position must have separate space.");
    }
}
