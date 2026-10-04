using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Eitmad.WindowsShell.Controls;
using Button = System.Windows.Controls.Button;
using MenuItem = System.Windows.Controls.MenuItem;
using TextBox = System.Windows.Controls.TextBox;
using OpenFileDialog = Microsoft.Win32.OpenFileDialog;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Furniture;

public partial class FurnitureView : UserControl
{
    public FurnitureView()
    {
        InitializeComponent();
        ViewModel = new FurnitureViewModel();
        DataContext = ViewModel;
    }

    public FurnitureViewModel ViewModel { get; }

    private void FurnitureRowInvoked(object sender, RowInvokedEventArgs eventArgs) =>
        OpenEditor((FurnitureListItem)eventArgs.Item);

    /// <summary>Resolves immutable references before staging an edit and requesting Rust review.</summary>
    private async void OpenEditor(FurnitureListItem item)
    {
        if (!await PrepareEditorAsync(item)) return;
        if (!ViewModel.BeginEdit(item)) return;
        await LoadEditorImageAsync();
        await ReviewAsync(false);
        await Dispatcher.BeginInvoke(FurnitureNameBox.Focus, DispatcherPriority.Input);
    }

    /// <summary>Loads picker references before opening an unsaved definition.</summary>
    private async void AddFurnitureClick(object sender, RoutedEventArgs eventArgs)
    {
        if (!await PrepareEditorAsync()) return;
        ViewModel.BeginCreate();
        await Dispatcher.BeginInvoke(FurnitureNameBox.Focus, DispatcherPriority.Input);
    }

    /// <summary>Imports through Rust and applies the preview only to the same authorized editor.</summary>
    private async void ChooseImageClick(object sender, RoutedEventArgs args) {
        if(bridge is null || !ViewModel.CanEditFields || !ViewModel.IsEditorOpen) return;
        var pickerSession=sessionVersion; var pickerEditor=ViewModel.ImageEditVersion;
        var dialog=new OpenFileDialog { Title="اختر صورة الأثاث", Filter="صور PNG وJPEG|*.png;*.jpg;*.jpeg", CheckFileExists=true, Multiselect=false };
        if(dialog.ShowDialog(Window.GetWindow(this)) != true) return;
        if(pickerSession!=sessionVersion || pickerEditor!=ViewModel.ImageEditVersion || !ViewModel.IsEditorOpen) return;
        ViewModel.InvalidateImageLoad();
        var session=sessionVersion; var editor=ViewModel.ImageEditVersion; ViewModel.IsBusy=true;
        try {
            var images=new CatalogImages.CatalogImageClient(bridge);
            var reference=await images.ImportAsync(Eitmad.Contracts.CatalogImageKind.Furniture,dialog.FileName);
            var image=await images.LoadAsync(reference,2048);
            if(session!=sessionVersion || editor!=ViewModel.ImageEditVersion || !ViewModel.IsEditorOpen) return;
            if(reference is null) ViewModel.ReportImageLoadError(); else ViewModel.SetImportedImage(reference,image);
        }
        catch(Exception e) when(e is Eitmad.Platform.Windows.LocalIpc.EngineIpcException or IOException or InvalidOperationException or ObjectDisposedException) { if(session==sessionVersion) ViewModel.ReportImageLoadError(); }
        finally { if(session==sessionVersion) ViewModel.IsBusy=false; }
    }
    /// <summary>Loads the selected reference without restoring an ended session or changed editor.</summary>
    private async Task LoadEditorImageAsync() {
        if(bridge is null) return;
        var session=sessionVersion; var editor=ViewModel.ImageEditVersion; var reference=ViewModel.EditorImageReference;
        var image=await new CatalogImages.CatalogImageClient(bridge).LoadAsync(reference,2048);
        if(session==sessionVersion && editor==ViewModel.ImageEditVersion && ViewModel.IsEditorOpen) ViewModel.SetImportedImage(reference,image);
    }
    /// <summary>Stages image removal while Rust retains historical revision assets.</summary>
    private void RemoveImageClick(object sender, RoutedEventArgs args) { if(ViewModel.CanEditFields) { ViewModel.InvalidateImageLoad(); ViewModel.SetImportedImage(null,null); } }

