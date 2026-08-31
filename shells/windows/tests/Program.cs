using System.Globalization;
using System.Threading.Channels;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Operations;
using Eitmad.WindowsShell.Features.Parts;
using Eitmad.WindowsShell.Features.RawMaterials;
using Eitmad.WindowsShell.Layout;

var tests = new ShellScenarios();
tests.StateMappingCoversOperationalContracts();
tests.ReferenceMarkerMapsArabicAndSyncState();
await tests.SaveReferenceMarkerUpdatesOnlyReturnedItem();
tests.EventOrderingRejectsDuplicatesAndStaleSequences();
tests.EventOrderingSerializesConcurrentDelivery();
tests.StaleSnapshotsCannotReplaceNewerState();
await tests.CommandFaultsAreOwned();
await tests.ReconnectionRefreshesWithoutDuplicateSubscriptions();
await tests.ResyncRefreshesRustSnapshots();
await tests.UnsupportedCapabilitiesAvoidRequestTraffic();
await tests.ConfigurationQueryFailureClearsStaleState();
tests.EngineFailureMapsToRecoveryUx();
await tests.ShutdownStopsEngineCleanly();
tests.RtlLayoutIncludesMixedDirectionFixtures();
tests.NativeWindowChromeIsDelegatedToWindows();
tests.DashboardUsesNativeInteractiveControls();
tests.RawMaterialsSearchAndFiltersUpdateVisibleList();
tests.RawMaterialCostsIgnoreTheAmbientCulture();
tests.RawMaterialsActionsRemainNonDestructiveAndEphemeral();
tests.RawMaterialReferencesCanBeManagedInline();
tests.RawMaterialsPageUsesTheDashboardVisualSystem();
tests.PartsSearchAndFiltersUpdateVisibleList();
tests.PartsActionsRemainNonDestructiveAndEphemeral();
tests.PartsPageMatchesTheRawMaterialsVisualSystem();
tests.DashboardVisualSystemIsConsistentAndRtlSafe();
tests.ResponsiveLayoutSelectsStableBreakpoints();
tests.DashboardReflowsInsteadOfScalingAFixedCanvas();
tests.SidebarFooterCardIsRemoved();
tests.ToolbarExpandsSearchAfterActionButtons();
tests.SearchBoxArabicTextAlignsToTheRtlEdge();
tests.LatestQuotesTitleAlignsToTheRtlEdge();
tests.SidebarNavigationKeepsArabicLabelsAtTheRtlEdge();
tests.NotificationRowsKeepArabicTextBesideIcons();
tests.WorkDistributionRowsKeepProgressBesideIcons();
tests.WorkDistributionFooterAlignsToTheLeft();
tests.SelectedSidebarNavigationKeepsWhiteHoverContent();
tests.ShellOwnershipRulesAreEnforced();
Console.WriteLine("Windows shell scenarios passed.");

internal sealed class ShellScenarios
{
    private static readonly string RepositoryRoot = FindRepositoryRoot();

    public void StateMappingCoversOperationalContracts()
    {
        var model = new OperationsViewModel();
        model.ObserveSync(new SyncStatus
        {
            Kind = SyncStatusKind.Syncing,
            Payload = new SyncStatusPayload { Completed = 40, Total = 100 },
        });
        model.ObserveUpdate(new UpdateState
        {
            Kind = UpdateStateKind.Downloading,
            Payload = new UpdateStatePayload { Version = "1.4.0", ProgressBps = 6250 },
        });
        model.ObserveJob(new BackgroundJobStatus
        {
            JobId = Guid.NewGuid(),
            JobKind = "export",
            State = BackgroundJobState.Running,
            CompletedUnits = 3,
            TotalUnits = 12,
            Scope = Scope(),
        }, 10);
        model.ObserveNotification(new Notification
        {
            NotificationId = Guid.NewGuid(),
            Scope = Scope(),
            Severity = NotificationSeverity.Success,
            MessageId = ProtocolIds.MessageIds.EitmadNotificationSyncCompleteV1,
            Parameters = [],
        }, 11);

        Assert.Equal("تجري الآن", model.SyncCard.Value, "sync state mapping");
        Assert.Equal(0.4d, model.SyncCard.Progress, "sync progress mapping");
        Assert.Equal("تنزيل", model.UpdateCard.Value, "update state mapping");
        Assert.Equal(0.625d, model.UpdateCard.Progress, "update progress mapping");
        Assert.Equal("تصدير البيانات", model.Jobs.Single().Title, "job mapping");
        Assert.Equal("اكتملت المزامنة", model.Activity.Single().Title, "notification mapping");
    }

    public void ReferenceMarkerMapsArabicAndSyncState()
    {
        var model = new OperationsViewModel();
        model.ObserveReferenceMarkers(new ReferenceMarkerPage
        {
            Items =
            [
                new ReferenceMarker
                {
                    Id = Guid.Parse("8b8ab1ab-731b-46f5-926a-3b5b2f8f6310"),
                    Label = "مرجع REF-١٢",
                    Revision = 4,
                    Scope = Scope(),
                    SyncState = ReferenceMarkerSyncState.Pending,
                    UpdatedAt = 1_800_000_000_000,
                },
            ],
        });

        Assert.Equal("مرجع REF-١٢", model.ReferenceMarkers.Single().Label, "mixed-direction label preserved");
        Assert.Equal("بانتظار المزامنة", model.ReferenceMarkers.Single().SyncState, "pending sync state localized");
        Assert.Equal("اللقطة محدّثة", model.ReferenceMarkerStatus, "marker snapshot status");
    }

    public async Task SaveReferenceMarkerUpdatesOnlyReturnedItem()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        var otherMarkerId = Guid.Parse("9c9bc2bc-842c-57f6-a37b-4c6c3f9f7421");
        model.ObserveReferenceMarkers(new ReferenceMarkerPage
        {
            Items =
            [
                new ReferenceMarker
                {
                    Id = Guid.Parse("8b8ab1ab-731b-46f5-926a-3b5b2f8f6310"),
                    Label = "مرجع قديم",
                    Revision = 1,
                    Scope = Scope(),
                    SyncState = ReferenceMarkerSyncState.Confirmed,
                    UpdatedAt = 1_800_000_000_000,
                },
                new ReferenceMarker
                {
                    Id = otherMarkerId,
                    Label = "مرجع محفوظ",
                    Revision = 3,
                    Scope = Scope(),
                    SyncState = ReferenceMarkerSyncState.Confirmed,
                    UpdatedAt = 1_800_000_000_000,
                },
            ],
        });
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        model.ReferenceMarkerLabel = "مرجع محدّث";

        model.SaveReferenceMarkerCommand.Execute(null);

