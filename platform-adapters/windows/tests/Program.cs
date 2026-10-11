using System.ComponentModel;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;

try
{
    if (args.Length != 0 && args is not ["--engine", _])
    {
        throw new ArgumentException("Usage: Eitmad.Platform.Windows.Tests [--engine <path>]");
    }

    var tests = new SupervisionScenarios();
    await tests.UnavailableEngineIsTyped();
    await tests.TypedRequestsRequireConnectedEngine();
    await tests.ConfigurationPatchRequiresIdempotency();
    tests.SubscriptionQueueIsBounded();
    tests.SubscriptionAcknowledgementNeverRegresses();
    await tests.SupervisedSubscriptionSurvivesReattach();
    await tests.SupervisedSubscriptionReportsProjectionInvalidation();
    await tests.SupervisedSubscriptionRecoversAfterQueueOverflow();
    await tests.IntentionalStopNeverRestarts();
    await tests.UnexpectedDeathRestartsOnce();
    await tests.FourthConsecutiveFailureExhaustsRestarts();
    await tests.StaleExitCannotReplaceCurrentGeneration();
    await tests.CleanShutdownAvoidsForcedTermination();
    await tests.ShutdownTimeoutTerminatesProcessGroup();
    await tests.TerminationFailureStillCompletesCleanup();

    if (args is ["--engine", var enginePath])
    {
        await tests.RealEngineStartsAndStopsCleanly(enginePath);
    }

    Console.WriteLine("Windows process supervision scenarios passed.");
}
catch (Exception error)
{
    if (error is EngineIpcException ipcError)
    {
        Console.Error.WriteLine($"IPC failure: {ipcError.Kind}; code: {ipcError.ContractError?.Code}");
    }
    Console.Error.WriteLine(error);
    Environment.ExitCode = 1;
}

internal sealed class SupervisionScenarios
{
    public async Task UnavailableEngineIsTyped()
    {
        try
        {
            await EngineIpcClient.ConnectAsync(
                $"missing-{Guid.NewGuid():N}",
                DevelopmentPeer(),
                "synthetic-token",
                TimeSpan.FromMilliseconds(20));
        }
        catch (EngineIpcException error)
        {
            Assert.Equal(
                EngineIpcFailureKind.EngineUnavailable,
                error.Kind,
                "unavailable engine failure kind");
            return;
        }

        throw new InvalidOperationException("Expected unavailable engine failure.");
    }

    public async Task TypedRequestsRequireConnectedEngine()
    {
        var fixture = new SupervisorFixture();
        await Assert.ThrowsAsync<EngineIpcException>(
            () => fixture.Supervisor.QueryAsync(Query.ForConfigGet(new GetConfiguration())),
            "typed query requires connected engine");
        await Assert.ThrowsAsync<EngineIpcException>(
            () => fixture.Supervisor.SubmitCommandAsync(
                Command.ForConfigUpdate(new UpdateConfiguration { ExpectedRevision = 1, Changes = [] }),
                Guid.NewGuid()),
            "typed configuration patch requires connected engine");
    }

    public async Task ConfigurationPatchRequiresIdempotency()
    {
        var fixture = new SupervisorFixture();
        await Assert.ThrowsAsync<ArgumentException>(
            () => fixture.Supervisor.SubmitCommandAsync(
                Command.ForConfigUpdate(new UpdateConfiguration { ExpectedRevision = 1, Changes = [] }),
                Guid.Empty),
            "typed configuration patch requires idempotency");
    }

    public void SubscriptionQueueIsBounded()
    {
        var subscription = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        for (var index = 0; index < EngineSubscription.Capacity; index++)
        {
            Assert.True(
                subscription.TryPublish(EventEnvelope(subscription.SubscriptionId)),
                "subscription queue accepts bounded item");
        }
        Assert.False(
            subscription.TryPublish(EventEnvelope(subscription.SubscriptionId)),
            "subscription queue rejects overflow");
    }

    public void SubscriptionAcknowledgementNeverRegresses()
    {
        var subscription = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        var newer = EventEnvelope(subscription.SubscriptionId, sequence: 2);
        var older = EventEnvelope(subscription.SubscriptionId, sequence: 1);

        subscription.Acknowledge(newer);
        subscription.Acknowledge(older);

        Assert.Equal(newer.Cursor, subscription.ProcessedCursor, "processed cursor remains monotonic");
    }

    public async Task SupervisedSubscriptionSurvivesReattach()
    {
        await using var supervised = new SupervisedEngineSubscription(
            Subscription.ForConfigChangedSubscribe(new ConfigurationChanges()));
        var first = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        supervised.Attach(first, resetCursor: false);
        var firstEvent = EventEnvelope(first.SubscriptionId);
        Assert.True(first.TryPublish(firstEvent), "first attachment publishes");
        var delivered = await ReadOne(supervised);
        supervised.Acknowledge(delivered);
        Assert.Equal(firstEvent.Cursor, supervised.ProcessedCursor, "processed cursor");
        first.Complete(new EngineIpcException(EngineIpcFailureKind.EngineUnavailable, "Synthetic transport loss."));

        var replacement = new EngineSubscription(Guid.NewGuid(), firstEvent.Cursor, resumed: true);
        supervised.Attach(replacement, resetCursor: false);
        var replacementEvent = EventEnvelope(replacement.SubscriptionId);
        Assert.True(replacement.TryPublish(replacementEvent), "replacement attachment publishes");
        Assert.Equal(replacementEvent.Cursor, (await ReadOne(supervised)).Cursor, "replacement event");
    }

