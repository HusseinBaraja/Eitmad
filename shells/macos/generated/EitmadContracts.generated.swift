// Generated from Rust contracts. Do not edit.
// This file was generated from JSON Schema using quicktype, do not modify it directly.
// To parse the JSON, add this file to your project and do:
//
//   let eitmadContractSchema = try? JSONDecoder().decode(EitmadContractSchema.self, from: jsonData)

import Foundation

// MARK: - EitmadContractSchema
public struct EitmadContractSchema: Codable, Sendable {
    public let ipcClientMessage, ipcServerMessage: [String: JSONAny]
    public let lifecycleSnapshot: LifecycleSnapshot
    public let unionPayloadKeepAlive: UnionPayloadKeepAlive

    public enum CodingKeys: String, CodingKey {
        case ipcClientMessage = "ipc_client_message"
        case ipcServerMessage = "ipc_server_message"
        case lifecycleSnapshot = "lifecycle_snapshot"
        case unionPayloadKeepAlive
    }

    public init(ipcClientMessage: [String: JSONAny], ipcServerMessage: [String: JSONAny], lifecycleSnapshot: LifecycleSnapshot, unionPayloadKeepAlive: UnionPayloadKeepAlive) {
        self.ipcClientMessage = ipcClientMessage
        self.ipcServerMessage = ipcServerMessage
        self.lifecycleSnapshot = lifecycleSnapshot
        self.unionPayloadKeepAlive = unionPayloadKeepAlive
    }
}

// MARK: - LifecycleSnapshot
public struct LifecycleSnapshot: Codable, Sendable {
    public let checks: [HealthCheckResult]
    public let error: ContractError?
    public let health: HealthStatus
    public let identity: EngineProcessIdentity
    public let live: Bool
    public let observedAt: Int
    public let ready: Bool
    public let state: LifecycleState

    public init(checks: [HealthCheckResult], error: ContractError?, health: HealthStatus, identity: EngineProcessIdentity, live: Bool, observedAt: Int, ready: Bool, state: LifecycleState) {
        self.checks = checks
        self.error = error
        self.health = health
        self.identity = identity
        self.live = live
        self.observedAt = observedAt
        self.ready = ready
        self.state = state
    }
}

// MARK: - HealthCheckResult
public struct HealthCheckResult: Codable, Sendable {
    public let elapsedMicros: Int
    public let error: ContractError?
    public let id: String
    public let impact: HealthCheckImpact
    public let observedAt: Int
    public let status: HealthStatus

    public init(elapsedMicros: Int, error: ContractError?, id: String, impact: HealthCheckImpact, observedAt: Int, status: HealthStatus) {
        self.elapsedMicros = elapsedMicros
        self.error = error
        self.id = id
        self.impact = impact
        self.observedAt = observedAt
        self.status = status
    }
}

// MARK: - ContractError
public struct ContractError: Codable, Sendable {
    public let code, correlationID: String
    public let detail: ErrorDetail?
    public let messageID: String
    public let parameters: [ErrorParameter]
    public let retry: RetryDisposition

    public enum CodingKeys: String, CodingKey {
        case code
        case correlationID = "correlationId"
        case detail
        case messageID = "messageId"
        case parameters, retry
    }

    public init(code: String, correlationID: String, detail: ErrorDetail?, messageID: String, parameters: [ErrorParameter], retry: RetryDisposition) {
        self.code = code
        self.correlationID = correlationID
        self.detail = detail
        self.messageID = messageID
        self.parameters = parameters
        self.retry = retry
    }
}

// MARK: - ErrorDetail
public struct ErrorDetail: Codable, Sendable {
    public let kind: DetailKind
    public let payload: DetailPayload

    public init(kind: DetailKind, payload: DetailPayload) {
        self.kind = kind
        self.payload = payload
    }
}

public enum DetailKind: String, Codable, Sendable {
    case compatibility = "compatibility"
    case deadline = "deadline"
    case lifecycle = "lifecycle"
    case payloadLimit = "payloadLimit"
    case revisionConflict = "revisionConflict"
    case validation = "validation"
}

// MARK: - DetailPayload
public struct DetailPayload: Codable, Sendable {
    public let fields: [String]?
    public let actual, expected: Int?
    public let reason: String?
    public let stage: LifecycleStage?
    public let deadline: Int?
    public let maximumBytes: Int?

    public enum CodingKeys: String, CodingKey {
        case fields, actual, expected, reason, stage, deadline
        case maximumBytes = "maximum_bytes"
    }

    public init(fields: [String]?, actual: Int?, expected: Int?, reason: String?, stage: LifecycleStage?, deadline: Int?, maximumBytes: Int?) {
        self.fields = fields
        self.actual = actual
        self.expected = expected
        self.reason = reason
        self.stage = stage
        self.deadline = deadline
        self.maximumBytes = maximumBytes
    }
}

public enum LifecycleStage: String, Codable, Sendable {
    case authorityLock = "authorityLock"
    case componentShutdown = "componentShutdown"
    case componentStartup = "componentStartup"
    case processIdentity = "processIdentity"
    case readinessCheck = "readinessCheck"
}

// MARK: - ErrorParameter
public struct ErrorParameter: Codable, Sendable {
    public let name: String
    public let value: ErrorParameterValue

    public init(name: String, value: ErrorParameterValue) {
        self.name = name
        self.value = value
    }
}

// MARK: - ErrorParameterValue
public struct ErrorParameterValue: Codable, Sendable {
    public let kind: ErrorParameterValueKind
    public let value: ErrorParameterValueValue

    public init(kind: ErrorParameterValueKind, value: ErrorParameterValueValue) {
        self.kind = kind
        self.value = value
    }
}

public enum ErrorParameterValueKind: String, Codable, Sendable {
    case identifier = "identifier"
    case integer = "integer"
    case text = "text"
}

public enum ErrorParameterValueValue: Codable, Sendable {
    case integer(Int)
    case string(String)

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let x = try? container.decode(Int.self) {
            self = .integer(x)
            return
        }
        if let x = try? container.decode(String.self) {
            self = .string(x)
            return
        }
        throw DecodingError.typeMismatch(ErrorParameterValueValue.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Wrong type for ErrorParameterValueValue"))
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .integer(let x):
            try container.encode(x)
        case .string(let x):
            try container.encode(x)
        }
    }
}

// MARK: - RetryDisposition
public struct RetryDisposition: Codable, Sendable {
    public let kind: RetryDispositionKind
    public let retryAfterMS: Int?

    public enum CodingKeys: String, CodingKey {
        case kind
        case retryAfterMS = "retryAfterMs"
    }

    public init(kind: RetryDispositionKind, retryAfterMS: Int?) {
        self.kind = kind
        self.retryAfterMS = retryAfterMS
    }
}

public enum RetryDispositionKind: String, Codable, Sendable {
    case never = "never"
    case safeAfterDelay = "safeAfterDelay"
    case safeImmediately = "safeImmediately"
}

public enum HealthCheckImpact: String, Codable, Sendable {
    case advisory = "advisory"
    case requiredForReadiness = "requiredForReadiness"
}

public enum HealthStatus: String, Codable, Sendable {
    case degraded = "degraded"
    case healthy = "healthy"
    case unhealthy = "unhealthy"
}

// MARK: - EngineProcessIdentity
public struct EngineProcessIdentity: Codable, Sendable {
    public let instanceID: String
    public let mode: EngineMode
    public let processID: Int
    public let productVersion: String
    public let protocolVersion: ProtocolVersion
    public let startedAt: Int

    public enum CodingKeys: String, CodingKey {
        case instanceID = "instanceId"
        case mode
        case processID = "processId"
        case productVersion, protocolVersion, startedAt
    }

    public init(instanceID: String, mode: EngineMode, processID: Int, productVersion: String, protocolVersion: ProtocolVersion, startedAt: Int) {
        self.instanceID = instanceID
        self.mode = mode
        self.processID = processID
        self.productVersion = productVersion
        self.protocolVersion = protocolVersion
        self.startedAt = startedAt
    }
}

public enum EngineMode: String, Codable, Sendable {
    case diagnostic = "diagnostic"
    case headless = "headless"
    case supervisedDesktop = "supervisedDesktop"
}

// MARK: - ProtocolVersion
public struct ProtocolVersion: Codable, Sendable {
    public let major, minor: Int

    public init(major: Int, minor: Int) {
        self.major = major
        self.minor = minor
    }
}

public enum LifecycleState: String, Codable, Sendable {
    case failed = "failed"
    case ready = "ready"
    case starting = "starting"
    case stopped = "stopped"
    case stopping = "stopping"
}

// MARK: - UnionPayloadKeepAlive
public struct UnionPayloadKeepAlive: Codable, Sendable {
    public let commandAuthorizationRelationshipGrant: GrantScopeRelationship?
    public let commandAuthorizationRelationshipRevoke: RevokeScopeRelationship?
    public let commandCatalogImageImport: ImportCatalogImage?
    public let commandConfigUpdate: UpdateConfiguration?
    public let commandCustomerCreate: CreateCustomer?
    public let commandCustomerUpdate: UpdateCustomer?
    public let commandDesktopAccountCreate: CreateDesktopAccount?
    public let commandDesktopAccountDeactivate: DeactivateDesktopAccount?
    public let commandDesktopAccountUpdate: UpdateDesktopAccount?
    public let commandFurnitureCategorySave: SaveFurnitureCategory?
    public let commandFurnitureSave: SaveFurniture?
    public let commandMaterialCategorySave: SaveMaterialCategory?
    public let commandMaterialSave: SaveMaterial?
    public let commandMaterialUnitSave: SaveMaterialUnit?
    public let commandPartCategorySave: SavePartCategory?
    public let commandPartSave: SavePart?
    public let commandPricingPublish: PublishPrice?
    public let commandProductCategorySave: SaveProductCategory?
    public let commandProductSave: SaveProduct?
    public let commandOutcomeFailed: ContractError?
    public let commandOutcomeSucceeded: [String: JSONAny]?
    public let commandResultCatalogImageImported: CatalogImageRef?
    public let commandResultConfigurationUpdated: ConfigSnapshot?
    public let commandResultCustomerCreated, commandResultCustomerUpdated: CustomerMutationResult?
    public let commandResultDesktopAccountCreated, commandResultDesktopAccountDeactivated, commandResultDesktopAccountUpdated: DesktopAccountSummary?
    public let commandResultFurnitureCategorySaved: FurnitureCategory?
    public let commandResultFurnitureSaved: Furniture?
    public let commandResultMaterialCategorySaved: MaterialCategory?
    public let commandResultMaterialSaved: Material?
    public let commandResultMaterialUnitSaved: MaterialUnit?
    public let commandResultPartCategorySaved: PartCategory?
    public let commandResultPartSaved: Part?
    public let commandResultPricePublished: PublishedPrice?
    public let commandResultProductCategorySaved: ProductCategory?
    public let commandResultProductSaved: Product?
    public let commandResultRelationshipGranted, commandResultRelationshipRevoked: RelationshipMutationResult?
    public let eventAuthorizationPolicyChangedEvent: AuthorizationPolicyChangeNotice?
    public let eventConfigChangedEvent: ConfigSnapshot?
    public let eventCustomerChangedEvent: CustomerChangeNotice?
    public let eventFurnitureChangedEvent: FurnitureChangeNotice?
    public let eventMaterialChangedEvent: MaterialChangeNotice?
    public let eventPartChangedEvent: PartChangeNotice?
    public let eventPermissionsChangedEvent: EffectivePermissions?
    public let eventPricingChangedEvent: PriceChangeNotice?
    public let eventProductChangedEvent: ProductChangeNotice?
    public let handshakeOutcomeAccepted: HandshakeAccepted?
    public let handshakeOutcomeRejected: HandshakeRejection?
    public let ipcClientMessageIPCCommand: CommandEnvelope?
    public let ipcClientMessageIPCDesktopSessionState: DesktopSessionRequest?
    public let ipcClientMessageIPCDesktopSignIn: DesktopSignInRequest?
    public let ipcClientMessageIPCDesktopSignOut: DesktopSessionRequest?
    public let ipcClientMessageIPCHandshake: HandshakeRequest?
    public let ipcClientMessageIPCQuery: QueryEnvelope?
    public let ipcClientMessageIPCShutdown: ShutdownRequest?
    public let ipcClientMessageIPCSubscribe: SubscriptionEnvelope?
    public let ipcClientMessageIPCUnsubscribe: UnsubscribeRequest?
    public let ipcServerMessageIPCCommandResponse: CommandResponseEnvelope?
    public let ipcServerMessageIPCDesktopSessionResponse: DesktopSessionResponse?
    public let ipcServerMessageIPCEvent: EventEnvelope?
    public let ipcServerMessageIPCFailure: IPCFailureResponse?
    public let ipcServerMessageIPCHandshakeResponse: HandshakeResponse?
    public let ipcServerMessageIPCQueryResponse: QueryResponseEnvelope?
    public let ipcServerMessageIPCShutdownResponse: ShutdownResponse?
    public let ipcServerMessageIPCSubscribeResponse: SubscriptionResponseEnvelope?
    public let ipcServerMessageIPCSubscriptionClosed: SubscriptionClosedEnvelope?
    public let ipcServerMessageIPCUnsubscribeResponse: UnsubscribeResponse?
    public let priceTargetFurniture: FurnitureReference?
    public let priceTargetProduct: ProductReference?
    public let queryAuthorizationRelationshipsList: ListScopeRelationships?
    public let queryCatalogImageGet: GetCatalogImage?
    public let queryConfigGet: [String: JSONAny]?
    public let queryCustomerGet: GetCustomer?
    public let queryCustomerSearch: SearchCustomers?
    public let queryDesktopAccountList: [String: JSONAny]?
    public let queryFurnitureCategoryList: ListFurnitureCategories?
    public let queryFurnitureList: ListFurnitures?
    public let queryFurnitureReview: SaveFurniture?
    public let queryFurnitureRevisionGet: GetFurnitureRevision?
    public let queryFurnitureSelectionCheck: CheckFurnitureSelection?
    public let queryMaterialList: ListMaterials?
    public let queryMaterialReferenceList: [String: JSONAny]?
    public let queryPartCategoryList: ListPartCategories?
    public let queryPartCompositionGet: GetPartComposition?
    public let queryPartCost: CalculatePartCost?
    public let queryPartList: ListParts?
    public let queryPermissionsGetEffective: [String: JSONAny]?
    public let queryPricingDiscount: CalculateDiscount?
    public let queryPricingList: ListPrices?
    public let queryPricingReview: ReviewPrice?
    public let queryPricingSelection: PriceSelection?
    public let queryProductCategoryList: ListProductCategories?
    public let queryProductList: ListProducts?
    public let queryProductRevisionGet: GetProductRevision?
    public let queryOutcomeFailed: ContractError?
    public let queryOutcomeSucceeded: [String: JSONAny]?
    public let queryResultCatalogImage: CatalogImageChunk?
    public let queryResultConfiguration: ConfigSnapshot?
    public let queryResultCustomer: Customer?
    public let queryResultCustomers: CustomerPage?
    public let queryResultDesktopAccounts: DesktopAccountPage?
    public let queryResultDiscountTotal: DiscountTotal?
    public let queryResultEffectivePermissions: EffectivePermissions?
    public let queryResultFurnitureCategories: FurnitureCategories?
    public let queryResultFurnitureReview: FurnitureReview?
    public let queryResultFurnitureRevision: Furniture?
    public let queryResultFurnitures: FurniturePage?
    public let queryResultFurnitureSelection: FurnitureSelection?
    public let queryResultMaterialReferences: MaterialReferences?
    public let queryResultMaterials: MaterialPage?
    public let queryResultPartCategories: PartCategories?
    public let queryResultPartComposition: Part?
    public let queryResultPartCost: PartCost?
    public let queryResultParts: PartPage?
    public let queryResultPriceReview: PriceReview?
    public let queryResultPrices: PricePage?
    public let queryResultProductCategories: ProductCategories?
    public let queryResultProductRevision: Product?
    public let queryResultProducts: ProductPage?
    public let queryResultScopeRelationships: RelationshipPage?
    public let queryResultSellingPrice: SellingPrice?
    public let subscriptionAuthorizationPolicyChangedSubscribe, subscriptionConfigChangedSubscribe, subscriptionCustomerChangedSubscribe, subscriptionFurnitureChangedSubscribe: [String: JSONAny]?
    public let subscriptionMaterialChangedSubscribe, subscriptionPartChangedSubscribe, subscriptionPermissionsChangedSubscribe, subscriptionPricingChangedSubscribe: [String: JSONAny]?
    public let subscriptionProductChangedSubscribe: [String: JSONAny]?
    public let subscriptionOutcomeFailed: ContractError?
    public let subscriptionOutcomeSucceeded: SubscriptionAccepted?

