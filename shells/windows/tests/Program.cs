using System.Threading.Channels;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Operations;

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
tests.DashboardVisualSystemIsConsistentAndRtlSafe();
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
        Assert.Contains("Text=\"العربية (اليمن) · ar-YE\"", xaml, "visible primary locale marker");
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
        Assert.Contains("x:Name=\"ToolbarGap\"", xaml, "search and new quotation action have a fixed gap");
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
        const string toolbarGrid = "<Grid Grid.Column=\"1\" FlowDirection=\"LeftToRight\" VerticalAlignment=\"Center\" HorizontalAlignment=\"Stretch\">";
        const string toolbarColumns = "<Grid.ColumnDefinitions><ColumnDefinition Width=\"178\" /><ColumnDefinition Width=\"*\" /></Grid.ColumnDefinitions>";
        const string titleSizedColumns = "<Grid.ColumnDefinitions><ColumnDefinition Width=\"Auto\" /><ColumnDefinition Width=\"*\" /><ColumnDefinition Width=\"196\" /></Grid.ColumnDefinitions>";

        Assert.Contains(titleSizedColumns, xaml, "toolbar sizes the dashboard title to its content");
        Assert.Contains(toolbarGrid, xaml, "toolbar keeps a stable mixed-direction boundary");
        Assert.Contains(toolbarColumns, xaml, "toolbar reserves actions before the flexible search field");
        Assert.Contains("<StackPanel Grid.Column=\"0\" Orientation=\"Horizontal\" Margin=\"0,0,20,0\">", xaml, "toolbar actions occupy the left slot before search");
        Assert.Contains("<Border Grid.Column=\"1\" Height=\"43\"", xaml, "search field expands in the slot beside the dashboard title");
        Assert.Contains("<Border Grid.Column=\"1\" Height=\"43\" HorizontalAlignment=\"Stretch\"", xaml, "search border stretches across the flexible slot");
        Assert.Contains("<Border Grid.Column=\"1\" Height=\"43\" HorizontalAlignment=\"Stretch\" Margin=\"0,0,16,0\"", xaml, "search border keeps a margin before the dashboard title");
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
        const string sharedArabicLabelAlignment = "<Style x:Key=\"NavText\" TargetType=\"TextBlock\"><Setter Property=\"FontSize\" Value=\"17\" /><Setter Property=\"VerticalAlignment\" Value=\"Center\" /><Setter Property=\"FlowDirection\" Value=\"RightToLeft\" /><Setter Property=\"TextAlignment\" Value=\"Right\" /><Setter Property=\"HorizontalAlignment\" Value=\"Stretch\" /></Style>";

        Assert.Equal(12, xaml.Split(physicalNavigationColumns, StringSplitOptions.None).Length - 1, "every sidebar row keeps a fixed label-to-icon gap");
        Assert.Contains(sharedArabicLabelAlignment, xaml, "sidebar Arabic alignment is owned by the shared label style");
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
