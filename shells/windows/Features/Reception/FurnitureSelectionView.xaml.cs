using System.Windows;
using System.Windows.Media;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class FurnitureSelectionView : UserControl
{
    public FurnitureSelectionView() => InitializeComponent();
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
    private void AddClick(object sender, RoutedEventArgs e)
    {
        var editing = ((FurnitureSelectionViewModel)DataContext).IsEditing;
        if (!((SalesCatalogViewModel)Catalog.DataContext).AddSelection()) return;
        if (editing) { Catalog.RestoreQuotationFocus(); return; }
        var selection = (FurnitureSelectionViewModel)DataContext;
        AddedNotice.Message = $"تمت الإضافة إلى عرض السعر (معاينة فقط)\n{selection.Item.Name} · {selection.SelectedSize!.Name} · الكمية: {selection.Quantity}";
        AddedNotice.RestartDuration();
    }
}
