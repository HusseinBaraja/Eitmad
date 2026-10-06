using System.Windows;
using System.Windows.Media;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class ProductSelectionView : UserControl
{
    public ProductSelectionView()
    {
        InitializeComponent();
        DataContextChanged += (_, _) => AddedNotice.Message = string.Empty;
    }
    private SalesCatalogView Catalog
    {
        get { DependencyObject parent = this; while (parent is not SalesCatalogView) parent = VisualTreeHelper.GetParent(parent); return (SalesCatalogView)parent; }
    }
    private void BackClick(object sender, RoutedEventArgs e)
    {
        var catalog = Catalog;
        ((SalesCatalogViewModel)catalog.DataContext).CloseSelection();
        catalog.RestoreSelectionFocus();
    }
    /// <summary>Rechecks the configuration through Rust before adding the unsaved line and restoring catalog or review focus.</summary>
    private async void AddClick(object sender, RoutedEventArgs e)
    {
        var editing = ((ProductSelectionViewModel)DataContext).IsEditing;
        if (!await ((SalesCatalogViewModel)Catalog.DataContext).AddValidatedSelectionAsync(true)) return;
        if (editing) { Catalog.RestoreQuotationFocus(); return; }
        var selection = (ProductSelectionViewModel)DataContext;
        AddedNotice.Message = $"تمت الإضافة إلى عرض السعر (معاينة فقط)\n{selection.Item.Name} · {selection.SelectedVariant?.Name ?? string.Empty} · الكمية: {selection.Quantity}";
        AddedNotice.RestartDuration();
    }
    /// <summary>Reloads current public choices and restores keyboard focus to the selection editor.</summary>
    private async void RefreshClick(object sender, RoutedEventArgs e) { await ((SalesCatalogViewModel)Catalog.DataContext).RefreshSelectionAsync(); Catalog.FocusEditor(); }
}