    public enum CodingKeys: String, CodingKey {
        case commandAuthorizationRelationshipGrant = "Command_AuthorizationRelationshipGrant"
        case commandAuthorizationRelationshipRevoke = "Command_AuthorizationRelationshipRevoke"
        case commandCatalogImageImport = "Command_CatalogImageImport"
        case commandConfigUpdate = "Command_ConfigUpdate"
        case commandCustomerCreate = "Command_CustomerCreate"
        case commandCustomerUpdate = "Command_CustomerUpdate"
        case commandDesktopAccountCreate = "Command_DesktopAccountCreate"
        case commandDesktopAccountDeactivate = "Command_DesktopAccountDeactivate"
        case commandDesktopAccountUpdate = "Command_DesktopAccountUpdate"
        case commandFurnitureCategorySave = "Command_FurnitureCategorySave"
        case commandFurnitureSave = "Command_FurnitureSave"
        case commandMaterialCategorySave = "Command_MaterialCategorySave"
        case commandMaterialSave = "Command_MaterialSave"
        case commandMaterialUnitSave = "Command_MaterialUnitSave"
        case commandPartCategorySave = "Command_PartCategorySave"
        case commandPartSave = "Command_PartSave"
        case commandPricingPublish = "Command_PricingPublish"
        case commandProductCategorySave = "Command_ProductCategorySave"
        case commandProductSave = "Command_ProductSave"
        case commandOutcomeFailed = "CommandOutcome_Failed"
        case commandOutcomeSucceeded = "CommandOutcome_Succeeded"
        case commandResultCatalogImageImported = "CommandResult_CatalogImageImported"
        case commandResultConfigurationUpdated = "CommandResult_ConfigurationUpdated"
        case commandResultCustomerCreated = "CommandResult_CustomerCreated"
        case commandResultCustomerUpdated = "CommandResult_CustomerUpdated"
        case commandResultDesktopAccountCreated = "CommandResult_DesktopAccountCreated"
        case commandResultDesktopAccountDeactivated = "CommandResult_DesktopAccountDeactivated"
        case commandResultDesktopAccountUpdated = "CommandResult_DesktopAccountUpdated"
        case commandResultFurnitureCategorySaved = "CommandResult_FurnitureCategorySaved"
        case commandResultFurnitureSaved = "CommandResult_FurnitureSaved"
        case commandResultMaterialCategorySaved = "CommandResult_MaterialCategorySaved"
        case commandResultMaterialSaved = "CommandResult_MaterialSaved"
        case commandResultMaterialUnitSaved = "CommandResult_MaterialUnitSaved"
        case commandResultPartCategorySaved = "CommandResult_PartCategorySaved"
        case commandResultPartSaved = "CommandResult_PartSaved"
        case commandResultPricePublished = "CommandResult_PricePublished"
        case commandResultProductCategorySaved = "CommandResult_ProductCategorySaved"
        case commandResultProductSaved = "CommandResult_ProductSaved"
        case commandResultRelationshipGranted = "CommandResult_RelationshipGranted"
        case commandResultRelationshipRevoked = "CommandResult_RelationshipRevoked"
        case eventAuthorizationPolicyChangedEvent = "Event_AuthorizationPolicyChangedEvent"
        case eventConfigChangedEvent = "Event_ConfigChangedEvent"
        case eventCustomerChangedEvent = "Event_CustomerChangedEvent"
        case eventFurnitureChangedEvent = "Event_FurnitureChangedEvent"
        case eventMaterialChangedEvent = "Event_MaterialChangedEvent"
        case eventPartChangedEvent = "Event_PartChangedEvent"
        case eventPermissionsChangedEvent = "Event_PermissionsChangedEvent"
        case eventPricingChangedEvent = "Event_PricingChangedEvent"
        case eventProductChangedEvent = "Event_ProductChangedEvent"
        case handshakeOutcomeAccepted = "HandshakeOutcome_Accepted"
        case handshakeOutcomeRejected = "HandshakeOutcome_Rejected"
        case ipcClientMessageIPCCommand = "IpcClientMessage_IpcCommand"
        case ipcClientMessageIPCDesktopSessionState = "IpcClientMessage_IpcDesktopSessionState"
        case ipcClientMessageIPCDesktopSignIn = "IpcClientMessage_IpcDesktopSignIn"
        case ipcClientMessageIPCDesktopSignOut = "IpcClientMessage_IpcDesktopSignOut"
        case ipcClientMessageIPCHandshake = "IpcClientMessage_IpcHandshake"
        case ipcClientMessageIPCQuery = "IpcClientMessage_IpcQuery"
        case ipcClientMessageIPCShutdown = "IpcClientMessage_IpcShutdown"
        case ipcClientMessageIPCSubscribe = "IpcClientMessage_IpcSubscribe"
        case ipcClientMessageIPCUnsubscribe = "IpcClientMessage_IpcUnsubscribe"
        case ipcServerMessageIPCCommandResponse = "IpcServerMessage_IpcCommandResponse"
        case ipcServerMessageIPCDesktopSessionResponse = "IpcServerMessage_IpcDesktopSessionResponse"
        case ipcServerMessageIPCEvent = "IpcServerMessage_IpcEvent"
        case ipcServerMessageIPCFailure = "IpcServerMessage_IpcFailure"
        case ipcServerMessageIPCHandshakeResponse = "IpcServerMessage_IpcHandshakeResponse"
        case ipcServerMessageIPCQueryResponse = "IpcServerMessage_IpcQueryResponse"
        case ipcServerMessageIPCShutdownResponse = "IpcServerMessage_IpcShutdownResponse"
        case ipcServerMessageIPCSubscribeResponse = "IpcServerMessage_IpcSubscribeResponse"
        case ipcServerMessageIPCSubscriptionClosed = "IpcServerMessage_IpcSubscriptionClosed"
        case ipcServerMessageIPCUnsubscribeResponse = "IpcServerMessage_IpcUnsubscribeResponse"
        case priceTargetFurniture = "PriceTarget_Furniture"
        case priceTargetProduct = "PriceTarget_Product"
        case queryAuthorizationRelationshipsList = "Query_AuthorizationRelationshipsList"
        case queryCatalogImageGet = "Query_CatalogImageGet"
        case queryConfigGet = "Query_ConfigGet"
        case queryCustomerGet = "Query_CustomerGet"
        case queryCustomerSearch = "Query_CustomerSearch"
        case queryDesktopAccountList = "Query_DesktopAccountList"
        case queryFurnitureCategoryList = "Query_FurnitureCategoryList"
        case queryFurnitureList = "Query_FurnitureList"
        case queryFurnitureReview = "Query_FurnitureReview"
        case queryFurnitureRevisionGet = "Query_FurnitureRevisionGet"
        case queryFurnitureSelectionCheck = "Query_FurnitureSelectionCheck"
        case queryMaterialList = "Query_MaterialList"
        case queryMaterialReferenceList = "Query_MaterialReferenceList"
        case queryPartCategoryList = "Query_PartCategoryList"
        case queryPartCompositionGet = "Query_PartCompositionGet"
        case queryPartCost = "Query_PartCost"
        case queryPartList = "Query_PartList"
        case queryPermissionsGetEffective = "Query_PermissionsGetEffective"
        case queryPricingDiscount = "Query_PricingDiscount"
        case queryPricingList = "Query_PricingList"
        case queryPricingReview = "Query_PricingReview"
        case queryPricingSelection = "Query_PricingSelection"
        case queryProductCategoryList = "Query_ProductCategoryList"
        case queryProductList = "Query_ProductList"
        case queryProductRevisionGet = "Query_ProductRevisionGet"
        case queryOutcomeFailed = "QueryOutcome_Failed"
        case queryOutcomeSucceeded = "QueryOutcome_Succeeded"
        case queryResultCatalogImage = "QueryResult_CatalogImage"
        case queryResultConfiguration = "QueryResult_Configuration"
        case queryResultCustomer = "QueryResult_Customer"
        case queryResultCustomers = "QueryResult_Customers"
        case queryResultDesktopAccounts = "QueryResult_DesktopAccounts"
        case queryResultDiscountTotal = "QueryResult_DiscountTotal"
        case queryResultEffectivePermissions = "QueryResult_EffectivePermissions"
        case queryResultFurnitureCategories = "QueryResult_FurnitureCategories"
        case queryResultFurnitureReview = "QueryResult_FurnitureReview"
        case queryResultFurnitureRevision = "QueryResult_FurnitureRevision"
        case queryResultFurnitures = "QueryResult_Furnitures"
        case queryResultFurnitureSelection = "QueryResult_FurnitureSelection"
        case queryResultMaterialReferences = "QueryResult_MaterialReferences"
        case queryResultMaterials = "QueryResult_Materials"
        case queryResultPartCategories = "QueryResult_PartCategories"
        case queryResultPartComposition = "QueryResult_PartComposition"
        case queryResultPartCost = "QueryResult_PartCost"
        case queryResultParts = "QueryResult_Parts"
        case queryResultPriceReview = "QueryResult_PriceReview"
        case queryResultPrices = "QueryResult_Prices"
        case queryResultProductCategories = "QueryResult_ProductCategories"
        case queryResultProductRevision = "QueryResult_ProductRevision"
        case queryResultProducts = "QueryResult_Products"
        case queryResultScopeRelationships = "QueryResult_ScopeRelationships"
        case queryResultSellingPrice = "QueryResult_SellingPrice"
        case subscriptionAuthorizationPolicyChangedSubscribe = "Subscription_AuthorizationPolicyChangedSubscribe"
        case subscriptionConfigChangedSubscribe = "Subscription_ConfigChangedSubscribe"
        case subscriptionCustomerChangedSubscribe = "Subscription_CustomerChangedSubscribe"
        case subscriptionFurnitureChangedSubscribe = "Subscription_FurnitureChangedSubscribe"
        case subscriptionMaterialChangedSubscribe = "Subscription_MaterialChangedSubscribe"
        case subscriptionPartChangedSubscribe = "Subscription_PartChangedSubscribe"
        case subscriptionPermissionsChangedSubscribe = "Subscription_PermissionsChangedSubscribe"
        case subscriptionPricingChangedSubscribe = "Subscription_PricingChangedSubscribe"
        case subscriptionProductChangedSubscribe = "Subscription_ProductChangedSubscribe"
        case subscriptionOutcomeFailed = "SubscriptionOutcome_Failed"
        case subscriptionOutcomeSucceeded = "SubscriptionOutcome_Succeeded"
    }

