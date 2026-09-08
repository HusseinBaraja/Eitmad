using System.Collections;
using System.Collections.ObjectModel;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Globalization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using System.Windows.Controls.Primitives;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;
using KeyEventHandler = System.Windows.Input.KeyEventHandler;
using ButtonBase = System.Windows.Controls.Primitives.ButtonBase;
using TextBoxBase = System.Windows.Controls.Primitives.TextBoxBase;
using ComboBox = System.Windows.Controls.ComboBox;

namespace Eitmad.WindowsShell.Controls;

/// <summary>A native, read-only row table with an independent presentation view.</summary>
public class OperationsTable : DataGrid
{
    public static readonly DependencyProperty EmptyContentProperty = DependencyProperty.Register(
        nameof(EmptyContent), typeof(object), typeof(OperationsTable));
    public static readonly DependencyProperty IsRowInvocationEnabledProperty = DependencyProperty.Register(
        nameof(IsRowInvocationEnabled), typeof(bool), typeof(OperationsTable), new PropertyMetadata(false));
    public static readonly RoutedEvent RowInvokedEvent = EventManager.RegisterRoutedEvent(
        nameof(RowInvoked), RoutingStrategy.Bubble, typeof(EventHandler<RowInvokedEventArgs>), typeof(OperationsTable));

    private IEnumerable? source;
    private INotifyCollectionChanged? observableSource;
    private readonly ObservableCollection<object> rows = [];
    private readonly ListCollectionView view;
    private readonly HashSet<INotifyPropertyChanged> observedRows = [];
    private bool refreshPending;
    private object? pendingSelection;
    private bool selectionRestorePending;

    static OperationsTable()
    {
        ItemsSourceProperty.OverrideMetadata(typeof(OperationsTable), new FrameworkPropertyMetadata(null, null, CoerceSource));
    }

    public OperationsTable()
    {
        view = new ListCollectionView(rows) { Culture = CultureInfo.GetCultureInfo("ar-YE") };
        AutoGenerateColumns = false;
        CanUserAddRows = false;
        CanUserDeleteRows = false;
        CanUserReorderColumns = false;
        CanUserResizeColumns = true;
        CanUserSortColumns = true;
        IsReadOnly = true;
        SelectionMode = DataGridSelectionMode.Single;
        SelectionUnit = DataGridSelectionUnit.FullRow;
        EnableRowVirtualization = true;
        EnableColumnVirtualization = true;
        AddHandler(LostKeyboardFocusEvent, new KeyboardFocusChangedEventHandler(OnEditorLostFocus), true);
        AddHandler(Mouse.PreviewMouseUpEvent, new MouseButtonEventHandler(OnPreviewMouseUp), true);
        AddHandler(Keyboard.PreviewKeyDownEvent, new KeyEventHandler(OnPreviewKeyDown), true);
        Loaded += OnLoaded;
        Unloaded += OnUnloaded;
    }

    public object? EmptyContent { get => GetValue(EmptyContentProperty); set => SetValue(EmptyContentProperty, value); }
    public bool IsRowInvocationEnabled { get => (bool)GetValue(IsRowInvocationEnabledProperty); set => SetValue(IsRowInvocationEnabledProperty, value); }
    public event EventHandler<RowInvokedEventArgs> RowInvoked { add => AddHandler(RowInvokedEvent, value); remove => RemoveHandler(RowInvokedEvent, value); }

    private void OnPreviewMouseUp(object sender, MouseButtonEventArgs eventArgs)
    {
        if (!IsRowInvocationEnabled || eventArgs.ChangedButton != MouseButton.Left ||
            eventArgs.OriginalSource is not DependencyObject source ||
            FindAncestor<ButtonBase>(source) is not null ||
            FindAncestor<DataGridRow>(source) is not { DataContext: { } item })
        {
            return;
        }

        InvokeRow(item, eventArgs);
    }

    private void OnPreviewKeyDown(object sender, KeyEventArgs eventArgs)
    {
        if (!IsRowInvocationEnabled || eventArgs.Key is not (Key.Enter or Key.Space) ||
            Keyboard.FocusedElement is ButtonBase or TextBoxBase or ComboBox || SelectedItem is null)
        {
            return;
        }

        InvokeRow(SelectedItem, eventArgs);
    }

    private void InvokeRow(object item, RoutedEventArgs inputEvent)
    {
        RaiseEvent(new RowInvokedEventArgs(RowInvokedEvent, this, item));
        inputEvent.Handled = true;
    }

    private static T? FindAncestor<T>(DependencyObject source) where T : DependencyObject
    {
        for (var current = source; current is not null; current = VisualTreeHelper.GetParent(current))
        {
            if (current is T match) return match;
        }

        return null;
    }

    private static object? CoerceSource(DependencyObject owner, object? value)
    {
        var table = (OperationsTable)owner;
        if (ReferenceEquals(value, table.view)) return value;
        table.SetSource(value as IEnumerable);
        return table.view;
    }

    private void SetSource(IEnumerable? value)
    {
        if (observableSource is not null) CollectionChangedEventManager.RemoveHandler(observableSource, OnSourceChanged);
        source = value;
        observableSource = value as INotifyCollectionChanged;
        if (observableSource is not null) CollectionChangedEventManager.AddHandler(observableSource, OnSourceChanged);
        ReplaceRows();
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        if (observableSource is not null)
        {
            CollectionChangedEventManager.RemoveHandler(observableSource, OnSourceChanged);
            CollectionChangedEventManager.AddHandler(observableSource, OnSourceChanged);
        }
        ReplaceRows();
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        if (observableSource is not null) CollectionChangedEventManager.RemoveHandler(observableSource, OnSourceChanged);
        foreach (var row in observedRows) PropertyChangedEventManager.RemoveHandler(row, OnRowChanged, string.Empty);
        observedRows.Clear();
    }

