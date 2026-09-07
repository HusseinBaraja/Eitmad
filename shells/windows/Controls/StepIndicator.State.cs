using System.Collections.Specialized;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;

namespace Eitmad.WindowsShell.Controls;

public class StepItem : DependencyObject
{
    public string Label { get; set; } = string.Empty;
    public string Description { get; set; } = string.Empty;
    public static readonly DependencyProperty StateProperty = DependencyProperty.Register(nameof(State), typeof(string), typeof(StepItem), new PropertyMetadata(""));
    public string State { get => (string)GetValue(StateProperty); set => SetValue(StateProperty, value); }
    public int Number { get; set; }
}
public partial class StepIndicator
{
    protected override void OnItemsChanged(NotifyCollectionChangedEventArgs e) { base.OnItemsChanged(e); UpdateSteps(); }
    private static void OnStepChanged(DependencyObject d, DependencyPropertyChangedEventArgs e) => ((StepIndicator)d).UpdateSteps();
    private void UpdateSteps()
    {
        for (int i = 0; i < Items.Count; i++) if (Items[i] is StepItem step)
        { step.Number = i + 1; step.State = i + 1 == CurrentStep ? "الحالية" : i + 1 < CurrentStep ? "السابقة" : "التالية"; }
    }
}
