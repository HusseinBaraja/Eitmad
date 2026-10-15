using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Parts;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Features.RawMaterials;
using Eitmad.WindowsShell.Features.Users;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Tests.Rendered;

public sealed partial class WorkflowEnvironmentTests
{
    private sealed record FieldReceipt(string Kind, string Name, string Record);

    private static Customer CreateCustomerThroughFields(MainWindow window, IEngineShellBridge bridge)
    {
        ClickNamed(window.ReceptionistSurface, "ProductsNavButton");
        var view = WpfTestHost.FindByName<SalesCatalogView>(window.ReceptionistSurface, "CatalogContent");
        var model = (SalesCatalogViewModel)view.DataContext;
        Finish(model.LastCatalogOperation);
        WaitField(() => model.VisibleItems.Count >= 2, "Receptionist public catalog");
        CaptureFields(window, "20-receptionist-catalog");
        model.IsReviewingQuotation = true;
        WpfTestHost.CompleteLayout(window);
        ClickAutomation(view, "عميل جديد");
        SetNamedField(view, "CustomerNameInput", "عميل رحلة تحقق " + Guid.NewGuid().ToString("N")[..6]);
        SetNamedField(view, "PhoneInput", "777123456");
        SetField(view, "Address", "عدن، المنصورة، شارع اختبار 42");
        SetField(view, "Notes", "ملاحظة عميل تجريبية\nالاتصال قبل التسليم");
        CaptureFields(window, "16-customer-input");
        ClickAutomation(view, "حفظ العميل");
        WaitField(() => !model.IsCustomerSaveBusy, "customer save");
        Assert.IsNotNull(model.SelectedCustomer, model.CustomerOperationError);
        var customer = ReadFields(bridge, Query.ForCustomerGet(new() { CustomerId = model.SelectedCustomer.Id!.Value })).AsCustomer()!;
        Finish(WaitForCustomerConfirmation(bridge, customer));
        Assert.AreEqual(model.CustomerName, customer.Name);
        Assert.AreEqual(model.Phone, customer.Phone);
        Assert.AreEqual(model.Address, customer.Address);
        Assert.AreEqual(model.Notes, customer.Notes);
        CaptureFields(window, "17-customer-saved");
        return customer;
    }