    private void OnSourceChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        if (e.Action == NotifyCollectionChangedAction.Reset)
        {
            pendingSelection = SelectedItem ?? pendingSelection;
            ReplaceRows();
            RestoreSelectionAfterBatch();
            return;
        }
        var selected = SelectedItem ?? pendingSelection;
        {
            if (e.Action == NotifyCollectionChangedAction.Move)
            {
                rows.Move(e.OldStartingIndex, e.NewStartingIndex);
            }
            else
            {
                if (e.OldItems is not null)
                    for (var index = 0; index < e.OldItems.Count; index++) rows.RemoveAt(e.OldStartingIndex);
                if (e.NewItems is not null)
                    for (var index = 0; index < e.NewItems.Count; index++) rows.Insert(e.NewStartingIndex + index, e.NewItems[index]!);
            }
        }
        RestoreSelection(selected);
        ObserveRows();
    }

    private void ReplaceRows()
    {
        var selected = SelectedItem;
        var current = source?.Cast<object>().ToArray() ?? [];

        rows.Clear();
        foreach (var row in current) rows.Add(row);
        RestoreSelection(selected);
        ObserveRows();
    }

    // Feature filters rebuild their visible collections with Clear followed by Add.
    // Retain identity during that dispatcher turn, then discard it if filtered out.
    private void RestoreSelectionAfterBatch()
    {
        if (selectionRestorePending) return;
        selectionRestorePending = true;
        Dispatcher.BeginInvoke(DispatcherPriority.DataBind, new Action(() =>
        {
            selectionRestorePending = false;
            if (SelectedItem is null) RestoreSelection(pendingSelection);
            pendingSelection = null;
        }));
    }

    private void RestoreSelection(object? selected)
    {
        if (selected is null) return;
        if (view.Contains(selected)) SetCurrentValue(SelectedItemProperty, selected);
        else if (!string.IsNullOrEmpty(SelectedValuePath))
        {
            var key = ReadProperty(selected, SelectedValuePath);
            var replacement = rows.FirstOrDefault(row => key is not null && Equals(ReadProperty(row, SelectedValuePath), key));
            if (replacement is not null) SetCurrentValue(SelectedItemProperty, replacement);
        }
    }

    private static object? ReadProperty(object? item, string path)
    {
        foreach (var segment in path.Split('.'))
            item = item is null ? null : TypeDescriptor.GetProperties(item)[segment]?.GetValue(item);
        return item;
    }

    private void ObserveRows()
    {
        var current = rows.OfType<INotifyPropertyChanged>().ToHashSet();
        foreach (var row in observedRows.Except(current).ToArray())
        {
            PropertyChangedEventManager.RemoveHandler(row, OnRowChanged, string.Empty);
            observedRows.Remove(row);
        }
        foreach (var row in current.Except(observedRows))
        {
            PropertyChangedEventManager.AddHandler(row, OnRowChanged, string.Empty);
            observedRows.Add(row);
        }
    }

    private void OnRowChanged(object? sender, PropertyChangedEventArgs e)
    {
        // A record can also be edited in a separate dialog. Its completed change
        // must refresh this view; text entry inside this table waits for focus loss.
        if (!IsEditingInput()) QueueSortRefresh();
    }

    protected override void OnSorting(DataGridSortingEventArgs eventArgs)
    {
        eventArgs.Handled = true;
        if (!eventArgs.Column.CanUserSort || string.IsNullOrWhiteSpace(eventArgs.Column.SortMemberPath)) return;
        var direction = eventArgs.Column.SortDirection == ListSortDirection.Ascending
            ? ListSortDirection.Descending : ListSortDirection.Ascending;
        var selected = SelectedItem;
        foreach (var column in Columns) column.SortDirection = null;
        eventArgs.Column.SortDirection = direction;
        view.CustomSort = new RowComparer(eventArgs.Column.SortMemberPath, direction);
        RestoreSelection(selected);
    }

    // Explicit TextBox/ComboBox controls in CellTemplate remain editable. Refresh after
    // focus leaves the input, once its normal LostFocus binding has updated the source.
    private void OnEditorLostFocus(object sender, KeyboardFocusChangedEventArgs e)
        => QueueSortRefresh();

    private void QueueSortRefresh()
    {
        if (view.CustomSort is null || refreshPending) return;
        refreshPending = true;
        Dispatcher.BeginInvoke(DispatcherPriority.ContextIdle, new Action(() =>
        {
            refreshPending = false;
            if (!IsLoaded || IsEditingInput()) return;
            var selected = SelectedItem;
            view.Refresh();
            RestoreSelection(selected);
        }));
    }

    private bool IsEditingInput()
    {
        if (!IsKeyboardFocusWithin) return false;
        return Keyboard.FocusedElement is System.Windows.Controls.Primitives.TextBoxBase
            or System.Windows.Controls.ComboBox;
    }

    private sealed class RowComparer(string path, ListSortDirection direction) : IComparer
    {
        private static readonly CompareInfo Arabic = CultureInfo.GetCultureInfo("ar-YE").CompareInfo;
        public int Compare(object? x, object? y)
        {
            var left = Read(x);
            var right = Read(y);
            var result = left is string leftText && right is string rightText
                ? Arabic.Compare(leftText, rightText, CompareOptions.None)
                : Comparer.DefaultInvariant.Compare(left, right);
            return direction == ListSortDirection.Ascending ? result : -Math.Sign(result);
        }

        private object? Read(object? item) => ReadProperty(item, path);
    }
}

public sealed class RowInvokedEventArgs(RoutedEvent routedEvent, object source, object item) : RoutedEventArgs(routedEvent, source)
{
    public object Item { get; } = item;
}
