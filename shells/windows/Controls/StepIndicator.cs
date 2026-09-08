using System.Windows;

namespace Eitmad.WindowsShell.Controls;

public partial class StepIndicator : System.Windows.Controls.ItemsControl
{
    public StepIndicator()
    {
        ItemContainerGenerator.StatusChanged += (_, _) => UpdateSteps();
    }

    public static readonly DependencyProperty CurrentStepProperty = DependencyProperty.Register(nameof(CurrentStep), typeof(int), typeof(StepIndicator), new PropertyMetadata(1, OnStepChanged));
    public int CurrentStep { get => (int)GetValue(CurrentStepProperty); set => SetValue(CurrentStepProperty, value); }

}
