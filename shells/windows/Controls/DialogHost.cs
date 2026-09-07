using System.Runtime.CompilerServices;
using Button = System.Windows.Controls.Button;
using ComboBox = System.Windows.Controls.ComboBox;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;
using MouseEventArgs = System.Windows.Input.MouseEventArgs;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;

namespace Eitmad.WindowsShell.Controls;

/// <summary>A presentation-only modal surface. The owner decides whether a close request changes state.</summary>
[TemplatePart(Name = "PART_Close", Type = typeof(Button))]
public sealed class DialogHost : ContentControl
{
    private static readonly ConditionalWeakTable<Window, List<DialogHost>> ActiveDialogs = new();
    private Window? owner;
    private IInputElement? returnFocus;
    private Button? closeButton;
    public static readonly DependencyProperty IsOpenProperty = DependencyProperty.Register(nameof(IsOpen), typeof(bool), typeof(DialogHost), new PropertyMetadata(false, OnOpenChanged));
    public static readonly DependencyProperty TitleProperty = DependencyProperty.Register(nameof(Title), typeof(string), typeof(DialogHost), new PropertyMetadata(string.Empty));
    public static readonly DependencyProperty FooterProperty = DependencyProperty.Register(nameof(Footer), typeof(object), typeof(DialogHost));
    public static readonly DependencyProperty PreferredWidthProperty = DependencyProperty.Register(nameof(PreferredWidth), typeof(double), typeof(DialogHost), new PropertyMetadata(640d, OnSizeOptionChanged), value => value is double number && double.IsFinite(number) && number > 0);
    public static readonly DependencyProperty InitialFocusTargetProperty = DependencyProperty.Register(nameof(InitialFocusTarget), typeof(IInputElement), typeof(DialogHost));
    public static readonly DependencyProperty CloseCommandProperty = DependencyProperty.Register(nameof(CloseCommand), typeof(ICommand), typeof(DialogHost));
    public static readonly DependencyProperty CloseCommandParameterProperty = DependencyProperty.Register(nameof(CloseCommandParameter), typeof(object), typeof(DialogHost));
    private static readonly DependencyPropertyKey DialogWidthPropertyKey = DependencyProperty.RegisterReadOnly(nameof(DialogWidth), typeof(double), typeof(DialogHost), new PropertyMetadata(640d));
    public static readonly DependencyProperty DialogWidthProperty = DialogWidthPropertyKey.DependencyProperty;
    private static readonly DependencyPropertyKey DialogMaxHeightPropertyKey = DependencyProperty.RegisterReadOnly(nameof(DialogMaxHeight), typeof(double), typeof(DialogHost), new PropertyMetadata(600d));
    public static readonly DependencyProperty DialogMaxHeightProperty = DialogMaxHeightPropertyKey.DependencyProperty;
    public static readonly RoutedEvent CloseRequestedEvent = EventManager.RegisterRoutedEvent(nameof(CloseRequested), RoutingStrategy.Bubble, typeof(RoutedEventHandler), typeof(DialogHost));

    public bool IsOpen { get => (bool)GetValue(IsOpenProperty); set => SetValue(IsOpenProperty, value); }
    public string Title { get => (string)GetValue(TitleProperty); set => SetValue(TitleProperty, value); }
    public object? Footer { get => GetValue(FooterProperty); set => SetValue(FooterProperty, value); }
    public double PreferredWidth { get => (double)GetValue(PreferredWidthProperty); set => SetValue(PreferredWidthProperty, value); }
    public IInputElement? InitialFocusTarget { get => (IInputElement?)GetValue(InitialFocusTargetProperty); set => SetValue(InitialFocusTargetProperty, value); }
    public ICommand? CloseCommand { get => (ICommand?)GetValue(CloseCommandProperty); set => SetValue(CloseCommandProperty, value); }
    public object? CloseCommandParameter { get => GetValue(CloseCommandParameterProperty); set => SetValue(CloseCommandParameterProperty, value); }
    public double DialogWidth => (double)GetValue(DialogWidthProperty);
    public double DialogMaxHeight => (double)GetValue(DialogMaxHeightProperty);
    public event RoutedEventHandler CloseRequested { add => AddHandler(CloseRequestedEvent, value); remove => RemoveHandler(CloseRequestedEvent, value); }

    public DialogHost()
    {
        Loaded += (_, _) => UpdateActivation();
        Unloaded += (_, _) => Deactivate();
        IsVisibleChanged += (_, _) => UpdateActivation();
        SizeChanged += (_, _) => UpdateViewport();
        KeyboardNavigation.SetTabNavigation(this, KeyboardNavigationMode.Cycle);
        KeyboardNavigation.SetControlTabNavigation(this, KeyboardNavigationMode.Cycle);
    }

    public override void OnApplyTemplate()
    {
        if (closeButton is not null) closeButton.Click -= OnCloseClick;
        base.OnApplyTemplate();
        closeButton = GetTemplateChild("PART_Close") as Button;
        if (closeButton is not null) closeButton.Click += OnCloseClick;
    }

    public void RequestClose()
    {
        var args = new RoutedEventArgs(CloseRequestedEvent, this);
        RaiseEvent(args);
        if (!args.Handled && CloseCommand?.CanExecute(CloseCommandParameter) == true)
            CloseCommand.Execute(CloseCommandParameter);
    }