    // Exercise actual editor bindings and save handlers against the supervised engine.
    // Retain independent read results so a second process run proves durable round trips.
    private static void RunDatabaseFieldJourney(MainWindow window, IEngineShellBridge bridge, string root)
    {
        var proofPath = Path.Combine(root, "database-field-proof.json");
        var receipts = File.Exists(proofPath)
            ? JsonSerializer.Deserialize<List<FieldReceipt>>(File.ReadAllText(proofPath))! : [];
        foreach (var receipt in receipts)
            Assert.AreEqual(receipt.Record, JsonSerializer.Serialize(ReadFieldRecord(bridge, receipt.Kind, receipt.Name)),
                $"Restart changed saved {receipt.Kind} fields: {receipt.Name}");
        Console.WriteLine($"Database field journey: verified {receipts.Count} retained records from prior runs.");

        void Remember(string kind, string name)
        {
            receipts.Add(new(kind, name, JsonSerializer.Serialize(ReadFieldRecord(bridge, kind, name))));
            File.WriteAllText(proofPath, JsonSerializer.Serialize(receipts));
        }
        var suffix = Guid.NewGuid().ToString("N")[..6];
        var materialName = $"خشب تحقق {suffix}";
        var categoryName = $"تصنيف تحقق {suffix}";
        var unitName = $"حزمة تحقق {suffix}";

        ClickNamed(window, "MaterialsNavButton");
        var materials = WpfTestHost.Descendants<RawMaterialsView>(window).Single();
        Finish(materials.ActivateAsync());
        materials.ViewModel.BeginAddCategory();
        SetField(materials, "ReferenceName", categoryName);
        ClickText(materials, "حفظ");
        WaitField(() => !materials.ViewModel.IsReferenceEditorOpen && materials.ViewModel.Categories.Any(item => item.Name == categoryName), "material category save");
        Remember("material-category", categoryName);
        materials.ViewModel.BeginAddUnit();
        SetField(materials, "ReferenceName", unitName);
        SetField(materials, "ReferenceShortName", "حزمة");
        WpfTestHost.FindByAutomationName<ComboBox>(materials, "نوع قياس الوحدة").SelectedValue = UnitDimension.Count;
        SetNamedField(materials, "UnitNumeratorBox", "12");
        SetNamedField(materials, "UnitDenominatorBox", "1");
        CaptureFields(window, "01-unit-input");
        ClickText(materials, "حفظ");
        WaitField(() => !materials.ViewModel.IsReferenceEditorOpen && materials.ViewModel.Units.Any(item => item.Name == unitName), "material unit save");
        Remember("unit", unitName);
        materials.ViewModel.BeginCreate();
        SetNamedField(materials, "EditorNameBox", materialName);
        WpfTestHost.CompleteLayout(window);
        WpfTestHost.FindByAutomationName<ComboBox>(materials, "تصنيف المادة الخام").SelectedValue = materials.ViewModel.Categories.Single(item => item.Name == categoryName).Id;
        WpfTestHost.FindByAutomationName<ComboBox>(materials, "وحدة المادة الخام").SelectedValue = materials.ViewModel.Units.Single(item => item.Name == unitName).Id;
        SetNamedField(materials, "EditorCostBox", "٨٧٥");
        CaptureFields(window, "02-material-input");
        SetNamedField(materials, "EditorCostBox", "8.5");
        ClickNamed(materials, "SaveEditorButton");
        Assert.IsTrue(materials.ViewModel.IsEditorOpen);
        Assert.IsTrue(materials.ViewModel.EditorCostError.Length > 0);
        Assert.IsFalse(ReadFields(bridge, Query.ForMaterialList(new() { Term = materialName, Limit = 100 }))
            .AsMaterials()!.Items.Any(item => item.Name == materialName));
        SetNamedField(materials, "EditorCostBox", "٨٧٥");
        ClickNamed(materials, "SaveEditorButton");
        WaitField(() => !materials.ViewModel.IsEditorOpen || materials.ViewModel.EditorError.Length > 0, "material save");
        Assert.IsFalse(materials.ViewModel.IsEditorOpen, materials.ViewModel.EditorError);
        Finish(materials.ActivateAsync());
        WaitField(() => materials.ViewModel.VisibleMaterials.Any(item => item.Name == materialName), "saved material projection");
        var material = materials.ViewModel.VisibleMaterials.Single(item => item.Name == materialName);
        Assert.AreEqual(875m, material.CurrentCost);
        Assert.AreEqual(categoryName, material.Category);
        SetNamedField(materials, "RawMaterialsSearchBox", materialName);
        WaitField(() => materials.ViewModel.VisibleMaterials.Count == 1, "material table search");
        CaptureFields(window, "03-material-table");
        Remember("material", materialName);

        ClickNamed(window, "PartsNavButton");
        var parts = WpfTestHost.Descendants<PartsView>(window).Single();
        Finish(parts.ActivateAsync());
        WaitField(() => parts.ViewModel.CanInteract && parts.ViewModel.EditorCategoryOptions.Count > 0, "part editor ready");
        parts.ViewModel.BeginCreate();
        var partName = $"جزء تحقق {suffix}";
        SetNamedField(parts, "EditorNameBox", partName);
        SetField(parts, "EditorDescription", "وصف عربي تجريبي\nمرجع PART-42");
        WpfTestHost.FindByName<Expander>(parts, "CategoryCreation").IsExpanded = true;
        SetNamedField(parts, "NewCategoryNameBox", $"أجزاء تحقق {suffix}");
        ClickAutomation(parts, "إضافة فئة الجزء");
        WaitField(() => !parts.ViewModel.IsBusy && parts.ViewModel.EditorCategoryOptions.Any(item => item.Name == $"أجزاء تحقق {suffix}"), "part category save");
        CaptureFields(window, "04-part-category-result");
        CaptureFieldSizes(window, "part-category");
        Assert.AreEqual($"أجزاء تحقق {suffix}", parts.ViewModel.EditorCategory?.Name,
            $"Created category must stay selected. Save error: {parts.ViewModel.EditorError}");
        Remember("part-category", $"أجزاء تحقق {suffix}");
        CaptureFields(window, "04-part-input");
        ClickAutomation(parts, "التالي إلى المواد الخام");
        ClickAutomation(parts, "إضافة مادة خام");
        WaitField(() => parts.ViewModel.FilteredMaterials.Any(item => item.Id == material.Id), "material picker");
        parts.ViewModel.AddMaterial(parts.ViewModel.FilteredMaterials.Single(item => item.Id == material.Id));
        WpfTestHost.CompleteLayout(window);
        SetField(parts, "Quantity", "2");
        CaptureFields(window, "05-part-composition-input");
        ClickAutomation(parts, "التالي إلى المراجعة");
        WaitField(() => parts.ViewModel.CurrentStep == 3, "part cost review");
        Assert.AreEqual("1,750", parts.ViewModel.TotalPartCostLabel);
        ClickAutomation(parts, "حفظ الجزء");
        WaitField(() => !parts.ViewModel.IsEditorOpen, "part save");
        Finish(parts.ActivateAsync());
        WaitField(() => parts.ViewModel.VisibleParts.Any(item => item.Name == partName), "saved part projection");
        SetNamedField(parts, "PartsSearchBox", partName);
        WaitField(() => parts.ViewModel.VisibleParts.Count == 1, "part table search");
        CaptureFields(window, "06-part-table");
        Remember("part", partName);

        RunProductFields(window, bridge, suffix, Remember);
        RunFurnitureFields(window, bridge, suffix, partName, Remember);
        RunUserFields(window, bridge, suffix, Remember);
        Console.WriteLine("Database field journey: material references, material, part, product, furniture and account saved and read back.");
    }

