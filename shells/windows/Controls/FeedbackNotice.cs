using System.Windows;

namespace Eitmad.WindowsShell.Controls;

public partial class FeedbackNotice : System.Windows.Controls.Control
{
    public static readonly DependencyProperty MessageProperty = DependencyProperty.Register(nameof(Message), typeof(string), typeof(FeedbackNotice), new PropertyMetadata(string.Empty, OnNoticeChanged));
    public string Message { get => (string)GetValue(MessageProperty); set => SetValue(MessageProperty, value); }

    public static readonly DependencyProperty ToneProperty = DependencyProperty.Register(nameof(Tone), typeof(PresentationTone), typeof(FeedbackNotice), new PropertyMetadata(PresentationTone.Information));
    public PresentationTone Tone { get => (PresentationTone)GetValue(ToneProperty); set => SetValue(ToneProperty, value); }

    public static readonly DependencyProperty IsFloatingProperty = DependencyProperty.Register(nameof(IsFloating), typeof(bool), typeof(FeedbackNotice), new PropertyMetadata(false));
    public bool IsFloating { get => (bool)GetValue(IsFloatingProperty); set => SetValue(IsFloatingProperty, value); }

    public static readonly DependencyProperty CanDismissProperty = DependencyProperty.Register(nameof(CanDismiss), typeof(bool), typeof(FeedbackNotice), new PropertyMetadata(false));
    public bool CanDismiss { get => (bool)GetValue(CanDismissProperty); set => SetValue(CanDismissProperty, value); }

    public static readonly DependencyProperty DisplayDurationProperty = DependencyProperty.Register(nameof(DisplayDuration), typeof(TimeSpan), typeof(FeedbackNotice), new PropertyMetadata(TimeSpan.Zero, OnNoticeChanged));
    public TimeSpan DisplayDuration { get => (TimeSpan)GetValue(DisplayDurationProperty); set => SetValue(DisplayDurationProperty, value); }

}
