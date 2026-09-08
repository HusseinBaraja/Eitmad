using System.Collections.Specialized;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;

namespace Eitmad.WindowsShell.Controls;

public partial class StepIndicator
{
    public static readonly DependencyProperty NumberProperty = DependencyProperty.RegisterAttached("Number", typeof(int), typeof(StepIndicator), new PropertyMetadata(0));
    public static int GetNumber(DependencyObject element) => (int)element.GetValue(NumberProperty);
    public static void SetNumber(DependencyObject element, int value) => element.SetValue(NumberProperty, value);
    public static readonly DependencyProperty StateProperty = DependencyProperty.RegisterAttached("State", typeof(string), typeof(StepIndicator), new PropertyMetadata(string.Empty));
    public static string GetState(DependencyObject element) => (string)element.GetValue(StateProperty);
    public static void SetState(DependencyObject element, string value) => element.SetValue(StateProperty, value);

    protected override void PrepareContainerForItemOverride(DependencyObject element, object item)
    {
        base.PrepareContainerForItemOverride(element, item);
        UpdateStep(element, ItemContainerGenerator.IndexFromContainer(element));
    }

    protected override void OnItemsChanged(NotifyCollectionChangedEventArgs e)
    {
        base.OnItemsChanged(e);
        UpdateSteps();
    }

    private static void OnStepChanged(DependencyObject d, DependencyPropertyChangedEventArgs e) => ((StepIndicator)d).UpdateSteps();

    private void UpdateSteps()
    {
        for (var i = 0; i < Items.Count; i++)
            if (ItemContainerGenerator.ContainerFromIndex(i) is DependencyObject container)
                UpdateStep(container, i);
    }

    private void UpdateStep(DependencyObject container, int index)
    {
        var number = index + 1;
        var state = number == CurrentStep ? "الحالية" : number < CurrentStep ? "السابقة" : "التالية";
        SetNumber(container, number);
        SetState(container, state);
        AutomationProperties.SetName(container, $"{number}. {Items[index]}، {state}");
    }
}