    public async Task SupervisedSubscriptionRecoversAfterQueueOverflow()
    {
        await using var supervised = new SupervisedEngineSubscription(
            Subscription.ForConfigChangedSubscribe(new ConfigurationChanges()));
        var overflowing = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        supervised.Attach(overflowing, resetCursor: false);
        for (var index = 0; index <= EngineSubscription.Capacity; index++)
        {
            while (!overflowing.TryPublish(EventEnvelope(overflowing.SubscriptionId, index + 1)))
            {
                await Task.Yield();
            }
            await Task.Yield();
        }
        await Task.Delay(20);
        await Assert.ThrowsAsync<EngineIpcException>(
            async () =>
            {
                await foreach (var _ in supervised.ReadAllAsync())
                {
                }
            },
            "overflow completes the supervised queue");

        var replacement = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: true);
        supervised.Attach(replacement, resetCursor: false);
        var replacementEvent = EventEnvelope(replacement.SubscriptionId);
        Assert.True(replacement.TryPublish(replacementEvent), "replacement publishes after overflow");
        Assert.Equal(replacementEvent.Cursor, (await ReadOne(supervised)).Cursor, "replacement event after overflow");
    }

    public async Task SupervisedSubscriptionReportsProjectionInvalidation()
    {
        await using var supervised = new SupervisedEngineSubscription(
            Subscription.ForProductChangedSubscribe(new ProductChanges()));
        var attached = new EngineSubscription(Guid.NewGuid(), Guid.NewGuid(), resumed: false);
        supervised.Attach(attached, resetCursor: false);
        attached.Complete(new EngineIpcException(
            EngineIpcFailureKind.SessionChanged, "Synthetic projection invalidation."));

        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(2));
        try
        {
            await foreach (var _ in supervised.ReadAllAsync(timeout.Token)) { }
        }
        catch (EngineIpcException error)
        {
            Assert.Equal(EngineIpcFailureKind.SessionChanged, error.Kind,
                "projection invalidation reaches the supervised consumer");
            return;
        }
        throw new InvalidOperationException("Expected supervised projection invalidation.");
    }

    private static EventEnvelope EventEnvelope(Guid subscriptionId, long sequence = 1) => new()
    {
        SubscriptionId = subscriptionId,
        CorrelationId = Guid.NewGuid(),
        Cursor = Guid.NewGuid(),
        Event = new Dictionary<string, object>
        {
            ["kind"] = Event.ConfigChangedEventKind,
            ["payload"] = new Dictionary<string, object>(),
        },
        OccurredAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
        Sequence = sequence,
    };

    private static async Task<EventEnvelope> ReadOne(SupervisedEngineSubscription subscription)
    {
        await using var enumerator = subscription.ReadAllAsync().GetAsyncEnumerator();
        Assert.True(await enumerator.MoveNextAsync(), "subscription yields event");
        return enumerator.Current;
    }

    public async Task IntentionalStopNeverRestarts()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);
        var process = fixture.Launcher.Current;

        var stop = fixture.Supervisor.StopAsync();
        Assert.True(process.InputClosed, "intentional stop closes engine stdin");
        process.Exit(0);
        await stop;

        await Task.Yield();
        Assert.Equal(1, fixture.Launcher.LaunchCount, "intentional stop must not restart");
        Assert.Equal(EngineSupervisionState.Stopped, fixture.Supervisor.Snapshot.State, "intentional stop state");
    }

    public async Task UnexpectedDeathRestartsOnce()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);

        fixture.Launcher.Current.Exit(9);
        await Eventually(() => fixture.Supervisor.Snapshot.State == EngineSupervisionState.RestartDelay
            && fixture.Clock.HasPendingDelay(TimeSpan.FromSeconds(1)));
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(1));
        await Eventually(() => fixture.Launcher.LaunchCount == 2
            && fixture.Supervisor.Snapshot.Generation == 2
            && fixture.Supervisor.Snapshot.State == EngineSupervisionState.Starting);

        Assert.Equal(2L, fixture.Supervisor.Snapshot.Generation, "replacement generation");
        await fixture.StopCurrent();
    }

    public async Task FourthConsecutiveFailureExhaustsRestarts()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);

        for (var restart = 1; restart <= 3; restart++)
        {
            fixture.Launcher.Current.Exit(20 + restart);
            var expectedDelay = TimeSpan.FromSeconds(1 << (restart - 1));
            await Eventually(() => fixture.Supervisor.Snapshot.State == EngineSupervisionState.RestartDelay
                && fixture.Clock.HasPendingDelay(expectedDelay));
            fixture.Clock.CompleteDelay(expectedDelay);
            var expectedLaunches = restart + 1;
            await Eventually(() => fixture.Launcher.LaunchCount == expectedLaunches
                && fixture.Supervisor.Snapshot.Generation == expectedLaunches
                && fixture.Supervisor.Snapshot.State == EngineSupervisionState.Starting);
        }

        fixture.Launcher.Current.Exit(24);
        await Eventually(() => fixture.Supervisor.Snapshot.State == EngineSupervisionState.RestartExhausted);

        Assert.Equal(4, fixture.Launcher.LaunchCount, "restart exhaustion launch count");
        Assert.Equal(3, fixture.Supervisor.Snapshot.RestartCount, "restart exhaustion counter");
        await fixture.Supervisor.StopAsync();
    }

    public async Task StaleExitCannotReplaceCurrentGeneration()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);
        fixture.Launcher.Current.Exit(7);
        await Eventually(() => fixture.Supervisor.Snapshot.State == EngineSupervisionState.RestartDelay
            && fixture.Clock.HasPendingDelay(TimeSpan.FromSeconds(1)));
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(1));
        await Eventually(() => fixture.Launcher.LaunchCount == 2
            && fixture.Supervisor.Snapshot.Generation == 2
            && fixture.Supervisor.Snapshot.State == EngineSupervisionState.Starting);

        await fixture.Supervisor.ObserveExitAsync(1, 99);

        Assert.Equal(2, fixture.Launcher.LaunchCount, "stale exit launch count");
        Assert.Equal(2L, fixture.Supervisor.Snapshot.Generation, "stale exit generation");
        Assert.Equal(EngineSupervisionState.Starting, fixture.Supervisor.Snapshot.State, "stale exit state");
        await fixture.StopCurrent();
    }

    public async Task CleanShutdownAvoidsForcedTermination()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);
        var process = fixture.Launcher.Current;

        var stop = fixture.Supervisor.StopAsync();
        process.Exit(0);
        await stop;

        Assert.False(fixture.Group.Terminated, "clean shutdown must not terminate job");
        Assert.False(fixture.Supervisor.Snapshot.LastExit?.Forced ?? true, "clean shutdown outcome");
    }

    public async Task ShutdownTimeoutTerminatesProcessGroup()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);

        var stop = fixture.Supervisor.StopAsync();
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(15));
        await stop;

        Assert.True(fixture.Group.Terminated, "shutdown timeout terminates job");
        Assert.True(fixture.Supervisor.Snapshot.LastExit?.Forced ?? false, "forced shutdown outcome");
    }

    public async Task TerminationFailureStillCompletesCleanup()
    {
        var fixture = new SupervisorFixture();
        await fixture.Supervisor.StartAsync(fixture.Request);
        fixture.Group.TerminationException = new Win32Exception(5);
        using var cancellation = new CancellationTokenSource();

        var stop = fixture.Supervisor.StopAsync(cancellation.Token);
        fixture.Clock.CompleteDelay(TimeSpan.FromSeconds(15));
        cancellation.Cancel();
        await Assert.ThrowsAsync<OperationCanceledException>(() => stop, "cancel stalled shutdown");

        Assert.True(fixture.Launcher.Current.Disposed, "failed termination still disposes process");
        Assert.Equal(EngineSupervisionState.Stopped, fixture.Supervisor.Snapshot.State, "failed termination stop state");
    }

    public async Task RealEngineStartsAndStopsCleanly(string enginePath)
    {
        var runtimeDirectory = Path.Combine(Path.GetTempPath(), $"eitmad-supervision-{Guid.NewGuid():N}");
        Directory.CreateDirectory(runtimeDirectory);
        try
        {
            await SeedDevelopmentAccounts(enginePath, runtimeDirectory);
            await using var supervisor = new EngineSupervisor();
            var request = new EngineLaunchRequest(enginePath, runtimeDirectory);
            var lifecycleStates = new List<Eitmad.Contracts.LifecycleState>();
            supervisor.StateChanged += state =>
            {
                if (state.LastLifecycle is { } lifecycle
                    && (lifecycleStates.Count == 0 || lifecycleStates[^1] != lifecycle.State))
                {
                    lifecycleStates.Add(lifecycle.State);
                }
            };
            await supervisor.StartAsync(request);
            await Eventually(() => supervisor.Snapshot.LastLifecycle?.Ready == true, TimeSpan.FromSeconds(10));
            await Eventually(() => supervisor.IpcConnected, TimeSpan.FromSeconds(10));
            Assert.Equal(EngineIpcHealthState.Connected, supervisor.Snapshot.IpcHealth, "real engine IPC health");
            Assert.Equal(
                ProtocolIds.Version.Minor,
                supervisor.Snapshot.LastLifecycle?.Identity.ProtocolVersion.Minor,
                "real engine protocol version");
            var desktopSession = await supervisor.SignInAsync("admin", "admin");
            Assert.Equal(
                DesktopAccountRole.Manager,
                desktopSession.AccountRole,
                "real engine desktop account role");
            Assert.True(
                supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityConfigV1),
                "real config capability negotiated");
            Assert.True(
                supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityCustomerV1),
                "real customer capability negotiated");
            Assert.True(
                supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilitySalesCatalogV1),
                "real public sales catalog capability negotiated");
            Assert.False(
                supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilitySyncV1),
                "unwired sync capability is not negotiated");
            Assert.False(
                supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityUpdateV1),
                "unwired update capability is not negotiated");
            Assert.True(desktopSession.CustomerAuthorization?.Scope.Kind == "branch",
                "engine-issued customer branch scope");
            await VerifyOrderBoundary(supervisor);
            await using var customerSubscription = await supervisor.SubscribeAsync(
                Subscription.ForCustomerChangedSubscribe(new CustomerChanges()));
            var createdResponse = await supervisor.SubmitCommandAsync(
                Command.ForCustomerCreate(new CreateCustomer
                {
                    Name = "عميل تجريبي", Phone = "+967777123456",
                    Address = null!, Notes = null!,
                }), Guid.NewGuid());
            Assert.Equal(CommandOutcomeStatus.Succeeded, createdResponse.Outcome.Status,
                "real customer create succeeds in branch scope");
            var created = createdResponse.Outcome.Payload.AsCustomerCreated()?.Customer
                ?? throw new InvalidOperationException("Real engine omitted the created customer.");

            var persistedPart = await SaveMultiMaterialPart(supervisor);
            var persistedFurniture = await SaveFurnitureDefinition(supervisor, persistedPart, runtimeDirectory);
            var configurationResponse = await supervisor.QueryAsync(Query.ForConfigGet(new GetConfiguration()));
            if (configurationResponse.Outcome.Status != CommandOutcomeStatus.Succeeded)
            {
                throw new InvalidOperationException(
                    $"Real configuration query failed: {configurationResponse.Outcome.Payload.Code} / {configurationResponse.Outcome.Payload.MessageId}.");
            }
            var configuration = configurationResponse.Outcome.Payload.AsConfiguration()
                ?? throw new InvalidOperationException("Real engine omitted the configuration snapshot.");
            await using var configurationSubscription = await supervisor.SubscribeAsync(
                Subscription.ForConfigChangedSubscribe(new ConfigurationChanges()));
            var productImage = await ImportSyntheticCatalogImage(supervisor, runtimeDirectory, CatalogImageKind.Product);
            var persistedProduct = await SaveSupplierProduct(supervisor, productImage);

            var patchResponse = await supervisor.SubmitCommandAsync(
                Command.ForConfigUpdate(new UpdateConfiguration
                {
                    ExpectedRevision = configuration.Revision,
                    Changes =
                    [
                        new ConfigChange
                        {
                            Key = "eitmad.config.locale.primary.v1",
                            Value = new ConfigWriteValue { Kind = ConfigWriteValueKind.Text, Value = "ar-YE" },
                        },
                    ],
                }),
                Guid.NewGuid());
            Assert.Equal(CommandOutcomeStatus.Failed, patchResponse.Outcome.Status, "manager configuration write denial");
            Assert.Equal(
                ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1,
                patchResponse.Outcome.Payload.Code,
                "manager configuration write denial code");
            await supervisor.StopAsync();

            Assert.Equal(EngineSupervisionState.Stopped, supervisor.Snapshot.State, "real engine stopped state");
            Assert.Equal(EngineIpcHealthState.Unavailable, supervisor.Snapshot.IpcHealth, "stopped IPC health");
            Assert.Equal(0, supervisor.Snapshot.LastExit?.ExitCode, "real engine exit code");
            Assert.False(supervisor.Snapshot.LastExit?.Forced ?? true, "real engine graceful exit");
            Assert.SequenceEqual(
                [
                    Eitmad.Contracts.LifecycleState.Starting,
                    Eitmad.Contracts.LifecycleState.Ready,
                    Eitmad.Contracts.LifecycleState.Stopping,
                    Eitmad.Contracts.LifecycleState.Stopped,
                ],
                lifecycleStates,
                "real engine lifecycle sequence");
            await supervisor.StartAsync(request);
            await Eventually(() => supervisor.IpcConnected, TimeSpan.FromSeconds(10));
            await supervisor.SignInAsync("admin", "admin");
            await VerifyOrderBoundary(supervisor);
            var reopenedProducts = await supervisor.QueryAsync(Query.ForProductList(new ListProducts { Term = "مرتبة", Limit = 100 }));
            var reopenedProduct = reopenedProducts.Outcome.Payload.AsProducts()!.Items.Single();
            Assert.Equal(persistedProduct.Id, reopenedProduct.Id, "product identity survives engine restart");
            Assert.Equal(productImage.Id, reopenedProduct.Image.Id, "image reference survives engine restart");
            var imageRead = await supervisor.QueryAsync(Query.ForCatalogImageGet(new GetCatalogImage { Reference = reopenedProduct.Image, Offset = 0 }));
            Assert.Equal(CommandOutcomeStatus.Succeeded, imageRead.Outcome.Status, "image bytes survive engine restart and source removal");
            Assert.True(Convert.FromBase64String(imageRead.Outcome.Payload.AsCatalogImage()!.Base64).Length > 0, "bounded image content returned");
            Assert.Equal(55000L, reopenedProduct.Variants.Single().PurchaseCostYer!.Value, "supplier cost survives engine restart");
            var reopenedFurniture = await supervisor.QueryAsync(Query.ForFurnitureList(new ListFurnitures { Term="خزانة اختبار",Limit=100 }));
            var furniture = reopenedFurniture.Outcome.Payload.AsFurnitures()!.Items.Single();
            Assert.Equal(persistedFurniture.Id,furniture.Id,"Furniture identity survives engine restart");
            Assert.Equal(18900L,furniture.PartsCostYer,"immutable Part cost survives engine restart");
            Assert.Equal(persistedFurniture.Parts[0].Reference.Revision,furniture.Parts[0].Reference.Revision,"composition reference survives restart");
            var reopenedParts = await supervisor.QueryAsync(Query.ForPartList(new ListParts { Term = "جانب خزانة", Limit = 20 }));
            var reopenedPart = reopenedParts.Outcome.Payload.AsParts()?.Items.Single().Part
                ?? throw new InvalidOperationException("Part was not durable after restart.");
            Assert.Equal(persistedPart.Id,reopenedPart.Id,"stable part identity after restart");
            Assert.Equal(9450L,reopenedPart.Cost.TotalCostYer,"Rust multi-material cost after restart");
            Assert.Equal(2,reopenedPart.Cost.Rows.Length,"multi-material composition after restart");
            var searchResponse = await supervisor.QueryAsync(Query.ForCustomerSearch(new SearchCustomers
            {
                Term = "تجريبي", Limit = 20,
            }));
            Assert.Equal(CommandOutcomeStatus.Succeeded, searchResponse.Outcome.Status,
                "real customer search after engine restart");
            var found = searchResponse.Outcome.Payload.AsCustomers()?.Items.SingleOrDefault()
                ?? throw new InvalidOperationException("Real engine did not find the persisted customer.");
            Assert.Equal(created.Id, found.Id, "stable customer identity after restart");
            var customerUpdateResponse = await supervisor.SubmitCommandAsync(Command.ForCustomerUpdate(new UpdateCustomer
            {
                CustomerId = found.Id, ExpectedRevision = found.Revision,
                Name = "عميل تجريبي محدث", Phone = found.Phone,
                Address = "عدن", Notes = null!,
            }), Guid.NewGuid());
            Assert.Equal(CommandOutcomeStatus.Succeeded, customerUpdateResponse.Outcome.Status,
                "real customer update after restart");
            var staleResponse = await supervisor.SubmitCommandAsync(Command.ForCustomerUpdate(new UpdateCustomer
            {
                CustomerId = found.Id, ExpectedRevision = found.Revision,
                Name = "تعديل قديم", Phone = found.Phone,
                Address = null!, Notes = null!,
            }), Guid.NewGuid());
            Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorCustomerRevisionConflictV1,
                staleResponse.Outcome.Payload.Code, "stale edit cannot overwrite current customer");
            await VerifyQuotationDraftBranchBoundary(supervisor, found);
            await supervisor.StopAsync();
        }
        finally
        {
            Directory.Delete(runtimeDirectory, recursive: true);
        }
    }

    private static async Task VerifyQuotationDraftBranchBoundary(EngineSupervisor supervisor, Customer customer)
    {
        Assert.True(supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1), "draft capability negotiated");
        await using var subscription = await supervisor.SubscribeAsync(Subscription.ForQuotationDraftChangedSubscribe(new()));
        var list = await supervisor.QueryAsync(Query.ForQuotationDraftList(new() { Limit = 100 }));
        Assert.Equal(CommandOutcomeStatus.Succeeded, list.Outcome.Status, "Manager draft list uses authorized branch");
        Assert.Equal(0, list.Outcome.Payload.AsQuotationDrafts()!.Items.Length, "no synthetic quotation rows from Rust");
        await VerifyQuotationLifecycleBoundary(supervisor, manager: true);
        var intent = new EvaluateQuotation { Customer = new() { Id = customer.Id, Revision = customer.Revision + 1 }, Lines = [], DiscountBasisPoints = 0 };
        var denied = await supervisor.SubmitCommandAsync(Command.ForQuotationDraftCreate(new() { Intent = intent }), Guid.NewGuid());
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1, denied.Outcome.Payload.Code, "Manager cannot create draft");
        // SaveSupplierProduct intentionally changes this account's role to invalidate subscriptions.
        var accounts = await supervisor.QueryAsync(Query.ForDesktopAccountList(new()));
        var account = accounts.Outcome.Payload.AsDesktopAccounts()!.Accounts.Single(a => a.Username == "rec");
        var restored = await supervisor.SubmitCommandAsync(Command.ForDesktopAccountUpdate(new()
        {
            AccountId = account.AccountId, ExpectedRevision = account.Revision,
            DisplayName = account.DisplayName, Role = DesktopAccountRole.Receptionist,
        }), Guid.NewGuid());
        Assert.Equal(CommandOutcomeStatus.Succeeded, restored.Outcome.Status, "restore Receptionist test authority");
        await supervisor.SignOutAsync();
        var reception = await supervisor.SignInAsync("rec", "rec");
        Assert.Equal(DesktopAccountRole.Receptionist, reception.AccountRole, "Rust receptionist role");
        Assert.Equal("branch", reception.CustomerAuthorization?.Scope.Kind, "Rust receptionist branch");
        await VerifyQuotationLifecycleBoundary(supervisor, manager: false);
        await VerifyOrderBoundary(supervisor, manager: false);
        var homeSearch = await supervisor.QueryAsync(Query.ForHomeRead(new() { Term = "تجريبي" }));
        Assert.Equal(customer.Id, homeSearch.Outcome.Payload.AsHome()!.Customers.Items.Single().Id,
            "Receptionist home searches the persisted authorized customer after restart");
        var approvalDenied = await supervisor.SubmitCommandAsync(Command.ForQuotationApprovalDecide(new() {
            DraftId = Guid.NewGuid(), RequestId = Guid.NewGuid(), QuotationRevision = 1, ExpectedRevision = 1,
            Fingerprint = "synthetic-forged", Decision = DiscountDecision.Approve,
        }), Guid.NewGuid());
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1, approvalDenied.Outcome.Payload.Code, "Receptionist cannot approve through direct IPC");
        var receptionList = await supervisor.QueryAsync(Query.ForQuotationDraftList(new() { Limit = 100 }));
        Assert.Equal(CommandOutcomeStatus.Succeeded, receptionList.Outcome.Status, "Receptionist branch read");
        var evaluation = await supervisor.QueryAsync(Query.ForQuotationEvaluate(intent));
        Assert.Equal(CommandOutcomeStatus.Succeeded, evaluation.Outcome.Status, "Receptionist branch evaluation");
        var failed = await supervisor.SubmitCommandAsync(Command.ForQuotationDraftCreate(new() { Intent = intent }), Guid.NewGuid());
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorQuotationDraftInvalidV1, failed.Outcome.Payload.Code, "Receptionist reaches draft field validation in branch");
        Assert.Equal(DetailKind.QuotationDraftValidation, failed.Outcome.Payload.Detail.Kind, "typed draft errors reach native client");
        Assert.True(failed.Outcome.Payload.Detail.Payload.Errors.Any(error => error.Field == QuotationField.Lines), "empty lines rejected by Rust");
    }

    private static async Task VerifyOrderBoundary(EngineSupervisor supervisor, bool manager = true)
    {
        Assert.True(supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityOrdersV1), "order capability negotiated");
        Assert.True(supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityWorkOrdersV1), "manufacturing capability negotiated");
        var production = await supervisor.QueryAsync(Query.ForWorkOrderList(new() { Limit = 100 }));
        if (manager) {
            Assert.Equal(CommandOutcomeStatus.Succeeded, production.Outcome.Status, "Manager production read through real IPC");
            Assert.False(production.Outcome.Payload.AsWorkOrders()!.ServerAvailable, "offline production has no confirmed action");
        } else {
            Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1, production.Outcome.Payload.Code, "Receptionist cannot read manufacturing through real IPC");
        }
        var response = await supervisor.QueryAsync(Query.ForOrderList(new() { Limit = 100 }));
        Assert.Equal(CommandOutcomeStatus.Succeeded, response.Outcome.Status, "authorized order cache read through IPC");
        var page = response.Outcome.Payload.AsOrders()!;
        Assert.False(page.ServerAvailable, "unconfigured order server is unavailable");
        Assert.Equal(0, page.Items.Length, "offline engine does not invent confirmed orders");
        await VerifyHomeBoundary(supervisor, manager);
        foreach (var query in new[] { Query.ForOrderCustomerDocument(new() { OrderId = Guid.NewGuid() }), Query.ForOrderQuotationDocument(new() { OrderId = Guid.NewGuid() }) }) {
            var missing = await supervisor.QueryAsync(query);
            Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorOrderInvalidV1, missing.Outcome.Payload.Code, "missing saved order document fails closed through real IPC");
        }
        var command = manager
            ? Command.ForOrderConvert(new() { DraftId = Guid.NewGuid(), ExpectedRevision = 1 })
            : Command.ForOrderCancel(new() { OrderId = Guid.NewGuid(), ExpectedRevision = 1, Reason = "إلغاء تجريبي" });
        var denied = await supervisor.SubmitCommandAsync(command, Guid.NewGuid());
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1, denied.Outcome.Payload.Code, "order role denial through direct IPC");
    }

    private static async Task VerifyHomeBoundary(EngineSupervisor supervisor, bool manager)
    {
        Assert.True(supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityHomeV1), "home capability negotiated");
        var response = await supervisor.QueryAsync(Query.ForHomeRead(new() { Term = "تجريبي" }));
        Assert.Equal(CommandOutcomeStatus.Succeeded, response.Outcome.Status, "real home read succeeds");
        var home = response.Outcome.Payload.AsHome()!;
        Assert.Equal(HomeAvailability.Available, home.Orders.Availability, "home order scope uses authenticated role");
        Assert.Equal(HomeAvailability.Available, home.Quotations.Availability, "Manager organization reads do not require branch drafts");
        Assert.Equal(HomeAvailability.Available, home.Catalog.Availability, "catalog organization is derived by Rust for both roles");
        Assert.Equal(manager ? HomeAvailability.Unavailable : HomeAvailability.Available, home.Customers.Availability,
            "unsupported organization customer search is explicit");
        Assert.Equal(0u, home.Orders.Count, "offline home does not invent orders");
        Assert.False(home.Orders.ServerAvailable, "offline home retains freshness state");
        Assert.Equal(HomeAvailability.Unavailable, home.Approvals.Availability, "offline approvals are unknown rather than zero");
    }

    private static async Task VerifyQuotationLifecycleBoundary(EngineSupervisor supervisor, bool manager)
    {
        Assert.True(supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1), "lifecycle capability negotiated");
        Assert.True(supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityCustomerDocumentsV1), "customer document capability negotiated");
        var missingDocument = await supervisor.QueryAsync(Query.ForQuotationCustomerDocument(new() { DraftId = Guid.NewGuid() }));
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorQuotationInvalidV1, missingDocument.Outcome.Payload.Code, "missing saved quotation fails closed through real IPC");
        var page = await supervisor.QueryAsync(Query.ForQuotationList(new() { Limit = 100 }));
        Assert.Equal(CommandOutcomeStatus.Succeeded, page.Outcome.Status, "authorized confirmed cache read");
        Assert.True(!page.Outcome.Payload.AsQuotations()!.ServerAvailable, "unconfigured server is unavailable");
        if (manager) {
            var denied = await supervisor.SubmitCommandAsync(Command.ForQuotationIssue(new() { DraftId = Guid.NewGuid(), ExpectedRevision = 1, ExpectedDraftRevision = 1 }), Guid.NewGuid());
            Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1, denied.Outcome.Payload.Code, "Manager cannot issue through IPC");
        } else {
            var denied = await supervisor.SubmitCommandAsync(Command.ForQuotationCancel(new() { DraftId = Guid.NewGuid(), ExpectedRevision = 1, Reason = "إلغاء تجريبي" }), Guid.NewGuid());
            Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1, denied.Outcome.Payload.Code, "Receptionist cannot cancel issued quotations through IPC");
            var unavailable = await supervisor.SubmitCommandAsync(Command.ForQuotationIssue(new() { DraftId = Guid.NewGuid(), ExpectedRevision = 1, ExpectedDraftRevision = 1 }), Guid.NewGuid());
            Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorQuotationUnavailableV1, unavailable.Outcome.Payload.Code, "no server means no issuance success");
        }
    }

    private static async Task<Furniture> SaveFurnitureDefinition(EngineSupervisor supervisor, Part part, string directory)
    {
        var categorySave = await supervisor.SubmitCommandAsync(Command.ForFurnitureCategorySave(new SaveFurnitureCategory { Name="غرف النوم" }),Guid.NewGuid());
        Assert.Equal(CommandOutcomeStatus.Succeeded,categorySave.Outcome.Status,"Furniture category commit");
        var categories = await supervisor.QueryAsync(Query.ForFurnitureCategoryList(new ListFurnitureCategories { Limit=100 }));
        await using var events=await supervisor.SubscribeAsync(Subscription.ForFurnitureChangedSubscribe(new FurnitureChanges()));
        var image = await ImportSyntheticCatalogImage(supervisor, directory, CatalogImageKind.Furniture);
        var input = new SaveFurniture {
            Image = image,
            Name="خزانة اختبار",CategoryId=categories.Outcome.Payload.AsFurnitureCategories()!.Items.Single().Id,Description="تعريف تجريبي",Notes="",State=FurnitureState.Active,
            Parts=[new FurniturePart { Reference=part.Composition,Quantity=2 }],
            Variants=[new FurnitureVariant { Id=Guid.NewGuid(),Name="صغير",Dimensions=new FurnitureDimensions { WidthMm=1200,HeightMm=2000,DepthMm=550 },SellingPriceYer=25000,ColorIds=[],HandleIds=[] }],
            Colors=[new FurnitureOption { Id=Guid.NewGuid(),Name="أبيض",Visual="#FFFFFF",PriceAdjustmentYer=500 }],
            Handles=[new FurnitureOption { Id=Guid.NewGuid(),Name="قياسي",Visual="Standard",PriceAdjustmentYer=1000 }],
        };
        var reviewed=await supervisor.QueryAsync(Query.ForFurnitureReview(input));Assert.Equal(18900L,reviewed.Outcome.Payload.AsFurnitureReview()!.PartsCostYer,"Rust Furniture composition review");
        var key=Guid.NewGuid();var saved=await supervisor.SubmitCommandAsync(Command.ForFurnitureSave(input),key);
        Assert.Equal(CommandOutcomeStatus.Succeeded,saved.Outcome.Status,"Furniture commit");
        var retried=await supervisor.SubmitCommandAsync(Command.ForFurnitureSave(input),key);Assert.Equal(saved.Outcome.Payload.AsFurnitureSaved()!.Id,retried.Outcome.Payload.AsFurnitureSaved()!.Id,"Furniture retry identity");
        var page=await supervisor.QueryAsync(Query.ForFurnitureList(new ListFurnitures { Term="خزانة اختبار",Limit=100 }));var value=page.Outcome.Payload.AsFurnitures()!.Items.Single();
        using var timeout=new CancellationTokenSource(TimeSpan.FromSeconds(10));
        await foreach(var delivered in events.ReadAllAsync(timeout.Token)) { var notice=EngineContractCodec.DecodeEvent(delivered).AsFurnitureChangedEvent();events.Acknowledge(delivered);Assert.Equal(value.Id,notice!.Id,"Furniture event after commit");break; }
        input.Id=value.Id;input.ExpectedRevision=value.Revision;input.State=FurnitureState.Draft;
        var updated=await supervisor.SubmitCommandAsync(Command.ForFurnitureSave(input),Guid.NewGuid());Assert.Equal(CommandOutcomeStatus.Succeeded,updated.Outcome.Status,"Furniture update");
        var stale=await supervisor.SubmitCommandAsync(Command.ForFurnitureSave(input),Guid.NewGuid());Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorFurnitureRevisionConflictV1,stale.Outcome.Payload.Code,"stale Furniture edit conflicts");
        var invalid=input;invalid.ExpectedRevision=value.Revision+1;invalid.Parts[0].Quantity=0;
        var rejected=await supervisor.SubmitCommandAsync(Command.ForFurnitureSave(invalid),Guid.NewGuid());Assert.Equal(CommandOutcomeStatus.Failed,rejected.Outcome.Status,"invalid Furniture quantity rejected");
        await supervisor.SignOutAsync();await supervisor.SignInAsync("rec","rec");
        var deniedImage = await supervisor.QueryAsync(Query.ForCatalogImageGet(new GetCatalogImage { Reference = image, Offset = 0 }));
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1, deniedImage.Outcome.Payload.Code, "Receptionist internal image read denied");
        var denied=await supervisor.SubmitCommandAsync(Command.ForFurnitureSave(input),Guid.NewGuid());Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1,denied.Outcome.Payload.Code,"Receptionist cannot write Furniture");
        await VerifyReceptionistCatalogBoundary(supervisor, value);
        await supervisor.SignOutAsync();await supervisor.SignInAsync("admin","admin");return value;
    }

    private static async Task VerifyReceptionistCatalogBoundary(EngineSupervisor supervisor, Furniture furniture)
    {
        var privateProducts = await supervisor.QueryAsync(Query.ForProductList(new ListProducts { Term = "", Limit = 100 }));
        Assert.Equal(CommandOutcomeStatus.Failed, privateProducts.Outcome.Status, "Receptionist private Product request fails through named-pipe IPC");
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1, privateProducts.Outcome.Payload.Code, "private Product definitions are absent from the response");
        var response = await supervisor.QueryAsync(Query.ForSalesCatalogList(new ListSalesCatalog { Term = "خزانة", Limit = 30 }));
        Assert.Equal(CommandOutcomeStatus.Succeeded, response.Outcome.Status, "public catalog typed IPC succeeds");
        var page = response.Outcome.Payload.AsSalesCatalog()!;
        Assert.Equal(0, page.Items.Length, "unpublished private definitions stay out of public catalog");
        Assert.False(page.ServerAvailable, "disconnected catalog explicitly reports last-confirmed cache");
        var target = PriceTarget.ForFurniture(new FurnitureReference {
            Scope = furniture.Scope, FurnitureId = furniture.Id, Revision = furniture.Revision,
            VariantId = furniture.Variants[0].Id, SchemaVersion = 1,
        });
        var fields = System.Text.Json.JsonSerializer.Deserialize<Dictionary<string, object>>(System.Text.Json.JsonSerializer.Serialize(target))!;
        var details = await supervisor.QueryAsync(Query.ForSalesCatalogGet(new GetSalesCatalogItem { Target = fields }));
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorPricingReferenceInvalidV1, details.Outcome.Payload.Code, "private Furniture cannot resolve through public detail query");
        var checkedSelection = await supervisor.QueryAsync(Query.ForSalesCatalogCheck(new CheckSalesConfiguration {
            Selection = new() { Target = fields, PriceRevision = 1, Quantity = 1 }, Dimensions = furniture.Variants[0].Dimensions,
        }));
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorPricingReferenceInvalidV1, checkedSelection.Outcome.Payload.Code, "unpublished configuration fails through native IPC");
    }

    private static async Task<CatalogImageRef> ImportSyntheticCatalogImage(EngineSupervisor supervisor, string directory, CatalogImageKind kind)
    {
        var source = Path.Combine(directory, "synthetic-catalog.png");
        await File.WriteAllBytesAsync(source, Convert.FromBase64String("iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAEklEQVR4nGPYkGCwIcGAAUIBACUOBQFezV2LAAAAAElFTkSuQmCC"));
        var request = Command.ForCatalogImageImport(new ImportCatalogImage { Kind = kind, SourcePath = source });
        var key = Guid.NewGuid();
        var imported = await supervisor.SubmitCommandAsync(request, key);
        Assert.Equal(CommandOutcomeStatus.Succeeded, imported.Outcome.Status, "Rust image import commits");
        File.Delete(source);
        var retried = await supervisor.SubmitCommandAsync(request, key);
        Assert.Equal(imported.Outcome.Payload.AsCatalogImageImported()!.Id, retried.Outcome.Payload.AsCatalogImageImported()!.Id, "image exact retry survives source removal");
        var invalid = await supervisor.SubmitCommandAsync(Command.ForCatalogImageImport(new ImportCatalogImage { Kind = kind, SourcePath = source }), Guid.NewGuid());
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorCatalogImageInvalidV1, invalid.Outcome.Payload.Code, "missing source fails without private diagnostics");
        return imported.Outcome.Payload.AsCatalogImageImported()!;
    }

    private static async Task<Product> SaveSupplierProduct(EngineSupervisor supervisor, CatalogImageRef image)
    {
        var categoryResponse = await supervisor.SubmitCommandAsync(
            Command.ForProductCategorySave(new SaveProductCategory { Name = "مراتب" }), Guid.NewGuid());
        Assert.Equal(CommandOutcomeStatus.Succeeded, categoryResponse.Outcome.Status, "product category commit");
        var categories = await supervisor.QueryAsync(Query.ForProductCategoryList(new ListProductCategories { Limit = 100 }));
        await using var events = await supervisor.SubscribeAsync(Subscription.ForProductChangedSubscribe(new ProductChanges()));
        var input = new SaveProduct
        {
            Image = image,
            Name = "مرتبة طبية", CategoryId = categories.Outcome.Payload.AsProductCategories()!.Items.Single().Id,
            Description = "منتج جاهز", Notes = "",
            Variants = [new SaveProductVariant { Id = Guid.NewGuid(), Name = "مفرد", PurchaseCostYer = 55000 }],
        };
        var key = Guid.NewGuid();
        var saved = await supervisor.SubmitCommandAsync(Command.ForProductSave(input), key);
        Assert.Equal(CommandOutcomeStatus.Succeeded, saved.Outcome.Status, "product commit");
        var retried = await supervisor.SubmitCommandAsync(Command.ForProductSave(input), key);
        Assert.Equal(saved.Outcome.Payload.AsProductSaved()!.Id, retried.Outcome.Payload.AsProductSaved()!.Id, "product exact retry identity");
        var page = await supervisor.QueryAsync(Query.ForProductList(new ListProducts { Term = "", Limit = 100 }));
        var product = page.Outcome.Payload.AsProducts()!.Items.Single();
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        await foreach (var delivered in events.ReadAllAsync(timeout.Token))
        {
            var notice = EngineContractCodec.DecodeEvent(delivered).AsProductChangedEvent();
            events.Acknowledge(delivered);
            Assert.Equal(product.Id, notice!.Id, "product event after commit");
            break;
        }
        var relationships = await supervisor.QueryAsync(Query.ForAuthorizationRelationshipsList(new ListScopeRelationships { Limit = 100 }));
        Assert.Equal(CommandOutcomeStatus.Failed, relationships.Outcome.Status, "Manager cannot read owner relationships");
        Assert.Equal(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1, relationships.Outcome.Payload.Code,
            "owner relationship query remains denied");
        var accounts = await supervisor.QueryAsync(Query.ForDesktopAccountList(new ListDesktopAccounts()));
        Assert.Equal(CommandOutcomeStatus.Succeeded, accounts.Outcome.Status, "Manager can list desktop accounts");
        var receptionist = accounts.Outcome.Payload.AsDesktopAccounts()!.Accounts.Single(account => account.Username == "rec");
        var changed = await supervisor.SubmitCommandAsync(Command.ForDesktopAccountUpdate(new UpdateDesktopAccount
        {
            AccountId = receptionist.AccountId,
            ExpectedRevision = receptionist.Revision,
            DisplayName = receptionist.DisplayName,
            Role = DesktopAccountRole.Manager,
        }), Guid.NewGuid());
        Assert.Equal(CommandOutcomeStatus.Succeeded, changed.Outcome.Status, "organization policy change commits");
        var invalidated = false;
        try
        {
            await foreach (var _ in events.ReadAllAsync(timeout.Token)) { }
        }
        catch (EngineIpcException error)
        {
            Assert.Equal(EngineIpcFailureKind.SessionChanged, error.Kind, "product policy closure requires projection invalidation");
            invalidated = true;
        }
        Assert.True(invalidated, "policy change closes Products even when read access remains");
        var retained = await supervisor.QueryAsync(Query.ForProductList(new ListProducts { Term = "", Limit = 100 }));
        Assert.Equal(CommandOutcomeStatus.Succeeded, retained.Outcome.Status, "Manager product read remains authorized");
        Assert.True(retained.Outcome.Payload.AsProducts()!.CanReadCosts, "existing Manager cost access remains authorized");
        return product;
    }

    private static async Task<Part> SaveMultiMaterialPart(EngineSupervisor supervisor)
    {
        Assert.True(supervisor.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityPartV1),"real part capability negotiated");
        await supervisor.SubmitCommandAsync(Command.ForMaterialCategorySave(new SaveMaterialCategory { Name = "أخشاب" }),Guid.NewGuid());
        await supervisor.SubmitCommandAsync(Command.ForMaterialUnitSave(new SaveMaterialUnit { Name = "متر مربع", Symbol = "m²", Dimension = UnitDimension.Area, Numerator = 1, Denominator = 1 }),Guid.NewGuid());
        await supervisor.SubmitCommandAsync(Command.ForMaterialUnitSave(new SaveMaterialUnit { Name = "متر", Symbol = "m", Dimension = UnitDimension.Length, Numerator = 1, Denominator = 1 }),Guid.NewGuid());
        var refsResponse = await supervisor.QueryAsync(Query.ForMaterialReferenceList(new ListMaterialReferences()));
        var refs = refsResponse.Outcome.Payload.AsMaterialReferences()!;
        var area = refs.Units.Single(u => u.Dimension == UnitDimension.Area);
        var length = refs.Units.Single(u => u.Dimension == UnitDimension.Length);
        foreach (var (name,unit,cost) in new[] { ("MDF 18mm",area,7250L),("شريط حافة",length,250L) })
            await supervisor.SubmitCommandAsync(Command.ForMaterialSave(new SaveMaterial { Name = name, CategoryId = refs.Categories.Single().Id, UnitId = unit.Id, CurrentCostYer = cost }),Guid.NewGuid());
        var materialResponse = await supervisor.QueryAsync(Query.ForMaterialList(new ListMaterials { Term = "", Limit = 100 }));
        var materials = materialResponse.Outcome.Payload.AsMaterials()!.Items;
        await supervisor.SubmitCommandAsync(Command.ForPartCategorySave(new SavePartCategory { Name = "خزانة ملابس" }),Guid.NewGuid());
        var categories = await supervisor.QueryAsync(Query.ForPartCategoryList(new ListPartCategories { Limit = 100 }));
        var usages = new[]
        {
            new PartUsage { MaterialId = materials.Single(m => m.UnitId == area.Id).Id, MaterialRevision = 1, UnitId = area.Id, UnitRevision = 1, Quantity = "1.2" },
            new PartUsage { MaterialId = materials.Single(m => m.UnitId == length.Id).Id, MaterialRevision = 1, UnitId = length.Id, UnitRevision = 1, Quantity = "3" },
        };
        var calculated = await supervisor.QueryAsync(Query.ForPartCost(new CalculatePartCost { Usages = usages }));
        Assert.Equal(9450L,calculated.Outcome.Payload.AsPartCost()!.TotalCostYer,"real Rust cost review");
        await using var partEvents = await supervisor.SubscribeAsync(Subscription.ForPartChangedSubscribe(new PartChanges()));
        var command = Command.ForPartSave(new SavePart { Name = "جانب خزانة", CategoryId = categories.Outcome.Payload.AsPartCategories()!.Items.Single().Id, Description = "جزء تجريبي", Usages = usages });
        var key = Guid.NewGuid();
        var saved = await supervisor.SubmitCommandAsync(command,key);
        Assert.Equal(CommandOutcomeStatus.Succeeded,saved.Outcome.Status,"real part save");
        var retried = await supervisor.SubmitCommandAsync(command,key);
        Assert.Equal(saved.Outcome.Payload.AsPartSaved()!.Id,retried.Outcome.Payload.AsPartSaved()!.Id,"part retry preserves identity");
        var parts = await supervisor.QueryAsync(Query.ForPartList(new ListParts { Term = "", Limit = 100 }));
        var part = parts.Outcome.Payload.AsParts()!.Items.Single().Part;
        using var cancellation = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        await foreach (var delivered in partEvents.ReadAllAsync(cancellation.Token))
        {
            var notice = EngineContractCodec.DecodeEvent(delivered).AsPartChangedEvent();
            partEvents.Acknowledge(delivered);
            Assert.Equal(part.Id,notice!.Id,"real part subscription event identity");
            Assert.Equal(part.Revision,notice.Revision,"real part subscription event revision");
            break;
        }
        return part;
    }

    private static async Task SeedDevelopmentAccounts(string enginePath, string runtimeDirectory)
    {
        var start = new System.Diagnostics.ProcessStartInfo(enginePath)
        {
            CreateNoWindow = true,
            RedirectStandardError = true,
            RedirectStandardOutput = true,
            UseShellExecute = false,
        };
        start.ArgumentList.Add("seed-development-accounts");
        start.ArgumentList.Add("--runtime-directory");
        start.ArgumentList.Add(runtimeDirectory);
        using var process = System.Diagnostics.Process.Start(start)
            ?? throw new InvalidOperationException("Development account seed process did not start.");
        await process.WaitForExitAsync();
        Assert.Equal(0, process.ExitCode, "development account seed exit code");
    }

    private static PeerHello DevelopmentPeer() => new()
    {
        PeerKind = PeerKind.Shell,
        ProductVersion = "0.0.0",
        Protocols = [new SupportedProtocol { Major = 1, MinimumMinor = 0, MaximumMinor = 3 }],
        Capabilities =
        [
            ProtocolIds.Capabilities.EitmadCapabilityLocalIpcV1,
            ProtocolIds.Capabilities.EitmadCapabilityAuthorizationScopesV1,
        ],
        RequiredCapabilities =
        [
            ProtocolIds.Capabilities.EitmadCapabilityLocalIpcV1,
            ProtocolIds.Capabilities.EitmadCapabilityAuthorizationScopesV1,
        ],
        Schemas = [],
    };

    private static async Task Eventually(Func<bool> condition, TimeSpan? timeout = null)
    {
        var deadline = DateTime.UtcNow + (timeout ?? TimeSpan.FromSeconds(2));
        while (!condition())
        {
            if (DateTime.UtcNow >= deadline)
            {
                throw new InvalidOperationException("Expected condition was not reached before timeout.");
            }

            await Task.Delay(5);
        }
    }
}

