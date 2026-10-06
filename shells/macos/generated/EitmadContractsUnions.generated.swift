// Generated from Rust contracts. Do not edit.
import Foundation

public enum Command: Codable, Sendable {
    case pricingPublish(PublishPrice)
    case catalogImageImport(ImportCatalogImage)
    case configUpdate(UpdateConfiguration)
    case authorizationRelationshipGrant(GrantScopeRelationship)
    case authorizationRelationshipRevoke(RevokeScopeRelationship)
    case customerCreate(CreateCustomer)
    case customerUpdate(UpdateCustomer)
    case materialCategorySave(SaveMaterialCategory)
    case materialUnitSave(SaveMaterialUnit)
    case materialSave(SaveMaterial)
    case furnitureSave(SaveFurniture)
    case furnitureCategorySave(SaveFurnitureCategory)
    case productSave(SaveProduct)
    case productCategorySave(SaveProductCategory)
    case partSave(SavePart)
    case partCategorySave(SavePartCategory)
    case desktopAccountCreate(CreateDesktopAccount)
    case desktopAccountUpdate(UpdateDesktopAccount)
    case desktopAccountDeactivate(DeactivateDesktopAccount)

    private enum Kind: String, Codable, Sendable {
        case pricingPublish = "eitmad.pricing.publish.v1"
        case catalogImageImport = "eitmad.catalog-image.import.v1"
        case configUpdate = "eitmad.config.update.v1"
        case authorizationRelationshipGrant = "eitmad.authorization.relationship.grant.v1"
        case authorizationRelationshipRevoke = "eitmad.authorization.relationship.revoke.v1"
        case customerCreate = "eitmad.customer.create.v1"
        case customerUpdate = "eitmad.customer.update.v1"
        case materialCategorySave = "eitmad.material-category.save.v1"
        case materialUnitSave = "eitmad.material-unit.save.v1"
        case materialSave = "eitmad.material.save.v1"
        case furnitureSave = "eitmad.furniture.save.v1"
        case furnitureCategorySave = "eitmad.furniture-category.save.v1"
        case productSave = "eitmad.product.save.v1"
        case productCategorySave = "eitmad.product-category.save.v1"
        case partSave = "eitmad.part.save.v1"
        case partCategorySave = "eitmad.part-category.save.v1"
        case desktopAccountCreate = "eitmad.desktop-account.create.v1"
        case desktopAccountUpdate = "eitmad.desktop-account.update.v1"
        case desktopAccountDeactivate = "eitmad.desktop-account.deactivate.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .pricingPublish: self = .pricingPublish(try container.decode(PublishPrice.self, forKey: .payload))
        case .catalogImageImport: self = .catalogImageImport(try container.decode(ImportCatalogImage.self, forKey: .payload))
        case .configUpdate: self = .configUpdate(try container.decode(UpdateConfiguration.self, forKey: .payload))
        case .authorizationRelationshipGrant: self = .authorizationRelationshipGrant(try container.decode(GrantScopeRelationship.self, forKey: .payload))
        case .authorizationRelationshipRevoke: self = .authorizationRelationshipRevoke(try container.decode(RevokeScopeRelationship.self, forKey: .payload))
        case .customerCreate: self = .customerCreate(try container.decode(CreateCustomer.self, forKey: .payload))
        case .customerUpdate: self = .customerUpdate(try container.decode(UpdateCustomer.self, forKey: .payload))
        case .materialCategorySave: self = .materialCategorySave(try container.decode(SaveMaterialCategory.self, forKey: .payload))
        case .materialUnitSave: self = .materialUnitSave(try container.decode(SaveMaterialUnit.self, forKey: .payload))
        case .materialSave: self = .materialSave(try container.decode(SaveMaterial.self, forKey: .payload))
        case .furnitureSave: self = .furnitureSave(try container.decode(SaveFurniture.self, forKey: .payload))
        case .furnitureCategorySave: self = .furnitureCategorySave(try container.decode(SaveFurnitureCategory.self, forKey: .payload))
        case .productSave: self = .productSave(try container.decode(SaveProduct.self, forKey: .payload))
        case .productCategorySave: self = .productCategorySave(try container.decode(SaveProductCategory.self, forKey: .payload))
        case .partSave: self = .partSave(try container.decode(SavePart.self, forKey: .payload))
        case .partCategorySave: self = .partCategorySave(try container.decode(SavePartCategory.self, forKey: .payload))
        case .desktopAccountCreate: self = .desktopAccountCreate(try container.decode(CreateDesktopAccount.self, forKey: .payload))
        case .desktopAccountUpdate: self = .desktopAccountUpdate(try container.decode(UpdateDesktopAccount.self, forKey: .payload))
        case .desktopAccountDeactivate: self = .desktopAccountDeactivate(try container.decode(DeactivateDesktopAccount.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .pricingPublish(let payload):
            try container.encode(Kind.pricingPublish, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .catalogImageImport(let payload):
            try container.encode(Kind.catalogImageImport, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .configUpdate(let payload):
            try container.encode(Kind.configUpdate, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationRelationshipGrant(let payload):
            try container.encode(Kind.authorizationRelationshipGrant, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationRelationshipRevoke(let payload):
            try container.encode(Kind.authorizationRelationshipRevoke, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customerCreate(let payload):
            try container.encode(Kind.customerCreate, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customerUpdate(let payload):
            try container.encode(Kind.customerUpdate, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialCategorySave(let payload):
            try container.encode(Kind.materialCategorySave, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialUnitSave(let payload):
            try container.encode(Kind.materialUnitSave, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialSave(let payload):
            try container.encode(Kind.materialSave, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureSave(let payload):
            try container.encode(Kind.furnitureSave, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureCategorySave(let payload):
            try container.encode(Kind.furnitureCategorySave, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productSave(let payload):
            try container.encode(Kind.productSave, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productCategorySave(let payload):
            try container.encode(Kind.productCategorySave, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partSave(let payload):
            try container.encode(Kind.partSave, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partCategorySave(let payload):
            try container.encode(Kind.partCategorySave, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .desktopAccountCreate(let payload):
            try container.encode(Kind.desktopAccountCreate, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .desktopAccountUpdate(let payload):
            try container.encode(Kind.desktopAccountUpdate, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .desktopAccountDeactivate(let payload):
            try container.encode(Kind.desktopAccountDeactivate, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum CommandResult: Codable, Sendable {
    case pricePublished(PublishedPrice)
    case catalogImageImported(CatalogImageRef)
    case configurationUpdated(ConfigSnapshot)
    case relationshipGranted(RelationshipMutationResult)
    case relationshipRevoked(RelationshipMutationResult)
    case customerCreated(CustomerMutationResult)
    case customerUpdated(CustomerMutationResult)
    case materialCategorySaved(MaterialCategory)
    case materialUnitSaved(MaterialUnit)
    case materialSaved(Material)
    case furnitureSaved(Furniture)
    case furnitureCategorySaved(FurnitureCategory)
    case productSaved(Product)
    case productCategorySaved(ProductCategory)
    case partSaved(Part)
    case partCategorySaved(PartCategory)
    case desktopAccountCreated(DesktopAccountSummary)
    case desktopAccountUpdated(DesktopAccountSummary)
    case desktopAccountDeactivated(DesktopAccountSummary)

    private enum Kind: String, Codable, Sendable {
        case pricePublished = "pricePublished"
        case catalogImageImported = "catalogImageImported"
        case configurationUpdated = "configurationUpdated"
        case relationshipGranted = "relationshipGranted"
        case relationshipRevoked = "relationshipRevoked"
        case customerCreated = "customerCreated"
        case customerUpdated = "customerUpdated"
        case materialCategorySaved = "materialCategorySaved"
        case materialUnitSaved = "materialUnitSaved"
        case materialSaved = "materialSaved"
        case furnitureSaved = "furnitureSaved"
        case furnitureCategorySaved = "furnitureCategorySaved"
        case productSaved = "productSaved"
        case productCategorySaved = "productCategorySaved"
        case partSaved = "partSaved"
        case partCategorySaved = "partCategorySaved"
        case desktopAccountCreated = "desktopAccountCreated"
        case desktopAccountUpdated = "desktopAccountUpdated"
        case desktopAccountDeactivated = "desktopAccountDeactivated"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .pricePublished: self = .pricePublished(try container.decode(PublishedPrice.self, forKey: .payload))
        case .catalogImageImported: self = .catalogImageImported(try container.decode(CatalogImageRef.self, forKey: .payload))
        case .configurationUpdated: self = .configurationUpdated(try container.decode(ConfigSnapshot.self, forKey: .payload))
        case .relationshipGranted: self = .relationshipGranted(try container.decode(RelationshipMutationResult.self, forKey: .payload))
        case .relationshipRevoked: self = .relationshipRevoked(try container.decode(RelationshipMutationResult.self, forKey: .payload))
        case .customerCreated: self = .customerCreated(try container.decode(CustomerMutationResult.self, forKey: .payload))
        case .customerUpdated: self = .customerUpdated(try container.decode(CustomerMutationResult.self, forKey: .payload))
        case .materialCategorySaved: self = .materialCategorySaved(try container.decode(MaterialCategory.self, forKey: .payload))
        case .materialUnitSaved: self = .materialUnitSaved(try container.decode(MaterialUnit.self, forKey: .payload))
        case .materialSaved: self = .materialSaved(try container.decode(Material.self, forKey: .payload))
        case .furnitureSaved: self = .furnitureSaved(try container.decode(Furniture.self, forKey: .payload))
        case .furnitureCategorySaved: self = .furnitureCategorySaved(try container.decode(FurnitureCategory.self, forKey: .payload))
        case .productSaved: self = .productSaved(try container.decode(Product.self, forKey: .payload))
        case .productCategorySaved: self = .productCategorySaved(try container.decode(ProductCategory.self, forKey: .payload))
        case .partSaved: self = .partSaved(try container.decode(Part.self, forKey: .payload))
        case .partCategorySaved: self = .partCategorySaved(try container.decode(PartCategory.self, forKey: .payload))
        case .desktopAccountCreated: self = .desktopAccountCreated(try container.decode(DesktopAccountSummary.self, forKey: .payload))
        case .desktopAccountUpdated: self = .desktopAccountUpdated(try container.decode(DesktopAccountSummary.self, forKey: .payload))
        case .desktopAccountDeactivated: self = .desktopAccountDeactivated(try container.decode(DesktopAccountSummary.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .pricePublished(let payload):
            try container.encode(Kind.pricePublished, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .catalogImageImported(let payload):
            try container.encode(Kind.catalogImageImported, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .configurationUpdated(let payload):
            try container.encode(Kind.configurationUpdated, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .relationshipGranted(let payload):
            try container.encode(Kind.relationshipGranted, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .relationshipRevoked(let payload):
            try container.encode(Kind.relationshipRevoked, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customerCreated(let payload):
            try container.encode(Kind.customerCreated, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customerUpdated(let payload):
            try container.encode(Kind.customerUpdated, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialCategorySaved(let payload):
            try container.encode(Kind.materialCategorySaved, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialUnitSaved(let payload):
            try container.encode(Kind.materialUnitSaved, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialSaved(let payload):
            try container.encode(Kind.materialSaved, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureSaved(let payload):
            try container.encode(Kind.furnitureSaved, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureCategorySaved(let payload):
            try container.encode(Kind.furnitureCategorySaved, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productSaved(let payload):
            try container.encode(Kind.productSaved, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productCategorySaved(let payload):
            try container.encode(Kind.productCategorySaved, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partSaved(let payload):
            try container.encode(Kind.partSaved, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partCategorySaved(let payload):
            try container.encode(Kind.partCategorySaved, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .desktopAccountCreated(let payload):
            try container.encode(Kind.desktopAccountCreated, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .desktopAccountUpdated(let payload):
            try container.encode(Kind.desktopAccountUpdated, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .desktopAccountDeactivated(let payload):
            try container.encode(Kind.desktopAccountDeactivated, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum Event: Codable, Sendable {
    case pricingChangedEvent(PriceChangeNotice)
    case configChangedEvent(ConfigSnapshot)
    case permissionsChangedEvent(EffectivePermissions)
    case authorizationPolicyChangedEvent(AuthorizationPolicyChangeNotice)
    case customerChangedEvent(CustomerChangeNotice)
    case materialChangedEvent(MaterialChangeNotice)
    case furnitureChangedEvent(FurnitureChangeNotice)
    case productChangedEvent(ProductChangeNotice)
    case partChangedEvent(PartChangeNotice)

    private enum Kind: String, Codable, Sendable {
        case pricingChangedEvent = "eitmad.pricing.changed.event.v1"
        case configChangedEvent = "eitmad.config.changed.event.v1"
        case permissionsChangedEvent = "eitmad.permissions.changed.event.v1"
        case authorizationPolicyChangedEvent = "eitmad.authorization.policy.changed.event.v1"
        case customerChangedEvent = "eitmad.customer.changed.event.v1"
        case materialChangedEvent = "eitmad.material.changed.event.v1"
        case furnitureChangedEvent = "eitmad.furniture.changed.event.v1"
        case productChangedEvent = "eitmad.product.changed.event.v1"
        case partChangedEvent = "eitmad.part.changed.event.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .pricingChangedEvent: self = .pricingChangedEvent(try container.decode(PriceChangeNotice.self, forKey: .payload))
        case .configChangedEvent: self = .configChangedEvent(try container.decode(ConfigSnapshot.self, forKey: .payload))
        case .permissionsChangedEvent: self = .permissionsChangedEvent(try container.decode(EffectivePermissions.self, forKey: .payload))
        case .authorizationPolicyChangedEvent: self = .authorizationPolicyChangedEvent(try container.decode(AuthorizationPolicyChangeNotice.self, forKey: .payload))
        case .customerChangedEvent: self = .customerChangedEvent(try container.decode(CustomerChangeNotice.self, forKey: .payload))
        case .materialChangedEvent: self = .materialChangedEvent(try container.decode(MaterialChangeNotice.self, forKey: .payload))
        case .furnitureChangedEvent: self = .furnitureChangedEvent(try container.decode(FurnitureChangeNotice.self, forKey: .payload))
        case .productChangedEvent: self = .productChangedEvent(try container.decode(ProductChangeNotice.self, forKey: .payload))
        case .partChangedEvent: self = .partChangedEvent(try container.decode(PartChangeNotice.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .pricingChangedEvent(let payload):
            try container.encode(Kind.pricingChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .configChangedEvent(let payload):
            try container.encode(Kind.configChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .permissionsChangedEvent(let payload):
            try container.encode(Kind.permissionsChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationPolicyChangedEvent(let payload):
            try container.encode(Kind.authorizationPolicyChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customerChangedEvent(let payload):
            try container.encode(Kind.customerChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialChangedEvent(let payload):
            try container.encode(Kind.materialChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureChangedEvent(let payload):
            try container.encode(Kind.furnitureChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productChangedEvent(let payload):
            try container.encode(Kind.productChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partChangedEvent(let payload):
            try container.encode(Kind.partChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum IpcClientMessage: Codable, Sendable {
    case ipcHandshake(HandshakeRequest)
    case ipcDesktopSignIn(DesktopSignInRequest)
    case ipcDesktopSessionState(DesktopSessionRequest)
    case ipcDesktopSignOut(DesktopSessionRequest)
    case ipcCommand(CommandEnvelope)
    case ipcQuery(QueryEnvelope)
    case ipcSubscribe(SubscriptionEnvelope)
    case ipcUnsubscribe(UnsubscribeRequest)
    case ipcShutdown(ShutdownRequest)

    private enum Kind: String, Codable, Sendable {
        case ipcHandshake = "eitmad.ipc.handshake.v1"
        case ipcDesktopSignIn = "eitmad.ipc.desktop-sign-in.v1"
        case ipcDesktopSessionState = "eitmad.ipc.desktop-session-state.v1"
        case ipcDesktopSignOut = "eitmad.ipc.desktop-sign-out.v1"
        case ipcCommand = "eitmad.ipc.command.v1"
        case ipcQuery = "eitmad.ipc.query.v1"
        case ipcSubscribe = "eitmad.ipc.subscribe.v1"
        case ipcUnsubscribe = "eitmad.ipc.unsubscribe.v1"
        case ipcShutdown = "eitmad.ipc.shutdown.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .ipcHandshake: self = .ipcHandshake(try container.decode(HandshakeRequest.self, forKey: .payload))
        case .ipcDesktopSignIn: self = .ipcDesktopSignIn(try container.decode(DesktopSignInRequest.self, forKey: .payload))
        case .ipcDesktopSessionState: self = .ipcDesktopSessionState(try container.decode(DesktopSessionRequest.self, forKey: .payload))
        case .ipcDesktopSignOut: self = .ipcDesktopSignOut(try container.decode(DesktopSessionRequest.self, forKey: .payload))
        case .ipcCommand: self = .ipcCommand(try container.decode(CommandEnvelope.self, forKey: .payload))
        case .ipcQuery: self = .ipcQuery(try container.decode(QueryEnvelope.self, forKey: .payload))
        case .ipcSubscribe: self = .ipcSubscribe(try container.decode(SubscriptionEnvelope.self, forKey: .payload))
        case .ipcUnsubscribe: self = .ipcUnsubscribe(try container.decode(UnsubscribeRequest.self, forKey: .payload))
        case .ipcShutdown: self = .ipcShutdown(try container.decode(ShutdownRequest.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .ipcHandshake(let payload):
            try container.encode(Kind.ipcHandshake, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcDesktopSignIn(let payload):
            try container.encode(Kind.ipcDesktopSignIn, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcDesktopSessionState(let payload):
            try container.encode(Kind.ipcDesktopSessionState, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcDesktopSignOut(let payload):
            try container.encode(Kind.ipcDesktopSignOut, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcCommand(let payload):
            try container.encode(Kind.ipcCommand, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcQuery(let payload):
            try container.encode(Kind.ipcQuery, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcSubscribe(let payload):
            try container.encode(Kind.ipcSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcUnsubscribe(let payload):
            try container.encode(Kind.ipcUnsubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcShutdown(let payload):
            try container.encode(Kind.ipcShutdown, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum IpcServerMessage: Codable, Sendable {
    case ipcHandshakeResponse(HandshakeResponse)
    case ipcDesktopSessionResponse(DesktopSessionResponse)
    case ipcCommandResponse(CommandResponseEnvelope)
    case ipcQueryResponse(QueryResponseEnvelope)
    case ipcSubscribeResponse(SubscriptionResponseEnvelope)
    case ipcUnsubscribeResponse(UnsubscribeResponse)
    case ipcEvent(EventEnvelope)
    case ipcSubscriptionClosed(SubscriptionClosedEnvelope)
    case ipcShutdownResponse(ShutdownResponse)
    case ipcFailure(IPCFailureResponse)

    private enum Kind: String, Codable, Sendable {
        case ipcHandshakeResponse = "eitmad.ipc.handshake-response.v1"
        case ipcDesktopSessionResponse = "eitmad.ipc.desktop-session-response.v1"
        case ipcCommandResponse = "eitmad.ipc.command-response.v1"
        case ipcQueryResponse = "eitmad.ipc.query-response.v1"
        case ipcSubscribeResponse = "eitmad.ipc.subscribe-response.v1"
        case ipcUnsubscribeResponse = "eitmad.ipc.unsubscribe-response.v1"
        case ipcEvent = "eitmad.ipc.event.v1"
        case ipcSubscriptionClosed = "eitmad.ipc.subscription-closed.v1"
        case ipcShutdownResponse = "eitmad.ipc.shutdown-response.v1"
        case ipcFailure = "eitmad.ipc.failure.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .ipcHandshakeResponse: self = .ipcHandshakeResponse(try container.decode(HandshakeResponse.self, forKey: .payload))
        case .ipcDesktopSessionResponse: self = .ipcDesktopSessionResponse(try container.decode(DesktopSessionResponse.self, forKey: .payload))
        case .ipcCommandResponse: self = .ipcCommandResponse(try container.decode(CommandResponseEnvelope.self, forKey: .payload))
        case .ipcQueryResponse: self = .ipcQueryResponse(try container.decode(QueryResponseEnvelope.self, forKey: .payload))
        case .ipcSubscribeResponse: self = .ipcSubscribeResponse(try container.decode(SubscriptionResponseEnvelope.self, forKey: .payload))
        case .ipcUnsubscribeResponse: self = .ipcUnsubscribeResponse(try container.decode(UnsubscribeResponse.self, forKey: .payload))
        case .ipcEvent: self = .ipcEvent(try container.decode(EventEnvelope.self, forKey: .payload))
        case .ipcSubscriptionClosed: self = .ipcSubscriptionClosed(try container.decode(SubscriptionClosedEnvelope.self, forKey: .payload))
        case .ipcShutdownResponse: self = .ipcShutdownResponse(try container.decode(ShutdownResponse.self, forKey: .payload))
        case .ipcFailure: self = .ipcFailure(try container.decode(IPCFailureResponse.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .ipcHandshakeResponse(let payload):
            try container.encode(Kind.ipcHandshakeResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcDesktopSessionResponse(let payload):
            try container.encode(Kind.ipcDesktopSessionResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcCommandResponse(let payload):
            try container.encode(Kind.ipcCommandResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcQueryResponse(let payload):
            try container.encode(Kind.ipcQueryResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcSubscribeResponse(let payload):
            try container.encode(Kind.ipcSubscribeResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcUnsubscribeResponse(let payload):
            try container.encode(Kind.ipcUnsubscribeResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcEvent(let payload):
            try container.encode(Kind.ipcEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcSubscriptionClosed(let payload):
            try container.encode(Kind.ipcSubscriptionClosed, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcShutdownResponse(let payload):
            try container.encode(Kind.ipcShutdownResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcFailure(let payload):
            try container.encode(Kind.ipcFailure, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum PriceTarget: Codable, Sendable {
    case product(ProductReference)
    case furniture(FurnitureReference)

    private enum Kind: String, Codable, Sendable {
        case product = "product"
        case furniture = "furniture"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .product: self = .product(try container.decode(ProductReference.self, forKey: .payload))
        case .furniture: self = .furniture(try container.decode(FurnitureReference.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .product(let payload):
            try container.encode(Kind.product, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furniture(let payload):
            try container.encode(Kind.furniture, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum Query: Codable, Sendable {
    case quotationEvaluate(EvaluateQuotation)
    case salesCatalogList(ListSalesCatalog)
    case salesCatalogGet(GetSalesCatalogItem)
    case salesCatalogCheck(CheckSalesConfiguration)
    case pricingList(ListPrices)
    case pricingReview(ReviewPrice)
    case pricingSelection(PriceSelection)
    case pricingDiscount(CalculateDiscount)
    case catalogImageGet(GetCatalogImage)
    case configGet(GetConfiguration)
    case permissionsGetEffective(GetEffectivePermissions)
    case authorizationRelationshipsList(ListScopeRelationships)
    case customerGet(GetCustomer)
    case customerSearch(SearchCustomers)
    case furnitureList(ListFurnitures)
    case furnitureCategoryList(ListFurnitureCategories)
    case furnitureRevisionGet(GetFurnitureRevision)
    case furnitureReview(SaveFurniture)
    case furnitureSelectionCheck(CheckFurnitureSelection)
    case productList(ListProducts)
    case productCategoryList(ListProductCategories)
    case productRevisionGet(GetProductRevision)
    case partList(ListParts)
    case partCategoryList(ListPartCategories)
    case partCost(CalculatePartCost)
    case partCompositionGet(GetPartComposition)
    case materialList(ListMaterials)
    case materialReferenceList(ListMaterialReferences)
    case desktopAccountList(ListDesktopAccounts)

    private enum Kind: String, Codable, Sendable {
        case quotationEvaluate = "eitmad.quotation.evaluate.v1"
        case salesCatalogList = "eitmad.sales-catalog.list.v1"
        case salesCatalogGet = "eitmad.sales-catalog.get.v1"
        case salesCatalogCheck = "eitmad.sales-catalog.check.v1"
        case pricingList = "eitmad.pricing.list.v1"
        case pricingReview = "eitmad.pricing.review.v1"
        case pricingSelection = "eitmad.pricing.selection.v1"
        case pricingDiscount = "eitmad.pricing.discount.v1"
        case catalogImageGet = "eitmad.catalog-image.get.v1"
        case configGet = "eitmad.config.get.v1"
        case permissionsGetEffective = "eitmad.permissions.get-effective.v1"
        case authorizationRelationshipsList = "eitmad.authorization.relationships.list.v1"
        case customerGet = "eitmad.customer.get.v1"
        case customerSearch = "eitmad.customer.search.v1"
        case furnitureList = "eitmad.furniture.list.v1"
        case furnitureCategoryList = "eitmad.furniture-category.list.v1"
        case furnitureRevisionGet = "eitmad.furniture-revision.get.v1"
        case furnitureReview = "eitmad.furniture.review.v1"
        case furnitureSelectionCheck = "eitmad.furniture-selection.check.v1"
        case productList = "eitmad.product.list.v1"
        case productCategoryList = "eitmad.product-category.list.v1"
        case productRevisionGet = "eitmad.product-revision.get.v1"
        case partList = "eitmad.part.list.v1"
        case partCategoryList = "eitmad.part-category.list.v1"
        case partCost = "eitmad.part.cost.v1"
        case partCompositionGet = "eitmad.part-composition.get.v1"
        case materialList = "eitmad.material.list.v1"
        case materialReferenceList = "eitmad.material-reference.list.v1"
        case desktopAccountList = "eitmad.desktop-account.list.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .quotationEvaluate: self = .quotationEvaluate(try container.decode(EvaluateQuotation.self, forKey: .payload))
        case .salesCatalogList: self = .salesCatalogList(try container.decode(ListSalesCatalog.self, forKey: .payload))
        case .salesCatalogGet: self = .salesCatalogGet(try container.decode(GetSalesCatalogItem.self, forKey: .payload))
        case .salesCatalogCheck: self = .salesCatalogCheck(try container.decode(CheckSalesConfiguration.self, forKey: .payload))
        case .pricingList: self = .pricingList(try container.decode(ListPrices.self, forKey: .payload))
        case .pricingReview: self = .pricingReview(try container.decode(ReviewPrice.self, forKey: .payload))
        case .pricingSelection: self = .pricingSelection(try container.decode(PriceSelection.self, forKey: .payload))
        case .pricingDiscount: self = .pricingDiscount(try container.decode(CalculateDiscount.self, forKey: .payload))
        case .catalogImageGet: self = .catalogImageGet(try container.decode(GetCatalogImage.self, forKey: .payload))
        case .configGet: self = .configGet(try container.decode(GetConfiguration.self, forKey: .payload))
        case .permissionsGetEffective: self = .permissionsGetEffective(try container.decode(GetEffectivePermissions.self, forKey: .payload))
        case .authorizationRelationshipsList: self = .authorizationRelationshipsList(try container.decode(ListScopeRelationships.self, forKey: .payload))
        case .customerGet: self = .customerGet(try container.decode(GetCustomer.self, forKey: .payload))
        case .customerSearch: self = .customerSearch(try container.decode(SearchCustomers.self, forKey: .payload))
        case .furnitureList: self = .furnitureList(try container.decode(ListFurnitures.self, forKey: .payload))
        case .furnitureCategoryList: self = .furnitureCategoryList(try container.decode(ListFurnitureCategories.self, forKey: .payload))
        case .furnitureRevisionGet: self = .furnitureRevisionGet(try container.decode(GetFurnitureRevision.self, forKey: .payload))
        case .furnitureReview: self = .furnitureReview(try container.decode(SaveFurniture.self, forKey: .payload))
        case .furnitureSelectionCheck: self = .furnitureSelectionCheck(try container.decode(CheckFurnitureSelection.self, forKey: .payload))
        case .productList: self = .productList(try container.decode(ListProducts.self, forKey: .payload))
        case .productCategoryList: self = .productCategoryList(try container.decode(ListProductCategories.self, forKey: .payload))
        case .productRevisionGet: self = .productRevisionGet(try container.decode(GetProductRevision.self, forKey: .payload))
        case .partList: self = .partList(try container.decode(ListParts.self, forKey: .payload))
        case .partCategoryList: self = .partCategoryList(try container.decode(ListPartCategories.self, forKey: .payload))
        case .partCost: self = .partCost(try container.decode(CalculatePartCost.self, forKey: .payload))
        case .partCompositionGet: self = .partCompositionGet(try container.decode(GetPartComposition.self, forKey: .payload))
        case .materialList: self = .materialList(try container.decode(ListMaterials.self, forKey: .payload))
        case .materialReferenceList: self = .materialReferenceList(try container.decode(ListMaterialReferences.self, forKey: .payload))
        case .desktopAccountList: self = .desktopAccountList(try container.decode(ListDesktopAccounts.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .quotationEvaluate(let payload):
            try container.encode(Kind.quotationEvaluate, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .salesCatalogList(let payload):
            try container.encode(Kind.salesCatalogList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .salesCatalogGet(let payload):
            try container.encode(Kind.salesCatalogGet, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .salesCatalogCheck(let payload):
            try container.encode(Kind.salesCatalogCheck, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .pricingList(let payload):
            try container.encode(Kind.pricingList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .pricingReview(let payload):
            try container.encode(Kind.pricingReview, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .pricingSelection(let payload):
            try container.encode(Kind.pricingSelection, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .pricingDiscount(let payload):
            try container.encode(Kind.pricingDiscount, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .catalogImageGet(let payload):
            try container.encode(Kind.catalogImageGet, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .configGet(let payload):
            try container.encode(Kind.configGet, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .permissionsGetEffective(let payload):
            try container.encode(Kind.permissionsGetEffective, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationRelationshipsList(let payload):
            try container.encode(Kind.authorizationRelationshipsList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customerGet(let payload):
            try container.encode(Kind.customerGet, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customerSearch(let payload):
            try container.encode(Kind.customerSearch, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureList(let payload):
            try container.encode(Kind.furnitureList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureCategoryList(let payload):
            try container.encode(Kind.furnitureCategoryList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureRevisionGet(let payload):
            try container.encode(Kind.furnitureRevisionGet, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureReview(let payload):
            try container.encode(Kind.furnitureReview, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureSelectionCheck(let payload):
            try container.encode(Kind.furnitureSelectionCheck, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productList(let payload):
            try container.encode(Kind.productList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productCategoryList(let payload):
            try container.encode(Kind.productCategoryList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productRevisionGet(let payload):
            try container.encode(Kind.productRevisionGet, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partList(let payload):
            try container.encode(Kind.partList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partCategoryList(let payload):
            try container.encode(Kind.partCategoryList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partCost(let payload):
            try container.encode(Kind.partCost, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partCompositionGet(let payload):
            try container.encode(Kind.partCompositionGet, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialList(let payload):
            try container.encode(Kind.materialList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialReferenceList(let payload):
            try container.encode(Kind.materialReferenceList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .desktopAccountList(let payload):
            try container.encode(Kind.desktopAccountList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum QueryResult: Codable, Sendable {
    case quotationEvaluation(QuotationEvaluation)
    case salesCatalog(SalesCatalogPage)
    case salesCatalogItem(SalesCatalogDetails)
    case salesConfiguration(SalesConfiguration)
    case prices(PricePage)
    case priceReview(PriceReview)
    case sellingPrice(SellingPrice)
    case discountTotal(DiscountTotal)
    case catalogImage(CatalogImageChunk)
    case configuration(ConfigSnapshot)
    case effectivePermissions(EffectivePermissions)
    case scopeRelationships(RelationshipPage)
    case customer(Customer)
    case customers(CustomerPage)
    case furnitures(FurniturePage)
    case furnitureCategories(FurnitureCategories)
    case furnitureRevision(Furniture)
    case furnitureReview(FurnitureReview)
    case furnitureSelection(FurnitureSelection)
    case products(ProductPage)
    case productCategories(ProductCategories)
    case productRevision(Product)
    case parts(PartPage)
    case partCategories(PartCategories)
    case partCost(PartCost)
    case partComposition(Part)
    case materials(MaterialPage)
    case materialReferences(MaterialReferences)
    case desktopAccounts(DesktopAccountPage)

    private enum Kind: String, Codable, Sendable {
        case quotationEvaluation = "quotationEvaluation"
        case salesCatalog = "salesCatalog"
        case salesCatalogItem = "salesCatalogItem"
        case salesConfiguration = "salesConfiguration"
        case prices = "prices"
        case priceReview = "priceReview"
        case sellingPrice = "sellingPrice"
        case discountTotal = "discountTotal"
        case catalogImage = "catalogImage"
        case configuration = "configuration"
        case effectivePermissions = "effectivePermissions"
        case scopeRelationships = "scopeRelationships"
        case customer = "customer"
        case customers = "customers"
        case furnitures = "furnitures"
        case furnitureCategories = "furnitureCategories"
        case furnitureRevision = "furnitureRevision"
        case furnitureReview = "furnitureReview"
        case furnitureSelection = "furnitureSelection"
        case products = "products"
        case productCategories = "productCategories"
        case productRevision = "productRevision"
        case parts = "parts"
        case partCategories = "partCategories"
        case partCost = "partCost"
        case partComposition = "partComposition"
        case materials = "materials"
        case materialReferences = "materialReferences"
        case desktopAccounts = "desktopAccounts"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .quotationEvaluation: self = .quotationEvaluation(try container.decode(QuotationEvaluation.self, forKey: .payload))
        case .salesCatalog: self = .salesCatalog(try container.decode(SalesCatalogPage.self, forKey: .payload))
        case .salesCatalogItem: self = .salesCatalogItem(try container.decode(SalesCatalogDetails.self, forKey: .payload))
        case .salesConfiguration: self = .salesConfiguration(try container.decode(SalesConfiguration.self, forKey: .payload))
        case .prices: self = .prices(try container.decode(PricePage.self, forKey: .payload))
        case .priceReview: self = .priceReview(try container.decode(PriceReview.self, forKey: .payload))
        case .sellingPrice: self = .sellingPrice(try container.decode(SellingPrice.self, forKey: .payload))
        case .discountTotal: self = .discountTotal(try container.decode(DiscountTotal.self, forKey: .payload))
        case .catalogImage: self = .catalogImage(try container.decode(CatalogImageChunk.self, forKey: .payload))
        case .configuration: self = .configuration(try container.decode(ConfigSnapshot.self, forKey: .payload))
        case .effectivePermissions: self = .effectivePermissions(try container.decode(EffectivePermissions.self, forKey: .payload))
        case .scopeRelationships: self = .scopeRelationships(try container.decode(RelationshipPage.self, forKey: .payload))
        case .customer: self = .customer(try container.decode(Customer.self, forKey: .payload))
        case .customers: self = .customers(try container.decode(CustomerPage.self, forKey: .payload))
        case .furnitures: self = .furnitures(try container.decode(FurniturePage.self, forKey: .payload))
        case .furnitureCategories: self = .furnitureCategories(try container.decode(FurnitureCategories.self, forKey: .payload))
        case .furnitureRevision: self = .furnitureRevision(try container.decode(Furniture.self, forKey: .payload))
        case .furnitureReview: self = .furnitureReview(try container.decode(FurnitureReview.self, forKey: .payload))
        case .furnitureSelection: self = .furnitureSelection(try container.decode(FurnitureSelection.self, forKey: .payload))
        case .products: self = .products(try container.decode(ProductPage.self, forKey: .payload))
        case .productCategories: self = .productCategories(try container.decode(ProductCategories.self, forKey: .payload))
        case .productRevision: self = .productRevision(try container.decode(Product.self, forKey: .payload))
        case .parts: self = .parts(try container.decode(PartPage.self, forKey: .payload))
        case .partCategories: self = .partCategories(try container.decode(PartCategories.self, forKey: .payload))
        case .partCost: self = .partCost(try container.decode(PartCost.self, forKey: .payload))
        case .partComposition: self = .partComposition(try container.decode(Part.self, forKey: .payload))
        case .materials: self = .materials(try container.decode(MaterialPage.self, forKey: .payload))
        case .materialReferences: self = .materialReferences(try container.decode(MaterialReferences.self, forKey: .payload))
        case .desktopAccounts: self = .desktopAccounts(try container.decode(DesktopAccountPage.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .quotationEvaluation(let payload):
            try container.encode(Kind.quotationEvaluation, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .salesCatalog(let payload):
            try container.encode(Kind.salesCatalog, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .salesCatalogItem(let payload):
            try container.encode(Kind.salesCatalogItem, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .salesConfiguration(let payload):
            try container.encode(Kind.salesConfiguration, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .prices(let payload):
            try container.encode(Kind.prices, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .priceReview(let payload):
            try container.encode(Kind.priceReview, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .sellingPrice(let payload):
            try container.encode(Kind.sellingPrice, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .discountTotal(let payload):
            try container.encode(Kind.discountTotal, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .catalogImage(let payload):
            try container.encode(Kind.catalogImage, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .configuration(let payload):
            try container.encode(Kind.configuration, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .effectivePermissions(let payload):
            try container.encode(Kind.effectivePermissions, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .scopeRelationships(let payload):
            try container.encode(Kind.scopeRelationships, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customer(let payload):
            try container.encode(Kind.customer, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customers(let payload):
            try container.encode(Kind.customers, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitures(let payload):
            try container.encode(Kind.furnitures, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureCategories(let payload):
            try container.encode(Kind.furnitureCategories, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureRevision(let payload):
            try container.encode(Kind.furnitureRevision, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureReview(let payload):
            try container.encode(Kind.furnitureReview, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureSelection(let payload):
            try container.encode(Kind.furnitureSelection, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .products(let payload):
            try container.encode(Kind.products, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productCategories(let payload):
            try container.encode(Kind.productCategories, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productRevision(let payload):
            try container.encode(Kind.productRevision, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .parts(let payload):
            try container.encode(Kind.parts, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partCategories(let payload):
            try container.encode(Kind.partCategories, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partCost(let payload):
            try container.encode(Kind.partCost, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partComposition(let payload):
            try container.encode(Kind.partComposition, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materials(let payload):
            try container.encode(Kind.materials, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialReferences(let payload):
            try container.encode(Kind.materialReferences, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .desktopAccounts(let payload):
            try container.encode(Kind.desktopAccounts, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum Subscription: Codable, Sendable {
    case pricingChangedSubscribe(PriceChanges)
    case configChangedSubscribe(ConfigurationChanges)
    case permissionsChangedSubscribe(PermissionChanges)
    case authorizationPolicyChangedSubscribe(AuthorizationPolicyChanges)
    case customerChangedSubscribe(CustomerChanges)
    case materialChangedSubscribe(MaterialChanges)
    case furnitureChangedSubscribe(FurnitureChanges)
    case productChangedSubscribe(ProductChanges)
    case partChangedSubscribe(PartChanges)

    private enum Kind: String, Codable, Sendable {
        case pricingChangedSubscribe = "eitmad.pricing.changed.subscribe.v1"
        case configChangedSubscribe = "eitmad.config.changed.subscribe.v1"
        case permissionsChangedSubscribe = "eitmad.permissions.changed.subscribe.v1"
        case authorizationPolicyChangedSubscribe = "eitmad.authorization.policy.changed.subscribe.v1"
        case customerChangedSubscribe = "eitmad.customer.changed.subscribe.v1"
        case materialChangedSubscribe = "eitmad.material.changed.subscribe.v1"
        case furnitureChangedSubscribe = "eitmad.furniture.changed.subscribe.v1"
        case productChangedSubscribe = "eitmad.product.changed.subscribe.v1"
        case partChangedSubscribe = "eitmad.part.changed.subscribe.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .pricingChangedSubscribe: self = .pricingChangedSubscribe(try container.decode(PriceChanges.self, forKey: .payload))
        case .configChangedSubscribe: self = .configChangedSubscribe(try container.decode(ConfigurationChanges.self, forKey: .payload))
        case .permissionsChangedSubscribe: self = .permissionsChangedSubscribe(try container.decode(PermissionChanges.self, forKey: .payload))
        case .authorizationPolicyChangedSubscribe: self = .authorizationPolicyChangedSubscribe(try container.decode(AuthorizationPolicyChanges.self, forKey: .payload))
        case .customerChangedSubscribe: self = .customerChangedSubscribe(try container.decode(CustomerChanges.self, forKey: .payload))
        case .materialChangedSubscribe: self = .materialChangedSubscribe(try container.decode(MaterialChanges.self, forKey: .payload))
        case .furnitureChangedSubscribe: self = .furnitureChangedSubscribe(try container.decode(FurnitureChanges.self, forKey: .payload))
        case .productChangedSubscribe: self = .productChangedSubscribe(try container.decode(ProductChanges.self, forKey: .payload))
        case .partChangedSubscribe: self = .partChangedSubscribe(try container.decode(PartChanges.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .pricingChangedSubscribe(let payload):
            try container.encode(Kind.pricingChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .configChangedSubscribe(let payload):
            try container.encode(Kind.configChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .permissionsChangedSubscribe(let payload):
            try container.encode(Kind.permissionsChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationPolicyChangedSubscribe(let payload):
            try container.encode(Kind.authorizationPolicyChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .customerChangedSubscribe(let payload):
            try container.encode(Kind.customerChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .materialChangedSubscribe(let payload):
            try container.encode(Kind.materialChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .furnitureChangedSubscribe(let payload):
            try container.encode(Kind.furnitureChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .productChangedSubscribe(let payload):
            try container.encode(Kind.productChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .partChangedSubscribe(let payload):
            try container.encode(Kind.partChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum CommandOutcome: Codable, Sendable {
    case succeeded(CommandResult)
    case failed(ContractError)

    private enum Kind: String, Codable, Sendable {
        case succeeded = "succeeded"
        case failed = "failed"
    }

    private enum CodingKeys: String, CodingKey {
        case status
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .status) {
        case .succeeded: self = .succeeded(try container.decode(CommandResult.self, forKey: .payload))
        case .failed: self = .failed(try container.decode(ContractError.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .succeeded(let payload):
            try container.encode(Kind.succeeded, forKey: .status)
            try container.encode(payload, forKey: .payload)
        case .failed(let payload):
            try container.encode(Kind.failed, forKey: .status)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum HandshakeOutcome: Codable, Sendable {
    case accepted(HandshakeAccepted)
    case rejected(HandshakeRejection)

    private enum Kind: String, Codable, Sendable {
        case accepted = "accepted"
        case rejected = "rejected"
    }

    private enum CodingKeys: String, CodingKey {
        case status
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .status) {
        case .accepted: self = .accepted(try container.decode(HandshakeAccepted.self, forKey: .payload))
        case .rejected: self = .rejected(try container.decode(HandshakeRejection.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .accepted(let payload):
            try container.encode(Kind.accepted, forKey: .status)
            try container.encode(payload, forKey: .payload)
        case .rejected(let payload):
            try container.encode(Kind.rejected, forKey: .status)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum QueryOutcome: Codable, Sendable {
    case succeeded(QueryResult)
    case failed(ContractError)

    private enum Kind: String, Codable, Sendable {
        case succeeded = "succeeded"
        case failed = "failed"
    }

    private enum CodingKeys: String, CodingKey {
        case status
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .status) {
        case .succeeded: self = .succeeded(try container.decode(QueryResult.self, forKey: .payload))
        case .failed: self = .failed(try container.decode(ContractError.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .succeeded(let payload):
            try container.encode(Kind.succeeded, forKey: .status)
            try container.encode(payload, forKey: .payload)
        case .failed(let payload):
            try container.encode(Kind.failed, forKey: .status)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum SubscriptionOutcome: Codable, Sendable {
    case succeeded(SubscriptionAccepted)
    case failed(ContractError)

    private enum Kind: String, Codable, Sendable {
        case succeeded = "succeeded"
        case failed = "failed"
    }

    private enum CodingKeys: String, CodingKey {
        case status
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .status) {
        case .succeeded: self = .succeeded(try container.decode(SubscriptionAccepted.self, forKey: .payload))
        case .failed: self = .failed(try container.decode(ContractError.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .succeeded(let payload):
            try container.encode(Kind.succeeded, forKey: .status)
            try container.encode(payload, forKey: .payload)
        case .failed(let payload):
            try container.encode(Kind.failed, forKey: .status)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public struct AuthorizationPolicyChanges: Codable, Sendable {}
public struct ConfigurationChanges: Codable, Sendable {}
public struct CustomerChanges: Codable, Sendable {}
public struct FurnitureChanges: Codable, Sendable {}
public struct GetConfiguration: Codable, Sendable {}
public struct GetEffectivePermissions: Codable, Sendable {}
public struct ListDesktopAccounts: Codable, Sendable {}
public struct ListMaterialReferences: Codable, Sendable {}
public struct MaterialChanges: Codable, Sendable {}
public struct PartChanges: Codable, Sendable {}
public struct PermissionChanges: Codable, Sendable {}
public struct PriceChanges: Codable, Sendable {}
public struct ProductChanges: Codable, Sendable {}