    public init(commandAuthorizationRelationshipGrant: GrantScopeRelationship?, commandAuthorizationRelationshipRevoke: RevokeScopeRelationship?, commandCatalogImageImport: ImportCatalogImage?, commandConfigUpdate: UpdateConfiguration?, commandCustomerCreate: CreateCustomer?, commandCustomerUpdate: UpdateCustomer?, commandDesktopAccountCreate: CreateDesktopAccount?, commandDesktopAccountDeactivate: DeactivateDesktopAccount?, commandDesktopAccountUpdate: UpdateDesktopAccount?, commandFurnitureCategorySave: SaveFurnitureCategory?, commandFurnitureSave: SaveFurniture?, commandMaterialCategorySave: SaveMaterialCategory?, commandMaterialSave: SaveMaterial?, commandMaterialUnitSave: SaveMaterialUnit?, commandPartCategorySave: SavePartCategory?, commandPartSave: SavePart?, commandPricingPublish: PublishPrice?, commandProductCategorySave: SaveProductCategory?, commandProductSave: SaveProduct?, commandOutcomeFailed: ContractError?, commandOutcomeSucceeded: [String: JSONAny]?, commandResultCatalogImageImported: CatalogImageRef?, commandResultConfigurationUpdated: ConfigSnapshot?, commandResultCustomerCreated: CustomerMutationResult?, commandResultCustomerUpdated: CustomerMutationResult?, commandResultDesktopAccountCreated: DesktopAccountSummary?, commandResultDesktopAccountDeactivated: DesktopAccountSummary?, commandResultDesktopAccountUpdated: DesktopAccountSummary?, commandResultFurnitureCategorySaved: FurnitureCategory?, commandResultFurnitureSaved: Furniture?, commandResultMaterialCategorySaved: MaterialCategory?, commandResultMaterialSaved: Material?, commandResultMaterialUnitSaved: MaterialUnit?, commandResultPartCategorySaved: PartCategory?, commandResultPartSaved: Part?, commandResultPricePublished: PublishedPrice?, commandResultProductCategorySaved: ProductCategory?, commandResultProductSaved: Product?, commandResultRelationshipGranted: RelationshipMutationResult?, commandResultRelationshipRevoked: RelationshipMutationResult?, eventAuthorizationPolicyChangedEvent: AuthorizationPolicyChangeNotice?, eventConfigChangedEvent: ConfigSnapshot?, eventCustomerChangedEvent: CustomerChangeNotice?, eventFurnitureChangedEvent: FurnitureChangeNotice?, eventMaterialChangedEvent: MaterialChangeNotice?, eventPartChangedEvent: PartChangeNotice?, eventPermissionsChangedEvent: EffectivePermissions?, eventPricingChangedEvent: PriceChangeNotice?, eventProductChangedEvent: ProductChangeNotice?, handshakeOutcomeAccepted: HandshakeAccepted?, handshakeOutcomeRejected: HandshakeRejection?, ipcClientMessageIPCCommand: CommandEnvelope?, ipcClientMessageIPCDesktopSessionState: DesktopSessionRequest?, ipcClientMessageIPCDesktopSignIn: DesktopSignInRequest?, ipcClientMessageIPCDesktopSignOut: DesktopSessionRequest?, ipcClientMessageIPCHandshake: HandshakeRequest?, ipcClientMessageIPCQuery: QueryEnvelope?, ipcClientMessageIPCShutdown: ShutdownRequest?, ipcClientMessageIPCSubscribe: SubscriptionEnvelope?, ipcClientMessageIPCUnsubscribe: UnsubscribeRequest?, ipcServerMessageIPCCommandResponse: CommandResponseEnvelope?, ipcServerMessageIPCDesktopSessionResponse: DesktopSessionResponse?, ipcServerMessageIPCEvent: EventEnvelope?, ipcServerMessageIPCFailure: IPCFailureResponse?, ipcServerMessageIPCHandshakeResponse: HandshakeResponse?, ipcServerMessageIPCQueryResponse: QueryResponseEnvelope?, ipcServerMessageIPCShutdownResponse: ShutdownResponse?, ipcServerMessageIPCSubscribeResponse: SubscriptionResponseEnvelope?, ipcServerMessageIPCSubscriptionClosed: SubscriptionClosedEnvelope?, ipcServerMessageIPCUnsubscribeResponse: UnsubscribeResponse?, priceTargetFurniture: FurnitureReference?, priceTargetProduct: ProductReference?, queryAuthorizationRelationshipsList: ListScopeRelationships?, queryCatalogImageGet: GetCatalogImage?, queryConfigGet: [String: JSONAny]?, queryCustomerGet: GetCustomer?, queryCustomerSearch: SearchCustomers?, queryDesktopAccountList: [String: JSONAny]?, queryFurnitureCategoryList: ListFurnitureCategories?, queryFurnitureList: ListFurnitures?, queryFurnitureReview: SaveFurniture?, queryFurnitureRevisionGet: GetFurnitureRevision?, queryFurnitureSelectionCheck: CheckFurnitureSelection?, queryMaterialList: ListMaterials?, queryMaterialReferenceList: [String: JSONAny]?, queryPartCategoryList: ListPartCategories?, queryPartCompositionGet: GetPartComposition?, queryPartCost: CalculatePartCost?, queryPartList: ListParts?, queryPermissionsGetEffective: [String: JSONAny]?, queryPricingDiscount: CalculateDiscount?, queryPricingList: ListPrices?, queryPricingReview: ReviewPrice?, queryPricingSelection: PriceSelection?, queryProductCategoryList: ListProductCategories?, queryProductList: ListProducts?, queryProductRevisionGet: GetProductRevision?, queryOutcomeFailed: ContractError?, queryOutcomeSucceeded: [String: JSONAny]?, queryResultCatalogImage: CatalogImageChunk?, queryResultConfiguration: ConfigSnapshot?, queryResultCustomer: Customer?, queryResultCustomers: CustomerPage?, queryResultDesktopAccounts: DesktopAccountPage?, queryResultDiscountTotal: DiscountTotal?, queryResultEffectivePermissions: EffectivePermissions?, queryResultFurnitureCategories: FurnitureCategories?, queryResultFurnitureReview: FurnitureReview?, queryResultFurnitureRevision: Furniture?, queryResultFurnitures: FurniturePage?, queryResultFurnitureSelection: FurnitureSelection?, queryResultMaterialReferences: MaterialReferences?, queryResultMaterials: MaterialPage?, queryResultPartCategories: PartCategories?, queryResultPartComposition: Part?, queryResultPartCost: PartCost?, queryResultParts: PartPage?, queryResultPriceReview: PriceReview?, queryResultPrices: PricePage?, queryResultProductCategories: ProductCategories?, queryResultProductRevision: Product?, queryResultProducts: ProductPage?, queryResultScopeRelationships: RelationshipPage?, queryResultSellingPrice: SellingPrice?, subscriptionAuthorizationPolicyChangedSubscribe: [String: JSONAny]?, subscriptionConfigChangedSubscribe: [String: JSONAny]?, subscriptionCustomerChangedSubscribe: [String: JSONAny]?, subscriptionFurnitureChangedSubscribe: [String: JSONAny]?, subscriptionMaterialChangedSubscribe: [String: JSONAny]?, subscriptionPartChangedSubscribe: [String: JSONAny]?, subscriptionPermissionsChangedSubscribe: [String: JSONAny]?, subscriptionPricingChangedSubscribe: [String: JSONAny]?, subscriptionProductChangedSubscribe: [String: JSONAny]?, subscriptionOutcomeFailed: ContractError?, subscriptionOutcomeSucceeded: SubscriptionAccepted?) {
        self.commandAuthorizationRelationshipGrant = commandAuthorizationRelationshipGrant
        self.commandAuthorizationRelationshipRevoke = commandAuthorizationRelationshipRevoke
        self.commandCatalogImageImport = commandCatalogImageImport
        self.commandConfigUpdate = commandConfigUpdate
        self.commandCustomerCreate = commandCustomerCreate
        self.commandCustomerUpdate = commandCustomerUpdate
        self.commandDesktopAccountCreate = commandDesktopAccountCreate
        self.commandDesktopAccountDeactivate = commandDesktopAccountDeactivate
        self.commandDesktopAccountUpdate = commandDesktopAccountUpdate
        self.commandFurnitureCategorySave = commandFurnitureCategorySave
        self.commandFurnitureSave = commandFurnitureSave
        self.commandMaterialCategorySave = commandMaterialCategorySave
        self.commandMaterialSave = commandMaterialSave
        self.commandMaterialUnitSave = commandMaterialUnitSave
        self.commandPartCategorySave = commandPartCategorySave
        self.commandPartSave = commandPartSave
        self.commandPricingPublish = commandPricingPublish
        self.commandProductCategorySave = commandProductCategorySave
        self.commandProductSave = commandProductSave
        self.commandOutcomeFailed = commandOutcomeFailed
        self.commandOutcomeSucceeded = commandOutcomeSucceeded
        self.commandResultCatalogImageImported = commandResultCatalogImageImported
        self.commandResultConfigurationUpdated = commandResultConfigurationUpdated
        self.commandResultCustomerCreated = commandResultCustomerCreated
        self.commandResultCustomerUpdated = commandResultCustomerUpdated
        self.commandResultDesktopAccountCreated = commandResultDesktopAccountCreated
        self.commandResultDesktopAccountDeactivated = commandResultDesktopAccountDeactivated
        self.commandResultDesktopAccountUpdated = commandResultDesktopAccountUpdated
        self.commandResultFurnitureCategorySaved = commandResultFurnitureCategorySaved
        self.commandResultFurnitureSaved = commandResultFurnitureSaved
        self.commandResultMaterialCategorySaved = commandResultMaterialCategorySaved
        self.commandResultMaterialSaved = commandResultMaterialSaved
        self.commandResultMaterialUnitSaved = commandResultMaterialUnitSaved
        self.commandResultPartCategorySaved = commandResultPartCategorySaved
        self.commandResultPartSaved = commandResultPartSaved
        self.commandResultPricePublished = commandResultPricePublished
        self.commandResultProductCategorySaved = commandResultProductCategorySaved
        self.commandResultProductSaved = commandResultProductSaved
        self.commandResultRelationshipGranted = commandResultRelationshipGranted
        self.commandResultRelationshipRevoked = commandResultRelationshipRevoked
        self.eventAuthorizationPolicyChangedEvent = eventAuthorizationPolicyChangedEvent
        self.eventConfigChangedEvent = eventConfigChangedEvent
        self.eventCustomerChangedEvent = eventCustomerChangedEvent
        self.eventFurnitureChangedEvent = eventFurnitureChangedEvent
        self.eventMaterialChangedEvent = eventMaterialChangedEvent
        self.eventPartChangedEvent = eventPartChangedEvent
        self.eventPermissionsChangedEvent = eventPermissionsChangedEvent
        self.eventPricingChangedEvent = eventPricingChangedEvent
        self.eventProductChangedEvent = eventProductChangedEvent
        self.handshakeOutcomeAccepted = handshakeOutcomeAccepted
        self.handshakeOutcomeRejected = handshakeOutcomeRejected
        self.ipcClientMessageIPCCommand = ipcClientMessageIPCCommand
        self.ipcClientMessageIPCDesktopSessionState = ipcClientMessageIPCDesktopSessionState
        self.ipcClientMessageIPCDesktopSignIn = ipcClientMessageIPCDesktopSignIn
        self.ipcClientMessageIPCDesktopSignOut = ipcClientMessageIPCDesktopSignOut
        self.ipcClientMessageIPCHandshake = ipcClientMessageIPCHandshake
        self.ipcClientMessageIPCQuery = ipcClientMessageIPCQuery
        self.ipcClientMessageIPCShutdown = ipcClientMessageIPCShutdown
        self.ipcClientMessageIPCSubscribe = ipcClientMessageIPCSubscribe
        self.ipcClientMessageIPCUnsubscribe = ipcClientMessageIPCUnsubscribe
        self.ipcServerMessageIPCCommandResponse = ipcServerMessageIPCCommandResponse
        self.ipcServerMessageIPCDesktopSessionResponse = ipcServerMessageIPCDesktopSessionResponse
        self.ipcServerMessageIPCEvent = ipcServerMessageIPCEvent
        self.ipcServerMessageIPCFailure = ipcServerMessageIPCFailure
        self.ipcServerMessageIPCHandshakeResponse = ipcServerMessageIPCHandshakeResponse
        self.ipcServerMessageIPCQueryResponse = ipcServerMessageIPCQueryResponse
        self.ipcServerMessageIPCShutdownResponse = ipcServerMessageIPCShutdownResponse
        self.ipcServerMessageIPCSubscribeResponse = ipcServerMessageIPCSubscribeResponse
        self.ipcServerMessageIPCSubscriptionClosed = ipcServerMessageIPCSubscriptionClosed
        self.ipcServerMessageIPCUnsubscribeResponse = ipcServerMessageIPCUnsubscribeResponse
        self.priceTargetFurniture = priceTargetFurniture
        self.priceTargetProduct = priceTargetProduct
        self.queryAuthorizationRelationshipsList = queryAuthorizationRelationshipsList
        self.queryCatalogImageGet = queryCatalogImageGet
        self.queryConfigGet = queryConfigGet
        self.queryCustomerGet = queryCustomerGet
        self.queryCustomerSearch = queryCustomerSearch
        self.queryDesktopAccountList = queryDesktopAccountList
        self.queryFurnitureCategoryList = queryFurnitureCategoryList
        self.queryFurnitureList = queryFurnitureList
        self.queryFurnitureReview = queryFurnitureReview
        self.queryFurnitureRevisionGet = queryFurnitureRevisionGet
        self.queryFurnitureSelectionCheck = queryFurnitureSelectionCheck
        self.queryMaterialList = queryMaterialList
        self.queryMaterialReferenceList = queryMaterialReferenceList
        self.queryPartCategoryList = queryPartCategoryList
        self.queryPartCompositionGet = queryPartCompositionGet
        self.queryPartCost = queryPartCost
        self.queryPartList = queryPartList
        self.queryPermissionsGetEffective = queryPermissionsGetEffective
        self.queryPricingDiscount = queryPricingDiscount
        self.queryPricingList = queryPricingList
        self.queryPricingReview = queryPricingReview
        self.queryPricingSelection = queryPricingSelection
        self.queryProductCategoryList = queryProductCategoryList
        self.queryProductList = queryProductList
        self.queryProductRevisionGet = queryProductRevisionGet
        self.queryOutcomeFailed = queryOutcomeFailed
        self.queryOutcomeSucceeded = queryOutcomeSucceeded
        self.queryResultCatalogImage = queryResultCatalogImage
        self.queryResultConfiguration = queryResultConfiguration
        self.queryResultCustomer = queryResultCustomer
        self.queryResultCustomers = queryResultCustomers
        self.queryResultDesktopAccounts = queryResultDesktopAccounts
        self.queryResultDiscountTotal = queryResultDiscountTotal
        self.queryResultEffectivePermissions = queryResultEffectivePermissions
        self.queryResultFurnitureCategories = queryResultFurnitureCategories
        self.queryResultFurnitureReview = queryResultFurnitureReview
        self.queryResultFurnitureRevision = queryResultFurnitureRevision
        self.queryResultFurnitures = queryResultFurnitures
        self.queryResultFurnitureSelection = queryResultFurnitureSelection
        self.queryResultMaterialReferences = queryResultMaterialReferences
        self.queryResultMaterials = queryResultMaterials
        self.queryResultPartCategories = queryResultPartCategories
        self.queryResultPartComposition = queryResultPartComposition
        self.queryResultPartCost = queryResultPartCost
        self.queryResultParts = queryResultParts
        self.queryResultPriceReview = queryResultPriceReview
        self.queryResultPrices = queryResultPrices
        self.queryResultProductCategories = queryResultProductCategories
        self.queryResultProductRevision = queryResultProductRevision
        self.queryResultProducts = queryResultProducts
        self.queryResultScopeRelationships = queryResultScopeRelationships
        self.queryResultSellingPrice = queryResultSellingPrice
        self.subscriptionAuthorizationPolicyChangedSubscribe = subscriptionAuthorizationPolicyChangedSubscribe
        self.subscriptionConfigChangedSubscribe = subscriptionConfigChangedSubscribe
        self.subscriptionCustomerChangedSubscribe = subscriptionCustomerChangedSubscribe
        self.subscriptionFurnitureChangedSubscribe = subscriptionFurnitureChangedSubscribe
        self.subscriptionMaterialChangedSubscribe = subscriptionMaterialChangedSubscribe
        self.subscriptionPartChangedSubscribe = subscriptionPartChangedSubscribe
        self.subscriptionPermissionsChangedSubscribe = subscriptionPermissionsChangedSubscribe
        self.subscriptionPricingChangedSubscribe = subscriptionPricingChangedSubscribe
        self.subscriptionProductChangedSubscribe = subscriptionProductChangedSubscribe
        self.subscriptionOutcomeFailed = subscriptionOutcomeFailed
        self.subscriptionOutcomeSucceeded = subscriptionOutcomeSucceeded
    }
}

// MARK: - GrantScopeRelationship
public struct GrantScopeRelationship: Codable, Sendable {
    public let expectedPolicyVersion: Int
    public let relation: String
    public let subject: RelationshipSubject

    public init(expectedPolicyVersion: Int, relation: String, subject: RelationshipSubject) {
        self.expectedPolicyVersion = expectedPolicyVersion
        self.relation = relation
        self.subject = subject
    }
}

