using System.Windows;
using Eitmad.WindowsShell.Features.Reception;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Customers;

/// <summary>Reuses the shared modal for a temporary contact draft. Cancel never changes the contact.</summary>
public partial class CustomerEditorView : UserControl
{
    private Action<PreviewCustomer>? apply;
    public CustomerEditorView() => InitializeComponent();

    private void NameChanged(object sender, System.Windows.Controls.TextChangedEventArgs e)
    {
        if (!string.IsNullOrWhiteSpace(NameInput.Text)) NameField.ErrorText = "";
    }

    public void Open(PreviewCustomer? contact, Action<PreviewCustomer> applyPreview)
    {
        apply = applyPreview;
        Dialog.Title = contact is null ? "عميل جديد" : "تعديل بيانات العميل";
        NameInput.Text = contact?.Name ?? "";
        PhoneInput.Text = contact?.Phone ?? "";
        AddressInput.Text = contact?.Address ?? "";
        NotesInput.Text = contact?.Notes ?? "";
        NameField.ErrorText = "";
        Dialog.IsOpen = true;
    }

    private void CancelClick(object sender, RoutedEventArgs e)
    {
        Dialog.IsOpen = false;
        apply = null;
    }

    private void ApplyClick(object sender, RoutedEventArgs e)
    {
        // Only a preview input guard. Rust will own production validation.
        if (string.IsNullOrWhiteSpace(NameInput.Text))
        {
            NameField.ErrorText = "أدخل اسم العميل للمعاينة.";
            NameInput.Focus();
            return;
        }
        var callback = apply;
        Dialog.IsOpen = false;
        apply = null;
        callback?.Invoke(new(NameInput.Text, PhoneInput.Text, AddressInput.Text, NotesInput.Text));
    }
}
