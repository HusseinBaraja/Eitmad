using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;

namespace Eitmad.WindowsShell.Controls;

public partial class FeedbackNotice
{
    private readonly DispatcherTimer timer = new();
    public static readonly RoutedEvent DismissedEvent = EventManager.RegisterRoutedEvent(nameof(Dismissed), RoutingStrategy.Bubble, typeof(RoutedEventHandler), typeof(FeedbackNotice));
    public event RoutedEventHandler Dismissed { add => AddHandler(DismissedEvent, value); remove => RemoveHandler(DismissedEvent, value); }
    public FeedbackNotice()
    {
        timer.Tick += (_, _) => Dismiss();
        Loaded += (_, _) => RestartDuration();
        Unloaded += (_, _) => timer.Stop();
    }
    public override void OnApplyTemplate()
    {
        if (GetTemplateChild("PART_Dismiss") is System.Windows.Controls.Button oldButton) oldButton.Click -= DismissClick;
        base.OnApplyTemplate();
        if (GetTemplateChild("PART_Dismiss") is System.Windows.Controls.Button button) button.Click += DismissClick;
    }
    private void DismissClick(object sender, RoutedEventArgs e) => Dismiss();
    private void Dismiss() { timer.Stop(); RaiseEvent(new RoutedEventArgs(DismissedEvent, this)); }
    public void RestartDuration()
    {
        timer.Stop();
        if (IsLoaded && !string.IsNullOrEmpty(Message) && DisplayDuration > TimeSpan.Zero) { timer.Interval = DisplayDuration; timer.Start(); }
    }
    private static void OnNoticeChanged(DependencyObject d, DependencyPropertyChangedEventArgs e) => ((FeedbackNotice)d).RestartDuration();
}