// MARK: - RelationshipSubject
public struct RelationshipSubject: Codable, Sendable {
    public let principalID: String
    public let principalKind: PrincipalKind

    public enum CodingKeys: String, CodingKey {
        case principalID = "principalId"
        case principalKind
    }

    public init(principalID: String, principalKind: PrincipalKind) {
        self.principalID = principalID
        self.principalKind = principalKind
    }
}

public enum PrincipalKind: String, Codable, Sendable {
    case device = "device"
    case service = "service"
    case user = "user"
}

// MARK: - RevokeScopeRelationship
public struct RevokeScopeRelationship: Codable, Sendable {
    public let expectedPolicyVersion: Int
    public let relationshipID: String

    public enum CodingKeys: String, CodingKey {
        case expectedPolicyVersion
        case relationshipID = "relationshipId"
    }

    public init(expectedPolicyVersion: Int, relationshipID: String) {
        self.expectedPolicyVersion = expectedPolicyVersion
        self.relationshipID = relationshipID
    }
}

// MARK: - ImportCatalogImage
public struct ImportCatalogImage: Codable, Sendable {
    public let kind: CatalogImageKind
    public let sourcePath: String

    public init(kind: CatalogImageKind, sourcePath: String) {
        self.kind = kind
        self.sourcePath = sourcePath
    }
}

public enum CatalogImageKind: String, Codable, Sendable {
    case furniture = "furniture"
    case product = "product"
}

// MARK: - UpdateConfiguration
public struct UpdateConfiguration: Codable, Sendable {
    public let changes: [ConfigChange]
    public let expectedRevision: Int

    public init(changes: [ConfigChange], expectedRevision: Int) {
        self.changes = changes
        self.expectedRevision = expectedRevision
    }
}

// MARK: - ConfigChange
public struct ConfigChange: Codable, Sendable {
    public let key: String
    public let value: ConfigWriteValue

    public init(key: String, value: ConfigWriteValue) {
        self.key = key
        self.value = value
    }
}

// MARK: - ConfigWriteValue
public struct ConfigWriteValue: Codable, Sendable {
    public let kind: ConfigWriteValueKind
    public let value: ConfigReadValueValue

    public init(kind: ConfigWriteValueKind, value: ConfigReadValueValue) {
        self.kind = kind
        self.value = value
    }
}

public enum ConfigWriteValueKind: String, Codable, Sendable {
    case boolean = "boolean"
    case decimal = "decimal"
    case integer = "integer"
    case secretReference = "secretReference"
    case text = "text"
    case textList = "textList"
}

public enum ConfigReadValueValue: Codable, Sendable {
    case bool(Bool)
    case integer(Int)
    case string(String)
    case stringArray([String])

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let x = try? container.decode(Bool.self) {
            self = .bool(x)
            return
        }
        if let x = try? container.decode(Int.self) {
            self = .integer(x)
            return
        }
        if let x = try? container.decode([String].self) {
            self = .stringArray(x)
            return
        }
        if let x = try? container.decode(String.self) {
            self = .string(x)
            return
        }
        throw DecodingError.typeMismatch(ConfigReadValueValue.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Wrong type for ConfigReadValueValue"))
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .bool(let x):
            try container.encode(x)
        case .integer(let x):
            try container.encode(x)
        case .string(let x):
            try container.encode(x)
        case .stringArray(let x):
            try container.encode(x)
        }
    }
}

// MARK: - CreateCustomer
public struct CreateCustomer: Codable, Sendable {
    public let address: String?
    public let name: String
    public let notes: String?
    public let phone: String

    public init(address: String?, name: String, notes: String?, phone: String) {
        self.address = address
        self.name = name
        self.notes = notes
        self.phone = phone
    }
}

// MARK: - UpdateCustomer
public struct UpdateCustomer: Codable, Sendable {
    public let address: String?
    public let customerID: String
    public let expectedRevision: Int
    public let name: String
    public let notes: String?
    public let phone: String

    public enum CodingKeys: String, CodingKey {
        case address
        case customerID = "customerId"
        case expectedRevision, name, notes, phone
    }

    public init(address: String?, customerID: String, expectedRevision: Int, name: String, notes: String?, phone: String) {
        self.address = address
        self.customerID = customerID
        self.expectedRevision = expectedRevision
        self.name = name
        self.notes = notes
        self.phone = phone
    }
}

// MARK: - CreateDesktopAccount
public struct CreateDesktopAccount: Codable, Sendable {
    public let displayName, password: String
    public let role: DesktopAccountRole
    public let username: String

    public init(displayName: String, password: String, role: DesktopAccountRole, username: String) {
        self.displayName = displayName
        self.password = password
        self.role = role
        self.username = username
    }
}

public enum DesktopAccountRole: String, Codable, Sendable {
    case manager = "manager"
    case receptionist = "receptionist"
}

// MARK: - DeactivateDesktopAccount
public struct DeactivateDesktopAccount: Codable, Sendable {
    public let accountID: String
    public let expectedRevision: Int

    public enum CodingKeys: String, CodingKey {
        case accountID = "accountId"
        case expectedRevision
    }

    public init(accountID: String, expectedRevision: Int) {
        self.accountID = accountID
        self.expectedRevision = expectedRevision
    }
}

// MARK: - UpdateDesktopAccount
public struct UpdateDesktopAccount: Codable, Sendable {
    public let accountID, displayName: String
    public let expectedRevision: Int
    public let role: DesktopAccountRole

    public enum CodingKeys: String, CodingKey {
        case accountID = "accountId"
        case displayName, expectedRevision, role
    }

    public init(accountID: String, displayName: String, expectedRevision: Int, role: DesktopAccountRole) {
        self.accountID = accountID
        self.displayName = displayName
        self.expectedRevision = expectedRevision
        self.role = role
    }
}

// MARK: - SaveFurnitureCategory
public struct SaveFurnitureCategory: Codable, Sendable {
    public let archived: Bool
    public let expectedRevision: Int?
    public let id: String?
    public let name: String

    public init(archived: Bool, expectedRevision: Int?, id: String?, name: String) {
        self.archived = archived
        self.expectedRevision = expectedRevision
        self.id = id
        self.name = name
    }
}

// MARK: - SaveFurniture
public struct SaveFurniture: Codable, Sendable {
    public let categoryID: String
    public let colors: [FurnitureOption]
    public let confirmBelowCost: Bool
    public let description: String
    public let expectedRevision: Int?
    public let handles: [FurnitureOption]
    public let id: String?
    public let image: CatalogImageRef?
    public let name, notes: String
    public let parts: [FurniturePart]
    public let state: FurnitureState
    public let variants: [FurnitureVariant]

    public enum CodingKeys: String, CodingKey {
        case categoryID = "categoryId"
        case colors, confirmBelowCost, description, expectedRevision, handles, id, image, name, notes, parts, state, variants
    }

    public init(categoryID: String, colors: [FurnitureOption], confirmBelowCost: Bool, description: String, expectedRevision: Int?, handles: [FurnitureOption], id: String?, image: CatalogImageRef?, name: String, notes: String, parts: [FurniturePart], state: FurnitureState, variants: [FurnitureVariant]) {
        self.categoryID = categoryID
        self.colors = colors
        self.confirmBelowCost = confirmBelowCost
        self.description = description
        self.expectedRevision = expectedRevision
        self.handles = handles
        self.id = id
        self.image = image
        self.name = name
        self.notes = notes
        self.parts = parts
        self.state = state
        self.variants = variants
    }
}

// MARK: - FurnitureOption
public struct FurnitureOption: Codable, Sendable {
    public let archived: Bool
    public let id, name: String
    public let priceAdjustmentYer: Int
    public let visual: String

    public init(archived: Bool, id: String, name: String, priceAdjustmentYer: Int, visual: String) {
        self.archived = archived
        self.id = id
        self.name = name
        self.priceAdjustmentYer = priceAdjustmentYer
        self.visual = visual
    }
}

// MARK: - CatalogImageRef
public struct CatalogImageRef: Codable, Sendable {
    public let id: String
    public let kind: CatalogImageKind
    public let sha256: String

    public init(id: String, kind: CatalogImageKind, sha256: String) {
        self.id = id
        self.kind = kind
        self.sha256 = sha256
    }
}

// MARK: - FurniturePart
public struct FurniturePart: Codable, Sendable {
    public let quantity: Int
    public let reference: CompositionReference

    public init(quantity: Int, reference: CompositionReference) {
        self.quantity = quantity
        self.reference = reference
    }
}

// MARK: - CompositionReference
public struct CompositionReference: Codable, Sendable {
    public let partID: String
    public let revision, schemaVersion: Int
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case partID = "partId"
        case revision, schemaVersion, scope
    }

    public init(partID: String, revision: Int, schemaVersion: Int, scope: ScopeRef) {
        self.partID = partID
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
    }
}

// MARK: - ScopeRef
public struct ScopeRef: Codable, Sendable {
    public let id, kind: String

    public init(id: String, kind: String) {
        self.id = id
        self.kind = kind
    }
}

public enum FurnitureState: String, Codable, Sendable {
    case active = "active"
    case archived = "archived"
    case draft = "draft"
}

// MARK: - FurnitureVariant
public struct FurnitureVariant: Codable, Sendable {
    public let archived: Bool
    /// Empty means all options of the corresponding kind are compatible.
    public let colorIDS: [String]
    public let customization: FurnitureCustomization?
    public let dimensions: FurnitureDimensions
    public let handleIDS: [String]
    public let id, name: String
    public let sellingPriceYer: Int

    public enum CodingKeys: String, CodingKey {
        case archived
        case colorIDS = "colorIds"
        case customization, dimensions
        case handleIDS = "handleIds"
        case id, name, sellingPriceYer
    }

    public init(archived: Bool, colorIDS: [String], customization: FurnitureCustomization?, dimensions: FurnitureDimensions, handleIDS: [String], id: String, name: String, sellingPriceYer: Int) {
        self.archived = archived
        self.colorIDS = colorIDS
        self.customization = customization
        self.dimensions = dimensions
        self.handleIDS = handleIDS
        self.id = id
        self.name = name
        self.sellingPriceYer = sellingPriceYer
    }
}

// MARK: - FurnitureCustomization
public struct FurnitureCustomization: Codable, Sendable {
    public let maximum, minimum: FurnitureDimensions

    public init(maximum: FurnitureDimensions, minimum: FurnitureDimensions) {
        self.maximum = maximum
        self.minimum = minimum
    }
}

// MARK: - FurnitureDimensions
public struct FurnitureDimensions: Codable, Sendable {
    public let depthMm, heightMm, widthMm: Int

    public init(depthMm: Int, heightMm: Int, widthMm: Int) {
        self.depthMm = depthMm
        self.heightMm = heightMm
        self.widthMm = widthMm
    }
}

// MARK: - SaveMaterialCategory
public struct SaveMaterialCategory: Codable, Sendable {
    public let archived: Bool
    public let expectedRevision: Int?
    public let id: String?
    public let name: String

    public init(archived: Bool, expectedRevision: Int?, id: String?, name: String) {
        self.archived = archived
        self.expectedRevision = expectedRevision
        self.id = id
        self.name = name
    }
}

// MARK: - SaveMaterial
public struct SaveMaterial: Codable, Sendable {
    public let archived: Bool
    public let categoryID: String
    public let currentCostYer: Int
    public let expectedRevision: Int?
    public let id: String?
    public let name, unitID: String

    public enum CodingKeys: String, CodingKey {
        case archived
        case categoryID = "categoryId"
        case currentCostYer, expectedRevision, id, name
        case unitID = "unitId"
    }

    public init(archived: Bool, categoryID: String, currentCostYer: Int, expectedRevision: Int?, id: String?, name: String, unitID: String) {
        self.archived = archived
        self.categoryID = categoryID
        self.currentCostYer = currentCostYer
        self.expectedRevision = expectedRevision
        self.id = id
        self.name = name
        self.unitID = unitID
    }
}

// MARK: - SaveMaterialUnit
public struct SaveMaterialUnit: Codable, Sendable {
    public let archived: Bool
    public let denominator: Int
    public let dimension: UnitDimension
    public let expectedRevision: Int?
    public let id: String?
    public let name: String
    public let numerator: Int
    public let symbol: String

    public init(archived: Bool, denominator: Int, dimension: UnitDimension, expectedRevision: Int?, id: String?, name: String, numerator: Int, symbol: String) {
        self.archived = archived
        self.denominator = denominator
        self.dimension = dimension
        self.expectedRevision = expectedRevision
        self.id = id
        self.name = name
        self.numerator = numerator
        self.symbol = symbol
    }
}

public enum UnitDimension: String, Codable, Sendable {
    case area = "area"
    case count = "count"
    case length = "length"
    case mass = "mass"
    case volume = "volume"
}

// MARK: - SavePartCategory
public struct SavePartCategory: Codable, Sendable {
    public let archived: Bool
    public let expectedRevision: Int?
    public let id: String?
    public let name: String

    public init(archived: Bool, expectedRevision: Int?, id: String?, name: String) {
        self.archived = archived
        self.expectedRevision = expectedRevision
        self.id = id
        self.name = name
    }
}

// MARK: - SavePart
public struct SavePart: Codable, Sendable {
    public let archived: Bool
    public let categoryID, description: String
    public let expectedRevision: Int?
    public let id: String?
    public let name: String
    public let usages: [PartUsage]

    public enum CodingKeys: String, CodingKey {
        case archived
        case categoryID = "categoryId"
        case description, expectedRevision, id, name, usages
    }

    public init(archived: Bool, categoryID: String, description: String, expectedRevision: Int?, id: String?, name: String, usages: [PartUsage]) {
        self.archived = archived
        self.categoryID = categoryID
        self.description = description
        self.expectedRevision = expectedRevision
        self.id = id
        self.name = name
        self.usages = usages
    }
}

// MARK: - PartUsage
public struct PartUsage: Codable, Sendable {
    public let materialID: String
    public let materialRevision: Int
    public let quantity, unitID: String
    public let unitRevision: Int

    public enum CodingKeys: String, CodingKey {
        case materialID = "materialId"
        case materialRevision, quantity
        case unitID = "unitId"
        case unitRevision
    }

    public init(materialID: String, materialRevision: Int, quantity: String, unitID: String, unitRevision: Int) {
        self.materialID = materialID
        self.materialRevision = materialRevision
        self.quantity = quantity
        self.unitID = unitID
        self.unitRevision = unitRevision
    }
}

// MARK: - PublishPrice
public struct PublishPrice: Codable, Sendable {
    public let confirmBelowCost: Bool
    public let expectedRevision: Int?
    public let sellingPriceYer: Int
    public let target: [String: JSONAny]

    public init(confirmBelowCost: Bool, expectedRevision: Int?, sellingPriceYer: Int, target: [String: JSONAny]) {
        self.confirmBelowCost = confirmBelowCost
        self.expectedRevision = expectedRevision
        self.sellingPriceYer = sellingPriceYer
        self.target = target
    }
}