    private void OpenRowMenuClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { ContextMenu: { } menu } button)
        {
            menu.PlacementTarget = button;
            menu.IsOpen = true;
            eventArgs.Handled = true;
        }
    }

    private static FurnitureListItem? FurnitureFromMenuItem(object sender) =>
        sender is MenuItem { DataContext: FurnitureListItem item } ? item : null;

    private void EditFurnitureClick(object sender, RoutedEventArgs eventArgs)
    {
        if (FurnitureFromMenuItem(sender) is { } item)
        {
            OpenEditor(item);
        }
    }

    /// <summary>Resolves the source references before opening an unsaved duplicate.</summary>
    private async void DuplicateFurnitureClick(object sender, RoutedEventArgs eventArgs)
    {
        if (FurnitureFromMenuItem(sender) is { } item)
        {
            if (!await PrepareEditorAsync(item)) return;
            ViewModel.DuplicateFurniture(item);
            await LoadEditorImageAsync();
            RestartFeedbackTimer();
            await Dispatcher.BeginInvoke(FurnitureNameBox.Focus, DispatcherPriority.Input);
        }
    }

    /// <summary>Resolves saved references before submitting an archive revision.</summary>
    private async void ArchiveFurnitureClick(object sender, RoutedEventArgs eventArgs)
    {
        if (FurnitureFromMenuItem(sender) is { } item)
        {
            if (!await PrepareEditorAsync(item)) return;
            await SaveAsync(Eitmad.Contracts.FurnitureState.Archived, item);
            RestartFeedbackTimer();
        }
    }

    /// <summary>Advances the wizard only after presentation input and required Rust review succeed.</summary>
    private async void NextStepClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ViewModel.IsStepOne)
        {
            if (!ViewModel.MoveToParts())
            {
                FurnitureNameBox.Focus();
            }

            return;
        }

        if (ViewModel.IsStepTwo)
        {
            if (await ReviewAsync(true)) ViewModel.MoveToVariants();
            return;
        }

        if (ViewModel.IsStepThree)
        {
            ViewModel.MoveToOptions();
            return;
        }

        if (ViewModel.IsStepFour)
        {
            if (await ReviewAsync(true)) ViewModel.MoveToPricing();
            return;
        }

        if (ViewModel.IsStepFive)
        {
            if (FindInvalidTextBox(PricingStep) is { } invalidPrice)
            {
                ViewModel.ReportPricingInputError();
                invalidPrice.Focus();
                return;
            }

            if (await ReviewAsync(true)) ViewModel.MoveToReview();
        }
    }

    private void PreviousStepClick(object sender, RoutedEventArgs eventArgs) => ViewModel.MoveToPreviousStep();

    private void CancelEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();

    /// <summary>Submits a draft through the authority and displays its result.</summary>
    private async void SaveDraftClick(object sender, RoutedEventArgs eventArgs)
    {
        await SaveAsync(Eitmad.Contracts.FurnitureState.Draft);
        RestartFeedbackTimer();
    }

    /// <summary>Submits a complete private definition through the authority.</summary>
    private async void SaveDefinitionClick(object sender, RoutedEventArgs eventArgs)
    {
        await SaveAsync(Eitmad.Contracts.FurnitureState.Active);
        RestartFeedbackTimer();
    }

    private void OpenPartPickerClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.OpenPartPicker();

    }

    private void ClosePartPickerClick(object sender, RoutedEventArgs eventArgs) => ViewModel.ClosePartPicker();

    private void SelectPartClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurniturePartOption part })
        {
            ViewModel.AddPart(part);
        }
    }

    private void RemovePartClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurniturePartUsage usage })
        {
            ViewModel.RemovePart(usage);
        }
    }

    private void AddVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginAddVariant();

    }

    private void EditVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureVariant variant })
        {
            ViewModel.BeginEditVariant(variant);

        }
    }

    private void DuplicateVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureVariant variant })
        {
            ViewModel.DuplicateVariant(variant);
            RestartFeedbackTimer();
        }
    }

    private void RemoveVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureVariant variant })
        {
            ViewModel.RemoveVariant(variant);
        }
    }

    /// <summary>Stages exact variant input and reports unsupported numeric values.</summary>
    private void SaveVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        if (FindInvalidTextBox(VariantDialog) is { } invalid) { ViewModel.Fail("صحّح المقاس غير الصالح."); invalid.Focus(); return; }
        try { ViewModel.SaveVariant(); }
        catch (Exception e) when (e is FormatException or OverflowException) { ViewModel.Fail("أدخل المقاسات بمنزلة عشرية واحدة ضمن النطاق المدعوم."); }
    }

    private void CancelVariantClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelVariantEditor();

    private void AddColorClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginAddColor();

    }

    private void SaveColorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.SaveColor();

    private void CancelColorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelColorEditor();

    private void ToggleColorClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureColorOption color })
        {
            ViewModel.ToggleColor(color);
        }
    }

    private void AddHandleClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginAddHandle();

    }

    private void SaveHandleClick(object sender, RoutedEventArgs eventArgs) => ViewModel.SaveHandle();

    private void CancelHandleClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelHandleEditor();

    private void ToggleHandleClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureHandleOption handle })
        {
            ViewModel.ToggleHandle(handle);
        }
    }

    private void RestartFeedbackTimer()
    {
        Feedback.RestartDuration();
    }

    private static TextBox? FindInvalidTextBox(DependencyObject root)
    {
        for (var index = 0; index < VisualTreeHelper.GetChildrenCount(root); index++)
        {
            var child = VisualTreeHelper.GetChild(root, index);
            if (child is TextBox textBox && Validation.GetHasError(textBox))
            {
                return textBox;
            }

            if (FindInvalidTextBox(child) is { } nested)
            {
                return nested;
            }
        }

        return null;
    }
    private void FeedbackDismissed(object sender, RoutedEventArgs e) => ViewModel.ClearFeedback();
}