internal sealed class SupervisorFixture
{
    public SupervisorFixture()
    {
        Group.Processes = Launcher.Processes;
        Supervisor = new EngineSupervisor(Launcher, new FakeProcessGroupFactory(Group), Clock);
    }

    public FakeClock Clock { get; } = new();
    public FakeProcessLauncher Launcher { get; } = new();
    public FakeProcessGroup Group { get; } = new();
    public EngineSupervisor Supervisor { get; }
    public EngineLaunchRequest Request { get; } = new("C:\\synthetic\\eitmad-engine-cli.exe");

    public async Task StopCurrent()
    {
        var process = Launcher.Current;
        var stop = Supervisor.StopAsync();
        process.Exit(0);
        await stop;
    }
}

internal sealed class FakeClock : ISupervisionClock
{
    private readonly object gate = new();
    private readonly List<PendingDelay> delays = [];

    public DateTimeOffset UtcNow { get; private set; } = new(2026, 7, 12, 0, 0, 0, TimeSpan.Zero);

    public Task DelayAsync(TimeSpan delay, CancellationToken cancellationToken)
    {
        var completion = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var registration = cancellationToken.Register(() => completion.TrySetCanceled(cancellationToken));
        lock (gate)
        {
            delays.Add(new PendingDelay(delay, completion, registration));
        }

        return completion.Task;
    }