// MARK: - SaveProductCategory
public struct SaveProductCategory: Codable, Sendable {
    public let archived: Bool
    public let expectedRevision: Int?
    public let id: String?
    public let name: String

    public init(archived: Bool, expectedRevision: Int?, id: String?, name: String) {
        self.archived = archived
        self.expectedRevision = expectedRevision
        self.id = id
        self.name = name
    }
}

// MARK: - SaveProduct
public struct SaveProduct: Codable, Sendable {
    public let archived: Bool
    public let categoryID, description: String
    public let expectedRevision: Int?
    public let id: String?
    public let image: CatalogImageRef?
    public let name, notes: String
    public let variants: [SaveProductVariant]

    public enum CodingKeys: String, CodingKey {
        case archived
        case categoryID = "categoryId"
        case description, expectedRevision, id, image, name, notes, variants
    }

    public init(archived: Bool, categoryID: String, description: String, expectedRevision: Int?, id: String?, image: CatalogImageRef?, name: String, notes: String, variants: [SaveProductVariant]) {
        self.archived = archived
        self.categoryID = categoryID
        self.description = description
        self.expectedRevision = expectedRevision
        self.id = id
        self.image = image
        self.name = name
        self.notes = notes
        self.variants = variants
    }
}

// MARK: - SaveProductVariant
public struct SaveProductVariant: Codable, Sendable {
    public let archived: Bool
    public let id, name: String
    public let purchaseCostYer: Int

    public init(archived: Bool, id: String, name: String, purchaseCostYer: Int) {
        self.archived = archived
        self.id = id
        self.name = name
        self.purchaseCostYer = purchaseCostYer
    }
}

// MARK: - ConfigSnapshot
public struct ConfigSnapshot: Codable, Sendable {
    public let entries: [ConfigEntry]
    public let revision, schemaVersion: Int
    public let scope: ScopeRef

    public init(entries: [ConfigEntry], revision: Int, schemaVersion: Int, scope: ScopeRef) {
        self.entries = entries
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
    }
}

// MARK: - ConfigEntry
public struct ConfigEntry: Codable, Sendable {
    public let key: String
    public let restartRequirement: RestartRequirement
    public let sensitivity: ConfigSensitivity
    public let value: ConfigReadValue

    public init(key: String, restartRequirement: RestartRequirement, sensitivity: ConfigSensitivity, value: ConfigReadValue) {
        self.key = key
        self.restartRequirement = restartRequirement
        self.sensitivity = sensitivity
        self.value = value
    }
}

public enum RestartRequirement: String, Codable, Sendable {
    case application = "application"
    case engine = "engine"
    case none = "none"
}

public enum ConfigSensitivity: String, Codable, Sendable {
    case configSensitivityPublic = "public"
    case secret = "secret"
    case sensitive = "sensitive"
}

// MARK: - ConfigReadValue
public struct ConfigReadValue: Codable, Sendable {
    public let kind: ConfigReadValueKind
    public let value: ConfigReadValueValue?

    public init(kind: ConfigReadValueKind, value: ConfigReadValueValue?) {
        self.kind = kind
        self.value = value
    }
}

public enum ConfigReadValueKind: String, Codable, Sendable {
    case boolean = "boolean"
    case decimal = "decimal"
    case integer = "integer"
    case redacted = "redacted"
    case secretReference = "secretReference"
    case text = "text"
    case textList = "textList"
}

// MARK: - CustomerMutationResult
public struct CustomerMutationResult: Codable, Sendable {
    public let customer: Customer
    public let potentialDuplicateIDS: [String]

    public enum CodingKeys: String, CodingKey {
        case customer
        case potentialDuplicateIDS = "potentialDuplicateIds"
    }

    public init(customer: Customer, potentialDuplicateIDS: [String]) {
        self.customer = customer
        self.potentialDuplicateIDS = potentialDuplicateIDS
    }
}

// MARK: - Customer
public struct Customer: Codable, Sendable {
    public let address: String?
    public let id, name: String
    public let notes: String?
    public let phone: String
    public let revision: Int
    public let scope: ScopeRef
    public let status: CustomerStatus
    public let syncState: CustomerSyncState
    public let updatedAt: Int

    public init(address: String?, id: String, name: String, notes: String?, phone: String, revision: Int, scope: ScopeRef, status: CustomerStatus, syncState: CustomerSyncState, updatedAt: Int) {
        self.address = address
        self.id = id
        self.name = name
        self.notes = notes
        self.phone = phone
        self.revision = revision
        self.scope = scope
        self.status = status
        self.syncState = syncState
        self.updatedAt = updatedAt
    }
}

public enum CustomerStatus: String, Codable, Sendable {
    case active = "active"
    case archived = "archived"
    case merged = "merged"
}

public enum CustomerSyncState: String, Codable, Sendable {
    case confirmed = "confirmed"
    case conflicted = "conflicted"
    case pending = "pending"
    case rejected = "rejected"
}

// MARK: - DesktopAccountSummary
public struct DesktopAccountSummary: Codable, Sendable {
    public let accountID: String
    public let active: Bool
    public let displayName: String
    public let revision: Int
    public let role: DesktopAccountRole
    public let userID, username: String

    public enum CodingKeys: String, CodingKey {
        case accountID = "accountId"
        case active, displayName, revision, role
        case userID = "userId"
        case username
    }

    public init(accountID: String, active: Bool, displayName: String, revision: Int, role: DesktopAccountRole, userID: String, username: String) {
        self.accountID = accountID
        self.active = active
        self.displayName = displayName
        self.revision = revision
        self.role = role
        self.userID = userID
        self.username = username
    }
}

// MARK: - FurnitureCategory
public struct FurnitureCategory: Codable, Sendable {
    public let archived: Bool
    public let id, name: String
    public let revision: Int
    public let scope: ScopeRef
    public let updatedAt: Int

    public init(archived: Bool, id: String, name: String, revision: Int, scope: ScopeRef, updatedAt: Int) {
        self.archived = archived
        self.id = id
        self.name = name
        self.revision = revision
        self.scope = scope
        self.updatedAt = updatedAt
    }
}

// MARK: - Furniture
public struct Furniture: Codable, Sendable {
    public let categoryID: String
    /// Category name at this revision, retained for historical reads.
    public let categoryName: String
    public let colors: [FurnitureOption]
    public let description: String
    public let handles: [FurnitureOption]
    public let id: String
    public let image: CatalogImageRef?
    public let name: String
    /// Internal notes are only exposed to Managers.
    public let notes: String
    public let parts: [FurniturePart]
    public let partsCostYer: Int
    public let revision: Int
    public let scope: ScopeRef
    public let state: FurnitureState
    public let updatedAt: Int
    public let variants: [FurnitureVariant]

    public enum CodingKeys: String, CodingKey {
        case categoryID = "categoryId"
        case categoryName, colors, description, handles, id, image, name, notes, parts, partsCostYer, revision, scope, state, updatedAt, variants
    }

    public init(categoryID: String, categoryName: String, colors: [FurnitureOption], description: String, handles: [FurnitureOption], id: String, image: CatalogImageRef?, name: String, notes: String, parts: [FurniturePart], partsCostYer: Int, revision: Int, scope: ScopeRef, state: FurnitureState, updatedAt: Int, variants: [FurnitureVariant]) {
        self.categoryID = categoryID
        self.categoryName = categoryName
        self.colors = colors
        self.description = description
        self.handles = handles
        self.id = id
        self.image = image
        self.name = name
        self.notes = notes
        self.parts = parts
        self.partsCostYer = partsCostYer
        self.revision = revision
        self.scope = scope
        self.state = state
        self.updatedAt = updatedAt
        self.variants = variants
    }
}

// MARK: - MaterialCategory
public struct MaterialCategory: Codable, Sendable {
    public let archived: Bool
    public let id, name: String
    public let revision: Int
    public let scope: ScopeRef
    public let updatedAt: Int

    public init(archived: Bool, id: String, name: String, revision: Int, scope: ScopeRef, updatedAt: Int) {
        self.archived = archived
        self.id = id
        self.name = name
        self.revision = revision
        self.scope = scope
        self.updatedAt = updatedAt
    }
}

// MARK: - Material
public struct Material: Codable, Sendable {
    public let archived: Bool
    public let categoryID: String
    /// Non-negative whole Yemeni rials per selected unit.
    public let currentCostYer: Int
    public let id, name: String
    public let revision: Int
    public let scope: ScopeRef
    public let unitID: String
    public let updatedAt: Int

    public enum CodingKeys: String, CodingKey {
        case archived
        case categoryID = "categoryId"
        case currentCostYer, id, name, revision, scope
        case unitID = "unitId"
        case updatedAt
    }

    public init(archived: Bool, categoryID: String, currentCostYer: Int, id: String, name: String, revision: Int, scope: ScopeRef, unitID: String, updatedAt: Int) {
        self.archived = archived
        self.categoryID = categoryID
        self.currentCostYer = currentCostYer
        self.id = id
        self.name = name
        self.revision = revision
        self.scope = scope
        self.unitID = unitID
        self.updatedAt = updatedAt
    }
}

// MARK: - MaterialUnit
public struct MaterialUnit: Codable, Sendable {
    public let archived: Bool
    public let denominator: Int
    public let dimension: UnitDimension
    public let id, name: String
    public let numerator, revision: Int
    public let scope: ScopeRef
    public let symbol: String
    public let updatedAt: Int

    public init(archived: Bool, denominator: Int, dimension: UnitDimension, id: String, name: String, numerator: Int, revision: Int, scope: ScopeRef, symbol: String, updatedAt: Int) {
        self.archived = archived
        self.denominator = denominator
        self.dimension = dimension
        self.id = id
        self.name = name
        self.numerator = numerator
        self.revision = revision
        self.scope = scope
        self.symbol = symbol
        self.updatedAt = updatedAt
    }
}

// MARK: - PartCategory
public struct PartCategory: Codable, Sendable {
    public let archived: Bool
    public let id, name: String
    public let revision: Int
    public let scope: ScopeRef
    public let updatedAt: Int

    public init(archived: Bool, id: String, name: String, revision: Int, scope: ScopeRef, updatedAt: Int) {
        self.archived = archived
        self.id = id
        self.name = name
        self.revision = revision
        self.scope = scope
        self.updatedAt = updatedAt
    }
}

// MARK: - Part
public struct Part: Codable, Sendable {
    public let archived: Bool
    public let categoryID: String
    public let composition: CompositionReference
    public let cost: PartCost
    public let description, id, name: String
    public let revision: Int
    public let scope: ScopeRef
    public let updatedAt: Int

    public enum CodingKeys: String, CodingKey {
        case archived
        case categoryID = "categoryId"
        case composition, cost, description, id, name, revision, scope, updatedAt
    }

    public init(archived: Bool, categoryID: String, composition: CompositionReference, cost: PartCost, description: String, id: String, name: String, revision: Int, scope: ScopeRef, updatedAt: Int) {
        self.archived = archived
        self.categoryID = categoryID
        self.composition = composition
        self.cost = cost
        self.description = description
        self.id = id
        self.name = name
        self.revision = revision
        self.scope = scope
        self.updatedAt = updatedAt
    }
}

/// Advisory current cost; commercial references resolve the immutable part snapshot instead.
// MARK: - PartCost
public struct PartCost: Codable, Sendable {
    public let rows: [CostedUsage]
    public let totalCostYer: Int

    public init(rows: [CostedUsage], totalCostYer: Int) {
        self.rows = rows
        self.totalCostYer = totalCostYer
    }
}

// MARK: - CostedUsage
public struct CostedUsage: Codable, Sendable {
    public let costUnit: MaterialUnit
    public let costYer: Int
    public let material: Material
    public let unit: MaterialUnit
    public let usage: PartUsage

    public init(costUnit: MaterialUnit, costYer: Int, material: Material, unit: MaterialUnit, usage: PartUsage) {
        self.costUnit = costUnit
        self.costYer = costYer
        self.material = material
        self.unit = unit
        self.usage = usage
    }
}

/// Immutable public snapshot. Contains no purchase cost, margin, BOM, or notes.
// MARK: - PublishedPrice
public struct PublishedPrice: Codable, Sendable {
    public let colors: [PriceAdjustment]
    public let confirmedAt: Int
    public let currency: String
    public let handles: [PriceAdjustment]
    public let revision: Int
    public let sellingPriceYer: Int
    public let target: [String: JSONAny]

    public init(colors: [PriceAdjustment], confirmedAt: Int, currency: String, handles: [PriceAdjustment], revision: Int, sellingPriceYer: Int, target: [String: JSONAny]) {
        self.colors = colors
        self.confirmedAt = confirmedAt
        self.currency = currency
        self.handles = handles
        self.revision = revision
        self.sellingPriceYer = sellingPriceYer
        self.target = target
    }
}

// MARK: - PriceAdjustment
public struct PriceAdjustment: Codable, Sendable {
    public let id: String
    public let priceAdjustmentYer: Int

    public init(id: String, priceAdjustmentYer: Int) {
        self.id = id
        self.priceAdjustmentYer = priceAdjustmentYer
    }
}

// MARK: - ProductCategory
public struct ProductCategory: Codable, Sendable {
    public let archived: Bool
    public let id, name: String
    public let revision: Int
    public let scope: ScopeRef
    public let updatedAt: Int

    public init(archived: Bool, id: String, name: String, revision: Int, scope: ScopeRef, updatedAt: Int) {
        self.archived = archived
        self.id = id
        self.name = name
        self.revision = revision
        self.scope = scope
        self.updatedAt = updatedAt
    }
}

// MARK: - Product
public struct Product: Codable, Sendable {
    public let archived: Bool
    public let categoryID: String
    /// Category name at this revision, retained for historical reads.
    public let categoryName: String
    public let description, id: String
    public let image: CatalogImageRef?
    public let name: String
    /// Internal notes are withheld with purchase costs.
    public let notes: String?
    public let revision: Int
    public let scope: ScopeRef
    public let updatedAt: Int
    public let variants: [ProductVariant]

    public enum CodingKeys: String, CodingKey {
        case archived
        case categoryID = "categoryId"
        case categoryName, description, id, image, name, notes, revision, scope, updatedAt, variants
    }

    public init(archived: Bool, categoryID: String, categoryName: String, description: String, id: String, image: CatalogImageRef?, name: String, notes: String?, revision: Int, scope: ScopeRef, updatedAt: Int, variants: [ProductVariant]) {
        self.archived = archived
        self.categoryID = categoryID
        self.categoryName = categoryName
        self.description = description
        self.id = id
        self.image = image
        self.name = name
        self.notes = notes
        self.revision = revision
        self.scope = scope
        self.updatedAt = updatedAt
        self.variants = variants
    }
}

// MARK: - ProductVariant
public struct ProductVariant: Codable, Sendable {
    public let archived: Bool
    public let id, name: String
    /// Omitted entirely when the caller cannot read internal purchase costs.
    public let purchaseCostYer: Int?