    private static void RunProductFields(MainWindow window, IEngineShellBridge bridge, string suffix, Action<string, string> remember)
    {
        ClickNamed(window, "ProductsNavButton");
        var view = WpfTestHost.Descendants<ProductsView>(window).Single();
        Finish(view.ActivateAsync());
        view.ViewModel.BeginAddCategory();
        SetNamedField(view, "CategoryNameBox", $"منتجات تحقق {suffix}");
        ClickText(view, "حفظ الفئة");
        WaitField(() => !view.ViewModel.IsCategoryEditorOpen && view.ViewModel.Categories.Any(item => item.Name == $"منتجات تحقق {suffix}"), "product category save");
        remember("product-category", $"منتجات تحقق {suffix}");
        view.ViewModel.BeginCreate();
        var name = $"مرتبة تحقق {suffix}";
        SetNamedField(view, "ProductNameBox", name);
        view.ViewModel.EditorCategory = $"منتجات تحقق {suffix}";
        SetField(view, "ShortDescription", "وصف المرتبة التجريبي MODEL-42");
        SetField(view, "Notes", "ملاحظات داخلية تجريبية\nالسطر الثاني");
        view.ViewModel.AddVariant();
        WpfTestHost.CompleteLayout(window);
        SetField(view, "Name", "مفرد 90 × 200");
        SetField(view, "PurchaseCost", "12345");
        CaptureFields(window, "07-product-input");
        ClickAutomation(view, "حفظ المنتج");
        WaitField(() => !view.ViewModel.IsEditorOpen, "product save: " + view.ViewModel.EditorError);
        Finish(view.ActivateAsync());
        WaitField(() => view.ViewModel.VisibleProducts.Any(item => item.Name == name), "saved product projection");
        var saved = ReadFields(bridge, Query.ForProductList(new() { Term = name, Limit = 100 })).AsProducts()!.Items.Single();
        Assert.AreEqual("وصف المرتبة التجريبي MODEL-42", saved.Description);
        Assert.AreEqual("ملاحظات داخلية تجريبية\nالسطر الثاني", saved.Notes);
        Assert.AreEqual("مفرد 90 × 200", saved.Variants.Single().Name);
        Assert.AreEqual(12345L, saved.Variants.Single().PurchaseCostYer);
        SetNamedField(view, "ProductSearchBox", name);
        WaitField(() => view.ViewModel.VisibleProducts.Count == 1, "product table search");
        CaptureFields(window, "08-product-table");
        view.ViewModel.BeginEdit(view.ViewModel.VisibleProducts.Single());
        Assert.AreEqual(saved.Description, view.ViewModel.ShortDescription);
        Assert.AreEqual(saved.Notes, view.ViewModel.Notes);
        CaptureFields(window, "09-product-reopened");
        view.ViewModel.CancelEditor();
        remember("product", name);
    }

