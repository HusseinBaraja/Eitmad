using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Data;
using Eitmad.WindowsShell.Controls;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class OperationsTableRenderedTests
{
    [TestMethod]
    [DataRow(1338d, 753d)]
    [DataRow(780d, 745d)]
    public void MigratedTablesUseExistingTypedPropertiesAtBothWidths(double width, double height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            void Check(string name)
            {
                WpfTestHost.CompleteLayout(window);
                var tables = WpfTestHost.Descendants<OperationsTable>(window).Where(table => table.IsVisible).ToArray();
                Assert.IsGreaterThan(0, tables.Length, name);
                foreach (var badge in WpfTestHost.Descendants<StatusBadge>(window).Where(badge => badge.IsVisible))
                    Assert.IsFalse(string.IsNullOrWhiteSpace(badge.Text), name + " status text");
                foreach (var table in tables)
                {
                    Assert.IsTrue(table.ActualWidth <= window.ActualWidth, name + " bounded table width");
                    Assert.IsFalse(table.CanUserAddRows);
                    Assert.IsFalse(table.CanUserDeleteRows);
                    Assert.IsTrue(table.IsReadOnly);
                    if (table.Items.Count > 0)
                        foreach (var column in table.Columns.Where(column => column.CanUserSort))
                        {
                            object? value = table.Items[0];
                            foreach (var part in column.SortMemberPath.Split('.'))
                            {
                                Assert.IsNotNull(value, name + " " + column.SortMemberPath);
                                var property = System.ComponentModel.TypeDescriptor.GetProperties(value)[part];
                                Assert.IsNotNull(property, name + " " + column.SortMemberPath);
                                value = property.GetValue(value);
                            }
                        }
                    table.BringIntoView();
                }
                WpfTestHost.CompleteLayout(window);
                var directory = Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR");
                if (!string.IsNullOrEmpty(directory))
                {
                    System.IO.Directory.CreateDirectory(directory);
                    var bitmap = new System.Windows.Media.Imaging.RenderTargetBitmap((int)window.ActualWidth, (int)window.ActualHeight, 96, 96, System.Windows.Media.PixelFormats.Pbgra32);
                    bitmap.Render(window);
                    var encoder = new System.Windows.Media.Imaging.PngBitmapEncoder();
                    encoder.Frames.Add(System.Windows.Media.Imaging.BitmapFrame.Create(bitmap));
                    using var stream = System.IO.File.Create(System.IO.Path.Combine(directory, $"table-{name}-{width}.png"));
                    encoder.Save(stream);
                }
            }
            Check("dashboard");
            foreach (var page in new[] { "Materials", "Parts", "Furniture", "Pricing", "Products", "Quotations", "Orders", "WorkOrders" })
            {
                WpfTestHost.FindByName<Button>(window, page + "NavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Check(page);
            }
            WpfTestHost.FindByName<Button>(window, "WorkOrdersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var workOrders = WpfTestHost.Descendants<Eitmad.WindowsShell.Features.WorkOrders.WorkOrdersView>(window).Single().ViewModel;
            workOrders.OpenWorkOrder(workOrders.VisibleWorkOrders[0]);
            Check("work-order-parts");
            WpfTestHost.FindByName<Button>(window, "OrdersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var orders = WpfTestHost.Descendants<Eitmad.WindowsShell.Features.Orders.OrdersView>(window).Single().ViewModel;
            orders.OpenOrder(orders.VisibleOrders[0]);
            Check("order-items");
            WpfTestHost.FindByName<Button>(window, "QuotationsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var quotations = WpfTestHost.Descendants<Eitmad.WindowsShell.Features.Quotations.QuotationsView>(window).Single().ViewModel;
            quotations.OpenQuotation(quotations.VisibleQuotations[0]);
            Check("quotation-items");
            WpfTestHost.FindByName<Button>(window, "ProductsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var products = WpfTestHost.Descendants<Eitmad.WindowsShell.Features.Products.ProductsView>(window).Single().ViewModel;
            products.BeginEdit(products.VisibleProducts[0]);
            Check("product-variants");
            WpfTestHost.FindByName<Button>(window, "PartsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var parts = WpfTestHost.Descendants<Eitmad.WindowsShell.Features.Parts.PartsView>(window).Single().ViewModel;
            parts.BeginEdit(parts.VisibleParts[0]);
            Assert.IsTrue(parts.MoveToMaterials());
            parts.OpenMaterialPicker();
            parts.AddMaterial(parts.FilteredMaterials[0]);
            Check("part-materials");
            Assert.IsTrue(parts.MoveToReview());
            Check("part-review");
            WpfTestHost.FindByName<Button>(window, "FurnitureNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var furniture = WpfTestHost.Descendants<Eitmad.WindowsShell.Features.Furniture.FurnitureView>(window).Single().ViewModel;
            furniture.BeginEdit(furniture.VisibleFurniture[0]);
            Assert.IsTrue(furniture.MoveToParts());
            Check("furniture-parts");
            Assert.IsTrue(furniture.MoveToVariants());
            Assert.IsTrue(furniture.MoveToOptions());
            Check("furniture-colors");
        });
    }

    [TestMethod]
    public void RealListRetainsSelectionThroughFilterRebuildAndUsesTypedPriceSorting()
    {
        WpfTestHost.Run(900, 700, window =>
        {
            var page = new Eitmad.WindowsShell.Features.Pricing.PricingView();
            window.Content = page;
            WpfTestHost.CompleteLayout(window);
            var table = WpfTestHost.FindByName<OperationsTable>(page, "PricingRows");
            var record = page.ViewModel.VisiblePrices[0];
            table.SelectedItem = record;
            page.ViewModel.SearchText = record.Product;
            WpfTestHost.CompleteLayout(window);
            Assert.AreSame(record, table.SelectedItem);
            Assert.IsFalse(page.ViewModel.IsEditorOpen);
            page.ViewModel.SearchText = "";
            WpfTestHost.CompleteLayout(window);
            Assert.AreSame(record, table.SelectedItem);
            var column = table.Columns.Single(item => item.SortMemberPath == "SellingPrice");
            var header = WpfTestHost.Descendants<DataGridColumnHeader>(table).Single(item => item.Column == column);
            Assert.IsTrue(table.CanUserSortColumns);
            Assert.IsTrue(column.CanUserSort);
            header.Focus();
            header.RaiseEvent(new System.Windows.Input.KeyEventArgs(System.Windows.Input.Keyboard.PrimaryDevice, PresentationSource.FromVisual(header), 0, System.Windows.Input.Key.Space) { RoutedEvent = System.Windows.Input.Keyboard.KeyDownEvent });
            header.RaiseEvent(new System.Windows.Input.KeyEventArgs(System.Windows.Input.Keyboard.PrimaryDevice, PresentationSource.FromVisual(header), 0, System.Windows.Input.Key.Space) { RoutedEvent = System.Windows.Input.Keyboard.KeyUpEvent });
            WpfTestHost.CompleteLayout(window);
            var values = table.Items.Cast<Eitmad.WindowsShell.Features.Pricing.PricingListItem>().Select(item => item.SellingPrice).ToArray();
            CollectionAssert.AreEqual(values.Order().ToArray(), values);
            Assert.AreSame(record, table.SelectedItem);
            page.ViewModel.SearchText = "لا توجد نتيجة تجريبية";
            WpfTestHost.CompleteLayout(window);
            Assert.IsNull(table.SelectedItem);
            var empty = WpfTestHost.Descendants<EmptyState>(table).Single();
            Assert.IsTrue(empty.IsVisible);
            Assert.AreEqual("لا توجد أسعار مطابقة", empty.Heading);
            page.ViewModel.SearchText = "";
            WpfTestHost.CompleteLayout(window);
            Assert.IsNull(table.SelectedItem, "A later filter must not revive an old selection.");
        });
    }

    [TestMethod]
    public void SortingUsesTypedValuesAndArabicAndDoesNotChangeOtherViews()
    {
        WpfTestHost.Run(900, 700, window =>
        {
            var rows = new ObservableCollection<Row>
            {
                new("ياسر", 12, new DateTime(2026, 1, 1)),
                new("أحمد", 2, new DateTime(2025, 1, 1)),
                new("سالم", 100, new DateTime(2024, 1, 1)),
            };
            var first = new SortableTable { ItemsSource = rows };
            var second = new SortableTable { ItemsSource = rows };
            window.Content = new StackPanel { Children = { first, second } };
            WpfTestHost.CompleteLayout(window);
            first.SelectedItem = rows[0];
            first.Sort(nameof(Row.Amount));
            CollectionAssert.AreEqual(new[] { 2, 12, 100 }, first.Items.Cast<Row>().Select(row => row.Amount).ToArray());
            Assert.AreSame(rows[0], first.SelectedItem);
            CollectionAssert.AreEqual(rows.ToArray(), second.Items.Cast<Row>().ToArray());
            first.Sort(nameof(Row.Date));
            CollectionAssert.AreEqual(new[] { 100, 2, 12 }, first.Items.Cast<Row>().Select(row => row.Amount).ToArray());
            first.Sort(nameof(Row.Name));
            var expected = rows.OrderBy(row => row.Name, StringComparer.Create(CultureInfo.GetCultureInfo("ar-YE"), false)).ToArray();
            CollectionAssert.AreEqual(expected, first.Items.Cast<Row>().ToArray());
            first.Sort(nameof(Row.Name));
            CollectionAssert.AreEqual(expected.Reverse().ToArray(), first.Items.Cast<Row>().ToArray());
            Assert.AreEqual(12, rows[0].Amount);
        });
    }

    [TestMethod]
    public void SelectionSurvivesUpdatesAndClearsWhenSourceFilterRemovesRecord()
    {
        WpfTestHost.Run(900, 700, window =>
        {
            var rows = new ObservableCollection<Row> { new("أحمد", 2, DateTime.Today), new("سالم", 3, DateTime.Today) };
            var source = new ListCollectionView(rows);
            var table = new SortableTable { ItemsSource = source };
            window.Content = table;
            WpfTestHost.CompleteLayout(window);
            table.SelectedItem = rows[0];
            rows.Add(new("ياسر", 5, DateTime.Today));
            Assert.AreSame(rows[0], table.SelectedItem);
            source.Filter = item => ((Row)item).Amount > 2;
            Assert.IsNull(table.SelectedItem);
            Assert.AreEqual(2, table.Items.Count);
        });
    }

    [TestMethod]
    public void ExplicitEditorBindingRemainsWritableAndSortRefreshesAfterFocusLeaves()
    {
        WpfTestHost.Run(900, 700, window =>
        {
            var rows = new ObservableCollection<Row> { new("سالم", 2, DateTime.Today), new("أحمد", 3, DateTime.Today) };
            var table = new SortableTable { ItemsSource = rows };
            var editor = new FrameworkElementFactory(typeof(TextBox));
            editor.SetBinding(TextBox.TextProperty, new Binding(nameof(Row.Name)) { Mode = BindingMode.TwoWay, UpdateSourceTrigger = UpdateSourceTrigger.LostFocus });
            table.Columns.Add(new DataGridTemplateColumn { Header = "الاسم", SortMemberPath = nameof(Row.Name), CellTemplate = new DataTemplate { VisualTree = editor } });
            var outside = new Button { Content = "خارج الجدول" };
            window.Content = new StackPanel { Children = { table, outside } };
            WpfTestHost.CompleteLayout(window);
            table.Sort(nameof(Row.Name));
            WpfTestHost.CompleteLayout(window);
            var text = WpfTestHost.Descendants<TextBox>(table).Single(box => ReferenceEquals(box.DataContext, rows[1]));
            text.Focus();
            text.Text = "ياسر";
            Assert.AreSame(rows[1], table.Items[0]);
            Assert.AreEqual("أحمد", rows[1].Name);
            var nextInput = WpfTestHost.Descendants<TextBox>(table).Single(box => ReferenceEquals(box.DataContext, rows[0]));
            nextInput.Focus();
            WpfTestHost.CompleteLayout(window);
            Assert.AreSame(rows[1], table.Items[0], "Moving to another input must not reorder active text entry.");
            outside.Focus();
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("ياسر", rows[1].Name);
            Assert.AreSame(rows[0], table.Items[0]);
            Assert.IsFalse(table.CanUserAddRows);
            Assert.IsFalse(table.CanUserDeleteRows);
            Assert.IsFalse(table.CanUserReorderColumns);
        });
    }

    [TestMethod]
    public void PointerInsideEditorDoesNotInvokeRow()
    {
        WpfTestHost.Run(900, 700, window =>
        {
            var row = new Row("أحمد", 2, DateTime.Today);
            var table = new OperationsTable { ItemsSource = new[] { row }, IsReadOnly = false, IsRowInvocationEnabled = true };
            var editor = new FrameworkElementFactory(typeof(TextBox));
            editor.SetBinding(TextBox.TextProperty, new Binding(nameof(Row.Name)) { Mode = BindingMode.TwoWay });
            table.Columns.Add(new DataGridTemplateColumn { CellTemplate = new DataTemplate { VisualTree = editor } });
            var invocations = 0;
            table.RowInvoked += (_, _) => invocations++;
            window.Content = table;
            WpfTestHost.CompleteLayout(window);
            var textBox = WpfTestHost.Descendants<TextBox>(table).Single();
            textBox.RaiseEvent(new System.Windows.Input.MouseButtonEventArgs(System.Windows.Input.Mouse.PrimaryDevice, 0, System.Windows.Input.MouseButton.Left)
            {
                RoutedEvent = System.Windows.Input.Mouse.PreviewMouseUpEvent,
            });
            Assert.AreEqual(0, invocations);
        });
    }

    [TestMethod]
    public void CompletedEditOutsideTableRefreshesSortAndPreservesSelection()
    {
        WpfTestHost.Run(900, 700, window =>
        {
            var rows = new ObservableCollection<Row> { new("أحمد", 2, DateTime.Today), new("سالم", 3, DateTime.Today) };
            var table = new SortableTable { ItemsSource = rows };
            var outside = new Button { Content = "حفظ" };
            window.Content = new StackPanel { Children = { table, outside } };
            WpfTestHost.CompleteLayout(window);
            table.Sort(nameof(Row.Name));
            table.SelectedItem = rows[0];
            outside.Focus();
            rows[0].Name = "ياسر";
            WpfTestHost.CompleteLayout(window);
            Assert.AreSame(rows[1], table.Items[0]);
            Assert.AreSame(rows[0], table.SelectedItem);
        });
    }

    [TestMethod]
    public void ResizedColumnsStayAlignedAndRowMenuKeepsItsRecordAfterSorting()
    {
        WpfTestHost.Run(900, 700, window =>
        {
            var rows = new ObservableCollection<Row> { new("سالم", 12, DateTime.Today), new("أحمد", 2, DateTime.Today) };
            var table = new SortableTable { ItemsSource = rows, Height = 250 };
            var nameColumn = new DataGridTextColumn { Header = "الاسم", Binding = new Binding(nameof(Row.Name)), SortMemberPath = nameof(Row.Name), Width = 180 };
            var button = new FrameworkElementFactory(typeof(Button));
            button.SetValue(ContentControl.ContentProperty, "إجراءات");
            button.AddHandler(Button.ClickEvent, new RoutedEventHandler((sender, _) =>
            {
                var target = (Button)sender;
                var menu = new ContextMenu { PlacementTarget = target };
                menu.SetBinding(FrameworkElement.DataContextProperty, new Binding("PlacementTarget.DataContext") { RelativeSource = RelativeSource.Self });
                target.ContextMenu = menu;
                menu.Items.Add(new MenuItem { Header = "تعديل" });
                menu.IsOpen = true;
            }));
            table.Columns.Add(nameColumn);
            table.Columns.Add(new DataGridTemplateColumn { Header = "الإجراءات", CanUserSort = false, CellTemplate = new DataTemplate { VisualTree = button }, Width = 100 });
            window.Content = table;
            WpfTestHost.CompleteLayout(window);
            table.Sort(nameof(Row.Name));
            nameColumn.Width = new DataGridLength(260);
            WpfTestHost.CompleteLayout(window);
            var header = WpfTestHost.Descendants<DataGridColumnHeader>(table).Single(item => ReferenceEquals(item.Column, nameColumn));
            var cell = WpfTestHost.Descendants<DataGridCell>(table).First(item => ReferenceEquals(item.Column, nameColumn));
            Assert.AreEqual(header.ActualWidth, cell.ActualWidth, 1);
            Assert.AreEqual(header.TranslatePoint(new Point(), table).X, cell.TranslatePoint(new Point(), table).X, 1);
            Assert.IsTrue(table.CanUserResizeColumns);
            var action = WpfTestHost.Descendants<Button>(table).Single(item => ReferenceEquals(item.DataContext, rows[1]) && Equals(item.Content, "إجراءات"));
            action.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreSame(rows[1], action.ContextMenu!.DataContext);
            Assert.AreSame(rows[1], ((MenuItem)action.ContextMenu.Items[0]).DataContext);
            action.ContextMenu.IsOpen = false;
        });
    }

    private sealed class SortableTable : OperationsTable
    {
        public void Sort(string member)
        {
            var column = Columns.FirstOrDefault(column => column.SortMemberPath == member);
            if (column is null)
            {
                column = new DataGridTextColumn { Header = member, Binding = new Binding(member), SortMemberPath = member };
                Columns.Add(column);
            }
            OnSorting(new DataGridSortingEventArgs(column));
        }
    }

    public sealed class Row(string name, int amount, DateTime date) : INotifyPropertyChanged
    {
        private string rowName = name;
        public event PropertyChangedEventHandler? PropertyChanged;
        public string Name
        {
            get => rowName;
            set
            {
                rowName = value;
                PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Name)));
            }
        }
        public int Amount { get; } = amount;
        public DateTime Date { get; } = date;
    }
}