        await Eventually(() => model.ReferenceMarkers.Any(marker => marker.Label == "مرجع محدّث"));
        Assert.Equal(2, model.ReferenceMarkers.Count, "upsert preserves other marker items");
        Assert.Equal("مرجع محفوظ", model.ReferenceMarkers.Single(marker => marker.Id == otherMarkerId).Label, "unrelated marker preserved");
    }

    public void EventOrderingRejectsDuplicatesAndStaleSequences()
    {
        var gate = new EventOrderGate();
        var subscription = Guid.NewGuid();
        Assert.True(gate.TryAccept("sync", Event(subscription, 3)), "first event accepted");
        Assert.False(gate.TryAccept("sync", Event(subscription, 3)), "duplicate event rejected");
        Assert.False(gate.TryAccept("sync", Event(subscription, 2)), "older sequence rejected");
        Assert.True(gate.TryAccept("sync", Event(subscription, 4)), "new sequence accepted");
        Assert.True(gate.TryAccept("sync", Event(Guid.NewGuid(), 1)), "replacement subscription sequence accepted");
    }

    public void EventOrderingSerializesConcurrentDelivery()
    {
        var gate = new EventOrderGate();
        var subscription = Guid.NewGuid();

        Parallel.For(1, 1_001, sequence => gate.TryAccept("sync", Event(subscription, sequence)));

        Assert.False(gate.TryAccept("sync", Event(subscription, 1_000)), "concurrent delivery retains highest sequence");
    }

    public void StaleSnapshotsCannotReplaceNewerState()
    {
        var model = new OperationsViewModel();
        model.ObserveConfiguration(Configuration(7, "ar-YE"), 200);
        model.ObserveConfiguration(Configuration(8, "en-US"), 200);
        model.ObserveConfiguration(Configuration(7, "ar-YE"), 300);
        model.ObserveConfiguration(Configuration(9, "ar-YE"), 100);
        Assert.Equal(8L, model.ConfigRevision, "equal timestamp accepted while stale state is rejected");
        Assert.Equal("en-US", model.SelectedLocale, "stale locale rejected");

        model.ObserveSync(new SyncStatus { Kind = SyncStatusKind.Current, Payload = new SyncStatusPayload() }, 200);
        model.ObserveSync(new SyncStatus { Kind = SyncStatusKind.Failed, Payload = new SyncStatusPayload { Reason = "old" } }, 100);
        Assert.Equal("محدّث", model.SyncCard.Value, "stale sync state rejected");
    }

    public async Task CommandFaultsAreOwned()
    {
        var handled = new TaskCompletionSource<Exception>(TaskCreationOptions.RunContinuationsAsynchronously);
        var expected = new InvalidOperationException("Synthetic command failure.");
        var command = new AsyncCommand(
            () => Task.FromException(expected),
            onError: error => handled.TrySetResult(error));

        command.Execute(null);

        Assert.Equal(expected, await handled.Task.WaitAsync(TimeSpan.FromSeconds(3)), "command failure routed to owner");
        await Eventually(() => command.CanExecute(null));
    }

    public async Task ReconnectionRefreshesWithoutDuplicateSubscriptions()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();
        engine.Connect();
        await Eventually(() => engine.QueryCount >= 4 && engine.SubscriptionCount == 4);

        engine.Disconnect();
        engine.Connect();
        await Eventually(() => engine.QueryCount >= 8);
        Assert.Equal(4, engine.SubscriptionCount, "reconnect reuses supervised subscriptions");
        Assert.False(model.ShowConnectionBanner, "fresh snapshots clear reconnect banner");
    }

    public async Task ResyncRefreshesRustSnapshots()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();
        engine.Connect();
        await Eventually(() => engine.QueryCount >= 4);
        engine.SignalResync(ProtocolIds.Subscriptions.EitmadSyncStatusSubscribeV1);
        await Eventually(() => engine.QueryCount >= 8);
        Assert.False(model.ShowConnectionBanner, "resync completes with current snapshots");
    }

    public async Task UnsupportedCapabilitiesAvoidRequestTraffic()
    {
        var engine = new FakeEngine
        {
            SupportedCapabilities = new HashSet<string>
            {
                ProtocolIds.Capabilities.EitmadCapabilityConfigV1,
                ProtocolIds.Capabilities.EitmadCapabilityReferenceMarkerV1,
            },
        };
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();
        engine.Connect();
        await Eventually(() => engine.QueryCount == 2 && engine.SubscriptionCount == 2);
        Assert.False(engine.WasQueried(Query.SyncGetStatusKind), "unsupported sync query omitted");
        Assert.False(engine.WasQueried(Query.UpdateGetStateKind), "unsupported update query omitted");
        Assert.Equal("غير متاحة", model.SyncCard.Value, "unsupported sync is explicit");
        Assert.Equal("غير متاحة", model.UpdateCard.Value, "unsupported update is explicit");
    }

    public async Task ConfigurationQueryFailureClearsStaleState()
    {
        var engine = new FakeEngine { FailConfigurationQuery = true };
        var model = new OperationsViewModel();
        model.ObserveConfiguration(Configuration(4, "en-US"));
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();

        engine.Connect();

        await Eventually(() => engine.QueryCount >= 4);
        Assert.Equal(-1L, model.ConfigRevision, "failed configuration query clears revision");
        Assert.Equal(0, model.Configuration.Count, "failed configuration query clears entries");
        Assert.Equal("غير متاح", model.ConfigurationRevisionLabel, "failed configuration query is visible");
    }

    public void EngineFailureMapsToRecoveryUx()
    {
        var model = new OperationsViewModel();
        model.ObserveSupervision(new EngineSupervisionSnapshot(
            EngineSupervisionState.RestartExhausted,
            4,
            3,
            EngineIpcHealthState.Unavailable,
            null,
            null,
            new EngineExitOutcome(24, false, false)));
        Assert.True(model.RestartExhausted, "restart exhaustion exposed");
        Assert.True(model.ShowConnectionBanner, "failure banner visible");
        Assert.Equal("Danger", model.ConnectionTone, "failure banner tone");
    }

    public async Task ShutdownStopsEngineCleanly()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();
        await coordinator.StopAsync();
        Assert.Equal(1, engine.StopCount, "shutdown delegates one clean stop");
    }

    /// <summary>Verifies the root RTL metadata and mixed-direction fixtures.</summary>
    public void RtlLayoutIncludesMixedDirectionFixtures()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));
        Assert.Contains("FlowDirection=\"RightToLeft\"", xaml, "root RTL layout");
        Assert.Contains("Language=\"ar-YE\"", xaml, "Arabic language metadata");
        Assert.False(xaml.Contains("Text=\"العربية (اليمن)", StringComparison.Ordinal), "visible brand header omits the locale label");
        Assert.Contains("Grid.Column=\"1\" FlowDirection=\"RightToLeft\" HorizontalAlignment=\"Stretch\" VerticalAlignment=\"Center\"", xaml, "brand text area fills the header column");
        Assert.Contains("HorizontalAlignment=\"Left\" TextAlignment=\"Right\"", xaml, "RTL brand text is anchored at the physical right edge");
        Assert.Contains("FlowDirection=\"LeftToRight\"", xaml, "mixed-direction isolation");
        Assert.Contains("CNC-04", xaml, "Arabic and English workshop fixture");
        Assert.Contains("Windows / Rust", xaml, "mixed product fixture");
        Assert.Contains("مرجع REF-١٢", xaml, "mixed reference marker fixture");
    }

    /// <summary>Verifies that Windows owns the native window frame.</summary>
    public void NativeWindowChromeIsDelegatedToWindows()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));
        var codeBehind = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml.cs"));
        var theme = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "Resources", "OperationsTheme.xaml"));

        Assert.Contains("WindowStyle=\"SingleBorderWindow\"", xaml, "Windows owns the non-client frame");
        Assert.Contains("ResizeMode=\"CanResize\"", xaml, "Windows owns standard resize behavior");
        Assert.False(xaml.Contains("ChromeButton", StringComparison.Ordinal), "custom caption buttons are absent");
        Assert.False(theme.Contains("ChromeButton", StringComparison.Ordinal), "custom caption style is absent");
        Assert.False(codeBehind.Contains("TitleBarMouseDown", StringComparison.Ordinal), "custom title-bar dragging is absent");
        Assert.False(codeBehind.Contains("MinimizeClick", StringComparison.Ordinal), "custom caption handlers are absent");
    }

    /// <summary>Verifies that dashboard preview actions use native controls.</summary>
    public void DashboardUsesNativeInteractiveControls()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));
        var codeBehind = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml.cs"));

        Assert.Contains("Resources/ShowroomHero.png", xaml, "standalone showroom asset");
        Assert.False(xaml.Contains("DashboardReference.png", StringComparison.Ordinal), "reference screenshot is not rendered");
        Assert.Contains("Click=\"NavigationClick\"", xaml, "sidebar navigation buttons");
        Assert.Contains("Click=\"OpenPreviewPanelClick\"", xaml, "new quotation preview action");
        Assert.Contains("KeyDown=\"SearchKeyDown\"", xaml, "search keyboard interaction");
        Assert.Contains("PreviewSubmitClick", codeBehind, "preview form validation");
        Assert.Contains("الحفظ معطل في وضع المعاينة", codeBehind, "preview cannot claim durable storage");
    }

    /// <summary>Verifies immediate search and combined category/status filtering.</summary>
    public void RawMaterialsSearchAndFiltersUpdateVisibleList()
    {
        var model = new RawMaterialsViewModel();

        Assert.Equal(4, model.VisibleMaterials.Count, "raw-material preview starts with all fixtures");
        model.SearchText = "mdf";
        Assert.Equal(1, model.VisibleMaterials.Count, "search is case-insensitive");
        Assert.Equal("لوح MDF سماكة 18 مم", model.VisibleMaterials.Single().Name, "search returns the expected board");

        model.SearchText = string.Empty;
        model.SearchText = "زان";
        Assert.Equal("خشب زان مجفف", model.VisibleMaterials.Single().Name, "Arabic search returns the expected timber");

        model.SearchText = "اخشاب";
        Assert.Equal(2, model.VisibleMaterials.Count, "Arabic search folds alef variants in natural-timber categories");

        model.SearchText = string.Empty;
        model.SelectedCategory = "أخشاب طبيعية";
        model.SelectedStatus = RawMaterialsViewModel.ArchivedStatus;
        Assert.Equal(1, model.VisibleMaterials.Count, "category and archived filters combine");
        Assert.True(model.VisibleMaterials.Single().IsArchived, "archived filter excludes active timber");

        model.SelectedStatus = RawMaterialsViewModel.ActiveStatus;
        Assert.Equal(1, model.VisibleMaterials.Count, "active filter excludes archived timber");
        Assert.Equal("خشب زان مجفف", model.VisibleMaterials.Single().Name, "active timber remains visible");
    }

    /// <summary>Verifies local create/edit/duplicate/archive behavior without permanent deletion.</summary>
    public void RawMaterialsActionsRemainNonDestructiveAndEphemeral()
    {
        var model = new RawMaterialsViewModel();
        var originalCount = model.VisibleMaterials.Count;
        var board = model.VisibleMaterials.Single(item => item.Name == "لوح MDF سماكة 18 مم");
        Assert.Equal("ر.س. 25,000", board.CostLabel, "raw-material costs use the Arabic Saudi Riyal prefix");

        model.Archive(board);
        Assert.Equal(originalCount, model.VisibleMaterials.Count, "archive keeps the material in the all-status list");
        Assert.True(board.IsArchived, "archive marks the row visually inactive");

        var timber = model.VisibleMaterials.Single(item => item.Name == "خشب زان مجفف");
        var duplicate = model.Duplicate(timber);
        Assert.True(model.IsEditorOpen, "duplicate opens the edit page");
        Assert.True(duplicate.Name.EndsWith("نسخة", StringComparison.Ordinal), "duplicate has a clear local name");
        Assert.Equal(originalCount + 1, model.VisibleMaterials.Count, "duplicate adds one local row");

        model.CancelEditor();
        model.BeginCreate();
        model.EditorName = "قماش صنعاء Fabric";
        model.EditorCategory = "أقمشة";
        model.EditorUnit = "متر";
        model.EditorCost = 4_200m;
        Assert.True(model.SaveEditor(), "valid create form updates preview state");
        Assert.Equal(originalCount + 2, model.VisibleMaterials.Count, "create adds one local row");
    }

    /// <summary>Verifies inline category and unit creation, editing, selection, and archival.</summary>
    public void RawMaterialReferencesCanBeManagedInline()
    {
        var model = new RawMaterialsViewModel();

        model.BeginAddCategory();
        model.ReferenceName = "إكسسوارات";
        Assert.True(model.SaveReferenceEditor(), "a category can be created without leaving the material editor");
        Assert.Equal("إكسسوارات", model.EditorCategory, "a new category is selected automatically");
        Assert.True(model.ActiveCategories.Any(item => item.Name == "إكسسوارات"), "a new category appears in the dropdown");

        model.BeginAddUnit();
        model.ReferenceName = "متر مربع";
        model.ReferenceShortName = "m²";
        Assert.True(model.SaveReferenceEditor(), "a unit can be created without leaving the material editor");
        Assert.Equal("متر مربع", model.EditorUnit, "a new unit is selected automatically");
        Assert.Equal("لوح", model.ActiveUnits.First().DisplayLabel, "identical unit names are not repeated in dropdown labels");
        var squareMeter = model.ActiveUnits.Single(item => item.Name == "متر مربع");
        Assert.Equal("متر مربع — m²", squareMeter.DisplayLabel, "unit dropdown labels include the short name");

        model.BeginManageUnits();
        model.BeginEditReference(squareMeter);
        model.ReferenceShortName = "م²";
        Assert.True(model.SaveReferenceEditor(), "an existing unit can be edited from the small manager");
        Assert.Equal("متر مربع — م²", squareMeter.DisplayLabel, "the edited short name updates in place");
        Assert.True(model.IsReferenceManagerOpen, "saving an edit returns to the unit manager");

        model.ArchiveReference(squareMeter);
        Assert.True(squareMeter.IsArchived, "archive keeps the unit record and marks it inactive");
        Assert.False(model.ActiveUnits.Contains(squareMeter), "archived units are no longer selectable for new materials");
        Assert.False(model.IsReferenceEditorOpen, "archive does not open another dialog");
    }

    /// <summary>Verifies raw-material amounts keep the selected Latin-digit presentation.</summary>
    public void RawMaterialCostsIgnoreTheAmbientCulture()
    {
        var originalCulture = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("ar-YE");
            var model = new RawMaterialsViewModel();
            var board = model.VisibleMaterials.Single(item => item.Name == "لوح MDF سماكة 18 مم");

            Assert.Equal("25,000", board.CostAmountLabel, "raw-material amounts use deterministic Latin digits and grouping");
        }
        finally
        {
            CultureInfo.CurrentCulture = originalCulture;
        }
    }

    /// <summary>Verifies the required Arabic-first table and preview ownership boundaries.</summary>
    public void RawMaterialsPageUsesTheDashboardVisualSystem()
    {
        var shell = Path.Combine(RepositoryRoot, "shells", "windows");
        var xaml = File.ReadAllText(Path.Combine(shell, "Features", "RawMaterials", "RawMaterialsView.xaml"));
        var rawMaterialItem = File.ReadAllText(Path.Combine(shell, "Features", "RawMaterials", "RawMaterialListItem.cs"));
        var mainWindow = File.ReadAllText(Path.Combine(shell, "MainWindow.xaml"));
        var theme = File.ReadAllText(Path.Combine(shell, "Resources", "OperationsTheme.xaml"));

        Assert.Contains("Text=\"المواد الخام\"", xaml, "raw-material page heading");
        Assert.Contains("Text=\"إضافة مادة خام\"", xaml, "primary create action");
        Assert.Contains("Grid.Column=\"1\" FlowDirection=\"RightToLeft\" HorizontalAlignment=\"Right\" Margin=\"24,0,0,0\"", xaml, "raw-material header is anchored at the physical right edge");
        Assert.Contains("HorizontalAlignment=\"Left\" TextAlignment=\"Right\"", xaml, "raw-material heading lines use RTL-safe physical alignment");
        Assert.Contains("Text=\"اسم المادة\"", xaml, "material-name column");
        Assert.Contains("Text=\"التكلفة الحالية\"", xaml, "current-cost column");
        Assert.Contains("التكلفة الحالية (ر.س.)", xaml, "Saudi Riyal editor label");
        Assert.Contains("CurrencyLabel => \"ر.س.\"", rawMaterialItem, "raw-material cost formatter uses Arabic Saudi Riyal");
        Assert.Contains("Text=\"{Binding CurrencyLabel}\"", xaml, "raw-material cost cell renders a separate currency element");
        Assert.Contains("Text=\"{Binding CostAmountLabel}\"", xaml, "raw-material cost cell renders a separate amount element");
        Assert.False(rawMaterialItem.Contains("ر.ي.", StringComparison.Ordinal), "raw-material formatter has no Yemeni Riyal marker");
        Assert.Contains("Text=\"ر.س.\" Style=\"{StaticResource MetricValue}\"", mainWindow, "dashboard amount uses Saudi Riyal prefix");
        Assert.Contains("Text=\"1,245,780\" Style=\"{StaticResource MetricValue}\"", mainWindow, "dashboard amount renders the number separately");
        Assert.False(mainWindow.Contains(" ر.س\"", StringComparison.Ordinal), "dashboard amounts do not suffix the currency");
        Assert.Contains("x:Key=\"PrimaryButton\"", theme, "primary action uses the shared button style");
        Assert.Contains("TextElement.Foreground=\"{Binding Foreground, RelativeSource={RelativeSource TemplatedParent}}\"", theme, "dark button content inherits white foreground");
        Assert.Contains("BorderBrush\" Value=\"#B79A80\"", xaml, "secondary button keeps a visible border");
        Assert.Contains("SnapsToDevicePixels\" Value=\"True\"", xaml, "secondary button border is pixel snapped");
        Assert.Contains("Header=\"تعديل\"", xaml, "compact edit action");
        Assert.Contains("Header=\"تكرار\"", xaml, "compact duplicate action");
        Assert.Contains("Header=\"أرشفة\"", xaml, "compact archive action");
        Assert.False(xaml.Contains("Header=\"حذف\"", StringComparison.Ordinal), "raw-material page has no permanent delete action");
        Assert.Contains("x:Key=\"RawMaterialsComboBox\"", xaml, "selectors use a custom modern control template");
        Assert.Contains("+ إضافة تصنيف جديد", xaml, "category dropdown exposes inline creation");
        Assert.Contains("+ إضافة وحدة جديدة", xaml, "unit dropdown exposes inline creation");
        Assert.Contains("إدارة التصنيفات", xaml, "category dropdown exposes its small manager");
        Assert.Contains("إدارة الوحدات", xaml, "unit dropdown exposes its small manager");
        Assert.Contains("ItemsSource=\"{Binding ManagedReferences}\"", xaml, "both managers share one compact row pattern");
        Assert.Contains("Content=\"أرشفة\"", xaml, "reference management archives instead of deleting");
        Assert.False(xaml.Contains("Content=\"حذف\"", StringComparison.Ordinal), "reference management has no permanent delete button");
        Assert.Contains("x:Key=\"RawMaterialsTextInput\"", xaml, "editor fields use a named input style");
        var textInputStyleStart = xaml.IndexOf("x:Key=\"RawMaterialsTextInput\"", StringComparison.Ordinal);
        var comboStyleStart = xaml.IndexOf("x:Key=\"RawMaterialsComboBoxItem\"", StringComparison.Ordinal);
        Assert.True(textInputStyleStart >= 0 && comboStyleStart > textInputStyleStart, "text input style precedes selector styles");
        var textInputStyle = xaml[textInputStyleStart..comboStyleStart];
        Assert.Contains("Property=\"VerticalContentAlignment\" Value=\"Center\"", textInputStyle, "editor text is vertically centered");
        Assert.Contains("VerticalAlignment=\"Center\"", textInputStyle, "text host is centered inside the input chrome");
        Assert.Contains("VerticalContentAlignment=\"{TemplateBinding VerticalContentAlignment}\"", textInputStyle, "text host uses the editor alignment");
        Assert.Contains("x:Name=\"PART_Popup\"", xaml, "selector popup is owned by the page visual system");
        Assert.Contains("x:Key=\"RawMaterialsContextMenu\"", xaml, "row actions use the matching modern popup surface");
        Assert.Contains("FlowDirection=\"LeftToRight\" Style=\"{StaticResource RawMaterialsContextMenu}\" Placement=\"Right\" HorizontalOffset=\"6\"", xaml, "row-action popup placement is isolated from RTL mirroring");
        Assert.Contains("Property=\"FlowDirection\" Value=\"RightToLeft\"", xaml, "Arabic row-action labels keep RTL text direction");
        Assert.Contains("Binding IsArchived", xaml, "archived rows have an inactive visual trigger");
        Assert.False(xaml.Contains("وضع المعاينة —", StringComparison.Ordinal), "raw-material list has no preview notification banner");
        Assert.Contains("rawMaterials:RawMaterialsView", mainWindow, "sidebar destination hosts the raw-material page");
    }

    /// <summary>Verifies immediate search and combined category/status filtering for parts.</summary>
    public void PartsSearchAndFiltersUpdateVisibleList()
    {
        var model = new PartsViewModel();

        Assert.Equal(4, model.VisibleParts.Count, "parts preview starts with all fixtures");
        model.SearchText = "wardrobe";
        Assert.Equal(1, model.VisibleParts.Count, "English search returns the mixed-direction example");
        Assert.Equal("Wardrobe Side Panel", model.VisibleParts.Single().Name, "search returns the expected part");

        model.SearchText = "خزانه";
        Assert.Equal(1, model.VisibleParts.Count, "Arabic search folds taa marbuta in part categories");

        model.SearchText = string.Empty;
        model.SelectedCategory = "أبواب";
        model.SelectedStatus = PartsViewModel.ArchivedStatus;
        Assert.Equal(1, model.VisibleParts.Count, "category and archived filters combine");
        Assert.True(model.VisibleParts.Single().IsArchived, "archived filter excludes active parts");

        model.SelectedStatus = PartsViewModel.ActiveStatus;
        Assert.Equal(1, model.VisibleParts.Count, "active filter excludes archived parts");
        Assert.False(model.VisibleParts.Single().IsArchived, "active door part remains visible");
    }

    /// <summary>Verifies local create, edit, duplicate, and archive behavior without deletion.</summary>
    public void PartsActionsRemainNonDestructiveAndEphemeral()
    {
        var model = new PartsViewModel();
        var originalCount = model.VisibleParts.Count;
        var wardrobePanel = model.VisibleParts.Single(item => item.Name == "Wardrobe Side Panel");
        Assert.Equal("9,450 YER", wardrobePanel.CostLabel, "example cost uses the requested ISO currency suffix");
        Assert.Equal("3 Products", wardrobePanel.UsedInLabel, "example usage count matches the requested row");

        model.Archive(wardrobePanel);
        Assert.Equal(originalCount, model.VisibleParts.Count, "archive keeps the part in the all-status list");
        Assert.True(wardrobePanel.IsArchived, "archive marks the row visually inactive");

        var shelf = model.VisibleParts.Single(item => item.Name == "رف داخلي قابل للتعديل");
        var duplicate = model.Duplicate(shelf);
        Assert.True(model.IsEditorOpen, "duplicate opens the edit page");
        Assert.True(duplicate.Name.EndsWith("نسخة", StringComparison.Ordinal), "duplicate has a clear local name");
        Assert.Equal(originalCount + 1, model.VisibleParts.Count, "duplicate adds one local row");

        model.CancelEditor();
        model.BeginCreate();
        model.EditorName = "واجهة درج صغيرة";
        model.EditorCategory = "أدراج";
        model.EditorCost = 2_750m;
        model.EditorUsedInCount = 2;
        Assert.True(model.SaveEditor(), "valid create form updates preview state");
        Assert.Equal(originalCount + 2, model.VisibleParts.Count, "create adds one local row");
    }

    /// <summary>Verifies Arabic-first labels, required columns, actions, and page navigation.</summary>
    public void PartsPageMatchesTheRawMaterialsVisualSystem()
    {
        var shell = Path.Combine(RepositoryRoot, "shells", "windows");
        var xaml = File.ReadAllText(Path.Combine(shell, "Features", "Parts", "PartsView.xaml"));
        var mainWindow = File.ReadAllText(Path.Combine(shell, "MainWindow.xaml"));
        var codeBehind = File.ReadAllText(Path.Combine(shell, "MainWindow.xaml.cs"));

        Assert.Contains("Text=\"الأجزاء\"", xaml, "parts page heading");
        Assert.Contains("Text=\"إضافة جزء\"", xaml, "primary create action");
        Assert.Contains("Text=\"البحث\"", xaml, "search control label");
        Assert.Contains("Text=\"الفئة\"", xaml, "category filter and column label");
        Assert.Contains("Text=\"الحالة\"", xaml, "status filter and column label");
        Assert.Contains("Text=\"اسم الجزء\"", xaml, "part-name column");
        Assert.Contains("Text=\"التكلفة\"", xaml, "cost column");
        Assert.Contains("Text=\"مستخدم في\"", xaml, "used-in column");
        Assert.Contains("Text=\"الإجراءات\"", xaml, "actions column");
        Assert.Contains("Header=\"تعديل\"", xaml, "compact edit action");
        Assert.Contains("Header=\"تكرار\"", xaml, "compact duplicate action");
        Assert.Contains("Header=\"أرشفة\"", xaml, "compact archive action");
        Assert.False(xaml.Contains("Header=\"حذف\"", StringComparison.Ordinal), "parts page has no permanent delete action");
        Assert.Contains("x:Key=\"PartsComboBox\"", xaml, "filters use page-owned modern selectors");
        Assert.Contains("x:Key=\"PartsContextMenu\"", xaml, "row actions use the matching popup surface");
        Assert.Contains("Placement=\"MousePoint\"", xaml, "row-action popup follows the clicked action point");
        Assert.Contains("Binding IsArchived", xaml, "archived rows have an inactive visual trigger");
        Assert.Contains("parts:PartsView", mainWindow, "sidebar destination hosts the parts page");
        Assert.Contains("x:Name=\"PartsNavButton\"", mainWindow, "parts navigation item can own selected state");
        Assert.Contains("destination == \"القطع\"", codeBehind, "parts destination selects the dedicated page");
        Assert.Contains("OpenPartsFromActionClick", codeBehind, "dashboard shortcut opens the parts page");
    }

    /// <summary>Verifies shared icons and explicit RTL layout boundaries.</summary>
    public void DashboardVisualSystemIsConsistentAndRtlSafe()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));
        var app = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "App.xaml"));
        var icons = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "Resources", "OperationsIcons.xaml"));

        Assert.Contains("Resources/OperationsIcons.xaml", app, "vector icon resources are loaded");
        Assert.Contains("x:Key=\"IconHome\"", icons, "sidebar uses repository-owned vector geometry");
        Assert.Contains("x:Key=\"IconSearch\"", icons, "toolbar search icon is available");
        Assert.False(xaml.Contains("FontFamily=\"Segoe Fluent Icons\"", StringComparison.Ordinal), "dashboard does not depend on font-code glyphs");
        Assert.Contains("x:Name=\"LatestQuotesHeader\"", xaml, "latest quotations header has an explicit RTL layout");
        Assert.Contains("x:Name=\"NotificationsCard\"", xaml, "notification card has an explicit RTL layout");
        Assert.Contains("x:Name=\"ToolbarLayout\"", xaml, "toolbar uses the shared responsive boundary");
    }

    /// <summary>Verifies the shared shell breakpoint policy at its exact boundaries.</summary>
    public void ResponsiveLayoutSelectsStableBreakpoints()
    {
        Assert.Equal(ResponsiveLayoutMode.Compact, ResponsiveLayout.ResolveMode(719), "narrow windows use the compact rail");
        Assert.Equal(ResponsiveLayoutMode.Compact, ResponsiveLayout.ResolveMode(899), "compact mode includes its upper edge");
        Assert.Equal(ResponsiveLayoutMode.Standard, ResponsiveLayout.ResolveMode(900), "standard mode starts at 900 DIPs");
        Assert.Equal(ResponsiveLayoutMode.Standard, ResponsiveLayout.ResolveMode(1599), "standard mode includes its upper edge");
        Assert.Equal(ResponsiveLayoutMode.Wide, ResponsiveLayout.ResolveMode(1600), "wide mode starts at 1600 DIPs");
    }

    /// <summary>Verifies that the dashboard uses reflow and overflow instead of whole-page scaling.</summary>
    public void DashboardReflowsInsteadOfScalingAFixedCanvas()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));

        Assert.False(xaml.Contains("<Viewbox Stretch=\"Uniform\">", StringComparison.Ordinal), "the root dashboard is not a scaled fixed canvas");
        Assert.False(xaml.Contains("Width=\"1670\" Height=\"939\"", StringComparison.Ordinal), "the root dashboard has no fixed design surface");
        Assert.Contains("layout:ResponsiveLayout.IsEnabled=\"True\"", xaml, "the shared responsive policy observes the page width");
        Assert.Contains("VerticalScrollBarVisibility=\"Auto\"", xaml, "short windows can scroll dashboard content");
        Assert.Contains("x:Name=\"MetricsGrid\"", xaml, "metrics expose a reflow target");
        Assert.Contains("x:Name=\"QuickActionsGrid\"", xaml, "quick actions expose a reflow target");
        Assert.Contains("Value=\"{x:Static layout:ResponsiveLayoutMode.Compact}\"", xaml, "compact layout rules are declared in XAML");
        Assert.Contains("Value=\"{x:Static layout:ResponsiveLayoutMode.Standard}\"", xaml, "standard layout rules are declared in XAML");
    }

    /// <summary>Verifies that the sidebar does not reserve a footer card.</summary>
    public void SidebarFooterCardIsRemoved()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));

        Assert.False(xaml.Contains("SidebarFooterCard", StringComparison.Ordinal), "sidebar footer card is removed");
        Assert.Contains("<Grid><Grid.RowDefinitions><RowDefinition Height=\"77\" /><RowDefinition Height=\"*\" /></Grid.RowDefinitions>", xaml, "sidebar no longer reserves a footer row");
    }

    /// <summary>Verifies the physical order and flexible width of toolbar controls.</summary>
    public void ToolbarExpandsSearchAfterActionButtons()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));
        const string toolbarColumns = "<Grid.ColumnDefinitions><ColumnDefinition Width=\"Auto\" /><ColumnDefinition Width=\"Auto\" /><ColumnDefinition Width=\"*\" /><ColumnDefinition Width=\"Auto\" /></Grid.ColumnDefinitions>";

        Assert.Contains("x:Name=\"ToolbarLayout\"", xaml, "toolbar exposes one responsive layout boundary");
        Assert.Contains(toolbarColumns, xaml, "toolbar reserves actions, flexible search, and content-sized title slots");
        Assert.Contains("<StackPanel Grid.Column=\"1\" Orientation=\"Horizontal\"", xaml, "toolbar status actions occupy their own slot");
        Assert.Contains("<Border Height=\"43\" HorizontalAlignment=\"Stretch\"", xaml, "search expands in the flexible toolbar slot");
        Assert.Contains("<Setter Property=\"Grid.Column\" Value=\"2\" />", xaml, "search base position remains style-overridable");
        Assert.Contains("<Setter Property=\"Grid.Row\" Value=\"1\" />", xaml, "compact search reflows below the primary toolbar row");
        Assert.Contains("x:Name=\"NewQuoteLabel\"", xaml, "compact action can hide only its text label");
    }

    /// <summary>Verifies Arabic search text alignment at the RTL edge.</summary>
    public void SearchBoxArabicTextAlignsToTheRtlEdge()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));

        Assert.Contains("x:Name=\"SearchBox\"", xaml, "search box has a stable automation name");
        Assert.Contains("x:Name=\"SearchBox\" Background=\"Transparent\" BorderThickness=\"0\" FontFamily=\"Noto Kufi Arabic, Segoe UI\" FontSize=\"13\" Foreground=\"#8D8781\" Padding=\"10,11,0,8\" TextAlignment=\"Left\" FlowDirection=\"RightToLeft\"", xaml, "search Arabic text aligns to the physical right edge");
    }

    /// <summary>Verifies the latest quotations title alignment.</summary>
    public void LatestQuotesTitleAlignsToTheRtlEdge()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));

        Assert.Contains("<TextBlock Grid.Column=\"0\" Text=\"آخر عروض الأسعار\" Style=\"{StaticResource SectionTitle}\" TextAlignment=\"Left\" HorizontalAlignment=\"Stretch\"", xaml, "latest quotations title aligns to the physical right edge");
    }

    /// <summary>Verifies shared Arabic navigation alignment and icon spacing.</summary>
    public void SidebarNavigationKeepsArabicLabelsAtTheRtlEdge()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));
        const string physicalNavigationColumns = "<Grid FlowDirection=\"LeftToRight\"><Grid.ColumnDefinitions><ColumnDefinition /><ColumnDefinition Width=\"12\" /><ColumnDefinition Width=\"34\" /></Grid.ColumnDefinitions>";

        Assert.Equal(12, xaml.Split(physicalNavigationColumns, StringSplitOptions.None).Length - 1, "every sidebar row keeps a fixed label-to-icon gap");
        Assert.Contains("<Style x:Key=\"NavText\" TargetType=\"TextBlock\">", xaml, "sidebar Arabic alignment is owned by the shared label style");
        Assert.Contains("<Setter Property=\"FlowDirection\" Value=\"RightToLeft\" />", xaml, "navigation labels preserve Arabic flow");
        Assert.Contains("<Setter Property=\"TextAlignment\" Value=\"Right\" />", xaml, "navigation labels align beside their icons");
        Assert.Contains("<Setter Property=\"HorizontalAlignment\" Value=\"Right\" />", xaml, "navigation labels anchor to the physical right edge");
        Assert.Contains("<Setter Property=\"TextWrapping\" Value=\"NoWrap\" />", xaml, "navigation labels do not wrap away from the icon");
        var theme = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "Resources", "OperationsTheme.xaml"));
        Assert.Contains("Property=\"ToolTip\" Value=\"{Binding RelativeSource={RelativeSource Self}, Path=Tag}\"", theme, "compact navigation keeps visible labels as tooltips");
        Assert.Contains("automation:AutomationProperties.Name", theme, "compact navigation keeps stable accessible names");
    }

    /// <summary>Verifies notification text alignment and icon spacing.</summary>
    public void NotificationRowsKeepArabicTextBesideIcons()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));
        const string headerColumns = "<Grid Margin=\"18,0\" FlowDirection=\"LeftToRight\"><Grid.ColumnDefinitions><ColumnDefinition /><ColumnDefinition Width=\"12\" /><ColumnDefinition Width=\"31\" /></Grid.ColumnDefinitions>";
        const string itemColumns = "<Grid.ColumnDefinitions><ColumnDefinition Width=\"18\" /><ColumnDefinition /><ColumnDefinition Width=\"12\" /><ColumnDefinition Width=\"34\" /></Grid.ColumnDefinitions>";
        const string itemTextAlignment = "<StackPanel Grid.Column=\"1\" FlowDirection=\"RightToLeft\" TextBlock.TextAlignment=\"Left\" VerticalAlignment=\"Center\">";

        Assert.Contains(headerColumns, xaml, "notification header keeps a fixed title-to-icon gap");
        Assert.Equal(4, xaml.Split(itemColumns, StringSplitOptions.None).Length - 1, "notification rows keep a fixed text-to-icon gap");
        Assert.Equal(4, xaml.Split(itemTextAlignment, StringSplitOptions.None).Length - 1, "notification rows preserve physical-right Arabic alignment");
    }

    /// <summary>Verifies work-distribution labels, progress, and icon placement.</summary>
    public void WorkDistributionRowsKeepProgressBesideIcons()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));
        const string rowColumns = "<Grid.ColumnDefinitions><ColumnDefinition Width=\"38\" /><ColumnDefinition /><ColumnDefinition Width=\"12\" /><ColumnDefinition Width=\"38\" /></Grid.ColumnDefinitions>";
        const string rowLabelAlignment = "<TextBlock FlowDirection=\"RightToLeft\" TextAlignment=\"Left\" HorizontalAlignment=\"Stretch\"";
        const string headerAlignment = "<StackPanel x:Name=\"WorkDistributionHeader\" FlowDirection=\"RightToLeft\" TextBlock.TextAlignment=\"Left\">";

        Assert.Equal(4, xaml.Split(rowColumns, StringSplitOptions.None).Length - 1, "work distribution rows keep a fixed progress-to-icon gap");
        Assert.Equal(4, xaml.Split(rowLabelAlignment, StringSplitOptions.None).Length - 1, "work distribution Arabic labels align to the physical right edge");
        Assert.Contains(headerAlignment, xaml, "work distribution title aligns to the physical right edge");
        Assert.Contains("<Path Grid.Column=\"3\" Data=\"{StaticResource IconCut}\"", xaml, "work distribution first icon stays on the right");
        Assert.Contains("<Path Grid.Column=\"3\" Data=\"{StaticResource IconMaterials}\"", xaml, "work distribution second icon stays on the right");
        Assert.Contains("<Path Grid.Column=\"3\" Data=\"{StaticResource IconWorkOrder}\"", xaml, "work distribution third icon stays on the right");
        Assert.Contains("<Path Grid.Column=\"3\" Data=\"{StaticResource IconFurniture}\"", xaml, "work distribution fourth icon stays on the right");
    }

    /// <summary>Verifies the work-distribution footer action alignment.</summary>
    public void WorkDistributionFooterAlignsToTheLeft()
    {
        var xaml = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml"));

        Assert.Contains("<Button Grid.Row=\"5\" Content=\"عرض تفاصيل سير العمل  ←\" Style=\"{StaticResource LinkButton}\" Tag=\"تفاصيل سير العمل\" Click=\"PreviewActionClick\" HorizontalContentAlignment=\"Left\"", xaml, "work distribution footer aligns with the notification footer");
    }

    /// <summary>Verifies readable selected navigation content during hover.</summary>
    public void SelectedSidebarNavigationKeepsWhiteHoverContent()
    {
        var codeBehind = File.ReadAllText(Path.Combine(RepositoryRoot, "shells", "windows", "MainWindow.xaml.cs"));

        Assert.Contains("VisualDescendants<Button>(SidebarNavigation)", codeBehind, "hover handling is limited to sidebar navigation");
        Assert.Contains("button.MouseEnter += NavigationMouseEnter", codeBehind, "sidebar buttons report hover entry");
        Assert.Contains("button.MouseLeave += NavigationMouseLeave", codeBehind, "sidebar buttons report hover exit");
        Assert.Contains("VisualDescendants<System.Windows.Shapes.Path>(button)", codeBehind, "navigation updates each vector icon");
        var hoverStart = codeBehind.IndexOf("private void NavigationMouseEnter", StringComparison.Ordinal);
        var hoverEnd = codeBehind.IndexOf("private void NavigationMouseLeave", StringComparison.Ordinal);
        Assert.True(hoverStart >= 0 && hoverEnd > hoverStart, "navigation hover handlers exist");
        var hoverHandler = codeBehind[hoverStart..hoverEnd];
        Assert.False(hoverHandler.Contains("SetNavigationContentTone(button, Brushes.Black)", StringComparison.Ordinal), "selected content does not become black on hover entry");
        Assert.Contains("SetNavigationContentTone(button, Brushes.White)", hoverHandler, "selected content stays white on hover entry");
        Assert.Contains("icon.ClearValue(System.Windows.Shapes.Shape.FillProperty)", codeBehind, "unselected navigation icon restores its themed brush");
    }

    public void ShellOwnershipRulesAreEnforced()
    {
        var shell = Path.Combine(RepositoryRoot, "shells", "windows");
        var sources = Directory.EnumerateFiles(shell, "*.cs", SearchOption.AllDirectories)
            .Where(path => !path.Contains($"{Path.DirectorySeparatorChar}generated{Path.DirectorySeparatorChar}", StringComparison.OrdinalIgnoreCase)
                && !path.Contains($"{Path.DirectorySeparatorChar}tests{Path.DirectorySeparatorChar}", StringComparison.OrdinalIgnoreCase));
        var forbidden = new[]
        {
            "Microsoft.Data." + "Sqlite", "System.Data.SqlClient", "Npgsql", "DbContext",
            "HttpClient", "File.WriteAll", "FileStream(", "ConfigurationManager",
            "ProtectedData", "RegistryKey", "Authorize(", "PermissionDecision.Granted",
        };
        foreach (var source in sources)
        {
            var text = File.ReadAllText(source);
            foreach (var token in forbidden)
            {
                Assert.False(text.Contains(token, StringComparison.Ordinal), $"ownership token {token} in {Path.GetFileName(source)}");
            }
        }

        var coordinator = File.ReadAllText(Path.Combine(shell, "Features", "Operations", "OperationsCoordinator.cs"));
        Assert.Contains("SubmitConfigurationPatchAsync", coordinator, "typed configuration patch boundary");
        Assert.Contains("SubmitReferenceMarkerAsync", coordinator, "typed reference marker boundary");
        Assert.False(coordinator.Contains("SendCommandAsync", StringComparison.Ordinal), "shell cannot submit generic commands");

        var app = File.ReadAllText(Path.Combine(shell, "App.xaml.cs"));
        Assert.Contains("WindowsEngineBridge.Create(e.Args)", app, "platform adapter owns engine bootstrap");
        Assert.False(app.Contains("EngineLaunchRequest", StringComparison.Ordinal), "shell cannot own engine launch configuration");
        Assert.False(app.Contains("DevelopmentIdentity", StringComparison.Ordinal), "shell cannot create permission assertions");
        Assert.False(app.Contains("SpecialFolder.LocalApplicationData", StringComparison.Ordinal), "shell cannot select runtime storage");
    }

    private static ScopeRef Scope() => new() { Kind = "organization", Id = Guid.NewGuid() };

    private static ConfigSnapshot Configuration(long revision, string locale) => new()
    {
        Revision = revision,
        SchemaVersion = 1,
        Scope = Scope(),
        Entries =
        [
            new ConfigEntry
            {
                Key = ProtocolIds.ConfigKeys.EitmadConfigLocalePrimaryV1,
                Sensitivity = ConfigSensitivity.Public,
                RestartRequirement = RestartRequirement.None,
                Value = new ConfigReadValue { Kind = ConfigReadValueKind.Text, Value = locale },
            },
        ],
    };

    private static EventEnvelope Event(Guid subscriptionId, long sequence) => new()
    {
        SubscriptionId = subscriptionId,
        CorrelationId = Guid.NewGuid(),
        Cursor = Guid.NewGuid(),
        Sequence = sequence,
        OccurredAt = sequence,
        Event = [],
    };

    private static async Task Eventually(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(3);
        while (!condition())
        {
            if (DateTime.UtcNow >= deadline) throw new InvalidOperationException("Expected asynchronous shell condition was not reached.");
            await Task.Delay(5);
        }
    }

    private static string FindRepositoryRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, "AGENTS.md"))) directory = directory.Parent;
        return directory?.FullName ?? throw new DirectoryNotFoundException("Repository root was not found.");
    }
}