    private static void RunFurnitureFields(MainWindow window, IEngineShellBridge bridge, string suffix, string partName, Action<string, string> remember)
    {
        ClickNamed(window, "FurnitureNavButton");
        var view = WpfTestHost.Descendants<FurnitureView>(window).Single();
        Finish(view.ActivateAsync());
        ClickAutomation(view, "إضافة منتج");
        WaitField(() => view.ViewModel.IsEditorOpen, "furniture editor ready");
        var name = $"خزانة تحقق {suffix}";
        SetNamedField(view, "FurnitureNameBox", name);
        SetField(view, "ShortDescription", "وصف خزانة تجريبي MIX-42");
        SetField(view, "InternalNotes", "ملاحظة ورشة داخلية تجريبية");
        CaptureFields(window, "10-furniture-input");
        ClickNamed(view, "NextButton");
        ClickAutomation(view, "إضافة جزء للأثاث");
        WaitField(() => view.ViewModel.FilteredParts.Any(item => item.Name == partName), "furniture part picker");
        view.ViewModel.AddPart(view.ViewModel.FilteredParts.Single(item => item.Name == partName));
        WpfTestHost.CompleteLayout(window);
        SetField(view, "Quantity", "3");
        ClickNamed(view, "NextButton");
        WaitField(() => view.ViewModel.CurrentStep == 3, "furniture part review");
        ClickAutomation(view, "إضافة مقاس");
        SetNamedField(view, "VariantNameBox", "مقاس تحقق");
        SetField(view, "VariantWidth", "123.4");
        SetField(view, "VariantHeight", "210");
        SetField(view, "VariantDepth", "60");
        WpfTestHost.FindByAutomationName<CheckBox>(view, "السماح بتخصيص المقاس").IsChecked = true;
        SetField(view, "MinWidth", "100.1");
        SetField(view, "MinHeight", "200.1");
        SetField(view, "MinDepth", "50.1");
        SetField(view, "MaxWidth", "150.1");
        SetField(view, "MaxHeight", "240.1");
        SetField(view, "MaxDepth", "70.1");
        CaptureFields(window, "11-furniture-size-input");
        CaptureFieldSizes(window, "furniture-size");
        ClickAutomation(view, "حفظ المقاس");
        Assert.IsFalse(view.ViewModel.IsVariantEditorOpen);
        ClickNamed(view, "NextButton");
        ClickAutomation(view, "إضافة لون");
        SetNamedField(view, "ColorNameBox", "أزرق تحقق");
        SetField(view, "ColorPriceAdjustment", "250");
        ClickAutomation(view, "حفظ اللون");
        ClickAutomation(view, "إضافة مقبض");
        SetNamedField(view, "HandleNameBox", "مقبض تحقق");
        SetField(view, "HandlePriceAdjustment", "150");
        ClickAutomation(view, "حفظ المقبض");
        ClickNamed(view, "NextButton");
        WaitField(() => view.ViewModel.CurrentStep == 5, "furniture options review");
        SetField(view, "SellingPriceInput", "9500");
        CaptureFields(window, "12-furniture-price-input");
        ClickNamed(view, "NextButton");
        WaitField(() => view.ViewModel.CurrentStep == 6, "furniture final review");
        ClickAutomation(view, "حفظ الأثاث كمسودة");
        WaitField(() => !view.ViewModel.IsEditorOpen, "furniture save: " + view.ViewModel.EditorError);
        Finish(view.ActivateAsync());
        var saved = ReadFields(bridge, Query.ForFurnitureList(new() { Term = name, Limit = 100 })).AsFurnitures()!.Items.Single();
        Assert.AreEqual("وصف خزانة تجريبي MIX-42", saved.Description);
        Assert.AreEqual("ملاحظة ورشة داخلية تجريبية", saved.Notes);
        Assert.AreEqual(3U, saved.Parts.Single().Quantity);
        Assert.AreEqual(1234U, saved.Variants.Single().Dimensions.WidthMm);
        Assert.AreEqual(1001U, saved.Variants.Single().Customization.Minimum.WidthMm);
        Assert.AreEqual(701U, saved.Variants.Single().Customization.Maximum.DepthMm);
        Assert.AreEqual(9500L, saved.Variants.Single().SellingPriceYer);
        Assert.AreEqual(250L, saved.Colors.Single(item => item.Name == "أزرق تحقق").PriceAdjustmentYer);
        Assert.AreEqual(150L, saved.Handles.Single(item => item.Name == "مقبض تحقق").PriceAdjustmentYer);
        SetNamedField(view, "FurnitureSearchBox", name);
        WaitField(() => view.ViewModel.VisibleFurniture.Count == 1, "furniture table search");
        CaptureFields(window, "13-furniture-table");
        remember("furniture", name);
    }

