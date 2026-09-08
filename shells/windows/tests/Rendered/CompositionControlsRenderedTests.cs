using System.Collections.ObjectModel;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Eitmad.WindowsShell.Controls;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class CompositionControlsRenderedTests
{
    [TestMethod]
    public void SharedCompositionsUseSystemColorsAndLargeAmountText()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            window.Resources[SystemParameters.HighContrastKey] = true;
            window.Resources[SystemColors.WindowBrushKey] = System.Windows.Media.Brushes.Black;
            window.Resources[SystemColors.WindowTextBrushKey] = System.Windows.Media.Brushes.White;
            var amount = new AmountDisplay { AmountText = "-12,345.50", UnitText = "ر.س.", FontSize = 28 };
            var notice = new FeedbackNotice { Message = "رسالة تجريبية طويلة توضح نتيجة الإجراء وتبقى قابلة للقراءة", Tone = PresentationTone.Warning, FontSize = 24 };
            var field = new FormField { Label = "اسم المادة", HelpText = "أدخل الاسم كما يظهر في القائمة", ErrorText = "راجع الاسم المدخل قبل الحفظ", Content = new TextBox { Text = "خشب زان", FontSize = 24, Style = (Style)window.FindResource("TextInput") } };
            var steps = new StepIndicator { CurrentStep = 2 };
            steps.Items.Add(new StepItem { Label = "المعلومات", Description = "بيانات المادة" });
            steps.Items.Add(new StepItem { Label = "المراجعة", Description = "تأكيد البيانات" });
            var panel = new StackPanel { Margin = new Thickness(24) };
            panel.Children.Add(new PageHeader { Title = "مراجعة العرض", Subtitle = "بيانات عربية تجريبية", FontSize = 30 });
            panel.Children.Add(new StatusBadge { Text = "بانتظار الموافقة", Tone = PresentationTone.Warning, FontSize = 24 });
            panel.Children.Add(amount);
            panel.Children.Add(notice);
            panel.Children.Add(steps);
            panel.Children.Add(field);
            panel.Children.Add(new EmptyState { Heading = "لا توجد نتائج مطابقة", Description = "جرّب تغيير البحث أو عوامل التصفية", IsCompact = true });
            window.Content = new ScrollViewer { Content = panel };
            WpfTestHost.CompleteLayout(window);
            var amountText = (TextBlock)amount.Template.FindName("Amount", amount);
            Assert.AreEqual(28d, amountText.FontSize);
            Assert.AreEqual(System.Windows.Media.Brushes.White, amountText.Foreground);
            Assert.AreEqual(System.Windows.Media.Brushes.White, WpfTestHost.Descendants<TextBlock>(notice).First(text => text.Text == notice.Message).Foreground);
            Assert.AreEqual(System.Windows.Media.Brushes.White, ((TextBlock)field.Template.FindName("Error", field)).Foreground);
            var directory = Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR");
            if (!string.IsNullOrEmpty(directory))
            {
                System.IO.Directory.CreateDirectory(directory);
                var bitmap = new System.Windows.Media.Imaging.RenderTargetBitmap((int)window.ActualWidth, (int)window.ActualHeight, 96, 96, System.Windows.Media.PixelFormats.Pbgra32);
                bitmap.Render(window);
                var encoder = new System.Windows.Media.Imaging.PngBitmapEncoder();
                encoder.Frames.Add(System.Windows.Media.Imaging.BitmapFrame.Create(bitmap));
                using var stream = System.IO.File.Create(System.IO.Path.Combine(directory, "compositions-large-system-colors.png"));
                encoder.Save(stream);
            }
        });
    }

    [TestMethod]
    public void FeedbackReplacementRestartsExpiryAndExpiresOnlyOnce()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var notice = new FeedbackNotice { Message = "تم الحفظ", DisplayDuration = TimeSpan.FromMilliseconds(400) };
            var dismissals = 0;
            notice.Dismissed += (_, _) => dismissals++;
            window.Content = notice;
            WpfTestHost.CompleteLayout(window);
            PumpFor(TimeSpan.FromMilliseconds(250));
            notice.Message = "تم تحديث الطلب";
            PumpFor(TimeSpan.FromMilliseconds(250));
            Assert.AreEqual(0, dismissals, "The replacement must have its own full display duration.");
            PumpFor(TimeSpan.FromMilliseconds(250));
            Assert.AreEqual(1, dismissals);
            Assert.AreEqual("تم تحديث الطلب", notice.Message, "Expiry signals the owner without rewriting its message.");
            PumpFor(TimeSpan.FromMilliseconds(450));
            Assert.AreEqual(1, dismissals, "An expired notice must not keep firing its timer.");
        });
    }

    [TestMethod]
    public void FeedbackUnloadStopsExpiryAndReloadRestartsIt()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var panel = new StackPanel();
            var notice = new FeedbackNotice { Message = "رسالة تجريبية", DisplayDuration = TimeSpan.FromMilliseconds(150) };
            var dismissals = 0;
            notice.Dismissed += (_, _) => dismissals++;
            panel.Children.Add(notice);
            window.Content = panel;
            WpfTestHost.CompleteLayout(window);
            panel.Children.Remove(notice);
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(notice.IsLoaded);
            PumpFor(TimeSpan.FromMilliseconds(300));
            Assert.AreEqual(0, dismissals, "Detached notices must release their active timer.");
            panel.Children.Add(notice);
            WpfTestHost.CompleteLayout(window);
            PumpFor(TimeSpan.FromMilliseconds(300));
            Assert.AreEqual(1, dismissals);
        });
    }

    [TestMethod]
    public void AmountKeepsNumericTextAndPhysicalUnitPlacementInRtl()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var amount = new AmountDisplay { AmountText = "-1,250.50", UnitText = "ر.ي", FlowDirection = FlowDirection.RightToLeft };
            window.Content = amount;
            WpfTestHost.CompleteLayout(window);
            var number = (TextBlock)amount.Template.FindName("Amount", amount);
            var after = (TextBlock)amount.Template.FindName("After", amount);
            var before = (TextBlock)amount.Template.FindName("Before", amount);
            Assert.AreEqual(FlowDirection.LeftToRight, number.FlowDirection);
            Assert.AreEqual("-1,250.50", number.Text);
            Assert.AreEqual(FlowDirection.RightToLeft, after.FlowDirection);
            Assert.IsTrue(ScreenBounds(after).Left >= ScreenBounds(number).Right);
            amount.UnitPlacement = UnitPlacement.Before;
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(Visibility.Collapsed, after.Visibility);
            Assert.IsTrue(ScreenBounds(before).Right <= ScreenBounds(number).Left);
            amount.AmountText = string.Empty;
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(amount.EmptyText, number.Text);
        });
    }

    [TestMethod]
    public void StepChangesAndCollectionInsertionUpdateRenderedStates()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var steps = new ObservableCollection<StepItem>
            {
                new() { Label = "بيانات الطلب" },
                new() { Label = "مراجعة الطلب" },
            };
            var indicator = new StepIndicator { ItemsSource = steps, CurrentStep = 1 };
            window.Content = indicator;
            WpfTestHost.CompleteLayout(window);
            AssertState(indicator, steps[0], "الحالية");
            AssertState(indicator, steps[1], "التالية");
            indicator.CurrentStep = 2;
            WpfTestHost.CompleteLayout(window);
            AssertState(indicator, steps[0], "السابقة");
            AssertState(indicator, steps[1], "الحالية");
            steps.Insert(0, new StepItem { Label = "اختيار العميل" });
            WpfTestHost.CompleteLayout(window);
            AssertState(indicator, steps[0], "السابقة");
            AssertState(indicator, steps[1], "الحالية");
            AssertState(indicator, steps[2], "التالية");
            CollectionAssert.AreEqual(new[] { 1, 2, 3 }, steps.Select(step => step.Number).ToArray());
        });
    }

    private static void AssertState(StepIndicator indicator, StepItem step, string state)
    {
        Assert.AreEqual(state, step.State);
        Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(indicator).Any(text => ReferenceEquals(text.DataContext, step) && text.Text == state));
    }

    private static Rect ScreenBounds(FrameworkElement element) =>
        new(element.PointToScreen(new Point()), element.PointToScreen(new Point(element.ActualWidth, element.ActualHeight)));

    private static void PumpFor(TimeSpan duration)
    {
        var frame = new DispatcherFrame();
        var timer = new DispatcherTimer(DispatcherPriority.ApplicationIdle) { Interval = duration };
        timer.Tick += (_, _) => { timer.Stop(); frame.Continue = false; };
        timer.Start();
        Dispatcher.PushFrame(frame);
    }
}