    public init(archived: Bool, id: String, name: String, purchaseCostYer: Int?) {
        self.archived = archived
        self.id = id
        self.name = name
        self.purchaseCostYer = purchaseCostYer
    }
}

// MARK: - RelationshipMutationResult
public struct RelationshipMutationResult: Codable, Sendable {
    public let changed: Bool
    public let policyVersion: Int
    public let relationship: ScopeRelationship

    public init(changed: Bool, policyVersion: Int, relationship: ScopeRelationship) {
        self.changed = changed
        self.policyVersion = policyVersion
        self.relationship = relationship
    }
}

// MARK: - ScopeRelationship
public struct ScopeRelationship: Codable, Sendable {
    public let relation, relationshipID: String
    public let scope: ScopeRef
    public let subject: RelationshipSubject

    public enum CodingKeys: String, CodingKey {
        case relation
        case relationshipID = "relationshipId"
        case scope, subject
    }

    public init(relation: String, relationshipID: String, scope: ScopeRef, subject: RelationshipSubject) {
        self.relation = relation
        self.relationshipID = relationshipID
        self.scope = scope
        self.subject = subject
    }
}

// MARK: - AuthorizationPolicyChangeNotice
public struct AuthorizationPolicyChangeNotice: Codable, Sendable {
    public let policyVersion: Int
    public let scope: ScopeRef

    public init(policyVersion: Int, scope: ScopeRef) {
        self.policyVersion = policyVersion
        self.scope = scope
    }
}

// MARK: - CustomerChangeNotice
public struct CustomerChangeNotice: Codable, Sendable {
    public let changedAt: Int
    public let changeID, customerID: String
    public let revision: Int
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case changedAt
        case changeID = "changeId"
        case customerID = "customerId"
        case revision, scope
    }

    public init(changedAt: Int, changeID: String, customerID: String, revision: Int, scope: ScopeRef) {
        self.changedAt = changedAt
        self.changeID = changeID
        self.customerID = customerID
        self.revision = revision
        self.scope = scope
    }
}

// MARK: - FurnitureChangeNotice
public struct FurnitureChangeNotice: Codable, Sendable {
    public let category: Bool
    public let changedAt: Int
    public let id: String
    public let revision: Int
    public let scope: ScopeRef

    public init(category: Bool, changedAt: Int, id: String, revision: Int, scope: ScopeRef) {
        self.category = category
        self.changedAt = changedAt
        self.id = id
        self.revision = revision
        self.scope = scope
    }
}

// MARK: - MaterialChangeNotice
public struct MaterialChangeNotice: Codable, Sendable {
    public let changedAt: Int
    public let id: String
    public let kind: MaterialRecordKind
    public let revision: Int
    public let scope: ScopeRef

    public init(changedAt: Int, id: String, kind: MaterialRecordKind, revision: Int, scope: ScopeRef) {
        self.changedAt = changedAt
        self.id = id
        self.kind = kind
        self.revision = revision
        self.scope = scope
    }
}

public enum MaterialRecordKind: String, Codable, Sendable {
    case category = "category"
    case material = "material"
    case unit = "unit"
}

// MARK: - PartChangeNotice
public struct PartChangeNotice: Codable, Sendable {
    public let category: Bool
    public let changedAt: Int
    public let id: String
    public let revision: Int
    public let scope: ScopeRef

    public init(category: Bool, changedAt: Int, id: String, revision: Int, scope: ScopeRef) {
        self.category = category
        self.changedAt = changedAt
        self.id = id
        self.revision = revision
        self.scope = scope
    }
}

// MARK: - EffectivePermissions
public struct EffectivePermissions: Codable, Sendable {
    public let permissions: [EffectivePermission]
    public let policyVersion: Int

    public init(permissions: [EffectivePermission], policyVersion: Int) {
        self.permissions = permissions
        self.policyVersion = policyVersion
    }
}

// MARK: - EffectivePermission
public struct EffectivePermission: Codable, Sendable {
    public let decision: PermissionDecision
    public let permission: String

    public init(decision: PermissionDecision, permission: String) {
        self.decision = decision
        self.permission = permission
    }
}

public enum PermissionDecision: String, Codable, Sendable {
    case denied = "denied"
    case granted = "granted"
}

// MARK: - PriceChangeNotice
public struct PriceChangeNotice: Codable, Sendable {
    public let revision: Int
    public let target: [String: JSONAny]

    public init(revision: Int, target: [String: JSONAny]) {
        self.revision = revision
        self.target = target
    }
}

// MARK: - ProductChangeNotice
public struct ProductChangeNotice: Codable, Sendable {
    public let category: Bool
    public let changedAt: Int
    public let id: String
    public let revision: Int
    public let scope: ScopeRef

    public init(category: Bool, changedAt: Int, id: String, revision: Int, scope: ScopeRef) {
        self.category = category
        self.changedAt = changedAt
        self.id = id
        self.revision = revision
        self.scope = scope
    }
}

// MARK: - HandshakeAccepted
public struct HandshakeAccepted: Codable, Sendable {
    public let authorization: AuthorizationContext
    public let engine: PeerHello
    public let negotiated: NegotiatedSession

    public init(authorization: AuthorizationContext, engine: PeerHello, negotiated: NegotiatedSession) {
        self.authorization = authorization
        self.engine = engine
        self.negotiated = negotiated
    }
}

// MARK: - AuthorizationContext
public struct AuthorizationContext: Codable, Sendable {
    public let identity: AuthenticatedIdentity
    public let scope: ScopeRef
    public let sessionID, tenantID: String
    public let workspaceID: String?

    public enum CodingKeys: String, CodingKey {
        case identity, scope
        case sessionID = "sessionId"
        case tenantID = "tenantId"
        case workspaceID = "workspaceId"
    }

    public init(identity: AuthenticatedIdentity, scope: ScopeRef, sessionID: String, tenantID: String, workspaceID: String?) {
        self.identity = identity
        self.scope = scope
        self.sessionID = sessionID
        self.tenantID = tenantID
        self.workspaceID = workspaceID
    }
}

// MARK: - AuthenticatedIdentity
public struct AuthenticatedIdentity: Codable, Sendable {
    public let deviceID: String?
    public let principalID: String
    public let principalKind: PrincipalKind
    public let serviceID: String?

    public enum CodingKeys: String, CodingKey {
        case deviceID = "deviceId"
        case principalID = "principalId"
        case principalKind
        case serviceID = "serviceId"
    }

    public init(deviceID: String?, principalID: String, principalKind: PrincipalKind, serviceID: String?) {
        self.deviceID = deviceID
        self.principalID = principalID
        self.principalKind = principalKind
        self.serviceID = serviceID
    }
}

// MARK: - PeerHello
public struct PeerHello: Codable, Sendable {
    public let capabilities: [String]
    public let peerKind: PeerKind
    public let productVersion: String
    public let protocols: [SupportedProtocol]
    public let requiredCapabilities: [String]
    public let schemas: [SchemaSupport]

    public init(capabilities: [String], peerKind: PeerKind, productVersion: String, protocols: [SupportedProtocol], requiredCapabilities: [String], schemas: [SchemaSupport]) {
        self.capabilities = capabilities
        self.peerKind = peerKind
        self.productVersion = productVersion
        self.protocols = protocols
        self.requiredCapabilities = requiredCapabilities
        self.schemas = schemas
    }
}

public enum PeerKind: String, Codable, Sendable {
    case diagnosticClient = "diagnosticClient"
    case engine = "engine"
    case server = "server"
    case shell = "shell"
}

// MARK: - SupportedProtocol
public struct SupportedProtocol: Codable, Sendable {
    public let major, maximumMinor, minimumMinor: Int

    public init(major: Int, maximumMinor: Int, minimumMinor: Int) {
        self.major = major
        self.maximumMinor = maximumMinor
        self.minimumMinor = minimumMinor
    }
}

// MARK: - SchemaSupport
public struct SchemaSupport: Codable, Sendable {
    public let maximumVersion, minimumVersion: Int
    public let schemaSupportRequired: Bool
    public let schemaID: String

    public enum CodingKeys: String, CodingKey {
        case maximumVersion, minimumVersion
        case schemaSupportRequired = "required"
        case schemaID = "schemaId"
    }

    public init(maximumVersion: Int, minimumVersion: Int, schemaSupportRequired: Bool, schemaID: String) {
        self.maximumVersion = maximumVersion
        self.minimumVersion = minimumVersion
        self.schemaSupportRequired = schemaSupportRequired
        self.schemaID = schemaID
    }
}

// MARK: - NegotiatedSession
public struct NegotiatedSession: Codable, Sendable {
    public let capabilities: [String]
    public let negotiatedSessionProtocol: ProtocolVersion
    public let schemas: [NegotiatedSchema]

    public enum CodingKeys: String, CodingKey {
        case capabilities
        case negotiatedSessionProtocol = "protocol"
        case schemas
    }

    public init(capabilities: [String], negotiatedSessionProtocol: ProtocolVersion, schemas: [NegotiatedSchema]) {
        self.capabilities = capabilities
        self.negotiatedSessionProtocol = negotiatedSessionProtocol
        self.schemas = schemas
    }
}

// MARK: - NegotiatedSchema
public struct NegotiatedSchema: Codable, Sendable {
    public let schemaID: String
    public let version: Int

    public enum CodingKeys: String, CodingKey {
        case schemaID = "schemaId"
        case version
    }

    public init(schemaID: String, version: Int) {
        self.schemaID = schemaID
        self.version = version
    }
}

// MARK: - HandshakeRejection
public struct HandshakeRejection: Codable, Sendable {
    public let kind: HandshakeRejectionKind
    public let payload: NegotiationRejection?

    public init(kind: HandshakeRejectionKind, payload: NegotiationRejection?) {
        self.kind = kind
        self.payload = payload
    }
}

public enum HandshakeRejectionKind: String, Codable, Sendable {
    case authenticationFailed = "authenticationFailed"
    case authenticationRequired = "authenticationRequired"
    case negotiation = "negotiation"
}

// MARK: - NegotiationRejection
public struct NegotiationRejection: Codable, Sendable {
    public let kind: NegotiationRejectionKind
    public let payload: NegotiationRejectionPayload?

    public init(kind: NegotiationRejectionKind, payload: NegotiationRejectionPayload?) {
        self.kind = kind
        self.payload = payload
    }
}

public enum NegotiationRejectionKind: String, Codable, Sendable {
    case incompatibleSchema = "incompatibleSchema"
    case missingCapability = "missingCapability"
    case noCommonProtocol = "noCommonProtocol"
}

// MARK: - NegotiationRejectionPayload
public struct NegotiationRejectionPayload: Codable, Sendable {
    public let capability: String?
    public let requiredBy: RequiredBy?
    public let schemaID: String?

    public enum CodingKeys: String, CodingKey {
        case capability
        case requiredBy = "required_by"
        case schemaID = "schema_id"
    }

    public init(capability: String?, requiredBy: RequiredBy?, schemaID: String?) {
        self.capability = capability
        self.requiredBy = requiredBy
        self.schemaID = schemaID
    }
}

public enum RequiredBy: String, Codable, Sendable {
    case local = "local"
    case remote = "remote"
}

// MARK: - CommandEnvelope
public struct CommandEnvelope: Codable, Sendable {
    public let authorization: AuthorizationContext
    public let causationID: String?
    public let command: [String: JSONAny]
    public let correlationID: String
    public let deadline: Int
    public let idempotencyKey: String
    public let protocolVersion: ProtocolVersion
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case authorization
        case causationID = "causationId"
        case command
        case correlationID = "correlationId"
        case deadline, idempotencyKey, protocolVersion
        case requestID = "requestId"
    }

    public init(authorization: AuthorizationContext, causationID: String?, command: [String: JSONAny], correlationID: String, deadline: Int, idempotencyKey: String, protocolVersion: ProtocolVersion, requestID: String) {
        self.authorization = authorization
        self.causationID = causationID
        self.command = command
        self.correlationID = correlationID
        self.deadline = deadline
        self.idempotencyKey = idempotencyKey
        self.protocolVersion = protocolVersion
        self.requestID = requestID
    }
}

// MARK: - DesktopSessionRequest
public struct DesktopSessionRequest: Codable, Sendable {
    public let correlationID, requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case requestID = "requestId"
    }

    public init(correlationID: String, requestID: String) {
        self.correlationID = correlationID
        self.requestID = requestID
    }
}

/// Password input is transient and must never be logged or persisted by a shell.
// MARK: - DesktopSignInRequest
public struct DesktopSignInRequest: Codable, Sendable {
    public let correlationID, password, requestID, username: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case password
        case requestID = "requestId"
        case username
    }

    public init(correlationID: String, password: String, requestID: String, username: String) {
        self.correlationID = correlationID
        self.password = password
        self.requestID = requestID
        self.username = username
    }
}

// MARK: - HandshakeRequest
public struct HandshakeRequest: Codable, Sendable {
    public let bootstrapToken, correlationID: String
    public let peer: PeerHello
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case bootstrapToken
        case correlationID = "correlationId"
        case peer
        case requestID = "requestId"
    }

    public init(bootstrapToken: String, correlationID: String, peer: PeerHello, requestID: String) {
        self.bootstrapToken = bootstrapToken
        self.correlationID = correlationID
        self.peer = peer
        self.requestID = requestID
    }
}

// MARK: - QueryEnvelope
public struct QueryEnvelope: Codable, Sendable {
    public let authorization: AuthorizationContext
    public let causationID: String?
    public let correlationID: String
    public let deadline: Int
    public let protocolVersion: ProtocolVersion
    public let query: [String: JSONAny]
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case authorization
        case causationID = "causationId"
        case correlationID = "correlationId"
        case deadline, protocolVersion, query
        case requestID = "requestId"
    }

    public init(authorization: AuthorizationContext, causationID: String?, correlationID: String, deadline: Int, protocolVersion: ProtocolVersion, query: [String: JSONAny], requestID: String) {
        self.authorization = authorization
        self.causationID = causationID
        self.correlationID = correlationID
        self.deadline = deadline
        self.protocolVersion = protocolVersion
        self.query = query
        self.requestID = requestID
    }
}

// MARK: - ShutdownRequest
public struct ShutdownRequest: Codable, Sendable {
    public let correlationID, requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case requestID = "requestId"
    }

    public init(correlationID: String, requestID: String) {
        self.correlationID = correlationID
        self.requestID = requestID
    }
}

// MARK: - SubscriptionEnvelope
public struct SubscriptionEnvelope: Codable, Sendable {
    public let authorization: AuthorizationContext
    public let correlationID: String
    public let protocolVersion: ProtocolVersion
    public let requestID: String
    public let resumeAfter: String?
    public let subscription: [String: JSONAny]

    public enum CodingKeys: String, CodingKey {
        case authorization
        case correlationID = "correlationId"
        case protocolVersion
        case requestID = "requestId"
        case resumeAfter, subscription
    }

