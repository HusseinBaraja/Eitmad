using System.IO;
using System.Windows;
using System.Windows.Documents;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using System.Windows.Input;
using System.Windows.Automation.Peers;
using System.Windows.Automation.Provider;
using System.Windows.Automation.Text;
using System.Windows.Xps.Packaging;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.Orders;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class CustomerDocumentsRenderedTests
{
    internal static CustomerDocument Saved(bool order = false) => new() {
        Number = order ? "OR-2026-00001" : "QT-2026-00001", DocumentRevision = 2, Status = order ? "جاهز" : "مقبول", CanPrint = true,
        SavedAt = 1791417600000, IssuedAt = 1791417600000, ValidUntil = order ? null : 1794085199999, ValidityDays = order ? null : 30,
        Customer = new() { Id = Guid.NewGuid(), Revision = 1, Name = "شركة الأثاث التجريبية A-12", Phone = "+967-777123456", Address = "صنعاء، شارع تجريبي B-7" },
        Lines = [
            new() { Name = "مرتبة الراحة A-12", Description = "نسيج عربي تجريبي", VariantName = "مفرد XL", Quantity = 2, UnitPriceYer = 12500, TotalYer = 25000 },
            new() { Name = "خزانة السكينة B-7", Description = "خشب مع بابين", VariantName = "بابان", ColorName = "جوزي", HandleName = "معدن H-2", Dimensions = new() { WidthMm = 1200, HeightMm = 1800, DepthMm = 600 }, Quantity = 1, UnitPriceYer = 25000, TotalYer = 25000 },
        ], DiscountBasisPoints = 500, SubtotalYer = 50000, DiscountYer = 2500, TotalYer = 47500,
    };

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void SavedDocumentsRenderAndNativePrintRoutesDenyAccess(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
            foreach (var order in new[] { false, true }) {
                var saved = Saved(order);
                var document = order ? OrderCustomerDocument.CreateSaved(saved) : QuotationCustomerDocument.CreateSaved(saved);
                var preview = new PrintPreview { Document = document, CanPrint = saved.CanPrint };
                window.Content = preview;
                WpfTestHost.CompleteLayout(window);
                var dpi = VisualTreeHelper.GetDpi(window);
                Console.WriteLine($"Document {(order ? "order" : "quotation")}: display {SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX:0}x{SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY:0} px; actual window {window.ActualWidth:0.0}x{window.ActualHeight:0.0} DIP; scaling {dpi.DpiScaleX * 100:0}%; high contrast {SystemParameters.HighContrast}");
                var text = new TextRange(document.ContentStart, document.ContentEnd).Text;
                foreach (var value in new[] { saved.Number, saved.Customer.Name, saved.Customer.Phone, saved.Customer.Address, "120 × 180 × 60 سم", "12,500 ر.ي", "25,000 ر.ي", "50,000 ر.ي", "2,500 ر.ي", "47,500 ر.ي", "5.00%", saved.Status }) Assert.IsTrue(text.Contains(value), value);
                Assert.AreEqual(FlowDirection.RightToLeft, document.FlowDirection);
                Assert.IsTrue(preview.PrintButton.Focus()); Assert.IsTrue(preview.PrintButton.IsKeyboardFocusWithin);
                var authorization = new TaskCompletionSource<bool>(); var requests = 0;
                preview.AuthorizePrint = () => { requests++; return authorization.Task; };
                var pages = WpfTestHost.FindByName<System.Windows.Controls.DocumentViewer>(preview, "Pages");
                AssertFitsPage(pages);
                var nativePage = ((FixedDocumentSequence)pages.Document).DocumentPaginator.GetPage(0);
                Assert.IsTrue(string.Concat(WpfTestHost.Descendants<Glyphs>(nativePage.Visual).Select(glyph => glyph.UnicodeString)).Contains("47,500"), "The preview must retain searchable saved text.");
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, $"saved-document-{(order ? "order" : "quotation")}-{width}");
                ApplicationCommands.Print.Execute(null, pages);
                ApplicationCommands.Print.Execute(null, pages);
                Assert.AreEqual(1, requests, "A pending authorization must not start another print request.");
                authorization.SetResult(false);
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(1, requests); Assert.IsFalse(preview.CanPrint);
                Assert.IsTrue(preview.PrintStatus.Text.Contains("صلاحية"));
                Assert.IsTrue(preview.BackButton.Focus()); Assert.IsTrue(preview.BackButton.IsKeyboardFocusWithin);
                ExportPages(document, $"saved-{(order ? "order" : "quotation")}", width == 1338, "47,500");
            }
            var draft = Saved(); draft.IsDraft = true; draft.CanPrint = false; draft.Number = null!; draft.Status = "مسودة"; draft.IssuedAt = null; draft.ValidUntil = null;
            var draftPreview = new PrintPreview { Document = QuotationCustomerDocument.CreateSaved(draft), CanPrint = draft.CanPrint };
            window.Content = draftPreview; WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(draftPreview.CanPrint);
            AssertFitsPage(draftPreview.Pages);
            Assert.IsTrue(new TextRange(draftPreview.Document.ContentStart, draftPreview.Document.ContentEnd).Text.Contains("غير مرقم"));
            WpfTestHost.Capture(window, $"saved-document-draft-{width}");
            ExportPages(draftPreview.Document, "saved-draft", width == 1338, "47,500");
        });
    }

    [TestMethod]
    [DataRow("percentage")]
    [DataRow("increase")]
    [DataRow("width")]
    [DataRow("height")]
    public void PreviewFitsOnResizeUntilTheUserChangesNativeZoom(string selection)
    {
        WpfTestHost.Run(720, 560, window => {
            var preview = new PrintPreview { Document = QuotationCustomerDocument.CreateSaved(Saved()) };
            window.Content = preview;
            WpfTestHost.CompleteLayout(window);
            AssertFitsPage(preview.Pages);
            var initialZoom = preview.Pages.Zoom;
            window.Width = 1338; window.Height = 753;
            // Arrange the native resize, then drain the preview's deferred fitting.
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.CompleteLayout(window);
            AssertFitsPage(preview.Pages);
            Assert.IsTrue(preview.Pages.Zoom > initialZoom, "Native whole-page mode must follow a larger viewport.");
            window.Width = 720; window.Height = 560;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.CompleteLayout(window);
            AssertFitsPage(preview.Pages);
            switch (selection) {
                case "percentage": NavigationCommands.Zoom.Execute(125.0, preview.Pages); break;
                case "increase": NavigationCommands.IncreaseZoom.Execute(null, preview.Pages); break;
                case "width": DocumentViewer.FitToWidthCommand.Execute(null, preview.Pages); break;
                case "height": DocumentViewer.FitToHeightCommand.Execute(null, preview.Pages); break;
            }
            WpfTestHost.CompleteLayout(window);
            var chosenZoom = preview.Pages.Zoom;
            if (selection is "percentage" or "increase") {
                window.Width = 1100;
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(chosenZoom, preview.Pages.Zoom, 0.01, "Resize must preserve the user's native zoom selection.");
            }
            preview.Document = OrderCustomerDocument.CreateSaved(Saved(true));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(chosenZoom, preview.Pages.Zoom, 0.01, "An authorization refresh must not reset the user's zoom.");
        });
    }

    [TestMethod]
    public void MultiPageNativeExportPreservesArabicAndTotals()
    {
        WpfTestHost.Run(1338, 753, window => {
            var saved = Saved(); saved.Lines = Enumerable.Range(0, 36).Select(i => new CustomerDocumentLine { Name = $"خزانة تجريبية A-{i}", Description = "نص عربي طويل لقياس التفاف السطر", VariantName = "بابان", Dimensions = new() { WidthMm = 1200, HeightMm = 1800, DepthMm = 600 }, Quantity = 1, UnitPriceYer = 12500, TotalYer = 12500 }).ToArray();
            // Keep the customer's technical identifier separate from the line-identity proof.
            saved.Customer.Name = "شركة الأثاث التجريبية";
            saved.SubtotalYer = 450000; saved.DiscountYer = 22500; saved.TotalYer = 427500;
            var document = QuotationCustomerDocument.CreateSaved(saved);
            var paginator = ((IDocumentPaginatorSource)document).DocumentPaginator; paginator.ComputePageCount();
            Assert.IsTrue(paginator.PageCount > 1);
            var preview = new PrintPreview { Document = document };
            window.Content = preview;
            WpfTestHost.CompleteLayout(window);
            AssertFitsPage(preview.Pages);
            Assert.AreEqual(paginator.PageCount, preview.Pages.PageCount);
            preview.Pages.LastPage();
            WpfTestHost.CompleteLayout(window);
            AssertFitsPage(preview.Pages);
            Assert.AreEqual(preview.Pages.PageCount, preview.Pages.MasterPageNumber);
            AssertNativePreviewCommands(window, preview);
            ExportPages(document, "saved-quotation-multipage", true, "427,500", "خزانة تجريبية",
                Enumerable.Range(0, saved.Lines.Length).Select(index => $"A-{index}").ToArray());
        });
    }

    internal static CustomerDocument MultiPageSaved(bool order)
    {
        var saved = Saved(order);
        saved.Customer.Name = "شركة الأثاث التجريبية";
        saved.Lines = Enumerable.Range(0, 36).Select(i => new CustomerDocumentLine {
            Name = $"خزانة تجريبية A-{i}", Description = "نص عربي طويل لقياس التفاف السطر", VariantName = "بابان",
            Dimensions = new() { WidthMm = 1200, HeightMm = 1800, DepthMm = 600 },
            Quantity = 1, UnitPriceYer = 12500, TotalYer = 12500,
        }).ToArray();
        saved.SubtotalYer = 450000; saved.DiscountYer = 22500; saved.TotalYer = 427500;
        return saved;
    }

    [TestMethod]
    [DataRow(false, 793.7)]
    [DataRow(false, 745.7)]
    [DataRow(false, 697.7)]
    [DataRow(true, 745.7)]
    [DataRow(true, 697.7)]
    public void PrintableWidthPreservesCompleteValuesAndContinuationColumns(bool order, double pageWidth)
    {
        WpfTestHost.Run(1338, 753, _ => {
            var saved = MultiPageSaved(order);
            foreach (var line in saved.Lines) { line.UnitPriceYer = 1234567; line.TotalYer = 1234567; }
            saved.SubtotalYer = 44444412; saved.DiscountBasisPoints = 0; saved.DiscountYer = 0; saved.TotalYer = 44444412;
            var document = order ? OrderCustomerDocument.CreateSaved(saved) : QuotationCustomerDocument.CreateSaved(saved);
            // The native print route changes PageWidth after the preview has already paginated.
            new CustomerDocumentPages(document);
            document.PageWidth = pageWidth;
            var pages = new CustomerDocumentPages(document);
            Assert.IsTrue(pages.PageCount > 1);
            var paginator = (DynamicDocumentPaginator)((IDocumentPaginatorSource)document).DocumentPaginator;
            var rows = document.Blocks.OfType<Table>().Single(table => Equals(table.Tag, CustomerDocumentLayout.ItemsTag)).RowGroups[0].Rows;
            foreach (var row in rows.Skip(1)) {
                var firstPage = paginator.GetPageNumber(row.Cells[0].Blocks.FirstBlock.ContentStart);
                foreach (var paragraph in row.Cells.SelectMany(cell => cell.Blocks.OfType<Paragraph>())) {
                    Assert.AreEqual(firstPage, paginator.GetPageNumber(paragraph.ContentStart), "A narrow page must retain row grouping.");
                    Assert.AreEqual(firstPage, paginator.GetPageNumber(paragraph.ContentEnd), "A narrow page must retain row grouping.");
                }
            }
            ExportPages(document, $"printable-{(order ? "order" : "quotation")}-{pageWidth:0.0}", true, "44,444,412", "خزانة تجريبية",
                Enumerable.Range(0, saved.Lines.Length).Select(index => $"A-{index}").ToArray(), checkBounds: true);
        });
    }

    [TestMethod]
    [DataRow(false)]
    [DataRow(true)]
    public void NormalLineRowsStayTogetherAcrossPages(bool order)
    {
        WpfTestHost.Run(1920, 1080, window => {
            window.Left = 0; window.WindowState = WindowState.Maximized;
            var saved = MultiPageSaved(order);
            saved.Lines[12].ColorName = "جوزي"; saved.Lines[12].HandleName = "معدن H-2";
            var document = order ? OrderCustomerDocument.CreateSaved(saved) : QuotationCustomerDocument.CreateSaved(saved);
            var pages = new CustomerDocumentPages(document);
            Assert.IsTrue(pages.PageCount > 1);
            var paginator = (DynamicDocumentPaginator)((IDocumentPaginatorSource)document).DocumentPaginator;
            var rows = document.Blocks.OfType<Table>().Single(table => Equals(table.Tag, CustomerDocumentLayout.ItemsTag)).RowGroups[0].Rows;
            for (var index = 0; index < saved.Lines.Length; index++) {
                var row = rows[index + 1];
                var firstPage = paginator.GetPageNumber(row.Cells[0].Blocks.FirstBlock.ContentStart);
                foreach (var paragraph in row.Cells.SelectMany(cell => cell.Blocks.OfType<Paragraph>())) {
                    Assert.AreEqual(firstPage, paginator.GetPageNumber(paragraph.ContentStart), $"A-{index}: cell starts on another page.");
                    Assert.AreEqual(firstPage, paginator.GetPageNumber(paragraph.ContentEnd), $"A-{index}: cell ends on another page.");
                }
            }
        });
    }

    [TestMethod]
    [DataRow(false)]
    [DataRow(true)]
    public void OversizedLineFlowsWithoutLosingTextOrBlockingPagination(bool order)
    {
        WpfTestHost.Run(1920, 1080, window => {
            window.Left = 0; window.WindowState = WindowState.Maximized;
            var saved = MultiPageSaved(order);
            var markers = Enumerable.Range(0, 130).Select(i => $"TALL-{i:D3}").ToArray();
            saved.Lines[8].Description = string.Join("\n", markers.Select(marker => "وصف عربي طويل " + marker));
            var document = order ? OrderCustomerDocument.CreateSaved(saved) : QuotationCustomerDocument.CreateSaved(saved);
            var preview = new PrintPreview { Document = document };
            window.Content = preview; WpfTestHost.CompleteLayout(window);
            var paginator = preview.Pages.Document.DocumentPaginator;
            Assert.IsTrue(paginator.PageCount > 1);
            var exportedText = "";
            for (var index = 0; index < paginator.PageCount; index++) {
                var page = paginator.GetPage(index);
                var text = string.Concat(WpfTestHost.Descendants<Glyphs>(page.Visual).Select(glyph => glyph.UnicodeString));
                exportedText += text;
                Assert.IsTrue(text.Contains($"{index + 1} / {paginator.PageCount}"), "Every page must retain its number.");
                if (index > 0 && text.Contains("TALL-"))
                    foreach (var header in new[] { "الصنف", "الخيارات المحددة", "الكمية", "سعر الوحدة", "الإجمالي" })
                        Assert.IsTrue(text.Contains(header), "Oversized row continuation must retain column headings.");
            }
            foreach (var marker in markers) Assert.AreEqual(1, System.Text.RegularExpressions.Regex.Matches(exportedText, marker).Count, marker);
            foreach (var line in saved.Lines)
                Assert.AreEqual(1, System.Text.RegularExpressions.Regex.Matches(exportedText, @"A-\d+").Count(match => match.Value == line.Name.Split(' ').Last()), line.Name);
            Assert.IsTrue(exportedText.Contains("427,500"), "The unchanged Rust total must survive oversized row pagination.");
        });
    }

    private static void AssertNativePreviewCommands(MainWindow window, PrintPreview preview)
    {
        // Execute with the same explicit target as a native Button, rather than the test's focus.
        void Invoke(RoutedCommand command)
        {
            var button = WpfTestHost.Descendants<Button>(preview).Single(item => item.Command == command);
            Assert.IsTrue(button.IsEnabled);
            Assert.IsTrue(button.Focus());
            command.Execute(button.CommandParameter, button.CommandTarget);
            WpfTestHost.CompleteLayout(window);
        }
        Invoke(ApplicationCommands.Find);
        var search = WpfTestHost.FindByName<TextBox>(preview, "FindTextBox");
        Assert.IsTrue(search.IsKeyboardFocusWithin, "Native Find must focus the localized field.");
        search.Text = "خزانة";
        WpfTestHost.FindByName<Button>(preview, "FindNextButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        WpfTestHost.CompleteLayout(window);
        var peer = FrameworkElementAutomationPeer.CreatePeerForElement(preview.Pages);
        var provider = (ITextProvider)peer.GetPattern(PatternInterface.Text);
        Assert.IsTrue(provider.GetSelection().Any(range => range.GetText(-1).Contains("خزانة")), "Native search must select the Arabic result.");
        var firstResult = provider.GetSelection()[0].Clone();
        WpfTestHost.FindByName<Button>(preview, "FindNextButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        WpfTestHost.CompleteLayout(window);
        Assert.IsTrue(provider.GetSelection()[0].CompareEndpoints(TextPatternRangeEndpoint.Start, firstResult, TextPatternRangeEndpoint.Start) > 0, "Next search must advance the native selection.");
        WpfTestHost.FindByName<Button>(preview, "FindPreviousButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        WpfTestHost.CompleteLayout(window);
        Assert.AreEqual(0, provider.GetSelection()[0].CompareEndpoints(TextPatternRangeEndpoint.Start, firstResult, TextPatternRangeEndpoint.Start), "Previous search must return to the preceding result.");
        Assert.IsTrue(search.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next)));
        Assert.IsTrue(WpfTestHost.FindByName<Button>(preview, "FindPreviousButton").IsKeyboardFocusWithin);
        search.Text = string.Empty;

    }

    private static void AssertFitsPage(DocumentViewer viewer)
    {
        AssertFitsWidth(viewer);
        var viewport = (ScrollViewer)viewer.Template.FindName("PART_ContentHost", viewer);
        var page = viewer.PageViews.Single(page => page.PageNumber == viewer.MasterPageNumber - 1);
        var bounds = page.TransformToAncestor(viewport).TransformBounds(new Rect(page.RenderSize));
        Assert.IsTrue(bounds.Top >= -1 && bounds.Bottom <= viewport.ViewportHeight + 1,
            $"The current page must be fully visible: {bounds}; viewport height {viewport.ViewportHeight:0.00} DIP at {viewer.Zoom:0.00}%.");
        if (viewer.PageCount == 1) Assert.IsTrue(viewport.ScrollableHeight < 1, "A one-page preview must not require vertical scrolling.");
    }

    private static void AssertFitsWidth(DocumentViewer viewer)
    {
        var viewport = (ScrollViewer)viewer.Template.FindName("PART_ContentHost", viewer);
        Assert.IsTrue(viewport.ViewportWidth > 0);
        Assert.IsTrue(viewport.ScrollableWidth < 1, $"Page requires horizontal scrolling: {viewport.ScrollableWidth:0.00} DIP at {viewer.Zoom:0.00}%.");
        foreach (var page in viewer.PageViews.Where(page => page.IsVisible)) {
            var bounds = page.TransformToAncestor(viewport).TransformBounds(new Rect(page.RenderSize));
            Assert.IsTrue(bounds.Left >= -1 && bounds.Right <= viewport.ActualWidth + 1, $"Page is clipped horizontally: {bounds}; viewport width {viewport.ActualWidth}.");
        }
    }

    private static void ExportPages(FlowDocument document, string name, bool capture, string expectedTotal, string? expectedArabic = null, string[]? lineIdentifiers = null, bool checkBounds = false)
    {
        if (!capture) return;
        var captureDirectory = Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR");
        var directory = string.IsNullOrEmpty(captureDirectory) ? Path.Combine(Path.GetTempPath(), "eitmad-documents-" + Guid.NewGuid()) : captureDirectory;
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, name + ".xps");
        File.Delete(path);
        using (var xps = new XpsDocument(path, FileAccess.ReadWrite)) XpsDocument.CreateXpsDocumentWriter(xps).Write(new CustomerDocumentPages(document));
        using var output = new XpsDocument(path, FileAccess.Read);
        var paginator = output.GetFixedDocumentSequence().DocumentPaginator; paginator.ComputePageCount();
        Console.WriteLine($"Export {name}: {paginator.PageCount} pages, {document.PageWidth:0.0}x{document.PageHeight:0.0} DIP at 96 DPI.");
        var exportedText = "";
        var exportedIdentifiers = new List<string>();
        for (var index = 0; index < paginator.PageCount; index++) {
            var page = paginator.GetPage(index); Assert.AreNotSame(DocumentPage.Missing, page);
            var glyphText = WpfTestHost.Descendants<Glyphs>(page.Visual).Select(glyph => glyph.UnicodeString).ToArray();
            var pageText = string.Concat(glyphText);
            exportedText += pageText;
            if (checkBounds) {
                Assert.IsTrue(pageText.Contains($"{index + 1} / {paginator.PageCount}"), "Every printed page must retain its number.");
                AssertGlyphsWithinPrintableWidth(page, document.PagePadding);
            }
            if (lineIdentifiers is not null) {
                var identifiers = System.Text.RegularExpressions.Regex.Matches(pageText, @"A-\d+")
                    .Select(match => match.Value).ToArray();
                exportedIdentifiers.AddRange(identifiers);
                if (index > 0 && identifiers.Length > 0)
                    foreach (var header in new[] { "الصنف", "الخيارات المحددة", "الكمية", "سعر الوحدة", "الإجمالي" })
                        Assert.IsTrue(pageText.Contains(header), $"Continuation page {index + 1} is missing item header {header}.");
                if (pageText.Contains(expectedTotal))
                    foreach (var label in new[] { "المجموع الفرعي", "نسبة الخصم", "الخصم", "الإجمالي النهائي" })
                        Assert.IsTrue(pageText.Contains(label), "Financial totals must stay together on the final page.");
            }
            if (string.IsNullOrEmpty(captureDirectory)) continue;
            var bitmap = new RenderTargetBitmap((int)Math.Ceiling(page.Size.Width), (int)Math.Ceiling(page.Size.Height), 96, 96, PixelFormats.Pbgra32);
            bitmap.Render(page.Visual);
            var encoder = new PngBitmapEncoder(); encoder.Frames.Add(BitmapFrame.Create(bitmap));
            using var stream = File.Create(Path.Combine(directory, $"{name}-{index + 1}.png")); encoder.Save(stream);
        }
        Assert.IsTrue(exportedText.Contains(expectedTotal), "Native export must retain the saved final total, not only the page decorations.");
        if (lineIdentifiers is not null) CollectionAssert.AreEquivalent(lineIdentifiers, exportedIdentifiers.ToArray(), "Every saved line must appear exactly once in native export.");
        if (expectedArabic is not null) Assert.IsTrue(exportedText.Contains(expectedArabic), "Native export must retain Arabic text.");
        if (checkBounds) {
            Assert.AreEqual(72, System.Text.RegularExpressions.Regex.Matches(exportedText, "1,234,567").Count,
                "Every unit price and line total must retain all digits.");
            var number = ((ValueTuple<string, string?>)document.Tag).Item2!;
            Assert.AreEqual(paginator.PageCount + 1, System.Text.RegularExpressions.Regex.Matches(exportedText, number).Count,
                "The complete identity must appear in the body and every page heading.");
        }
        output.Close();
        if (string.IsNullOrEmpty(captureDirectory)) { File.Delete(path); Directory.Delete(directory); }
    }

    private static void AssertGlyphsWithinPrintableWidth(DocumentPage page, Thickness padding)
    {
        foreach (var glyph in WpfTestHost.Descendants<Glyphs>(page.Visual)) {
            var alignment = glyph.ToGlyphRun().ComputeAlignmentBox();
            alignment.Offset(glyph.OriginX, glyph.OriginY);
            var transform = glyph.TransformToAncestor(page.Visual);
            var bounds = transform.TransformBounds(alignment);
            Assert.AreEqual(1.0, (transform.Transform(new Point(1, 0)) - transform.Transform(new Point(0, 0))).Length, 0.001,
                "Printing must wrap cells without scaling the complete document.");
            Assert.IsTrue(bounds.Left >= padding.Left - 1 && bounds.Right <= page.Size.Width - padding.Right + 1,
                $"Printed text '{glyph.UnicodeString}' exceeds the content width: {bounds} on {page.Size.Width} DIP paper.");
            DependencyObject? ancestor = glyph;
            while (ancestor is Visual visual) {
                if (VisualTreeHelper.GetClip(visual) is { } clip) {
                    var clipBounds = visual.TransformToAncestor(page.Visual).TransformBounds(clip.Bounds);
                    Assert.IsTrue(bounds.Left >= clipBounds.Left - 1 && bounds.Right <= clipBounds.Right + 1,
                        $"Printed text '{glyph.UnicodeString}' is clipped: {bounds}; clip {clipBounds}.");
                }
                if (ancestor == page.Visual) break;
                ancestor = VisualTreeHelper.GetParent(ancestor);
            }
        }
    }
}