internal sealed class ImmediateDispatcher : IShellDispatcher
{
    public void Invoke(Action action) => action();
}

internal sealed class FakeEngine : IEngineShellBridge
{
    private readonly Dictionary<string, FakeSubscription> subscriptions = [];
    private EngineSupervisionSnapshot snapshot = new(EngineSupervisionState.Stopped, 0, 0, EngineIpcHealthState.Unavailable, null, null, null);
    private int queryCount;

    public event Action<EngineSupervisionSnapshot>? StateChanged;
    public EngineSupervisionSnapshot Snapshot => snapshot;
    public bool FailConfigurationQuery { get; init; }
    public int QueryCount => Volatile.Read(ref queryCount);
    public int SubscriptionCount => subscriptions.Count;
    public int StopCount { get; private set; }
    public IReadOnlySet<string> SupportedCapabilities { get; init; } = new HashSet<string>
    {
        ProtocolIds.Capabilities.EitmadCapabilityConfigV1,
        ProtocolIds.Capabilities.EitmadCapabilitySyncV1,
        ProtocolIds.Capabilities.EitmadCapabilityUpdateV1,
        ProtocolIds.Capabilities.EitmadCapabilityReferenceMarkerV1,
    };
    private HashSet<string> QueriedKinds { get; } = [];