    private static void RunUserFields(MainWindow window, IEngineShellBridge bridge, string suffix, Action<string, string> remember)
    {
        ClickNamed(window, "UsersNavButton");
        var view = WpfTestHost.Descendants<UsersView>(window).Single();
        Finish(view.ViewModel.LoadAsync());
        ClickAutomation(view, "إضافة مستخدم");
        var name = $"مستخدم تحقق {suffix}";
        SetNamedField(view, "UserNameInput", name);
        SetNamedField(view, "UsernameInput", $"audit{suffix}");
        WpfTestHost.FindByName<PasswordBox>(view, "UserPasswordInput").Password = Guid.NewGuid().ToString("N");
        WpfTestHost.FindByName<ComboBox>(view, "UserRoleInput").SelectedItem = "موظف الاستقبال";
        CaptureFields(window, "14-user-input");
        ClickAutomation(view, "حفظ");
        WaitField(() => !view.ViewModel.IsEditorOpen, "account save");
        Finish(view.ViewModel.LoadAsync());
        var saved = ReadFields(bridge, Query.ForDesktopAccountList(new())).AsDesktopAccounts()!.Accounts.Single(item => item.DisplayName == name);
        Assert.AreEqual($"audit{suffix}", saved.Username);
        Assert.AreEqual(DesktopAccountRole.Receptionist, saved.Role);
        SetNamedField(view, "UsersSearchBox", name);
        CaptureFields(window, "15-users-table");
        remember("account", name);
    }

    private static object ReadFieldRecord(IEngineShellBridge bridge, string kind, string name) => kind switch
    {
        "material-category" => ReadFields(bridge, Query.ForMaterialReferenceList(new())).AsMaterialReferences()!.Categories.Single(item => item.Name == name),
        "unit" => ReadFields(bridge, Query.ForMaterialReferenceList(new())).AsMaterialReferences()!.Units.Single(item => item.Name == name),
        "material" => ReadFields(bridge, Query.ForMaterialList(new() { Term = name, Limit = 100 })).AsMaterials()!.Items.Single(item => item.Name == name),
        "part-category" => ReadFields(bridge, Query.ForPartCategoryList(new() { Limit = 100 })).AsPartCategories()!.Items.Single(item => item.Name == name),
        "part" => ReadFields(bridge, Query.ForPartList(new() { Term = name, Limit = 100 })).AsParts()!.Items.Single(item => item.Part.Name == name).Part,
        "product-category" => ReadFields(bridge, Query.ForProductCategoryList(new() { Limit = 100 })).AsProductCategories()!.Items.Single(item => item.Name == name),
        "product" => ReadFields(bridge, Query.ForProductList(new() { Term = name, Limit = 100 })).AsProducts()!.Items.Single(item => item.Name == name),
        "furniture" => ReadFields(bridge, Query.ForFurnitureList(new() { Term = name, Limit = 100 })).AsFurnitures()!.Items.Single(item => item.Name == name),
        "account" => ReadFields(bridge, Query.ForDesktopAccountList(new())).AsDesktopAccounts()!.Accounts.Single(item => item.DisplayName == name),
        _ => throw new InvalidOperationException("Unknown field receipt kind."),
    };

    private static QueryResult ReadFields(IEngineShellBridge bridge, Query query)
    {
        var task = bridge.QueryAsync(query); Finish(task);
        Assert.AreEqual(CommandOutcomeStatus.Succeeded, task.Result.Outcome.Status, task.Result.Outcome.Payload.Code);
        return task.Result.Outcome.Payload;
    }

    private static void SetField(DependencyObject root, string path, string text)
    {
        WpfTestHost.PumpDispatcher();
        var box = WpfTestHost.Descendants<TextBox>(root).Single(input => input.IsVisible
            && input.GetBindingExpression(TextBox.TextProperty)?.ParentBinding.Path?.Path == path);
        box.BringIntoView(); box.Focus(); box.Text = text;
        box.GetBindingExpression(TextBox.TextProperty)?.UpdateSource();
        WpfTestHost.PumpDispatcher();
    }

