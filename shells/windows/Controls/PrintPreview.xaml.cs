using System.Windows;
using System.Windows.Documents;
using System.Windows.Controls;
using System.IO;
using System.IO.Packaging;
using System.Windows.Xps.Packaging;
using System.Windows.Threading;
using System.Windows.Input;
using Button = System.Windows.Controls.Button;
using Label = System.Windows.Controls.Label;
using TextBox = System.Windows.Controls.TextBox;

namespace Eitmad.WindowsShell.Controls;

// Reusable native preview. Callers provide a document containing only printable content.
public partial class PrintPreview : System.Windows.Controls.UserControl
{
    public PrintPreview()
    {
        InitializeComponent();
        Loaded += (_, _) => {
            if (document is not null && Pages.Document is null) RefreshPages();
            ScheduleFitWidth();
        };
        Pages.SizeChanged += (_, _) => ScheduleFitWidth();
        Unloaded += (_, _) => ReleasePages();
        // The viewer's keyboard/menu print route must use the same Rust gate.
        System.Windows.Input.CommandManager.AddPreviewExecutedHandler(this, (sender, args) => {
            if (args.Command == NavigationCommands.Zoom || args.Command == NavigationCommands.IncreaseZoom ||
                args.Command == NavigationCommands.DecreaseZoom || args.Command == DocumentViewer.FitToWidthCommand ||
                args.Command == DocumentViewer.FitToHeightCommand || args.Command == DocumentViewer.FitToMaxPagesAcrossCommand ||
                args.Command == DocumentViewer.ViewThumbnailsCommand) followViewportWidth = false;
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
    private bool followViewportWidth = true;
    private bool fitPending;
    private void NativeSearchLoaded(object sender, RoutedEventArgs e)
    {
        if (((ContentControl)sender).Content is not ToolBar toolbar) return;
        // WPF constructs this toolbar internally. Localize its named XAML items through
        // public control properties; keep the original search, selection and option handlers.
        // CustomerDocumentsRenderedTests guards these .NET 8 names and native search behavior.
        toolbar.Template = (ControlTemplate)Resources["PreviewSearchBar"];
        toolbar.SetResourceReference(BackgroundProperty, "PanelBrush");
        toolbar.FlowDirection = System.Windows.FlowDirection.RightToLeft;
        toolbar.Language = Language;
        toolbar.FontFamily = PrintButton.FontFamily;
        var border = (Border)toolbar.FindName("FindTextBoxBorder");
        border.BorderThickness = new Thickness(0);
        border.Margin = new Thickness(0);
        border.Background = System.Windows.Media.Brushes.Transparent;
        var label = (Label)toolbar.FindName("FindTextLabel");
        label.Content = string.Empty;
        var text = (TextBox)toolbar.FindName("FindTextBox");
        text.Style = (Style)FindResource("TextInput");
        text.Width = 240;
        text.MinHeight = 36;
        text.ToolTip = "البحث عن كلمة أو عبارة في المستند";
        ControlOptions.SetPlaceholder(text, "اكتب نصًا للبحث…");
        text.ContextMenu = new ContextMenu { FlowDirection = System.Windows.FlowDirection.RightToLeft };
        text.ContextMenu.Items.Add(new MenuItem { Header = "نسخ", Command = ApplicationCommands.Copy });
        text.ContextMenu.Items.Add(new MenuItem { Header = "لصق", Command = ApplicationCommands.Paste });
        text.ContextMenu.Items.Add(new MenuItem { Header = "تحديد الكل", Command = ApplicationCommands.SelectAll });
        foreach (var (name, caption) in new[] { ("FindPreviousButton", "السابق"), ("FindNextButton", "التالي") }) {
            var button = (Button)toolbar.FindName(name);
            button.Style = (Style)Resources["PreviewAction"];
            button.Content = caption;
            button.ToolTip = caption == "السابق" ? "نتيجة البحث السابقة" : "نتيجة البحث التالية";
            button.Width = double.NaN;
            button.Height = 36;
            button.Margin = new Thickness(6, 0, 0, 0);
        }
        var options = (MenuItem)toolbar.FindName("OptionsMenuItem");
        options.Header = "خيارات البحث";
        options.ToolTip = "خيارات مطابقة نص البحث";
        foreach (var (name, caption) in new[] {
            ("OptionsWholeWordMenuItem", "كلمة كاملة فقط"), ("OptionsCaseMenuItem", "مطابقة حالة الأحرف"),
            ("OptionsDiacriticMenuItem", "مطابقة التشكيل"), ("OptionsKashidaMenuItem", "مطابقة التطويل"),
            ("OptionsAlefHamzaMenuItem", "مطابقة همزة الألف") })
            ((MenuItem)toolbar.FindName(name)).Header = caption;
    }
    private void ScheduleFitWidth()
    {
        if (!IsLoaded || !followViewportWidth || fitPending) return;
        fitPending = true;
        Dispatcher.BeginInvoke(DispatcherPriority.Background, new Action(() => {
            fitPending = false;
            // Fit after layout, until a native zoom/layout command gives control
            // to the user. Pagination can also change Zoom, so do not infer intent
            // from the Zoom property itself.
            if (IsLoaded && followViewportWidth && Pages.Document is not null) Pages.FitToWidth();
        }));
    }
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
            ScheduleFitWidth();
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
