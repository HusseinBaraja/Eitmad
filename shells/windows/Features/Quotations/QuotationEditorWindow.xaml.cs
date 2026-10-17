using System.ComponentModel;
using System.Windows;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Features.Quotations;

internal partial class QuotationEditorWindow : Window
{
    private readonly SalesCatalogViewModel editor;
    private bool allowClose, saving;

    internal QuotationEditorWindow(SalesCatalogViewModel editor, SalesCatalogView view)
    {
        InitializeComponent();
        this.editor = editor;
        EditorContent.Content = view;
    }

    // Session/authority invalidation must still remove protected editor content immediately.
    internal void CloseInvalidated() { allowClose = true; Close(); }

    private void EditorClosing(object? sender, CancelEventArgs e)
    {
        if (allowClose) return;
        if (saving || editor.IsDraftBusy) { e.Cancel = true; return; }
        if (!editor.HasUnsavedDraftChanges) return;
        e.Cancel = true;
        UnsavedDialog.IsOpen = true;
    }

    private void ContinueEditingClick(object sender, RoutedEventArgs e)
    {
        if (saving) return;
        UnsavedDialog.IsOpen = false;
        e.Handled = true;
    }

    private void DiscardClick(object sender, RoutedEventArgs e)
    {
        if (saving || editor.IsDraftBusy) return;
        allowClose = true;
        Close();
    }

    private async void SaveAndCloseClick(object sender, RoutedEventArgs e)
    {
        if (saving || editor.IsDraftBusy) return;
        saving = true;
        SaveAndCloseButton.IsEnabled = DiscardButton.IsEnabled = ContinueEditingButton.IsEnabled = false;
        try
        {
            var saved = await editor.SaveDraftAsync();
            if (allowClose) return;
            UnsavedDialog.IsOpen = false;
            if (saved && !editor.HasUnsavedDraftChanges) { allowClose = true; Close(); }
            else
            {
                editor.IsReviewingQuotation = true;
                ((SalesCatalogView)EditorContent.Content).RestoreQuotationFocus();
            }
        }
        finally
        {
            saving = false;
            SaveAndCloseButton.IsEnabled = DiscardButton.IsEnabled = ContinueEditingButton.IsEnabled = true;
        }
    }
}
