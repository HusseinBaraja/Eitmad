using System.Windows;
using System.Windows.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class ReceptionistHomeView : UserControl
{
    private Features.Customers.CustomerClient? customerClient;
    private CancellationTokenSource? customerLoadCancellation;
    private string customerReturnDestination = "الطلبات";
    private IInputElement? customerReturnFocus;
    public ReceptionHandoffPreview Handoffs { get; private set; } = null!;
    public Features.Orders.OrdersView PreviewOrders { get; private set; } = null!;

    public ReceptionistHomeView()
    {
        InitializeComponent();
        ((Button)ReceptionistSidebar.FindName("OrdersNavButton")).Visibility = Visibility.Visible;
        ReceptionQuotations.CustomerRequested += id => _ = OpenQuotationCustomerAsync(id);
    }

    public event EventHandler? AccountSwitchRequested;

    public void AttachCustomerClient(Features.Customers.CustomerClient client)
    {
        if (ReferenceEquals(customerClient, client)) return;
        if (customerClient is not null) customerClient.Changed -= CustomerChanged;
        customerClient = client;
        customerClient.Changed += CustomerChanged;
        CustomerDetail.Attach(client);
    }

    public Task ActivateCustomersAsync(CancellationToken cancellationToken = default) =>
        customerClient?.ActivateAsync(cancellationToken) ?? Task.CompletedTask;

    public Task DeactivateCustomersAsync() => customerClient?.DeactivateAsync() ?? Task.CompletedTask;

    public void SetCatalogSources(Features.Furniture.FurnitureViewModel furniture, Features.Products.ProductsViewModel products)
    {
        var catalog = new SalesCatalogViewModel(furniture, products, customerClient);
        CatalogContent.DataContext = catalog;
        ((Button)ReceptionistSidebar.FindName("QuotationsNavButton")).Visibility = Visibility.Visible;
        ReceptionQuotations.ConfigureReceptionist(quotation =>
        {
            var editor = QuotationPreviewProjection.Create(quotation, furniture, products);
            if (customerClient is not null) editor.AttachCustomerClient(customerClient);
            return Handoffs.Attach(editor);
        });
        Handoffs = new(ReceptionQuotations.ViewModel.PreviewQuotations);
        ReceptionQuotations.ViewModel.UsePreviewQuotations(Handoffs.Quotations);
        Handoffs.Attach(catalog);
        PreviewOrders = new Features.Orders.OrdersView();
        PreviewOrders.ConfigureReceptionist();
        PreviewOrders.CustomerRequested += id => _ = OpenOrderCustomerAsync(id);
        ReceptionOrders.Content = PreviewOrders;
        ReadyOrdersList.ItemsSource = PreviewOrders.ViewModel.NewReadyOrders;
        ReadyCountLabel.SetBinding(System.Windows.Controls.TextBlock.TextProperty,
            new System.Windows.Data.Binding(nameof(Features.Orders.OrdersViewModel.ReadyCount)) { Source = PreviewOrders.ViewModel });
        ReceptionistTitleBar.PrimaryActionButton.Width = 220;
        System.Windows.Automation.AutomationProperties.SetName(ReceptionistTitleBar.PrimaryActionButton, "فتح عرض السعر");
        ReceptionistTitleBar.SetBinding(Controls.ShellTitleBar.PrimaryActionLabelProperty,
            new System.Windows.Data.Binding(nameof(SalesCatalogViewModel.QuotationLabel)) { Source = catalog });
    }

    private void OpenReadyOrderClick(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { DataContext: Features.Orders.OrderListItem order }) return;
        Navigate("الطلبات");
        PreviewOrders.ViewModel.OpenOrder(order);
        Dispatcher.BeginInvoke(PreviewOrders.BackToOrdersButton.Focus, System.Windows.Threading.DispatcherPriority.Input);
    }

    private void SharedAccountSwitchRequested(object? sender, EventArgs eventArgs) =>
        AccountSwitchRequested?.Invoke(this, EventArgs.Empty);

    private void TitleBarActionRequested(object? sender, Controls.ShellActionEventArgs eventArgs)
    {
        if (eventArgs.IsPrimary)
        {
            if (!CatalogContent.IsVisible) Navigate("المنتجات");
            ((SalesCatalogViewModel)CatalogContent.DataContext).IsReviewingQuotation = true;
            CatalogContent.RestoreQuotationFocus();
            return;
        }
        ShowPreviewFeedback(eventArgs.Action);
    }

    private void TitleBarSearchSubmitted(object? sender, Controls.ShellSearchEventArgs eventArgs) =>
        ShowNotice($"نتائج المعاينة عن: {eventArgs.Query}");

    private void SidebarNavigationRequested(object? sender, Controls.NavigationRequestedEventArgs eventArgs) =>
        Navigate(eventArgs.Destination);

    private void Navigate(string destination)
    {
        if (destination is not ("المنتجات" or "الرئيسية" or "عروض الأسعار" or "الطلبات"))
        {
            ShowNotice($"تم اختيار {destination} في وضع المعاينة");
            return;
        }
        var catalog = destination == "المنتجات";
        CustomerDetail.Visibility = Visibility.Collapsed;
        if (catalog) ((SalesCatalogViewModel)CatalogContent.DataContext).Reload();
        CatalogContent.Visibility = catalog ? Visibility.Visible : Visibility.Collapsed;
        ReceptionQuotations.Visibility = destination == "عروض الأسعار" ? Visibility.Visible : Visibility.Collapsed;
        ReceptionOrders.Visibility = destination == "الطلبات" ? Visibility.Visible : Visibility.Collapsed;
        HomeContent.Visibility = destination == "الرئيسية" ? Visibility.Visible : Visibility.Collapsed;
        ReceptionistTitleBar.Title = destination;
        ReceptionistSidebar.SelectDestination(destination);
    }

    private async Task OpenQuotationCustomerAsync(Guid id)
    {
        var quotation = ReceptionQuotations.ViewModel.PreviewQuotations.FirstOrDefault(item => item.Id == id);
        if (quotation is null) return;
        await OpenCustomerAsync(quotation.CustomerId, quotation.Customer, quotation.Phone, "عروض الأسعار");
    }

    private async Task OpenOrderCustomerAsync(Guid id)
    {
        var order = PreviewOrders.ViewModel.PreviewOrders.FirstOrDefault(item => item.Id == id);
        if (order is null) return;
        await OpenCustomerAsync(order.CustomerId, order.Customer, order.Phone, "الطلبات");
    }

    private async Task OpenCustomerAsync(Guid? customerId, string name, string phone, string returnDestination)
    {
        if (customerClient is null)
        {
            ShowNotice("تعذر الاتصال ببيانات العملاء. حاول مرة أخرى.");
            return;
        }
        customerLoadCancellation?.Cancel();
        customerLoadCancellation?.Dispose();
        customerLoadCancellation = new CancellationTokenSource();
        var cancellationToken = customerLoadCancellation.Token;
        var result = customerId is { } id
            ? await customerClient.GetAsync(id, cancellationToken)
            : await FindExactCustomerAsync(name, phone, cancellationToken);
        if (cancellationToken.IsCancellationRequested) return;
        if (!result.Succeeded)
        {
            ShowNotice(result.Failure == Features.Customers.CustomerFailureKind.NotFound
                ? "لم يُحفظ هذا العميل بعد. اختره أو أنشئه من عرض السعر أولاً."
                : Features.Customers.CustomerClient.ArabicMessage(result.Failure));
            return;
        }
        var customer = Features.Customers.CustomerHistoryProjection.Create(result.Value!,
            ReceptionQuotations.ViewModel.PreviewQuotations, PreviewOrders.ViewModel.PreviewOrders);
        customerReturnDestination = returnDestination;
        customerReturnFocus = System.Windows.Input.Keyboard.FocusedElement;
        ReceptionOrders.Visibility = Visibility.Collapsed;
        ReceptionQuotations.Visibility = Visibility.Collapsed;
        CustomerDetail.DataContext = customer;
        CustomerDetail.Visibility = Visibility.Visible;
        ReceptionistTitleBar.Title = "تفاصيل العميل";
        await Dispatcher.BeginInvoke(CustomerDetail.BackButton.Focus, System.Windows.Threading.DispatcherPriority.Input);
    }

    private async Task<Features.Customers.CustomerResult<Eitmad.Contracts.Customer>> FindExactCustomerAsync(
        string name, string phone, CancellationToken cancellationToken)
    {
        var result = await customerClient!.SearchAsync(phone.Length > 0 ? phone : name, cancellationToken);
        if (!result.Succeeded)
            return Features.Customers.CustomerResult<Eitmad.Contracts.Customer>.Failed(result.Failure, result.InvalidFields);
        var match = result.Value!.FirstOrDefault(customer =>
            string.Equals(customer.Name, name, StringComparison.Ordinal)
            && string.Equals(customer.Phone, phone, StringComparison.Ordinal));
        return match is null
            ? Features.Customers.CustomerResult<Eitmad.Contracts.Customer>.Failed(Features.Customers.CustomerFailureKind.NotFound)
            : Features.Customers.CustomerResult<Eitmad.Contracts.Customer>.Success(match);
    }

    private void CustomerChanged(object? sender, Guid? customerId)
    {
        if (CustomerDetail.DataContext is not Features.Customers.CustomerPreview current
            || customerId is not null && customerId != current.Id) return;
        _ = RefreshOpenCustomerAsync(current.Id);
    }

    private async Task RefreshOpenCustomerAsync(Guid customerId)
    {
        if (customerClient is null) return;
        var result = await customerClient.GetAsync(customerId);
        if (result.Succeeded && CustomerDetail.DataContext is Features.Customers.CustomerPreview current
            && current.Id == customerId) current.Observe(result.Value!);
    }

    private void CustomerBackRequested(object? sender, EventArgs e)
    {
        Navigate(customerReturnDestination);
        if (customerReturnFocus is { } target)
            Dispatcher.BeginInvoke(() => System.Windows.Input.Keyboard.Focus(target), System.Windows.Threading.DispatcherPriority.Input);
    }

    private void ActionClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is not Button { Tag: string action })
        {
            return;
        }

        ShowPreviewFeedback(action);
    }

    private void ShowPreviewFeedback(string action)
    {
        if (action is "المنتجات" or "عروض الأسعار" or "الطلبات") { Navigate(action); return; }
        if (action == "عرض سعر جديد")
        {
            Navigate("المنتجات");
            var catalog = (SalesCatalogViewModel)CatalogContent.DataContext;
            catalog.CloseSelection();
            catalog.IsReviewingQuotation = false;
            CatalogContent.RestoreCatalogFocus();
            return;
        }
        ShowNotice($"تم اختيار {action} في وضع المعاينة");
    }

    private void ShowNotice(string message)
    {
        ReceptionistNotice.Message = message;
        ReceptionistNotice.RestartDuration();
    }

    private void DismissNotice(object sender, RoutedEventArgs eventArgs) => ReceptionistNotice.Message = string.Empty;
}

