using System.Windows;
using System.Windows.Documents;
using System.Windows.Controls;

namespace Eitmad.WindowsShell.Controls;

// Reusable native preview. Callers provide a document containing only printable content.
public partial class PrintPreview : System.Windows.Controls.UserControl
{
    public PrintPreview() => InitializeComponent();
    public FlowDocument Document { get => (FlowDocument)Pages.Document; set => Pages.Document = value; }
    public string JobName { get; set; } = "عرض السعر";
    public event EventHandler? BackRequested;
    private void BackClick(object sender, RoutedEventArgs e) => BackRequested?.Invoke(this, EventArgs.Empty);
    private void PrintClick(object sender, RoutedEventArgs e)
    {
        var dialog = new System.Windows.Controls.PrintDialog();
        try
        {
            if (dialog.ShowDialog() != true) return;
            var width = Document.PageWidth;
            var height = Document.PageHeight;
            try
            {
                Document.PageWidth = dialog.PrintableAreaWidth;
                Document.PageHeight = dialog.PrintableAreaHeight;
                dialog.PrintDocument(((IDocumentPaginatorSource)Document).DocumentPaginator, JobName);
                PrintStatus.Text = "تم إرسال المستند إلى الطابعة";
            }
            finally { Document.PageWidth = width; Document.PageHeight = height; }
        }
        catch (Exception error) when (error is System.Printing.PrintSystemException or System.IO.IOException or InvalidOperationException)
        {
            PrintStatus.Text = "تعذرت الطباعة. تحقق من الطابعة ثم حاول مرة أخرى";
        }
    }
}