    public init(authorization: AuthorizationContext, correlationID: String, protocolVersion: ProtocolVersion, requestID: String, resumeAfter: String?, subscription: [String: JSONAny]) {
        self.authorization = authorization
        self.correlationID = correlationID
        self.protocolVersion = protocolVersion
        self.requestID = requestID
        self.resumeAfter = resumeAfter
        self.subscription = subscription
    }
}

// MARK: - UnsubscribeRequest
public struct UnsubscribeRequest: Codable, Sendable {
    public let correlationID, requestID, subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case requestID = "requestId"
        case subscriptionID = "subscriptionId"
    }

    public init(correlationID: String, requestID: String, subscriptionID: String) {
        self.correlationID = correlationID
        self.requestID = requestID
        self.subscriptionID = subscriptionID
    }
}

// MARK: - CommandResponseEnvelope
public struct CommandResponseEnvelope: Codable, Sendable {
    public let correlationID: String
    public let outcome: [String: JSONAny]
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case outcome
        case requestID = "requestId"
    }

    public init(correlationID: String, outcome: [String: JSONAny], requestID: String) {
        self.correlationID = correlationID
        self.outcome = outcome
        self.requestID = requestID
    }
}

// MARK: - DesktopSessionResponse
public struct DesktopSessionResponse: Codable, Sendable {
    public let correlationID: String
    public let error: ContractError?
    public let requestID: String
    public let state: DesktopSessionState?
    public let status: DesktopSessionStatus

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case error
        case requestID = "requestId"
        case state, status
    }

    public init(correlationID: String, error: ContractError?, requestID: String, state: DesktopSessionState?, status: DesktopSessionStatus) {
        self.correlationID = correlationID
        self.error = error
        self.requestID = requestID
        self.state = state
        self.status = status
    }
}

// MARK: - DesktopSessionState
public struct DesktopSessionState: Codable, Sendable {
    public let accountRole: DesktopAccountRole
    public let authorization, customerAuthorization: AuthorizationContext?
    public let expiresAt: Int?

    public init(accountRole: DesktopAccountRole, authorization: AuthorizationContext?, customerAuthorization: AuthorizationContext?, expiresAt: Int?) {
        self.accountRole = accountRole
        self.authorization = authorization
        self.customerAuthorization = customerAuthorization
        self.expiresAt = expiresAt
    }
}

public enum DesktopSessionStatus: String, Codable, Sendable {
    case active = "active"
    case failed = "failed"
    case signedOut = "signedOut"
}

// MARK: - EventEnvelope
public struct EventEnvelope: Codable, Sendable {
    public let correlationID, cursor: String
    public let event: [String: JSONAny]
    public let occurredAt: Int
    public let sequence: Int
    public let subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case cursor, event, occurredAt, sequence
        case subscriptionID = "subscriptionId"
    }

    public init(correlationID: String, cursor: String, event: [String: JSONAny], occurredAt: Int, sequence: Int, subscriptionID: String) {
        self.correlationID = correlationID
        self.cursor = cursor
        self.event = event
        self.occurredAt = occurredAt
        self.sequence = sequence
        self.subscriptionID = subscriptionID
    }
}

// MARK: - IPCFailureResponse
public struct IPCFailureResponse: Codable, Sendable {
    public let error: ContractError
    public let requestID: String?

    public enum CodingKeys: String, CodingKey {
        case error
        case requestID = "requestId"
    }

    public init(error: ContractError, requestID: String?) {
        self.error = error
        self.requestID = requestID
    }
}

// MARK: - HandshakeResponse
public struct HandshakeResponse: Codable, Sendable {
    public let correlationID: String
    public let outcome: [String: JSONAny]
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case outcome
        case requestID = "requestId"
    }

    public init(correlationID: String, outcome: [String: JSONAny], requestID: String) {
        self.correlationID = correlationID
        self.outcome = outcome
        self.requestID = requestID
    }
}

// MARK: - QueryResponseEnvelope
public struct QueryResponseEnvelope: Codable, Sendable {
    public let correlationID: String
    public let outcome: [String: JSONAny]
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case outcome
        case requestID = "requestId"
    }

    public init(correlationID: String, outcome: [String: JSONAny], requestID: String) {
        self.correlationID = correlationID
        self.outcome = outcome
        self.requestID = requestID
    }
}

// MARK: - ShutdownResponse
public struct ShutdownResponse: Codable, Sendable {
    public let accepted: Bool
    public let correlationID, requestID: String

    public enum CodingKeys: String, CodingKey {
        case accepted
        case correlationID = "correlationId"
        case requestID = "requestId"
    }

    public init(accepted: Bool, correlationID: String, requestID: String) {
        self.accepted = accepted
        self.correlationID = correlationID
        self.requestID = requestID
    }
}

// MARK: - SubscriptionResponseEnvelope
public struct SubscriptionResponseEnvelope: Codable, Sendable {
    public let correlationID: String
    public let outcome: [String: JSONAny]
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case outcome
        case requestID = "requestId"
    }

    public init(correlationID: String, outcome: [String: JSONAny], requestID: String) {
        self.correlationID = correlationID
        self.outcome = outcome
        self.requestID = requestID
    }
}

// MARK: - SubscriptionClosedEnvelope
public struct SubscriptionClosedEnvelope: Codable, Sendable {
    public let correlationID: String
    public let lastDeliveredCursor: String?
    public let reason: SubscriptionCloseReason
    public let subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case lastDeliveredCursor, reason
        case subscriptionID = "subscriptionId"
    }

    public init(correlationID: String, lastDeliveredCursor: String?, reason: SubscriptionCloseReason, subscriptionID: String) {
        self.correlationID = correlationID
        self.lastDeliveredCursor = lastDeliveredCursor
        self.reason = reason
        self.subscriptionID = subscriptionID
    }
}

/// Cached projections must be cleared and queried again after a policy change.
public enum SubscriptionCloseReason: String, Codable, Sendable {
    case authorizationRevoked = "authorizationRevoked"
    case backpressure = "backpressure"
    case clientRequested = "clientRequested"
    case engineStopping = "engineStopping"
    case projectionInvalidated = "projectionInvalidated"
}

// MARK: - UnsubscribeResponse
public struct UnsubscribeResponse: Codable, Sendable {
    public let accepted: Bool
    public let correlationID, requestID, subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case accepted
        case correlationID = "correlationId"
        case requestID = "requestId"
        case subscriptionID = "subscriptionId"
    }

    public init(accepted: Bool, correlationID: String, requestID: String, subscriptionID: String) {
        self.accepted = accepted
        self.correlationID = correlationID
        self.requestID = requestID
        self.subscriptionID = subscriptionID
    }
}

// MARK: - FurnitureReference
public struct FurnitureReference: Codable, Sendable {
    public let furnitureID: String
    public let revision, schemaVersion: Int
    public let scope: ScopeRef
    public let variantID: String

    public enum CodingKeys: String, CodingKey {
        case furnitureID = "furnitureId"
        case revision, schemaVersion, scope
        case variantID = "variantId"
    }

    public init(furnitureID: String, revision: Int, schemaVersion: Int, scope: ScopeRef, variantID: String) {
        self.furnitureID = furnitureID
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
        self.variantID = variantID
    }
}

// MARK: - ProductReference
public struct ProductReference: Codable, Sendable {
    public let productID: String
    public let revision, schemaVersion: Int
    public let scope: ScopeRef
    public let variantID: String

    public enum CodingKeys: String, CodingKey {
        case productID = "productId"
        case revision, schemaVersion, scope
        case variantID = "variantId"
    }

    public init(productID: String, revision: Int, schemaVersion: Int, scope: ScopeRef, variantID: String) {
        self.productID = productID
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
        self.variantID = variantID
    }
}

// MARK: - ListScopeRelationships
public struct ListScopeRelationships: Codable, Sendable {
    public let after: String?
    public let limit: Int

    public init(after: String?, limit: Int) {
        self.after = after
        self.limit = limit
    }
}

// MARK: - GetCatalogImage
public struct GetCatalogImage: Codable, Sendable {
    public let offset: Int
    public let reference: CatalogImageRef

    public init(offset: Int, reference: CatalogImageRef) {
        self.offset = offset
        self.reference = reference
    }
}

// MARK: - GetCustomer
public struct GetCustomer: Codable, Sendable {
    public let customerID: String

    public enum CodingKeys: String, CodingKey {
        case customerID = "customerId"
    }

    public init(customerID: String) {
        self.customerID = customerID
    }
}

// MARK: - SearchCustomers
public struct SearchCustomers: Codable, Sendable {
    public let after: String?
    public let limit: Int
    public let term: String

    public init(after: String?, limit: Int, term: String) {
        self.after = after
        self.limit = limit
        self.term = term
    }
}

// MARK: - ListFurnitureCategories
public struct ListFurnitureCategories: Codable, Sendable {
    public let after: String?
    public let limit: Int

    public init(after: String?, limit: Int) {
        self.after = after
        self.limit = limit
    }
}

// MARK: - ListFurnitures
public struct ListFurnitures: Codable, Sendable {
    public let after: String?
    public let limit: Int
    public let selectableOnly: Bool
    public let term: String

    public init(after: String?, limit: Int, selectableOnly: Bool, term: String) {
        self.after = after
        self.limit = limit
        self.selectableOnly = selectableOnly
        self.term = term
    }
}

// MARK: - GetFurnitureRevision
public struct GetFurnitureRevision: Codable, Sendable {
    /// New work requires the current revision and an active furniture, category, and variant.
    public let forNewWork: Bool
    public let reference: FurnitureReference

    public init(forNewWork: Bool, reference: FurnitureReference) {
        self.forNewWork = forNewWork
        self.reference = reference
    }
}

// MARK: - CheckFurnitureSelection
public struct CheckFurnitureSelection: Codable, Sendable {
    public let colorID: String?
    public let dimensions: FurnitureDimensions
    public let handleID: String?
    public let quantity: Int
    public let reference: FurnitureReference

    public enum CodingKeys: String, CodingKey {
        case colorID = "colorId"
        case dimensions
        case handleID = "handleId"
        case quantity, reference
    }

    public init(colorID: String?, dimensions: FurnitureDimensions, handleID: String?, quantity: Int, reference: FurnitureReference) {
        self.colorID = colorID
        self.dimensions = dimensions
        self.handleID = handleID
        self.quantity = quantity
        self.reference = reference
    }
}

// MARK: - ListMaterials
public struct ListMaterials: Codable, Sendable {
    public let after: String?
    public let limit: Int
    public let term: String

    public init(after: String?, limit: Int, term: String) {
        self.after = after
        self.limit = limit
        self.term = term
    }
}

// MARK: - ListPartCategories
public struct ListPartCategories: Codable, Sendable {
    public let after: String?
    public let limit: Int

    public init(after: String?, limit: Int) {
        self.after = after
        self.limit = limit
    }
}

// MARK: - GetPartComposition
public struct GetPartComposition: Codable, Sendable {
    public let reference: CompositionReference

    public init(reference: CompositionReference) {
        self.reference = reference
    }
}

// MARK: - CalculatePartCost
public struct CalculatePartCost: Codable, Sendable {
    public let partID: String?
    public let usages: [PartUsage]

    public enum CodingKeys: String, CodingKey {
        case partID = "partId"
        case usages
    }

    public init(partID: String?, usages: [PartUsage]) {
        self.partID = partID
        self.usages = usages
    }
}

// MARK: - ListParts
public struct ListParts: Codable, Sendable {
    public let after: String?
    public let limit: Int
    public let term: String

    public init(after: String?, limit: Int, term: String) {
        self.after = after
        self.limit = limit
        self.term = term
    }
}

// MARK: - CalculateDiscount
public struct CalculateDiscount: Codable, Sendable {
    public let discountBasisPoints: Int
    public let lineTotalsYer: [Int]

    public init(discountBasisPoints: Int, lineTotalsYer: [Int]) {
        self.discountBasisPoints = discountBasisPoints
        self.lineTotalsYer = lineTotalsYer
    }
}

// MARK: - ListPrices
public struct ListPrices: Codable, Sendable {
    public let after: String?
    public let limit: Int
    public let term: String

    public init(after: String?, limit: Int, term: String) {
        self.after = after
        self.limit = limit
        self.term = term
    }
}

// MARK: - ReviewPrice
public struct ReviewPrice: Codable, Sendable {
    public let sellingPriceYer: Int
    public let target: [String: JSONAny]

    public init(sellingPriceYer: Int, target: [String: JSONAny]) {
        self.sellingPriceYer = sellingPriceYer
        self.target = target
    }
}

// MARK: - PriceSelection
public struct PriceSelection: Codable, Sendable {
    public let colorID, handleID: String?
    public let priceRevision, quantity: Int
    public let target: [String: JSONAny]

    public enum CodingKeys: String, CodingKey {
        case colorID = "colorId"
        case handleID = "handleId"
        case priceRevision, quantity, target
    }

    public init(colorID: String?, handleID: String?, priceRevision: Int, quantity: Int, target: [String: JSONAny]) {
        self.colorID = colorID
        self.handleID = handleID
        self.priceRevision = priceRevision
        self.quantity = quantity
        self.target = target
    }
}

// MARK: - ListProductCategories
public struct ListProductCategories: Codable, Sendable {
    public let after: String?
    public let limit: Int

    public init(after: String?, limit: Int) {
        self.after = after
        self.limit = limit
    }
}

// MARK: - ListProducts
public struct ListProducts: Codable, Sendable {
    public let after: String?
    public let limit: Int
    public let selectableOnly: Bool
    public let term: String

    public init(after: String?, limit: Int, selectableOnly: Bool, term: String) {
        self.after = after
        self.limit = limit
        self.selectableOnly = selectableOnly
        self.term = term
    }
}

// MARK: - GetProductRevision
public struct GetProductRevision: Codable, Sendable {
    /// New work requires the current revision and an active product, category, and variant.
    public let forNewWork: Bool
    public let reference: ProductReference

    public init(forNewWork: Bool, reference: ProductReference) {
        self.forNewWork = forNewWork
        self.reference = reference
    }
}

// MARK: - CatalogImageChunk
public struct CatalogImageChunk: Codable, Sendable {
    public let base64: String
    public let offset: Int
    public let reference: CatalogImageRef
    public let totalBytes: Int

    public init(base64: String, offset: Int, reference: CatalogImageRef, totalBytes: Int) {
        self.base64 = base64
        self.offset = offset
        self.reference = reference
        self.totalBytes = totalBytes
    }
}

// MARK: - CustomerPage
public struct CustomerPage: Codable, Sendable {
    public let items: [Customer]
    public let next: String?

    public init(items: [Customer], next: String?) {
        self.items = items
        self.next = next
    }
}

// MARK: - DesktopAccountPage
public struct DesktopAccountPage: Codable, Sendable {
    public let accounts: [DesktopAccountSummary]

    public init(accounts: [DesktopAccountSummary]) {
        self.accounts = accounts
    }
}

// MARK: - DiscountTotal
public struct DiscountTotal: Codable, Sendable {
    public let approvalRequired: Bool
    public let discountYer, subtotalYer, totalYer: Int

