using System.Windows;
using Eitmad.Contracts;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Customers;

/// <summary>Stages unsaved input and submits it through the typed Rust customer command.</summary>
public partial class CustomerEditorView : UserControl
{
    private Customer? customer;
    private CustomerClient? client;
    private Action<Customer>? observe;
    private CancellationTokenSource? saveCancellation;
    public CustomerEditorView() => InitializeComponent();

    private void NameChanged(object sender, System.Windows.Controls.TextChangedEventArgs e)
    {
        if (!string.IsNullOrWhiteSpace(NameInput.Text)) NameField.ErrorText = "";
    }

    public void Open(Customer current, CustomerClient customerClient, Action<Customer> observeCustomer)
    {
        customer = current;
        client = customerClient;
        observe = observeCustomer;
        Dialog.Title = "تعديل بيانات العميل";
        NameInput.Text = current.Name;
        PhoneInput.Text = current.Phone;
        AddressInput.Text = current.Address ?? "";
        NotesInput.Text = current.Notes ?? "";
        NameField.ErrorText = "";
        PhoneField.ErrorText = "";
        EditorError.Text = "";
        ApplyButton.IsEnabled = true;
        Dialog.IsOpen = true;
    }

    private void CancelClick(object sender, RoutedEventArgs e)
    {
        saveCancellation?.Cancel();
        Dialog.IsOpen = false;
        ClearTarget();
    }

    private async void ApplyClick(object sender, RoutedEventArgs e)
    {
        if (customer is null || client is null) return;
        saveCancellation?.Cancel();
        saveCancellation?.Dispose();
        saveCancellation = new CancellationTokenSource();
        var cancellation = saveCancellation;
        ApplyButton.IsEnabled = false;
        EditorError.Text = "";
        NameField.ErrorText = "";
        PhoneField.ErrorText = "";
        CustomerResult<Customer> result;
        try
        {
            result = await client.UpdateAsync(customer, NameInput.Text, PhoneInput.Text,
                AddressInput.Text, NotesInput.Text, cancellation.Token);
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested)
        {
            return;
        }
        ApplyButton.IsEnabled = true;
        if (result.Succeeded)
        {
            observe?.Invoke(result.Value!);
            Dialog.IsOpen = false;
            ClearTarget();
            return;
        }
        EditorError.Text = CustomerClient.ArabicMessage(result.Failure);
        if (result.Failure == CustomerFailureKind.Validation)
        {
            if (result.InvalidFields.Count == 0 || result.InvalidFields.Contains("name"))
                NameField.ErrorText = "تحقق من اسم العميل.";
            if (result.InvalidFields.Count == 0 || result.InvalidFields.Contains("phone"))
                PhoneField.ErrorText = "تحقق من رقم الهاتف.";
        }
        (NameField.ErrorText.Length > 0 ? NameInput : PhoneInput).Focus();
    }

    private void ClearTarget()
    {
        customer = null;
        client = null;
        observe = null;
        saveCancellation?.Dispose();
        saveCancellation = null;
    }
}
