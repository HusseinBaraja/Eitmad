using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;

namespace Eitmad.WindowsShell.Controls;

public class FormField : ContentControl
{
    public static readonly DependencyProperty LabelProperty = DependencyProperty.Register(nameof(Label), typeof(string), typeof(FormField), new PropertyMetadata(string.Empty));
    public static readonly DependencyProperty HelpTextProperty = DependencyProperty.Register(nameof(HelpText), typeof(string), typeof(FormField), new PropertyMetadata(string.Empty));
    public static readonly DependencyProperty ErrorTextProperty = DependencyProperty.Register(nameof(ErrorText), typeof(string), typeof(FormField), new PropertyMetadata(string.Empty));
    public static readonly DependencyProperty IsRequiredProperty = DependencyProperty.Register(nameof(IsRequired), typeof(bool), typeof(FormField), new PropertyMetadata(false));
    public static readonly DependencyProperty LabelTargetProperty = DependencyProperty.Register(nameof(LabelTarget), typeof(UIElement), typeof(FormField), new PropertyMetadata(null, OnTargetChanged));
    public string Label { get => (string)GetValue(LabelProperty); set => SetValue(LabelProperty, value); }
    public string HelpText { get => (string)GetValue(HelpTextProperty); set => SetValue(HelpTextProperty, value); }
    public string ErrorText { get => (string)GetValue(ErrorTextProperty); set => SetValue(ErrorTextProperty, value); }
    public bool IsRequired { get => (bool)GetValue(IsRequiredProperty); set => SetValue(IsRequiredProperty, value); }
    public UIElement? LabelTarget { get => (UIElement?)GetValue(LabelTargetProperty); set => SetValue(LabelTargetProperty, value); }
    private UIElement? associatedTarget;
    private UIElement? previousLabel;
    private System.Windows.Controls.Label? fieldLabel;

    public FormField() { Loaded += (_, _) => AssociateLabel(); Unloaded += (_, _) => ReleaseLabel(); }
    public override void OnApplyTemplate()
    {
        ReleaseLabel();
        base.OnApplyTemplate();
        fieldLabel = GetTemplateChild("PART_Label") as System.Windows.Controls.Label;
        AssociateLabel();
    }
    protected override void OnContentChanged(object oldContent, object newContent)
    {
        base.OnContentChanged(oldContent, newContent);
        Dispatcher.BeginInvoke(DispatcherPriority.Loaded, new Action(AssociateLabel));
    }
    private static void OnTargetChanged(DependencyObject d, DependencyPropertyChangedEventArgs e) => ((FormField)d).AssociateLabel();
    private void ReleaseLabel()
    {
        if (associatedTarget is not null && AutomationProperties.GetLabeledBy(associatedTarget) == fieldLabel)
            associatedTarget.SetCurrentValue(AutomationProperties.LabeledByProperty, previousLabel);
        associatedTarget = null;
    }
    private void AssociateLabel()
    {
        ReleaseLabel();
        if (fieldLabel is null) return;
        var target = LabelTarget ?? FindInput(Content as DependencyObject);
        fieldLabel.Target = target;
        if (target is null) return;
        associatedTarget = target;
        previousLabel = AutomationProperties.GetLabeledBy(target);
        target.SetCurrentValue(AutomationProperties.LabeledByProperty, fieldLabel);
    }
    private static UIElement? FindInput(DependencyObject? root)
    {
        if (root is null) return null;
        if (root is System.Windows.Controls.Primitives.TextBoxBase or System.Windows.Controls.ComboBox or DatePicker
            or System.Windows.Controls.Primitives.ToggleButton or PasswordBox or Slider) return (UIElement)root;
        for (int i = 0; i < VisualTreeHelper.GetChildrenCount(root); i++)
            if (FindInput(VisualTreeHelper.GetChild(root, i)) is UIElement input) return input;
        return null;
    }
}