    public bool SupportsCapability(string capability) => SupportedCapabilities.Contains(capability);

    public bool WasQueried(string kind)
    {
        lock (QueriedKinds) return QueriedKinds.Contains(kind);
    }

    public Task StartAsync(CancellationToken cancellationToken = default)
    {
        snapshot = snapshot with { State = EngineSupervisionState.Starting, Generation = snapshot.Generation + 1 };
        StateChanged?.Invoke(snapshot);
        return Task.CompletedTask;
    }

    public Task StopAsync(CancellationToken cancellationToken = default)
    {
        StopCount++;
        snapshot = snapshot with { State = EngineSupervisionState.Stopped, IpcHealth = EngineIpcHealthState.Unavailable };
        StateChanged?.Invoke(snapshot);
        return Task.CompletedTask;
    }

    public Task<QueryResponseEnvelope> QueryAsync(Query query, CancellationToken cancellationToken = default)
    {
        Interlocked.Increment(ref queryCount);
        lock (QueriedKinds) QueriedKinds.Add(query.Kind);
        if (FailConfigurationQuery && query.Kind == Query.ConfigGetKind)
        {
            return Task.FromResult(new QueryResponseEnvelope
            {
                RequestId = Guid.NewGuid(),
                CorrelationId = Guid.NewGuid(),
                Outcome = new QueryOutcome
                {
                    Status = CommandOutcomeStatus.Failed,
                    Payload = new QueryResult { Code = "CONFIG_UNAVAILABLE" },
                },
            });
        }

        var result = query.Kind switch
        {
            Query.ConfigGetKind => QueryResult.ForConfiguration(ShellScenariosConfiguration()),
            Query.SyncGetStatusKind => QueryResult.ForSyncStatus(new SyncStatus { Kind = SyncStatusKind.Current, Payload = new SyncStatusPayload() }),
            Query.UpdateGetStateKind => QueryResult.ForUpdateState(new UpdateState { Kind = UpdateStateKind.Idle, Payload = new UpdateStatePayload() }),
            Query.ReferenceMarkerListKind => QueryResult.ForReferenceMarkers(new ReferenceMarkerPage { Items = [] }),
            _ => throw new InvalidOperationException("Unexpected fake query."),
        };
        return Task.FromResult(new QueryResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = result },
        });
    }

    public Task<CommandResponseEnvelope> SubmitConfigurationPatchAsync(UpdateConfiguration patch, Guid idempotencyKey, CancellationToken cancellationToken = default) =>
        Task.FromResult(new CommandResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = new CommandResult() },
        });

    public Task<CommandResponseEnvelope> SubmitReferenceMarkerAsync(UpsertReferenceMarker marker, Guid idempotencyKey, CancellationToken cancellationToken = default) =>
        Task.FromResult(new CommandResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new CommandOutcome
            {
                Status = CommandOutcomeStatus.Succeeded,
                Payload = new CommandResult
                {
                    Kind = PurpleKind.ReferenceMarkerUpserted,
                    Payload = new PayloadClass
                    {
                        Id = marker.MarkerId,
                        Label = marker.Label,
                        Revision = (marker.ExpectedRevision ?? 0) + 1,
                        Scope = new ScopeRef { Kind = "organization", Id = Guid.Parse("2ef36635-1d9d-4bd5-b0e4-fc4a67dfac90") },
                        SyncState = ReferenceMarkerSyncState.Pending,
                        UpdatedAt = 1_800_000_000_001,
                    },
                },
            },
        });

    public Task<IEngineSubscription> SubscribeAsync(Subscription subscription, CancellationToken cancellationToken = default)
    {
        var item = new FakeSubscription();
        subscriptions.Add(subscription.Kind, item);
        return Task.FromResult<IEngineSubscription>(item);
    }

    public void Connect()
    {
        snapshot = snapshot with
        {
            State = EngineSupervisionState.Running,
            IpcHealth = EngineIpcHealthState.Connected,
            LastLifecycle = new LifecycleSnapshot
            {
                Live = true,
                Ready = true,
                State = LifecycleState.Ready,
                Health = HealthStatus.Healthy,
                Checks = [],
                ObservedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                Identity = new EngineProcessIdentity
                {
                    InstanceId = Guid.NewGuid(),
                    Mode = EngineMode.SupervisedDesktop,
                    ProcessId = 100,
                    ProductVersion = "0.0.0",
                    ProtocolVersion = new ProtocolVersion { Major = 1, Minor = 3 },
                    StartedAt = 1,
                },
            },
        };
        StateChanged?.Invoke(snapshot);
    }

    public void Disconnect()
    {
        snapshot = snapshot with { IpcHealth = EngineIpcHealthState.Connecting };
        StateChanged?.Invoke(snapshot);
    }

    public void SignalResync(string kind) => subscriptions[kind].SignalResync();

    public ValueTask DisposeAsync()
    {
        foreach (var subscription in subscriptions.Values) subscription.DisposeAsync();
        return ValueTask.CompletedTask;
    }

    private static ConfigSnapshot ShellScenariosConfiguration() => new()
    {
        Revision = 1,
        SchemaVersion = 1,
        Scope = new ScopeRef { Kind = "organization", Id = Guid.NewGuid() },
        Entries =
        [
            new ConfigEntry
            {
                Key = ProtocolIds.ConfigKeys.EitmadConfigLocalePrimaryV1, Sensitivity = ConfigSensitivity.Public,
                RestartRequirement = RestartRequirement.None,
                Value = new ConfigReadValue { Kind = ConfigReadValueKind.Text, Value = "ar-YE" },
            },
        ],
    };
}

internal sealed class FakeSubscription : IEngineSubscription
{
    private readonly Channel<EventEnvelope> events = Channel.CreateUnbounded<EventEnvelope>();
    public event Action? ResyncRequired;
    public IAsyncEnumerable<EventEnvelope> ReadAllAsync(CancellationToken cancellationToken = default) => events.Reader.ReadAllAsync(cancellationToken);
    public void Acknowledge(EventEnvelope delivered) { }
    public void SignalResync() => ResyncRequired?.Invoke();
    public ValueTask DisposeAsync()
    {
        events.Writer.TryComplete();
        return ValueTask.CompletedTask;
    }
}

internal static class Assert
{
    public static void True(bool value, string message) { if (!value) throw new InvalidOperationException($"Assertion failed: {message}."); }
    public static void False(bool value, string message) => True(!value, message);
    public static void Equal<T>(T expected, T actual, string message)
    {
        if (!EqualityComparer<T>.Default.Equals(expected, actual)) throw new InvalidOperationException($"Assertion failed: {message}. Expected {expected}; actual {actual}.");
    }
    public static void Contains(string expected, string actual, string message) => True(actual.Contains(expected, StringComparison.Ordinal), message);
}