    private void OnCloseClick(object sender, RoutedEventArgs e) => RequestClose();
    private static void OnOpenChanged(DependencyObject source, DependencyPropertyChangedEventArgs e) => ((DialogHost)source).UpdateActivation();
    private static void OnSizeOptionChanged(DependencyObject source, DependencyPropertyChangedEventArgs e) => ((DialogHost)source).UpdateViewport();
    private void UpdateViewport()
    {
        SetValue(DialogWidthPropertyKey, Math.Min(PreferredWidth, Math.Max(0, ActualWidth - 48)));
        SetValue(DialogMaxHeightPropertyKey, Math.Max(0, ActualHeight - 48));
    }

    private bool IsActive => owner is not null && ActiveDialogs.GetOrCreateValue(owner).LastOrDefault() == this;
    private void UpdateActivation()
    {
        if (!IsOpen || !IsVisible || !IsLoaded) { Deactivate(); return; }
        if (owner is not null) return;
        owner = Window.GetWindow(this);
        if (owner is null) return;
        returnFocus = Keyboard.FocusedElement;
        ActiveDialogs.GetOrCreateValue(owner).Add(this);
        owner.PreviewGotKeyboardFocus += KeepFocusInside;
        owner.PreviewKeyDown += BlockBackgroundKeys;
        owner.PreviewMouseDown += BlockBackgroundPointer;
        owner.PreviewMouseWheel += BlockBackgroundPointer;
        owner.KeyDown += HandleEscape;
        owner.AddHandler(AccessKeyManager.AccessKeyPressedEvent, new AccessKeyPressedEventHandler(ScopeAccessKeys));
        Dispatcher.BeginInvoke(DispatcherPriority.Input, new Action(FocusInitial));
    }

    private void Deactivate()
    {
        if (owner is null) return;
        var previousOwner = owner;
        var wasActive = IsActive;
        owner.PreviewGotKeyboardFocus -= KeepFocusInside;
        owner.PreviewKeyDown -= BlockBackgroundKeys;
        owner.PreviewMouseDown -= BlockBackgroundPointer;
        owner.PreviewMouseWheel -= BlockBackgroundPointer;
        owner.KeyDown -= HandleEscape;
        owner.RemoveHandler(AccessKeyManager.AccessKeyPressedEvent, new AccessKeyPressedEventHandler(ScopeAccessKeys));
        ActiveDialogs.GetOrCreateValue(owner).Remove(this);
        owner = null;
        var target = returnFocus;
        returnFocus = null;
        if (!wasActive) return;
        Dispatcher.BeginInvoke(DispatcherPriority.Input, new Action(() =>
        {
            var active = ActiveDialogs.GetOrCreateValue(previousOwner).LastOrDefault();
            if (target is UIElement element && element.IsVisible && element.IsEnabled && element.Focusable && (active is null || active.Contains(element)))
                Keyboard.Focus(element);
            else if (active is not null) active.FocusInitial();
            else previousOwner.MoveFocus(new TraversalRequest(FocusNavigationDirection.First));
        }));
    }

    private void FocusInitial()
    {
        if (!IsActive) return;
        if (InitialFocusTarget is UIElement target && target.IsVisible && target.IsEnabled && Contains(target) && target.Focus()) return;
        MoveFocus(new TraversalRequest(FocusNavigationDirection.First));
    }

    private bool Contains(DependencyObject? target)
    {
        while (target is not null)
        {
            if (ReferenceEquals(target, this)) return true;
            target = target is Visual || target is System.Windows.Media.Media3D.Visual3D
                ? VisualTreeHelper.GetParent(target) ?? LogicalTreeHelper.GetParent(target)
                : LogicalTreeHelper.GetParent(target);
        }
        return false;
    }

    private bool ContainsIncludingPopup(DependencyObject? target)
    {
        if (target is null) return false;
        if (Contains(target)) return true;
        return Descendants<Popup>(this).Any(popup => popup.IsOpen && popup.Child is not null && (ReferenceEquals(target, popup.Child) || popup.Child.IsAncestorOf(target)));
    }

    private void KeepFocusInside(object sender, KeyboardFocusChangedEventArgs e)
    {
        if (!IsActive || ContainsIncludingPopup(e.NewFocus as DependencyObject)) return;
        e.Handled = true;
        Dispatcher.BeginInvoke(DispatcherPriority.Input, new Action(FocusInitial));
    }

    private void ScopeAccessKeys(object sender, AccessKeyPressedEventArgs e)
    {
        if (!IsActive) return;
        e.Scope = this;
        if (!Contains(e.Target as DependencyObject)) e.Target = null;
        e.Handled = true;
    }

    private void BlockBackgroundKeys(object sender, KeyEventArgs e)
    {
        if (IsActive && !ContainsIncludingPopup(e.OriginalSource as DependencyObject)) e.Handled = true;
    }

    private void BlockBackgroundPointer(object sender, MouseEventArgs e)
    {
        if (IsActive && !ContainsIncludingPopup(e.OriginalSource as DependencyObject)) e.Handled = true;
    }

    private void HandleEscape(object sender, KeyEventArgs e)
    {
        if (!IsActive || e.Key != Key.Escape) return;
        // ComboBox handles its own Escape first. This also covers custom popup footer focus.
        var dropdown = Descendants<ComboBox>(this).FirstOrDefault(combo => combo.IsDropDownOpen);
        if (dropdown is not null) dropdown.SetCurrentValue(ComboBox.IsDropDownOpenProperty, false);
        else RequestClose();
        e.Handled = true;
    }

    private static IEnumerable<T> Descendants<T>(DependencyObject root) where T : DependencyObject
    {
        if (root is T match) yield return match;
        for (var index = 0; index < VisualTreeHelper.GetChildrenCount(root); index++)
            foreach (var child in Descendants<T>(VisualTreeHelper.GetChild(root, index))) yield return child;
    }
}


