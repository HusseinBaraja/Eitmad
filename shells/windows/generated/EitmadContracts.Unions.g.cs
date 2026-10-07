// Generated from Rust contracts. Do not edit.
#nullable enable
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Eitmad.Contracts;

public partial class Command
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string QuotationApprovalRequestKind = "eitmad.quotation-approval.request.v1";

    public static Command ForQuotationApprovalRequest(RequestDiscountApproval payload) =>
        new() { Kind = QuotationApprovalRequestKind, Payload = payload };

    public RequestDiscountApproval? AsQuotationApprovalRequest() =>
        Kind == QuotationApprovalRequestKind ? PayloadAs<RequestDiscountApproval>() : null;

    public const string QuotationApprovalDecideKind = "eitmad.quotation-approval.decide.v1";

    public static Command ForQuotationApprovalDecide(DecideDiscountApproval payload) =>
        new() { Kind = QuotationApprovalDecideKind, Payload = payload };

    public DecideDiscountApproval? AsQuotationApprovalDecide() =>
        Kind == QuotationApprovalDecideKind ? PayloadAs<DecideDiscountApproval>() : null;

    public const string PricingPublishKind = "eitmad.pricing.publish.v1";

    public static Command ForPricingPublish(PublishPrice payload) =>
        new() { Kind = PricingPublishKind, Payload = payload };

    public PublishPrice? AsPricingPublish() =>
        Kind == PricingPublishKind ? PayloadAs<PublishPrice>() : null;

    public const string CatalogImageImportKind = "eitmad.catalog-image.import.v1";

    public static Command ForCatalogImageImport(ImportCatalogImage payload) =>
        new() { Kind = CatalogImageImportKind, Payload = payload };

    public ImportCatalogImage? AsCatalogImageImport() =>
        Kind == CatalogImageImportKind ? PayloadAs<ImportCatalogImage>() : null;

    public const string ConfigUpdateKind = "eitmad.config.update.v1";

    public static Command ForConfigUpdate(UpdateConfiguration payload) =>
        new() { Kind = ConfigUpdateKind, Payload = payload };

    public UpdateConfiguration? AsConfigUpdate() =>
        Kind == ConfigUpdateKind ? PayloadAs<UpdateConfiguration>() : null;

    public const string AuthorizationRelationshipGrantKind = "eitmad.authorization.relationship.grant.v1";

    public static Command ForAuthorizationRelationshipGrant(GrantScopeRelationship payload) =>
        new() { Kind = AuthorizationRelationshipGrantKind, Payload = payload };

    public GrantScopeRelationship? AsAuthorizationRelationshipGrant() =>
        Kind == AuthorizationRelationshipGrantKind ? PayloadAs<GrantScopeRelationship>() : null;

    public const string AuthorizationRelationshipRevokeKind = "eitmad.authorization.relationship.revoke.v1";

    public static Command ForAuthorizationRelationshipRevoke(RevokeScopeRelationship payload) =>
        new() { Kind = AuthorizationRelationshipRevokeKind, Payload = payload };

    public RevokeScopeRelationship? AsAuthorizationRelationshipRevoke() =>
        Kind == AuthorizationRelationshipRevokeKind ? PayloadAs<RevokeScopeRelationship>() : null;

    public const string QuotationDraftCreateKind = "eitmad.quotation-draft.create.v1";

    public static Command ForQuotationDraftCreate(CreateQuotationDraft payload) =>
        new() { Kind = QuotationDraftCreateKind, Payload = payload };

    public CreateQuotationDraft? AsQuotationDraftCreate() =>
        Kind == QuotationDraftCreateKind ? PayloadAs<CreateQuotationDraft>() : null;

    public const string QuotationDraftUpdateKind = "eitmad.quotation-draft.update.v1";

    public static Command ForQuotationDraftUpdate(UpdateQuotationDraft payload) =>
        new() { Kind = QuotationDraftUpdateKind, Payload = payload };

    public UpdateQuotationDraft? AsQuotationDraftUpdate() =>
        Kind == QuotationDraftUpdateKind ? PayloadAs<UpdateQuotationDraft>() : null;

    public const string CustomerCreateKind = "eitmad.customer.create.v1";

    public static Command ForCustomerCreate(CreateCustomer payload) =>
        new() { Kind = CustomerCreateKind, Payload = payload };

    public CreateCustomer? AsCustomerCreate() =>
        Kind == CustomerCreateKind ? PayloadAs<CreateCustomer>() : null;

    public const string CustomerUpdateKind = "eitmad.customer.update.v1";

    public static Command ForCustomerUpdate(UpdateCustomer payload) =>
        new() { Kind = CustomerUpdateKind, Payload = payload };

    public UpdateCustomer? AsCustomerUpdate() =>
        Kind == CustomerUpdateKind ? PayloadAs<UpdateCustomer>() : null;

    public const string MaterialCategorySaveKind = "eitmad.material-category.save.v1";

    public static Command ForMaterialCategorySave(SaveMaterialCategory payload) =>
        new() { Kind = MaterialCategorySaveKind, Payload = payload };

    public SaveMaterialCategory? AsMaterialCategorySave() =>
        Kind == MaterialCategorySaveKind ? PayloadAs<SaveMaterialCategory>() : null;

    public const string MaterialUnitSaveKind = "eitmad.material-unit.save.v1";

    public static Command ForMaterialUnitSave(SaveMaterialUnit payload) =>
        new() { Kind = MaterialUnitSaveKind, Payload = payload };

    public SaveMaterialUnit? AsMaterialUnitSave() =>
        Kind == MaterialUnitSaveKind ? PayloadAs<SaveMaterialUnit>() : null;

    public const string MaterialSaveKind = "eitmad.material.save.v1";

    public static Command ForMaterialSave(SaveMaterial payload) =>
        new() { Kind = MaterialSaveKind, Payload = payload };

    public SaveMaterial? AsMaterialSave() =>
        Kind == MaterialSaveKind ? PayloadAs<SaveMaterial>() : null;

    public const string FurnitureSaveKind = "eitmad.furniture.save.v1";

    public static Command ForFurnitureSave(SaveFurniture payload) =>
        new() { Kind = FurnitureSaveKind, Payload = payload };

    public SaveFurniture? AsFurnitureSave() =>
        Kind == FurnitureSaveKind ? PayloadAs<SaveFurniture>() : null;

    public const string FurnitureCategorySaveKind = "eitmad.furniture-category.save.v1";

    public static Command ForFurnitureCategorySave(SaveFurnitureCategory payload) =>
        new() { Kind = FurnitureCategorySaveKind, Payload = payload };

    public SaveFurnitureCategory? AsFurnitureCategorySave() =>
        Kind == FurnitureCategorySaveKind ? PayloadAs<SaveFurnitureCategory>() : null;

    public const string ProductSaveKind = "eitmad.product.save.v1";

    public static Command ForProductSave(SaveProduct payload) =>
        new() { Kind = ProductSaveKind, Payload = payload };

    public SaveProduct? AsProductSave() =>
        Kind == ProductSaveKind ? PayloadAs<SaveProduct>() : null;

    public const string ProductCategorySaveKind = "eitmad.product-category.save.v1";

    public static Command ForProductCategorySave(SaveProductCategory payload) =>
        new() { Kind = ProductCategorySaveKind, Payload = payload };

    public SaveProductCategory? AsProductCategorySave() =>
        Kind == ProductCategorySaveKind ? PayloadAs<SaveProductCategory>() : null;

    public const string PartSaveKind = "eitmad.part.save.v1";

    public static Command ForPartSave(SavePart payload) =>
        new() { Kind = PartSaveKind, Payload = payload };

    public SavePart? AsPartSave() =>
        Kind == PartSaveKind ? PayloadAs<SavePart>() : null;

    public const string PartCategorySaveKind = "eitmad.part-category.save.v1";

    public static Command ForPartCategorySave(SavePartCategory payload) =>
        new() { Kind = PartCategorySaveKind, Payload = payload };

    public SavePartCategory? AsPartCategorySave() =>
        Kind == PartCategorySaveKind ? PayloadAs<SavePartCategory>() : null;

    public const string DesktopAccountCreateKind = "eitmad.desktop-account.create.v1";

    public static Command ForDesktopAccountCreate(CreateDesktopAccount payload) =>
        new() { Kind = DesktopAccountCreateKind, Payload = payload };

    public CreateDesktopAccount? AsDesktopAccountCreate() =>
        Kind == DesktopAccountCreateKind ? PayloadAs<CreateDesktopAccount>() : null;

    public const string DesktopAccountUpdateKind = "eitmad.desktop-account.update.v1";

    public static Command ForDesktopAccountUpdate(UpdateDesktopAccount payload) =>
        new() { Kind = DesktopAccountUpdateKind, Payload = payload };

    public UpdateDesktopAccount? AsDesktopAccountUpdate() =>
        Kind == DesktopAccountUpdateKind ? PayloadAs<UpdateDesktopAccount>() : null;

    public const string DesktopAccountDeactivateKind = "eitmad.desktop-account.deactivate.v1";

    public static Command ForDesktopAccountDeactivate(DeactivateDesktopAccount payload) =>
        new() { Kind = DesktopAccountDeactivateKind, Payload = payload };

    public DeactivateDesktopAccount? AsDesktopAccountDeactivate() =>
        Kind == DesktopAccountDeactivateKind ? PayloadAs<DeactivateDesktopAccount>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class CommandResult
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string DiscountApprovalKind = "discountApproval";

    public static CommandResult ForDiscountApproval(DiscountApproval payload) =>
        new() { Kind = DiscountApprovalKind, Payload = payload };

    public DiscountApproval? AsDiscountApproval() =>
        Kind == DiscountApprovalKind ? PayloadAs<DiscountApproval>() : null;

    public const string PricePublishedKind = "pricePublished";

    public static CommandResult ForPricePublished(PublishedPrice payload) =>
        new() { Kind = PricePublishedKind, Payload = payload };

    public PublishedPrice? AsPricePublished() =>
        Kind == PricePublishedKind ? PayloadAs<PublishedPrice>() : null;

    public const string CatalogImageImportedKind = "catalogImageImported";

    public static CommandResult ForCatalogImageImported(CatalogImageRef payload) =>
        new() { Kind = CatalogImageImportedKind, Payload = payload };

    public CatalogImageRef? AsCatalogImageImported() =>
        Kind == CatalogImageImportedKind ? PayloadAs<CatalogImageRef>() : null;

    public const string ConfigurationUpdatedKind = "configurationUpdated";

    public static CommandResult ForConfigurationUpdated(ConfigSnapshot payload) =>
        new() { Kind = ConfigurationUpdatedKind, Payload = payload };

    public ConfigSnapshot? AsConfigurationUpdated() =>
        Kind == ConfigurationUpdatedKind ? PayloadAs<ConfigSnapshot>() : null;

    public const string RelationshipGrantedKind = "relationshipGranted";

    public static CommandResult ForRelationshipGranted(RelationshipMutationResult payload) =>
        new() { Kind = RelationshipGrantedKind, Payload = payload };

    public RelationshipMutationResult? AsRelationshipGranted() =>
        Kind == RelationshipGrantedKind ? PayloadAs<RelationshipMutationResult>() : null;

    public const string RelationshipRevokedKind = "relationshipRevoked";

    public static CommandResult ForRelationshipRevoked(RelationshipMutationResult payload) =>
        new() { Kind = RelationshipRevokedKind, Payload = payload };

    public RelationshipMutationResult? AsRelationshipRevoked() =>
        Kind == RelationshipRevokedKind ? PayloadAs<RelationshipMutationResult>() : null;

    public const string QuotationDraftCreatedKind = "quotationDraftCreated";

    public static CommandResult ForQuotationDraftCreated(QuotationDraft payload) =>
        new() { Kind = QuotationDraftCreatedKind, Payload = payload };

    public QuotationDraft? AsQuotationDraftCreated() =>
        Kind == QuotationDraftCreatedKind ? PayloadAs<QuotationDraft>() : null;

    public const string QuotationDraftUpdatedKind = "quotationDraftUpdated";

    public static CommandResult ForQuotationDraftUpdated(QuotationDraft payload) =>
        new() { Kind = QuotationDraftUpdatedKind, Payload = payload };

    public QuotationDraft? AsQuotationDraftUpdated() =>
        Kind == QuotationDraftUpdatedKind ? PayloadAs<QuotationDraft>() : null;

    public const string CustomerCreatedKind = "customerCreated";

    public static CommandResult ForCustomerCreated(CustomerMutationResult payload) =>
        new() { Kind = CustomerCreatedKind, Payload = payload };

    public CustomerMutationResult? AsCustomerCreated() =>
        Kind == CustomerCreatedKind ? PayloadAs<CustomerMutationResult>() : null;

    public const string CustomerUpdatedKind = "customerUpdated";

    public static CommandResult ForCustomerUpdated(CustomerMutationResult payload) =>
        new() { Kind = CustomerUpdatedKind, Payload = payload };

    public CustomerMutationResult? AsCustomerUpdated() =>
        Kind == CustomerUpdatedKind ? PayloadAs<CustomerMutationResult>() : null;

    public const string MaterialCategorySavedKind = "materialCategorySaved";

    public static CommandResult ForMaterialCategorySaved(MaterialCategory payload) =>
        new() { Kind = MaterialCategorySavedKind, Payload = payload };

    public MaterialCategory? AsMaterialCategorySaved() =>
        Kind == MaterialCategorySavedKind ? PayloadAs<MaterialCategory>() : null;

    public const string MaterialUnitSavedKind = "materialUnitSaved";

    public static CommandResult ForMaterialUnitSaved(MaterialUnit payload) =>
        new() { Kind = MaterialUnitSavedKind, Payload = payload };

    public MaterialUnit? AsMaterialUnitSaved() =>
        Kind == MaterialUnitSavedKind ? PayloadAs<MaterialUnit>() : null;

    public const string MaterialSavedKind = "materialSaved";

    public static CommandResult ForMaterialSaved(Material payload) =>
        new() { Kind = MaterialSavedKind, Payload = payload };

    public Material? AsMaterialSaved() =>
        Kind == MaterialSavedKind ? PayloadAs<Material>() : null;

    public const string FurnitureSavedKind = "furnitureSaved";

    public static CommandResult ForFurnitureSaved(Furniture payload) =>
        new() { Kind = FurnitureSavedKind, Payload = payload };

    public Furniture? AsFurnitureSaved() =>
        Kind == FurnitureSavedKind ? PayloadAs<Furniture>() : null;

    public const string FurnitureCategorySavedKind = "furnitureCategorySaved";

    public static CommandResult ForFurnitureCategorySaved(FurnitureCategory payload) =>
        new() { Kind = FurnitureCategorySavedKind, Payload = payload };

    public FurnitureCategory? AsFurnitureCategorySaved() =>
        Kind == FurnitureCategorySavedKind ? PayloadAs<FurnitureCategory>() : null;

    public const string ProductSavedKind = "productSaved";

    public static CommandResult ForProductSaved(Product payload) =>
        new() { Kind = ProductSavedKind, Payload = payload };

    public Product? AsProductSaved() =>
        Kind == ProductSavedKind ? PayloadAs<Product>() : null;

    public const string ProductCategorySavedKind = "productCategorySaved";

    public static CommandResult ForProductCategorySaved(ProductCategory payload) =>
        new() { Kind = ProductCategorySavedKind, Payload = payload };

    public ProductCategory? AsProductCategorySaved() =>
        Kind == ProductCategorySavedKind ? PayloadAs<ProductCategory>() : null;

    public const string PartSavedKind = "partSaved";

    public static CommandResult ForPartSaved(Part payload) =>
        new() { Kind = PartSavedKind, Payload = payload };

    public Part? AsPartSaved() =>
        Kind == PartSavedKind ? PayloadAs<Part>() : null;

    public const string PartCategorySavedKind = "partCategorySaved";

    public static CommandResult ForPartCategorySaved(PartCategory payload) =>
        new() { Kind = PartCategorySavedKind, Payload = payload };

    public PartCategory? AsPartCategorySaved() =>
        Kind == PartCategorySavedKind ? PayloadAs<PartCategory>() : null;

    public const string DesktopAccountCreatedKind = "desktopAccountCreated";

    public static CommandResult ForDesktopAccountCreated(DesktopAccountSummary payload) =>
        new() { Kind = DesktopAccountCreatedKind, Payload = payload };

    public DesktopAccountSummary? AsDesktopAccountCreated() =>
        Kind == DesktopAccountCreatedKind ? PayloadAs<DesktopAccountSummary>() : null;

    public const string DesktopAccountUpdatedKind = "desktopAccountUpdated";

    public static CommandResult ForDesktopAccountUpdated(DesktopAccountSummary payload) =>
        new() { Kind = DesktopAccountUpdatedKind, Payload = payload };

    public DesktopAccountSummary? AsDesktopAccountUpdated() =>
        Kind == DesktopAccountUpdatedKind ? PayloadAs<DesktopAccountSummary>() : null;

    public const string DesktopAccountDeactivatedKind = "desktopAccountDeactivated";

    public static CommandResult ForDesktopAccountDeactivated(DesktopAccountSummary payload) =>
        new() { Kind = DesktopAccountDeactivatedKind, Payload = payload };

    public DesktopAccountSummary? AsDesktopAccountDeactivated() =>
        Kind == DesktopAccountDeactivatedKind ? PayloadAs<DesktopAccountSummary>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class Event
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string PricingChangedEventKind = "eitmad.pricing.changed.event.v1";

    public static Event ForPricingChangedEvent(PriceChangeNotice payload) =>
        new() { Kind = PricingChangedEventKind, Payload = payload };

    public PriceChangeNotice? AsPricingChangedEvent() =>
        Kind == PricingChangedEventKind ? PayloadAs<PriceChangeNotice>() : null;

    public const string ConfigChangedEventKind = "eitmad.config.changed.event.v1";

    public static Event ForConfigChangedEvent(ConfigSnapshot payload) =>
        new() { Kind = ConfigChangedEventKind, Payload = payload };

    public ConfigSnapshot? AsConfigChangedEvent() =>
        Kind == ConfigChangedEventKind ? PayloadAs<ConfigSnapshot>() : null;

    public const string PermissionsChangedEventKind = "eitmad.permissions.changed.event.v1";

    public static Event ForPermissionsChangedEvent(EffectivePermissions payload) =>
        new() { Kind = PermissionsChangedEventKind, Payload = payload };

    public EffectivePermissions? AsPermissionsChangedEvent() =>
        Kind == PermissionsChangedEventKind ? PayloadAs<EffectivePermissions>() : null;

    public const string AuthorizationPolicyChangedEventKind = "eitmad.authorization.policy.changed.event.v1";

    public static Event ForAuthorizationPolicyChangedEvent(AuthorizationPolicyChangeNotice payload) =>
        new() { Kind = AuthorizationPolicyChangedEventKind, Payload = payload };

    public AuthorizationPolicyChangeNotice? AsAuthorizationPolicyChangedEvent() =>
        Kind == AuthorizationPolicyChangedEventKind ? PayloadAs<AuthorizationPolicyChangeNotice>() : null;

    public const string QuotationApprovalChangedEventKind = "eitmad.quotation-approval.changed.event.v1";

    public static Event ForQuotationApprovalChangedEvent(DiscountApprovalNotice payload) =>
        new() { Kind = QuotationApprovalChangedEventKind, Payload = payload };

    public DiscountApprovalNotice? AsQuotationApprovalChangedEvent() =>
        Kind == QuotationApprovalChangedEventKind ? PayloadAs<DiscountApprovalNotice>() : null;

    public const string QuotationDraftChangedEventKind = "eitmad.quotation-draft.changed.event.v1";

    public static Event ForQuotationDraftChangedEvent(QuotationDraftChangeNotice payload) =>
        new() { Kind = QuotationDraftChangedEventKind, Payload = payload };

    public QuotationDraftChangeNotice? AsQuotationDraftChangedEvent() =>
        Kind == QuotationDraftChangedEventKind ? PayloadAs<QuotationDraftChangeNotice>() : null;

    public const string CustomerChangedEventKind = "eitmad.customer.changed.event.v1";

    public static Event ForCustomerChangedEvent(CustomerChangeNotice payload) =>
        new() { Kind = CustomerChangedEventKind, Payload = payload };

    public CustomerChangeNotice? AsCustomerChangedEvent() =>
        Kind == CustomerChangedEventKind ? PayloadAs<CustomerChangeNotice>() : null;

    public const string MaterialChangedEventKind = "eitmad.material.changed.event.v1";

    public static Event ForMaterialChangedEvent(MaterialChangeNotice payload) =>
        new() { Kind = MaterialChangedEventKind, Payload = payload };

    public MaterialChangeNotice? AsMaterialChangedEvent() =>
        Kind == MaterialChangedEventKind ? PayloadAs<MaterialChangeNotice>() : null;

    public const string FurnitureChangedEventKind = "eitmad.furniture.changed.event.v1";

    public static Event ForFurnitureChangedEvent(FurnitureChangeNotice payload) =>
        new() { Kind = FurnitureChangedEventKind, Payload = payload };

    public FurnitureChangeNotice? AsFurnitureChangedEvent() =>
        Kind == FurnitureChangedEventKind ? PayloadAs<FurnitureChangeNotice>() : null;

    public const string ProductChangedEventKind = "eitmad.product.changed.event.v1";

    public static Event ForProductChangedEvent(ProductChangeNotice payload) =>
        new() { Kind = ProductChangedEventKind, Payload = payload };

    public ProductChangeNotice? AsProductChangedEvent() =>
        Kind == ProductChangedEventKind ? PayloadAs<ProductChangeNotice>() : null;

    public const string PartChangedEventKind = "eitmad.part.changed.event.v1";

    public static Event ForPartChangedEvent(PartChangeNotice payload) =>
        new() { Kind = PartChangedEventKind, Payload = payload };

    public PartChangeNotice? AsPartChangedEvent() =>
        Kind == PartChangedEventKind ? PayloadAs<PartChangeNotice>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class IpcClientMessage
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string IpcHandshakeKind = "eitmad.ipc.handshake.v1";

    public static IpcClientMessage ForIpcHandshake(HandshakeRequest payload) =>
        new() { Kind = IpcHandshakeKind, Payload = payload };

    public HandshakeRequest? AsIpcHandshake() =>
        Kind == IpcHandshakeKind ? PayloadAs<HandshakeRequest>() : null;

    public const string IpcDesktopSignInKind = "eitmad.ipc.desktop-sign-in.v1";

    public static IpcClientMessage ForIpcDesktopSignIn(DesktopSignInRequest payload) =>
        new() { Kind = IpcDesktopSignInKind, Payload = payload };

    public DesktopSignInRequest? AsIpcDesktopSignIn() =>
        Kind == IpcDesktopSignInKind ? PayloadAs<DesktopSignInRequest>() : null;

    public const string IpcDesktopSessionStateKind = "eitmad.ipc.desktop-session-state.v1";

    public static IpcClientMessage ForIpcDesktopSessionState(DesktopSessionRequest payload) =>
        new() { Kind = IpcDesktopSessionStateKind, Payload = payload };

    public DesktopSessionRequest? AsIpcDesktopSessionState() =>
        Kind == IpcDesktopSessionStateKind ? PayloadAs<DesktopSessionRequest>() : null;

    public const string IpcDesktopSignOutKind = "eitmad.ipc.desktop-sign-out.v1";

    public static IpcClientMessage ForIpcDesktopSignOut(DesktopSessionRequest payload) =>
        new() { Kind = IpcDesktopSignOutKind, Payload = payload };

    public DesktopSessionRequest? AsIpcDesktopSignOut() =>
        Kind == IpcDesktopSignOutKind ? PayloadAs<DesktopSessionRequest>() : null;

    public const string IpcCommandKind = "eitmad.ipc.command.v1";

    public static IpcClientMessage ForIpcCommand(CommandEnvelope payload) =>
        new() { Kind = IpcCommandKind, Payload = payload };

    public CommandEnvelope? AsIpcCommand() =>
        Kind == IpcCommandKind ? PayloadAs<CommandEnvelope>() : null;

    public const string IpcQueryKind = "eitmad.ipc.query.v1";

    public static IpcClientMessage ForIpcQuery(QueryEnvelope payload) =>
        new() { Kind = IpcQueryKind, Payload = payload };

    public QueryEnvelope? AsIpcQuery() =>
        Kind == IpcQueryKind ? PayloadAs<QueryEnvelope>() : null;

    public const string IpcSubscribeKind = "eitmad.ipc.subscribe.v1";

    public static IpcClientMessage ForIpcSubscribe(SubscriptionEnvelope payload) =>
        new() { Kind = IpcSubscribeKind, Payload = payload };

    public SubscriptionEnvelope? AsIpcSubscribe() =>
        Kind == IpcSubscribeKind ? PayloadAs<SubscriptionEnvelope>() : null;

    public const string IpcUnsubscribeKind = "eitmad.ipc.unsubscribe.v1";

    public static IpcClientMessage ForIpcUnsubscribe(UnsubscribeRequest payload) =>
        new() { Kind = IpcUnsubscribeKind, Payload = payload };

    public UnsubscribeRequest? AsIpcUnsubscribe() =>
        Kind == IpcUnsubscribeKind ? PayloadAs<UnsubscribeRequest>() : null;

    public const string IpcShutdownKind = "eitmad.ipc.shutdown.v1";

    public static IpcClientMessage ForIpcShutdown(ShutdownRequest payload) =>
        new() { Kind = IpcShutdownKind, Payload = payload };

    public ShutdownRequest? AsIpcShutdown() =>
        Kind == IpcShutdownKind ? PayloadAs<ShutdownRequest>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class IpcServerMessage
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string IpcHandshakeResponseKind = "eitmad.ipc.handshake-response.v1";

    public static IpcServerMessage ForIpcHandshakeResponse(HandshakeResponse payload) =>
        new() { Kind = IpcHandshakeResponseKind, Payload = payload };

    public HandshakeResponse? AsIpcHandshakeResponse() =>
        Kind == IpcHandshakeResponseKind ? PayloadAs<HandshakeResponse>() : null;

    public const string IpcDesktopSessionResponseKind = "eitmad.ipc.desktop-session-response.v1";

    public static IpcServerMessage ForIpcDesktopSessionResponse(DesktopSessionResponse payload) =>
        new() { Kind = IpcDesktopSessionResponseKind, Payload = payload };

    public DesktopSessionResponse? AsIpcDesktopSessionResponse() =>
        Kind == IpcDesktopSessionResponseKind ? PayloadAs<DesktopSessionResponse>() : null;

    public const string IpcCommandResponseKind = "eitmad.ipc.command-response.v1";

    public static IpcServerMessage ForIpcCommandResponse(CommandResponseEnvelope payload) =>
        new() { Kind = IpcCommandResponseKind, Payload = payload };

    public CommandResponseEnvelope? AsIpcCommandResponse() =>
        Kind == IpcCommandResponseKind ? PayloadAs<CommandResponseEnvelope>() : null;

    public const string IpcQueryResponseKind = "eitmad.ipc.query-response.v1";

    public static IpcServerMessage ForIpcQueryResponse(QueryResponseEnvelope payload) =>
        new() { Kind = IpcQueryResponseKind, Payload = payload };

    public QueryResponseEnvelope? AsIpcQueryResponse() =>
        Kind == IpcQueryResponseKind ? PayloadAs<QueryResponseEnvelope>() : null;

    public const string IpcSubscribeResponseKind = "eitmad.ipc.subscribe-response.v1";

    public static IpcServerMessage ForIpcSubscribeResponse(SubscriptionResponseEnvelope payload) =>
        new() { Kind = IpcSubscribeResponseKind, Payload = payload };

    public SubscriptionResponseEnvelope? AsIpcSubscribeResponse() =>
        Kind == IpcSubscribeResponseKind ? PayloadAs<SubscriptionResponseEnvelope>() : null;

    public const string IpcUnsubscribeResponseKind = "eitmad.ipc.unsubscribe-response.v1";

    public static IpcServerMessage ForIpcUnsubscribeResponse(UnsubscribeResponse payload) =>
        new() { Kind = IpcUnsubscribeResponseKind, Payload = payload };

    public UnsubscribeResponse? AsIpcUnsubscribeResponse() =>
        Kind == IpcUnsubscribeResponseKind ? PayloadAs<UnsubscribeResponse>() : null;

    public const string IpcEventKind = "eitmad.ipc.event.v1";

    public static IpcServerMessage ForIpcEvent(EventEnvelope payload) =>
        new() { Kind = IpcEventKind, Payload = payload };

    public EventEnvelope? AsIpcEvent() =>
        Kind == IpcEventKind ? PayloadAs<EventEnvelope>() : null;

    public const string IpcSubscriptionClosedKind = "eitmad.ipc.subscription-closed.v1";

    public static IpcServerMessage ForIpcSubscriptionClosed(SubscriptionClosedEnvelope payload) =>
        new() { Kind = IpcSubscriptionClosedKind, Payload = payload };

    public SubscriptionClosedEnvelope? AsIpcSubscriptionClosed() =>
        Kind == IpcSubscriptionClosedKind ? PayloadAs<SubscriptionClosedEnvelope>() : null;

    public const string IpcShutdownResponseKind = "eitmad.ipc.shutdown-response.v1";

    public static IpcServerMessage ForIpcShutdownResponse(ShutdownResponse payload) =>
        new() { Kind = IpcShutdownResponseKind, Payload = payload };

    public ShutdownResponse? AsIpcShutdownResponse() =>
        Kind == IpcShutdownResponseKind ? PayloadAs<ShutdownResponse>() : null;

    public const string IpcFailureKind = "eitmad.ipc.failure.v1";

    public static IpcServerMessage ForIpcFailure(IpcFailureResponse payload) =>
        new() { Kind = IpcFailureKind, Payload = payload };

    public IpcFailureResponse? AsIpcFailure() =>
        Kind == IpcFailureKind ? PayloadAs<IpcFailureResponse>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class PriceTarget
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string ProductKind = "product";

    public static PriceTarget ForProduct(ProductReference payload) =>
        new() { Kind = ProductKind, Payload = payload };

    public ProductReference? AsProduct() =>
        Kind == ProductKind ? PayloadAs<ProductReference>() : null;

    public const string FurnitureKind = "furniture";

    public static PriceTarget ForFurniture(FurnitureReference payload) =>
        new() { Kind = FurnitureKind, Payload = payload };

    public FurnitureReference? AsFurniture() =>
        Kind == FurnitureKind ? PayloadAs<FurnitureReference>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class Query
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string QuotationDraftGetKind = "eitmad.quotation-draft.get.v1";

    public static Query ForQuotationDraftGet(GetQuotationDraft payload) =>
        new() { Kind = QuotationDraftGetKind, Payload = payload };

    public GetQuotationDraft? AsQuotationDraftGet() =>
        Kind == QuotationDraftGetKind ? PayloadAs<GetQuotationDraft>() : null;

    public const string QuotationApprovalListKind = "eitmad.quotation-approval.list.v1";

    public static Query ForQuotationApprovalList(ListDiscountApprovals payload) =>
        new() { Kind = QuotationApprovalListKind, Payload = payload };

    public ListDiscountApprovals? AsQuotationApprovalList() =>
        Kind == QuotationApprovalListKind ? PayloadAs<ListDiscountApprovals>() : null;

    public const string QuotationDraftListKind = "eitmad.quotation-draft.list.v1";

    public static Query ForQuotationDraftList(ListQuotationDrafts payload) =>
        new() { Kind = QuotationDraftListKind, Payload = payload };

    public ListQuotationDrafts? AsQuotationDraftList() =>
        Kind == QuotationDraftListKind ? PayloadAs<ListQuotationDrafts>() : null;

    public const string QuotationEvaluateKind = "eitmad.quotation.evaluate.v1";

    public static Query ForQuotationEvaluate(EvaluateQuotation payload) =>
        new() { Kind = QuotationEvaluateKind, Payload = payload };

    public EvaluateQuotation? AsQuotationEvaluate() =>
        Kind == QuotationEvaluateKind ? PayloadAs<EvaluateQuotation>() : null;

    public const string SalesCatalogListKind = "eitmad.sales-catalog.list.v1";

    public static Query ForSalesCatalogList(ListSalesCatalog payload) =>
        new() { Kind = SalesCatalogListKind, Payload = payload };

    public ListSalesCatalog? AsSalesCatalogList() =>
        Kind == SalesCatalogListKind ? PayloadAs<ListSalesCatalog>() : null;

    public const string SalesCatalogGetKind = "eitmad.sales-catalog.get.v1";

    public static Query ForSalesCatalogGet(GetSalesCatalogItem payload) =>
        new() { Kind = SalesCatalogGetKind, Payload = payload };

    public GetSalesCatalogItem? AsSalesCatalogGet() =>
        Kind == SalesCatalogGetKind ? PayloadAs<GetSalesCatalogItem>() : null;

    public const string SalesCatalogCheckKind = "eitmad.sales-catalog.check.v1";

    public static Query ForSalesCatalogCheck(CheckSalesConfiguration payload) =>
        new() { Kind = SalesCatalogCheckKind, Payload = payload };

    public CheckSalesConfiguration? AsSalesCatalogCheck() =>
        Kind == SalesCatalogCheckKind ? PayloadAs<CheckSalesConfiguration>() : null;

    public const string PricingListKind = "eitmad.pricing.list.v1";

    public static Query ForPricingList(ListPrices payload) =>
        new() { Kind = PricingListKind, Payload = payload };

    public ListPrices? AsPricingList() =>
        Kind == PricingListKind ? PayloadAs<ListPrices>() : null;

    public const string PricingReviewKind = "eitmad.pricing.review.v1";

    public static Query ForPricingReview(ReviewPrice payload) =>
        new() { Kind = PricingReviewKind, Payload = payload };

    public ReviewPrice? AsPricingReview() =>
        Kind == PricingReviewKind ? PayloadAs<ReviewPrice>() : null;

    public const string PricingSelectionKind = "eitmad.pricing.selection.v1";

    public static Query ForPricingSelection(PriceSelection payload) =>
        new() { Kind = PricingSelectionKind, Payload = payload };

    public PriceSelection? AsPricingSelection() =>
        Kind == PricingSelectionKind ? PayloadAs<PriceSelection>() : null;

    public const string PricingDiscountKind = "eitmad.pricing.discount.v1";

    public static Query ForPricingDiscount(CalculateDiscount payload) =>
        new() { Kind = PricingDiscountKind, Payload = payload };

    public CalculateDiscount? AsPricingDiscount() =>
        Kind == PricingDiscountKind ? PayloadAs<CalculateDiscount>() : null;

    public const string CatalogImageGetKind = "eitmad.catalog-image.get.v1";

    public static Query ForCatalogImageGet(GetCatalogImage payload) =>
        new() { Kind = CatalogImageGetKind, Payload = payload };

    public GetCatalogImage? AsCatalogImageGet() =>
        Kind == CatalogImageGetKind ? PayloadAs<GetCatalogImage>() : null;

    public const string ConfigGetKind = "eitmad.config.get.v1";

    public static Query ForConfigGet(GetConfiguration payload) =>
        new() { Kind = ConfigGetKind, Payload = payload };

    public GetConfiguration? AsConfigGet() =>
        Kind == ConfigGetKind ? PayloadAs<GetConfiguration>() : null;

    public const string PermissionsGetEffectiveKind = "eitmad.permissions.get-effective.v1";

    public static Query ForPermissionsGetEffective(GetEffectivePermissions payload) =>
        new() { Kind = PermissionsGetEffectiveKind, Payload = payload };

    public GetEffectivePermissions? AsPermissionsGetEffective() =>
        Kind == PermissionsGetEffectiveKind ? PayloadAs<GetEffectivePermissions>() : null;

    public const string AuthorizationRelationshipsListKind = "eitmad.authorization.relationships.list.v1";

    public static Query ForAuthorizationRelationshipsList(ListScopeRelationships payload) =>
        new() { Kind = AuthorizationRelationshipsListKind, Payload = payload };

    public ListScopeRelationships? AsAuthorizationRelationshipsList() =>
        Kind == AuthorizationRelationshipsListKind ? PayloadAs<ListScopeRelationships>() : null;

    public const string CustomerGetKind = "eitmad.customer.get.v1";

    public static Query ForCustomerGet(GetCustomer payload) =>
        new() { Kind = CustomerGetKind, Payload = payload };

    public GetCustomer? AsCustomerGet() =>
        Kind == CustomerGetKind ? PayloadAs<GetCustomer>() : null;

    public const string CustomerSearchKind = "eitmad.customer.search.v1";

    public static Query ForCustomerSearch(SearchCustomers payload) =>
        new() { Kind = CustomerSearchKind, Payload = payload };

    public SearchCustomers? AsCustomerSearch() =>
        Kind == CustomerSearchKind ? PayloadAs<SearchCustomers>() : null;

    public const string FurnitureListKind = "eitmad.furniture.list.v1";

    public static Query ForFurnitureList(ListFurnitures payload) =>
        new() { Kind = FurnitureListKind, Payload = payload };

    public ListFurnitures? AsFurnitureList() =>
        Kind == FurnitureListKind ? PayloadAs<ListFurnitures>() : null;

    public const string FurnitureCategoryListKind = "eitmad.furniture-category.list.v1";

    public static Query ForFurnitureCategoryList(ListFurnitureCategories payload) =>
        new() { Kind = FurnitureCategoryListKind, Payload = payload };

    public ListFurnitureCategories? AsFurnitureCategoryList() =>
        Kind == FurnitureCategoryListKind ? PayloadAs<ListFurnitureCategories>() : null;

    public const string FurnitureRevisionGetKind = "eitmad.furniture-revision.get.v1";

    public static Query ForFurnitureRevisionGet(GetFurnitureRevision payload) =>
        new() { Kind = FurnitureRevisionGetKind, Payload = payload };

    public GetFurnitureRevision? AsFurnitureRevisionGet() =>
        Kind == FurnitureRevisionGetKind ? PayloadAs<GetFurnitureRevision>() : null;

    public const string FurnitureReviewKind = "eitmad.furniture.review.v1";

    public static Query ForFurnitureReview(SaveFurniture payload) =>
        new() { Kind = FurnitureReviewKind, Payload = payload };

    public SaveFurniture? AsFurnitureReview() =>
        Kind == FurnitureReviewKind ? PayloadAs<SaveFurniture>() : null;

    public const string FurnitureSelectionCheckKind = "eitmad.furniture-selection.check.v1";

    public static Query ForFurnitureSelectionCheck(CheckFurnitureSelection payload) =>
        new() { Kind = FurnitureSelectionCheckKind, Payload = payload };

    public CheckFurnitureSelection? AsFurnitureSelectionCheck() =>
        Kind == FurnitureSelectionCheckKind ? PayloadAs<CheckFurnitureSelection>() : null;

    public const string ProductListKind = "eitmad.product.list.v1";

    public static Query ForProductList(ListProducts payload) =>
        new() { Kind = ProductListKind, Payload = payload };

    public ListProducts? AsProductList() =>
        Kind == ProductListKind ? PayloadAs<ListProducts>() : null;

    public const string ProductCategoryListKind = "eitmad.product-category.list.v1";

    public static Query ForProductCategoryList(ListProductCategories payload) =>
        new() { Kind = ProductCategoryListKind, Payload = payload };

    public ListProductCategories? AsProductCategoryList() =>
        Kind == ProductCategoryListKind ? PayloadAs<ListProductCategories>() : null;

    public const string ProductRevisionGetKind = "eitmad.product-revision.get.v1";

    public static Query ForProductRevisionGet(GetProductRevision payload) =>
        new() { Kind = ProductRevisionGetKind, Payload = payload };

    public GetProductRevision? AsProductRevisionGet() =>
        Kind == ProductRevisionGetKind ? PayloadAs<GetProductRevision>() : null;

    public const string PartListKind = "eitmad.part.list.v1";

    public static Query ForPartList(ListParts payload) =>
        new() { Kind = PartListKind, Payload = payload };

    public ListParts? AsPartList() =>
        Kind == PartListKind ? PayloadAs<ListParts>() : null;

    public const string PartCategoryListKind = "eitmad.part-category.list.v1";

    public static Query ForPartCategoryList(ListPartCategories payload) =>
        new() { Kind = PartCategoryListKind, Payload = payload };

    public ListPartCategories? AsPartCategoryList() =>
        Kind == PartCategoryListKind ? PayloadAs<ListPartCategories>() : null;

    public const string PartCostKind = "eitmad.part.cost.v1";

    public static Query ForPartCost(CalculatePartCost payload) =>
        new() { Kind = PartCostKind, Payload = payload };

    public CalculatePartCost? AsPartCost() =>
        Kind == PartCostKind ? PayloadAs<CalculatePartCost>() : null;

    public const string PartCompositionGetKind = "eitmad.part-composition.get.v1";

    public static Query ForPartCompositionGet(GetPartComposition payload) =>
        new() { Kind = PartCompositionGetKind, Payload = payload };

    public GetPartComposition? AsPartCompositionGet() =>
        Kind == PartCompositionGetKind ? PayloadAs<GetPartComposition>() : null;

    public const string MaterialListKind = "eitmad.material.list.v1";

    public static Query ForMaterialList(ListMaterials payload) =>
        new() { Kind = MaterialListKind, Payload = payload };

    public ListMaterials? AsMaterialList() =>
        Kind == MaterialListKind ? PayloadAs<ListMaterials>() : null;

    public const string MaterialReferenceListKind = "eitmad.material-reference.list.v1";

    public static Query ForMaterialReferenceList(ListMaterialReferences payload) =>
        new() { Kind = MaterialReferenceListKind, Payload = payload };

    public ListMaterialReferences? AsMaterialReferenceList() =>
        Kind == MaterialReferenceListKind ? PayloadAs<ListMaterialReferences>() : null;

    public const string DesktopAccountListKind = "eitmad.desktop-account.list.v1";

    public static Query ForDesktopAccountList(ListDesktopAccounts payload) =>
        new() { Kind = DesktopAccountListKind, Payload = payload };

    public ListDesktopAccounts? AsDesktopAccountList() =>
        Kind == DesktopAccountListKind ? PayloadAs<ListDesktopAccounts>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class QueryResult
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string DiscountApprovalsKind = "discountApprovals";

    public static QueryResult ForDiscountApprovals(DiscountApprovalPage payload) =>
        new() { Kind = DiscountApprovalsKind, Payload = payload };

    public DiscountApprovalPage? AsDiscountApprovals() =>
        Kind == DiscountApprovalsKind ? PayloadAs<DiscountApprovalPage>() : null;

    public const string QuotationDraftKind = "quotationDraft";

    public static QueryResult ForQuotationDraft(QuotationDraft payload) =>
        new() { Kind = QuotationDraftKind, Payload = payload };

    public QuotationDraft? AsQuotationDraft() =>
        Kind == QuotationDraftKind ? PayloadAs<QuotationDraft>() : null;

    public const string QuotationDraftsKind = "quotationDrafts";

    public static QueryResult ForQuotationDrafts(QuotationDraftPage payload) =>
        new() { Kind = QuotationDraftsKind, Payload = payload };

    public QuotationDraftPage? AsQuotationDrafts() =>
        Kind == QuotationDraftsKind ? PayloadAs<QuotationDraftPage>() : null;

    public const string QuotationEvaluationKind = "quotationEvaluation";

    public static QueryResult ForQuotationEvaluation(QuotationEvaluation payload) =>
        new() { Kind = QuotationEvaluationKind, Payload = payload };

    public QuotationEvaluation? AsQuotationEvaluation() =>
        Kind == QuotationEvaluationKind ? PayloadAs<QuotationEvaluation>() : null;

    public const string SalesCatalogKind = "salesCatalog";

    public static QueryResult ForSalesCatalog(SalesCatalogPage payload) =>
        new() { Kind = SalesCatalogKind, Payload = payload };

    public SalesCatalogPage? AsSalesCatalog() =>
        Kind == SalesCatalogKind ? PayloadAs<SalesCatalogPage>() : null;

    public const string SalesCatalogItemKind = "salesCatalogItem";

    public static QueryResult ForSalesCatalogItem(SalesCatalogDetails payload) =>
        new() { Kind = SalesCatalogItemKind, Payload = payload };

    public SalesCatalogDetails? AsSalesCatalogItem() =>
        Kind == SalesCatalogItemKind ? PayloadAs<SalesCatalogDetails>() : null;

    public const string SalesConfigurationKind = "salesConfiguration";

    public static QueryResult ForSalesConfiguration(SalesConfiguration payload) =>
        new() { Kind = SalesConfigurationKind, Payload = payload };

    public SalesConfiguration? AsSalesConfiguration() =>
        Kind == SalesConfigurationKind ? PayloadAs<SalesConfiguration>() : null;

    public const string PricesKind = "prices";

    public static QueryResult ForPrices(PricePage payload) =>
        new() { Kind = PricesKind, Payload = payload };

    public PricePage? AsPrices() =>
        Kind == PricesKind ? PayloadAs<PricePage>() : null;

    public const string PriceReviewKind = "priceReview";

    public static QueryResult ForPriceReview(PriceReview payload) =>
        new() { Kind = PriceReviewKind, Payload = payload };

    public PriceReview? AsPriceReview() =>
        Kind == PriceReviewKind ? PayloadAs<PriceReview>() : null;

    public const string SellingPriceKind = "sellingPrice";

    public static QueryResult ForSellingPrice(SellingPrice payload) =>
        new() { Kind = SellingPriceKind, Payload = payload };

    public SellingPrice? AsSellingPrice() =>
        Kind == SellingPriceKind ? PayloadAs<SellingPrice>() : null;

    public const string DiscountTotalKind = "discountTotal";

    public static QueryResult ForDiscountTotal(DiscountTotal payload) =>
        new() { Kind = DiscountTotalKind, Payload = payload };

    public DiscountTotal? AsDiscountTotal() =>
        Kind == DiscountTotalKind ? PayloadAs<DiscountTotal>() : null;

    public const string CatalogImageKind = "catalogImage";

    public static QueryResult ForCatalogImage(CatalogImageChunk payload) =>
        new() { Kind = CatalogImageKind, Payload = payload };

    public CatalogImageChunk? AsCatalogImage() =>
        Kind == CatalogImageKind ? PayloadAs<CatalogImageChunk>() : null;

    public const string ConfigurationKind = "configuration";

    public static QueryResult ForConfiguration(ConfigSnapshot payload) =>
        new() { Kind = ConfigurationKind, Payload = payload };

    public ConfigSnapshot? AsConfiguration() =>
        Kind == ConfigurationKind ? PayloadAs<ConfigSnapshot>() : null;

    public const string EffectivePermissionsKind = "effectivePermissions";

    public static QueryResult ForEffectivePermissions(EffectivePermissions payload) =>
        new() { Kind = EffectivePermissionsKind, Payload = payload };

    public EffectivePermissions? AsEffectivePermissions() =>
        Kind == EffectivePermissionsKind ? PayloadAs<EffectivePermissions>() : null;

    public const string ScopeRelationshipsKind = "scopeRelationships";

    public static QueryResult ForScopeRelationships(RelationshipPage payload) =>
        new() { Kind = ScopeRelationshipsKind, Payload = payload };

    public RelationshipPage? AsScopeRelationships() =>
        Kind == ScopeRelationshipsKind ? PayloadAs<RelationshipPage>() : null;

    public const string CustomerKind = "customer";

    public static QueryResult ForCustomer(Customer payload) =>
        new() { Kind = CustomerKind, Payload = payload };

    public Customer? AsCustomer() =>
        Kind == CustomerKind ? PayloadAs<Customer>() : null;

    public const string CustomersKind = "customers";

    public static QueryResult ForCustomers(CustomerPage payload) =>
        new() { Kind = CustomersKind, Payload = payload };

    public CustomerPage? AsCustomers() =>
        Kind == CustomersKind ? PayloadAs<CustomerPage>() : null;

    public const string FurnituresKind = "furnitures";

    public static QueryResult ForFurnitures(FurniturePage payload) =>
        new() { Kind = FurnituresKind, Payload = payload };

    public FurniturePage? AsFurnitures() =>
        Kind == FurnituresKind ? PayloadAs<FurniturePage>() : null;

    public const string FurnitureCategoriesKind = "furnitureCategories";

    public static QueryResult ForFurnitureCategories(FurnitureCategories payload) =>
        new() { Kind = FurnitureCategoriesKind, Payload = payload };

    public FurnitureCategories? AsFurnitureCategories() =>
        Kind == FurnitureCategoriesKind ? PayloadAs<FurnitureCategories>() : null;

    public const string FurnitureRevisionKind = "furnitureRevision";

    public static QueryResult ForFurnitureRevision(Furniture payload) =>
        new() { Kind = FurnitureRevisionKind, Payload = payload };

    public Furniture? AsFurnitureRevision() =>
        Kind == FurnitureRevisionKind ? PayloadAs<Furniture>() : null;

    public const string FurnitureReviewKind = "furnitureReview";

    public static QueryResult ForFurnitureReview(FurnitureReview payload) =>
        new() { Kind = FurnitureReviewKind, Payload = payload };

    public FurnitureReview? AsFurnitureReview() =>
        Kind == FurnitureReviewKind ? PayloadAs<FurnitureReview>() : null;

    public const string FurnitureSelectionKind = "furnitureSelection";

    public static QueryResult ForFurnitureSelection(FurnitureSelection payload) =>
        new() { Kind = FurnitureSelectionKind, Payload = payload };

    public FurnitureSelection? AsFurnitureSelection() =>
        Kind == FurnitureSelectionKind ? PayloadAs<FurnitureSelection>() : null;

    public const string ProductsKind = "products";

    public static QueryResult ForProducts(ProductPage payload) =>
        new() { Kind = ProductsKind, Payload = payload };

    public ProductPage? AsProducts() =>
        Kind == ProductsKind ? PayloadAs<ProductPage>() : null;

    public const string ProductCategoriesKind = "productCategories";

    public static QueryResult ForProductCategories(ProductCategories payload) =>
        new() { Kind = ProductCategoriesKind, Payload = payload };

    public ProductCategories? AsProductCategories() =>
        Kind == ProductCategoriesKind ? PayloadAs<ProductCategories>() : null;

    public const string ProductRevisionKind = "productRevision";

    public static QueryResult ForProductRevision(Product payload) =>
        new() { Kind = ProductRevisionKind, Payload = payload };

    public Product? AsProductRevision() =>
        Kind == ProductRevisionKind ? PayloadAs<Product>() : null;

    public const string PartsKind = "parts";

    public static QueryResult ForParts(PartPage payload) =>
        new() { Kind = PartsKind, Payload = payload };

    public PartPage? AsParts() =>
        Kind == PartsKind ? PayloadAs<PartPage>() : null;

    public const string PartCategoriesKind = "partCategories";

    public static QueryResult ForPartCategories(PartCategories payload) =>
        new() { Kind = PartCategoriesKind, Payload = payload };

    public PartCategories? AsPartCategories() =>
        Kind == PartCategoriesKind ? PayloadAs<PartCategories>() : null;

    public const string PartCostKind = "partCost";

    public static QueryResult ForPartCost(PartCost payload) =>
        new() { Kind = PartCostKind, Payload = payload };

    public PartCost? AsPartCost() =>
        Kind == PartCostKind ? PayloadAs<PartCost>() : null;

    public const string PartCompositionKind = "partComposition";

    public static QueryResult ForPartComposition(Part payload) =>
        new() { Kind = PartCompositionKind, Payload = payload };

    public Part? AsPartComposition() =>
        Kind == PartCompositionKind ? PayloadAs<Part>() : null;

    public const string MaterialsKind = "materials";

    public static QueryResult ForMaterials(MaterialPage payload) =>
        new() { Kind = MaterialsKind, Payload = payload };

    public MaterialPage? AsMaterials() =>
        Kind == MaterialsKind ? PayloadAs<MaterialPage>() : null;

    public const string MaterialReferencesKind = "materialReferences";

    public static QueryResult ForMaterialReferences(MaterialReferences payload) =>
        new() { Kind = MaterialReferencesKind, Payload = payload };

    public MaterialReferences? AsMaterialReferences() =>
        Kind == MaterialReferencesKind ? PayloadAs<MaterialReferences>() : null;

    public const string DesktopAccountsKind = "desktopAccounts";

    public static QueryResult ForDesktopAccounts(DesktopAccountPage payload) =>
        new() { Kind = DesktopAccountsKind, Payload = payload };

    public DesktopAccountPage? AsDesktopAccounts() =>
        Kind == DesktopAccountsKind ? PayloadAs<DesktopAccountPage>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class Subscription
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string PricingChangedSubscribeKind = "eitmad.pricing.changed.subscribe.v1";

    public static Subscription ForPricingChangedSubscribe(PriceChanges payload) =>
        new() { Kind = PricingChangedSubscribeKind, Payload = payload };

    public PriceChanges? AsPricingChangedSubscribe() =>
        Kind == PricingChangedSubscribeKind ? PayloadAs<PriceChanges>() : null;

    public const string ConfigChangedSubscribeKind = "eitmad.config.changed.subscribe.v1";

    public static Subscription ForConfigChangedSubscribe(ConfigurationChanges payload) =>
        new() { Kind = ConfigChangedSubscribeKind, Payload = payload };

    public ConfigurationChanges? AsConfigChangedSubscribe() =>
        Kind == ConfigChangedSubscribeKind ? PayloadAs<ConfigurationChanges>() : null;

    public const string PermissionsChangedSubscribeKind = "eitmad.permissions.changed.subscribe.v1";

    public static Subscription ForPermissionsChangedSubscribe(PermissionChanges payload) =>
        new() { Kind = PermissionsChangedSubscribeKind, Payload = payload };

    public PermissionChanges? AsPermissionsChangedSubscribe() =>
        Kind == PermissionsChangedSubscribeKind ? PayloadAs<PermissionChanges>() : null;

    public const string AuthorizationPolicyChangedSubscribeKind = "eitmad.authorization.policy.changed.subscribe.v1";

    public static Subscription ForAuthorizationPolicyChangedSubscribe(AuthorizationPolicyChanges payload) =>
        new() { Kind = AuthorizationPolicyChangedSubscribeKind, Payload = payload };

    public AuthorizationPolicyChanges? AsAuthorizationPolicyChangedSubscribe() =>
        Kind == AuthorizationPolicyChangedSubscribeKind ? PayloadAs<AuthorizationPolicyChanges>() : null;

    public const string QuotationApprovalChangedSubscribeKind = "eitmad.quotation-approval.changed.subscribe.v1";

    public static Subscription ForQuotationApprovalChangedSubscribe(DiscountApprovalChanges payload) =>
        new() { Kind = QuotationApprovalChangedSubscribeKind, Payload = payload };

    public DiscountApprovalChanges? AsQuotationApprovalChangedSubscribe() =>
        Kind == QuotationApprovalChangedSubscribeKind ? PayloadAs<DiscountApprovalChanges>() : null;

    public const string QuotationDraftChangedSubscribeKind = "eitmad.quotation-draft.changed.subscribe.v1";

    public static Subscription ForQuotationDraftChangedSubscribe(QuotationDraftChanges payload) =>
        new() { Kind = QuotationDraftChangedSubscribeKind, Payload = payload };

    public QuotationDraftChanges? AsQuotationDraftChangedSubscribe() =>
        Kind == QuotationDraftChangedSubscribeKind ? PayloadAs<QuotationDraftChanges>() : null;

    public const string CustomerChangedSubscribeKind = "eitmad.customer.changed.subscribe.v1";

    public static Subscription ForCustomerChangedSubscribe(CustomerChanges payload) =>
        new() { Kind = CustomerChangedSubscribeKind, Payload = payload };

    public CustomerChanges? AsCustomerChangedSubscribe() =>
        Kind == CustomerChangedSubscribeKind ? PayloadAs<CustomerChanges>() : null;

    public const string MaterialChangedSubscribeKind = "eitmad.material.changed.subscribe.v1";

    public static Subscription ForMaterialChangedSubscribe(MaterialChanges payload) =>
        new() { Kind = MaterialChangedSubscribeKind, Payload = payload };

    public MaterialChanges? AsMaterialChangedSubscribe() =>
        Kind == MaterialChangedSubscribeKind ? PayloadAs<MaterialChanges>() : null;

    public const string FurnitureChangedSubscribeKind = "eitmad.furniture.changed.subscribe.v1";

    public static Subscription ForFurnitureChangedSubscribe(FurnitureChanges payload) =>
        new() { Kind = FurnitureChangedSubscribeKind, Payload = payload };

    public FurnitureChanges? AsFurnitureChangedSubscribe() =>
        Kind == FurnitureChangedSubscribeKind ? PayloadAs<FurnitureChanges>() : null;

    public const string ProductChangedSubscribeKind = "eitmad.product.changed.subscribe.v1";

    public static Subscription ForProductChangedSubscribe(ProductChanges payload) =>
        new() { Kind = ProductChangedSubscribeKind, Payload = payload };

    public ProductChanges? AsProductChangedSubscribe() =>
        Kind == ProductChangedSubscribeKind ? PayloadAs<ProductChanges>() : null;

    public const string PartChangedSubscribeKind = "eitmad.part.changed.subscribe.v1";

    public static Subscription ForPartChangedSubscribe(PartChanges payload) =>
        new() { Kind = PartChangedSubscribeKind, Payload = payload };

    public PartChanges? AsPartChangedSubscribe() =>
        Kind == PartChangedSubscribeKind ? PayloadAs<PartChanges>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class AuthorizationPolicyChanges
{
}

public partial class ConfigurationChanges
{
}

public partial class CustomerChanges
{
}

public partial class DiscountApprovalChanges
{
}

public partial class FurnitureChanges
{
}

public partial class GetConfiguration
{
}

public partial class GetEffectivePermissions
{
}

public partial class ListDesktopAccounts
{
}

public partial class ListMaterialReferences
{
}

public partial class MaterialChanges
{
}

public partial class PartChanges
{
}

public partial class PermissionChanges
{
}

public partial class PriceChanges
{
}

public partial class ProductChanges
{
}

public partial class QuotationDraftChanges
{
}
