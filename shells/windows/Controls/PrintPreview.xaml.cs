using System.Windows;
using System.Windows.Documents;
using System.Windows.Controls;
using System.IO;
using System.IO.Packaging;
using System.Windows.Xps.Packaging;

namespace Eitmad.WindowsShell.Controls;

// Reusable native preview. Callers provide a document containing only printable content.
public partial class PrintPreview : System.Windows.Controls.UserControl
{
    public PrintPreview()
    {
        InitializeComponent();
        Loaded += (_, _) => { if (document is not null && Pages.Document is null) RefreshPages(); };
        Unloaded += (_, _) => ReleasePages();
        // The viewer's keyboard/menu print route must use the same Rust gate.
        System.Windows.Input.CommandManager.AddPreviewExecutedHandler(this, (sender, args) => {
            if (args.Command != System.Windows.Input.ApplicationCommands.Print) return;
            args.Handled = true; PrintClick(sender, args);
        });
        System.Windows.Input.CommandManager.AddPreviewCanExecuteHandler(this, (_, args) => {
            if (args.Command != System.Windows.Input.ApplicationCommands.Print) return;
            args.CanExecute = CanPrint && !printing && AuthorizePrint is not null; args.Handled = true;
        });
    }
    private FlowDocument document = null!;
    public FlowDocument Document { get => document; set { document = value; RefreshPages(); } }
    private MemoryStream? previewStream;
    private Package? previewPackage;
    private XpsDocument? previewArchive;
    private Uri? previewUri;
    private bool printing;
    private void RefreshPages()
    {
        ReleasePages();
        try {
            // Preserve native glyphs, text selection and search without creating a customer file.
            previewStream = new MemoryStream();
            previewPackage = Package.Open(previewStream, FileMode.Create, FileAccess.ReadWrite);
            previewUri = new Uri($"memorystream://eitmad/{Guid.NewGuid():N}.xps");
            PackageStore.AddPackage(previewUri, previewPackage);
            previewArchive = new XpsDocument(previewPackage, CompressionOption.Normal, previewUri.AbsoluteUri);
            XpsDocument.CreateXpsDocumentWriter(previewArchive).Write(new CustomerDocumentPages(document));
            Pages.Document = previewArchive.GetFixedDocumentSequence();
        } catch { ReleasePages(); throw; }
    }
    private void ReleasePages()
    {
        Pages.Document = null;
        previewArchive?.Close(); previewArchive = null;
        if (previewUri is not null) PackageStore.RemovePackage(previewUri);
        previewUri = null;
        previewPackage?.Close(); previewPackage = null;
        previewStream?.Dispose(); previewStream = null;
    }
    public bool CanPrint { get => PrintButton.IsEnabled; set => PrintButton.IsEnabled = value; }
    public string JobName { get; set; } = "عرض السعر";
    public event EventHandler? BackRequested;
    private void BackClick(object sender, RoutedEventArgs e) => BackRequested?.Invoke(this, EventArgs.Empty);
    public Func<Task<bool>>? AuthorizePrint { get; set; }
    private async void PrintClick(object sender, RoutedEventArgs e)
    {
        if (!CanPrint || printing || AuthorizePrint is null) return;
        printing = true;
        try
        {
            if (!await AuthorizePrint()) { CanPrint = false; PrintStatus.Text = "ليس لديك صلاحية للطباعة أو المستند غير متاح."; return; }
            if (!IsVisible) return;
            var dialog = new System.Windows.Controls.PrintDialog();
            if (dialog.ShowDialog() != true) return;
            if (!await AuthorizePrint() || !IsVisible) { CanPrint = false; return; }
            var width = Document.PageWidth;
            var height = Document.PageHeight;
            try
            {
                Document.PageWidth = dialog.PrintableAreaWidth;
                Document.PageHeight = dialog.PrintableAreaHeight;
                dialog.PrintDocument(new CustomerDocumentPages(Document), JobName);
                PrintStatus.Text = "تم إرسال المستند إلى الطابعة";
            }
            finally {
                Document.PageWidth = width; Document.PageHeight = height;
            }
        }
        catch (Exception error) when (error is System.Printing.PrintSystemException or System.IO.IOException or InvalidOperationException)
        {
            PrintStatus.Text = "تعذرت الطباعة. تحقق من الطابعة ثم حاول مرة أخرى";
        }
        finally { printing = false; }
    }
}