    public void CompleteDelay(TimeSpan delay)
    {
        PendingDelay pending;
        lock (gate)
        {
            var index = delays.FindIndex(item => item.Duration == delay);
            if (index < 0)
            {
                throw new InvalidOperationException($"No pending delay for {delay}.");
            }

            pending = delays[index];
            delays.RemoveAt(index);
            UtcNow += delay;
        }

        pending.Registration.Dispose();
        pending.Completion.SetResult();
    }

    public bool HasPendingDelay(TimeSpan delay)
    {
        lock (gate)
        {
            return delays.Any(item => item.Duration == delay);
        }
    }

    private sealed record PendingDelay(
        TimeSpan Duration,
        TaskCompletionSource Completion,
        CancellationTokenRegistration Registration);
}

internal sealed class FakeProcessLauncher : IEngineProcessLauncher
{
    public List<FakeEngineProcess> Processes { get; } = [];
    public int LaunchCount => Processes.Count;
    public FakeEngineProcess Current => Processes[^1];

    public IEngineProcess Launch(EngineLaunchRequest request, long generation, IProcessGroup group)
    {
        var process = new FakeEngineProcess(1000 + (int)generation);
        Processes.Add(process);
        group.Assign(process);
        return process;
    }
}

internal sealed class FakeEngineProcess(int processId) : IEngineProcess
{
    private readonly TaskCompletionSource<int> exit = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private readonly TrackingWriter input = new();