    private static void SetNamedField(DependencyObject root, string name, string text)
    {
        if (root is FrameworkElement element) WpfTestHost.CompleteLayout(element);
        var box = WpfTestHost.FindByName<TextBox>(root, name);
        box.BringIntoView(); box.Focus(); box.Text = text;
        box.GetBindingExpression(TextBox.TextProperty)?.UpdateSource();
        WpfTestHost.PumpDispatcher();
    }

    private static void ClickNamed(DependencyObject root, string name) => ClickField(WpfTestHost.FindByName<Button>(root, name));
    private static void ClickAutomation(DependencyObject root, string name)
    {
        if (root is FrameworkElement element) WpfTestHost.CompleteLayout(element);
        var button = WpfTestHost.Descendants<Button>(root).SingleOrDefault(button => button.IsVisible && AutomationProperties.GetName(button) == name);
        if (button is null && Application.Current.Windows.OfType<MainWindow>().FirstOrDefault() is { } window)
            CaptureFields(window, "failure");
        Assert.IsNotNull(button, $"Expected visible form action: {name}");
        ClickField(button);
    }
    private static void ClickText(DependencyObject root, string text) => ClickField(WpfTestHost.Descendants<Button>(root)
        .Single(button => button.IsVisible && (Equals(button.Content, text) || button.Content is TextBlock block && block.Text == text)));
    private static void ClickField(Button button)
    {
        WaitField(() => button.IsEnabled, $"enable action: {AutomationProperties.GetName(button)}");
        button.Focus(); button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.PumpDispatcher();
    }

    private static void WaitField(Func<bool> complete, string stage)
    {
        var watch = Stopwatch.StartNew();
        while (!complete() && watch.Elapsed < TimeSpan.FromSeconds(25)) Finish(Task.Delay(50));
        if (!complete() && Application.Current.Windows.OfType<MainWindow>().FirstOrDefault() is { } window)
            CaptureFields(window, "failure");
        Assert.IsTrue(complete(), $"Database field journey did not complete: {stage}");
    }

    private static void CaptureFields(MainWindow window, string name)
    {
        WpfTestHost.CompleteLayout(window);
        var dpi = VisualTreeHelper.GetDpi(window);
        Console.WriteLine($"Field evidence {name}: window {window.ActualWidth}x{window.ActualHeight} DIP; scaling {dpi.DpiScaleX * 100}%; display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP.");
        WpfTestHost.Capture(window, "database-" + name);
        var directory = Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR");
        if (!string.IsNullOrWhiteSpace(directory)) File.AppendAllText(Path.Combine(directory, "capture-metadata.jsonl"),
            JsonSerializer.Serialize(new { name, widthDip = window.ActualWidth, heightDip = window.ActualHeight,
                scalingPercent = dpi.DpiScaleX * 100, displayWidthDip = SystemParameters.PrimaryScreenWidth,
                displayHeightDip = SystemParameters.PrimaryScreenHeight }) + Environment.NewLine);
    }

    private static void ShowSharedQuotationToManager(MainWindow window, Customer customer)
    {
        var search = window.ManagerTitleBar.SearchBox;
        search.Text = customer.Name;
        search.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(search), 0, Key.Enter)
            { RoutedEvent = Keyboard.KeyDownEvent });
        var model = (Features.Home.HomeViewModel)window.ManagerHome.DataContext;
        Finish(model.LastLoad);
        WaitField(() => model.Activity.Any(row => row.Item.Title == customer.Name && row.Item.Destination == HomeDestination.Quotation),
            "Receptionist quotation visible to Manager");
        CaptureFields(window, "21-manager-shared-quotation");
    }

    private static void CaptureFieldSizes(MainWindow window, string name)
    {
        var left = window.Left;
        window.Width = 720; window.Height = 560;
        CaptureFields(window, name + "-minimum");
        window.Left = 0; window.WindowState = WindowState.Maximized;
        CaptureFields(window, name + "-maximized");
        window.WindowState = WindowState.Normal;
        window.Left = left; window.Width = 1338; window.Height = 753;
        CaptureFields(window, name + "-default");
    }
}