    public init(approvalRequired: Bool, discountYer: Int, subtotalYer: Int, totalYer: Int) {
        self.approvalRequired = approvalRequired
        self.discountYer = discountYer
        self.subtotalYer = subtotalYer
        self.totalYer = totalYer
    }
}

// MARK: - FurnitureCategories
public struct FurnitureCategories: Codable, Sendable {
    public let items: [FurnitureCategory]
    public let next: String?

    public init(items: [FurnitureCategory], next: String?) {
        self.items = items
        self.next = next
    }
}

// MARK: - FurnitureReview
public struct FurnitureReview: Codable, Sendable {
    public let marginsYer: [Int]
    public let partsCostYer: Int
    public let rowCostsYer: [Int]

    public init(marginsYer: [Int], partsCostYer: Int, rowCostsYer: [Int]) {
        self.marginsYer = marginsYer
        self.partsCostYer = partsCostYer
        self.rowCostsYer = rowCostsYer
    }
}

// MARK: - FurnitureSelection
public struct FurnitureSelection: Codable, Sendable {
    public let definition: Furniture
    public let totalYer, unitPriceYer: Int

    public init(definition: Furniture, totalYer: Int, unitPriceYer: Int) {
        self.definition = definition
        self.totalYer = totalYer
        self.unitPriceYer = unitPriceYer
    }
}

// MARK: - FurniturePage
public struct FurniturePage: Codable, Sendable {
    public let canManage, canReadCosts: Bool
    public let items: [Furniture]
    public let next: String?

    public init(canManage: Bool, canReadCosts: Bool, items: [Furniture], next: String?) {
        self.canManage = canManage
        self.canReadCosts = canReadCosts
        self.items = items
        self.next = next
    }
}

// MARK: - MaterialReferences
public struct MaterialReferences: Codable, Sendable {
    public let categories: [MaterialCategory]
    public let units: [MaterialUnit]

    public init(categories: [MaterialCategory], units: [MaterialUnit]) {
        self.categories = categories
        self.units = units
    }
}

// MARK: - MaterialPage
public struct MaterialPage: Codable, Sendable {
    public let items: [Material]
    public let next: String?

    public init(items: [Material], next: String?) {
        self.items = items
        self.next = next
    }
}

// MARK: - PartCategories
public struct PartCategories: Codable, Sendable {
    public let items: [PartCategory]
    public let next: String?

    public init(items: [PartCategory], next: String?) {
        self.items = items
        self.next = next
    }
}

// MARK: - PartPage
public struct PartPage: Codable, Sendable {
    public let items: [PartProjection]
    public let next: String?

    public init(items: [PartProjection], next: String?) {
        self.items = items
        self.next = next
    }
}

// MARK: - PartProjection
public struct PartProjection: Codable, Sendable {
    /// Advisory current cost; commercial references resolve the immutable part snapshot instead.
    public let currentCost: PartCost
    public let part: Part

    public init(currentCost: PartCost, part: Part) {
        self.currentCost = currentCost
        self.part = part
    }
}

// MARK: - PriceReview
public struct PriceReview: Codable, Sendable {
    public let belowCost: Bool
    public let costYer, marginYer: Int

    public init(belowCost: Bool, costYer: Int, marginYer: Int) {
        self.belowCost = belowCost
        self.costYer = costYer
        self.marginYer = marginYer
    }
}

// MARK: - PricePage
public struct PricePage: Codable, Sendable {
    public let canManage, canReadCosts: Bool
    /// Unresolved catalog transfer failures, returned only to an authorized Manager.
    public let catalogSyncIssues: [CatalogSyncIssue]
    public let items: [PriceItem]
    public let next: String?
    public let serverAvailable: Bool

    public init(canManage: Bool, canReadCosts: Bool, catalogSyncIssues: [CatalogSyncIssue], items: [PriceItem], next: String?, serverAvailable: Bool) {
        self.canManage = canManage
        self.canReadCosts = canReadCosts
        self.catalogSyncIssues = catalogSyncIssues
        self.items = items
        self.next = next
        self.serverAvailable = serverAvailable
    }
}

/// Repair information for a Manager. The rejected payload and server response stay in Rust
/// storage.
// MARK: - CatalogSyncIssue
public struct CatalogSyncIssue: Codable, Sendable {
    public let conflicted: Bool
    public let id, kind, name: String
    public let revision: Int

    public init(conflicted: Bool, id: String, kind: String, name: String, revision: Int) {
        self.conflicted = conflicted
        self.id = id
        self.kind = kind
        self.name = name
        self.revision = revision
    }
}

// MARK: - PriceItem
public struct PriceItem: Codable, Sendable {
    public let categoryName: String
    public let costYer, marginYer: Int?
    public let name: String
    public let publicationRequired: Bool
    public let published: PriceSummary?
    public let target: [String: JSONAny]
    public let variantName: String

    public init(categoryName: String, costYer: Int?, marginYer: Int?, name: String, publicationRequired: Bool, published: PriceSummary?, target: [String: JSONAny], variantName: String) {
        self.categoryName = categoryName
        self.costYer = costYer
        self.marginYer = marginYer
        self.name = name
        self.publicationRequired = publicationRequired
        self.published = published
        self.target = target
        self.variantName = variantName
    }
}

// MARK: - PriceSummary
public struct PriceSummary: Codable, Sendable {
    public let confirmedAt: Int
    public let currency: String
    public let revision: Int
    public let sellingPriceYer: Int

    public init(confirmedAt: Int, currency: String, revision: Int, sellingPriceYer: Int) {
        self.confirmedAt = confirmedAt
        self.currency = currency
        self.revision = revision
        self.sellingPriceYer = sellingPriceYer
    }
}

// MARK: - ProductCategories
public struct ProductCategories: Codable, Sendable {
    public let items: [ProductCategory]
    public let next: String?

    public init(items: [ProductCategory], next: String?) {
        self.items = items
        self.next = next
    }
}

// MARK: - ProductPage
public struct ProductPage: Codable, Sendable {
    public let canManage, canReadCosts: Bool
    public let items: [Product]
    public let next: String?

    public init(canManage: Bool, canReadCosts: Bool, items: [Product], next: String?) {
        self.canManage = canManage
        self.canReadCosts = canReadCosts
        self.items = items
        self.next = next
    }
}

// MARK: - RelationshipPage
public struct RelationshipPage: Codable, Sendable {
    public let nextAfter: String?
    public let policyVersion: Int
    public let relationships: [ScopeRelationship]

    public init(nextAfter: String?, policyVersion: Int, relationships: [ScopeRelationship]) {
        self.nextAfter = nextAfter
        self.policyVersion = policyVersion
        self.relationships = relationships
    }
}

// MARK: - SellingPrice
public struct SellingPrice: Codable, Sendable {
    public let snapshot: PublishedPrice
    public let totalYer, unitPriceYer: Int

    public init(snapshot: PublishedPrice, totalYer: Int, unitPriceYer: Int) {
        self.snapshot = snapshot
        self.totalYer = totalYer
        self.unitPriceYer = unitPriceYer
    }
}

// MARK: - SubscriptionAccepted
public struct SubscriptionAccepted: Codable, Sendable {
    public let resumed: Bool
    public let streamCursor, subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case resumed, streamCursor
        case subscriptionID = "subscriptionId"
    }

    public init(resumed: Bool, streamCursor: String, subscriptionID: String) {
        self.resumed = resumed
        self.streamCursor = streamCursor
        self.subscriptionID = subscriptionID
    }
}

// MARK: - Encode/decode helpers

public class JSONNull: Codable, Hashable {

    public static func == (lhs: JSONNull, rhs: JSONNull) -> Bool {
            return true
    }

    public var hashValue: Int {
            return 0
    }

    public func hash(into hasher: inout Hasher) {
            // No-op
    }

    public init() {}

    public required init(from decoder: Decoder) throws {
            let container = try decoder.singleValueContainer()
            if !container.decodeNil() {
                    throw DecodingError.typeMismatch(JSONNull.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Wrong type for JSONNull"))
            }
    }

    public func encode(to encoder: Encoder) throws {
            var container = encoder.singleValueContainer()
            try container.encodeNil()
    }
}

class JSONCodingKey: CodingKey {
    let key: String

    required init?(intValue: Int) {
            return nil
    }

    required init?(stringValue: String) {
            key = stringValue
    }

    var intValue: Int? {
            return nil
    }

    var stringValue: String {
            return key
    }
}

public class JSONAny: Codable {

    public let value: Any

    static func decodingError(forCodingPath codingPath: [CodingKey]) -> DecodingError {
            let context = DecodingError.Context(codingPath: codingPath, debugDescription: "Cannot decode JSONAny")
            return DecodingError.typeMismatch(JSONAny.self, context)
    }

    static func encodingError(forValue value: Any, codingPath: [CodingKey]) -> EncodingError {
            let context = EncodingError.Context(codingPath: codingPath, debugDescription: "Cannot encode JSONAny")
            return EncodingError.invalidValue(value, context)
    }

    static func decode(from container: SingleValueDecodingContainer) throws -> Any {
            if let value = try? container.decode(Bool.self) {
                    return value
            }
            if let value = try? container.decode(Int64.self) {
                    return value
            }
            if let value = try? container.decode(Double.self) {
                    return value
            }
            if let value = try? container.decode(String.self) {
                    return value
            }
            if container.decodeNil() {
                    return JSONNull()
            }
            throw decodingError(forCodingPath: container.codingPath)
    }

    static func decode(from container: inout UnkeyedDecodingContainer) throws -> Any {
            if let value = try? container.decode(Bool.self) {
                    return value
            }
            if let value = try? container.decode(Int64.self) {
                    return value
            }
            if let value = try? container.decode(Double.self) {
                    return value
            }
            if let value = try? container.decode(String.self) {
                    return value
            }
            if let value = try? container.decodeNil() {
                    if value {
                            return JSONNull()
                    }
            }
            if var container = try? container.nestedUnkeyedContainer() {
                    return try decodeArray(from: &container)
            }
            if var container = try? container.nestedContainer(keyedBy: JSONCodingKey.self) {
                    return try decodeDictionary(from: &container)
            }
            throw decodingError(forCodingPath: container.codingPath)
    }

    static func decode(from container: inout KeyedDecodingContainer<JSONCodingKey>, forKey key: JSONCodingKey) throws -> Any {
            if let value = try? container.decode(Bool.self, forKey: key) {
                    return value
            }
            if let value = try? container.decode(Int64.self, forKey: key) {
                    return value
            }
            if let value = try? container.decode(Double.self, forKey: key) {
                    return value
            }
            if let value = try? container.decode(String.self, forKey: key) {
                    return value
            }
            if let value = try? container.decodeNil(forKey: key) {
                    if value {
                            return JSONNull()
                    }
            }
            if var container = try? container.nestedUnkeyedContainer(forKey: key) {
                    return try decodeArray(from: &container)
            }
            if var container = try? container.nestedContainer(keyedBy: JSONCodingKey.self, forKey: key) {
                    return try decodeDictionary(from: &container)
            }
            throw decodingError(forCodingPath: container.codingPath)
    }

    static func decodeArray(from container: inout UnkeyedDecodingContainer) throws -> [Any] {
            var arr: [Any] = []
            while !container.isAtEnd {
                    let value = try decode(from: &container)
                    arr.append(value)
            }
            return arr
    }

    static func decodeDictionary(from container: inout KeyedDecodingContainer<JSONCodingKey>) throws -> [String: Any] {
            var dict = [String: Any]()
            for key in container.allKeys {
                    let value = try decode(from: &container, forKey: key)
                    dict[key.stringValue] = value
            }
            return dict
    }

    static func encode(to container: inout UnkeyedEncodingContainer, array: [Any]) throws {
            for value in array {
                    if let value = value as? Bool {
                            try container.encode(value)
                    } else if let value = value as? Int64 {
                            try container.encode(value)
                    } else if let value = value as? Double {
                            try container.encode(value)
                    } else if let value = value as? String {
                            try container.encode(value)
                    } else if value is JSONNull {
                            try container.encodeNil()
                    } else if let value = value as? [Any] {
                            var container = container.nestedUnkeyedContainer()
                            try encode(to: &container, array: value)
                    } else if let value = value as? [String: Any] {
                            var container = container.nestedContainer(keyedBy: JSONCodingKey.self)
                            try encode(to: &container, dictionary: value)
                    } else {
                            throw encodingError(forValue: value, codingPath: container.codingPath)
                    }
            }
    }

    static func encode(to container: inout KeyedEncodingContainer<JSONCodingKey>, dictionary: [String: Any]) throws {
            for (key, value) in dictionary {
                    let key = JSONCodingKey(stringValue: key)!
                    if let value = value as? Bool {
                            try container.encode(value, forKey: key)
                    } else if let value = value as? Int64 {
                            try container.encode(value, forKey: key)
                    } else if let value = value as? Double {
                            try container.encode(value, forKey: key)
                    } else if let value = value as? String {
                            try container.encode(value, forKey: key)
                    } else if value is JSONNull {
                            try container.encodeNil(forKey: key)
                    } else if let value = value as? [Any] {
                            var container = container.nestedUnkeyedContainer(forKey: key)
                            try encode(to: &container, array: value)
                    } else if let value = value as? [String: Any] {
                            var container = container.nestedContainer(keyedBy: JSONCodingKey.self, forKey: key)
                            try encode(to: &container, dictionary: value)
                    } else {
                            throw encodingError(forValue: value, codingPath: container.codingPath)
                    }
            }
    }

    static func encode(to container: inout SingleValueEncodingContainer, value: Any) throws {
            if let value = value as? Bool {
                    try container.encode(value)
            } else if let value = value as? Int64 {
                    try container.encode(value)
            } else if let value = value as? Double {
                    try container.encode(value)
            } else if let value = value as? String {
                    try container.encode(value)
            } else if value is JSONNull {
                    try container.encodeNil()
            } else {
                    throw encodingError(forValue: value, codingPath: container.codingPath)
            }
    }

    public required init(from decoder: Decoder) throws {
            if var arrayContainer = try? decoder.unkeyedContainer() {
                    self.value = try JSONAny.decodeArray(from: &arrayContainer)
            } else if var container = try? decoder.container(keyedBy: JSONCodingKey.self) {
                    self.value = try JSONAny.decodeDictionary(from: &container)
            } else {
                    let container = try decoder.singleValueContainer()
                    self.value = try JSONAny.decode(from: container)
            }
    }

    public func encode(to encoder: Encoder) throws {
            if let arr = self.value as? [Any] {
                    var container = encoder.unkeyedContainer()
                    try JSONAny.encode(to: &container, array: arr)
            } else if let dict = self.value as? [String: Any] {
                    var container = encoder.container(keyedBy: JSONCodingKey.self)
                    try JSONAny.encode(to: &container, dictionary: dict)
            } else {
                    var container = encoder.singleValueContainer()
                    try JSONAny.encode(to: &container, value: self.value)
            }
    }
}