    public int ProcessId { get; } = processId;
    public nint NativeHandle => 0;
    public TextReader StandardOutput { get; } = new StringReader(string.Empty);
    public TextReader StandardError { get; } = new StringReader(string.Empty);
    public TextWriter StandardInput => input;
    public string IpcPipeName { get; } = $"fake-{processId}";
    public string IpcBootstrapToken { get; } = "fake-bootstrap-token";
    public bool InputClosed => input.IsClosed;
    public bool Disposed { get; private set; }

    public Task<int> WaitForExitAsync(CancellationToken cancellationToken) => exit.Task.WaitAsync(cancellationToken);
    public void Kill() => Exit(137);
    public void Exit(int code) => exit.TrySetResult(code);
    public ValueTask DisposeAsync()
    {
        Disposed = true;
        input.Dispose();
        return ValueTask.CompletedTask;
    }

    private sealed class TrackingWriter : StringWriter
    {
        public bool IsClosed { get; private set; }
        protected override void Dispose(bool disposing)
        {
            IsClosed = true;
            base.Dispose(disposing);
        }
    }
}

internal sealed class FakeProcessGroupFactory(FakeProcessGroup group) : IProcessGroupFactory
{
    public IProcessGroup Create() => group;
}

internal sealed class FakeProcessGroup : IProcessGroup
{
    public List<FakeEngineProcess> Processes { get; set; } = [];
    public bool Terminated { get; private set; }
    public Exception? TerminationException { get; set; }
    public void Assign(IEngineProcess process) { }
    public void Terminate()
    {
        Terminated = true;
        if (TerminationException is { } exception)
        {
            throw exception;
        }

        foreach (var process in Processes)
        {
            process.Exit(137);
        }
    }

    public void Dispose() { }
}

internal static class Assert
{
    public static void True(bool value, string message)
    {
        if (!value) throw new InvalidOperationException($"Assertion failed: {message}.");
    }

    public static void False(bool value, string message) => True(!value, message);

    public static void Equal<T>(T expected, T actual, string message)
    {
        if (!EqualityComparer<T>.Default.Equals(expected, actual))
        {
            throw new InvalidOperationException($"Assertion failed: {message}. Expected {expected}; actual {actual}.");
        }
    }

    public static void SequenceEqual<T>(IEnumerable<T> expected, IEnumerable<T> actual, string message)
    {
        if (!expected.SequenceEqual(actual))
        {
            throw new InvalidOperationException($"Assertion failed: {message}.");
        }
    }

    public static async Task ThrowsAsync<TException>(Func<Task> action, string message)
        where TException : Exception
    {
        try
        {
            await action();
        }
        catch (TException)
        {
            return;
        }

        throw new InvalidOperationException($"Assertion failed: {message}. Expected {typeof(TException).Name}.");
    }
}
