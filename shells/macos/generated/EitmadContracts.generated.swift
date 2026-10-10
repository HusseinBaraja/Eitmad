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
    case quotationDraftValidation = "quotationDraftValidation"
    case revisionConflict = "revisionConflict"
    case validation = "validation"
}

// MARK: - DetailPayload
public struct DetailPayload: Codable, Sendable {
    public let fields: [String]?
    public let errors: [QuotationFieldError]?
    public let actual, expected: Int?
    public let reason: String?
    public let stage: LifecycleStage?
    public let deadline: Int?
    public let maximumBytes: Int?

    public enum CodingKeys: String, CodingKey {
        case fields, errors, actual, expected, reason, stage, deadline
        case maximumBytes = "maximum_bytes"
    }

    public init(fields: [String]?, errors: [QuotationFieldError]?, actual: Int?, expected: Int?, reason: String?, stage: LifecycleStage?, deadline: Int?, maximumBytes: Int?) {
        self.fields = fields
        self.errors = errors
        self.actual = actual
        self.expected = expected
        self.reason = reason
        self.stage = stage
        self.deadline = deadline
        self.maximumBytes = maximumBytes
    }
}

// MARK: - QuotationFieldError
public struct QuotationFieldError: Codable, Sendable {
    public let field: QuotationField
    public let issue: QuotationIssue
    public let lineID: String?

    public enum CodingKeys: String, CodingKey {
        case field, issue
        case lineID = "lineId"
    }

    public init(field: QuotationField, issue: QuotationIssue, lineID: String?) {
        self.field = field
        self.issue = issue
        self.lineID = lineID
    }
}

public enum QuotationField: String, Codable, Sendable {
    case colorID = "colorId"
    case customer = "customer"
    case customerRevision = "customerRevision"
    case dimensions = "dimensions"
    case discountBasisPoints = "discountBasisPoints"
    case handleID = "handleId"
    case lineID = "lineId"
    case lines = "lines"
    case priceRevision = "priceRevision"
    case quantity = "quantity"
    case target = "target"
    case total = "total"
}

public enum QuotationIssue: String, Codable, Sendable {
    case duplicate = "duplicate"
    case invalid = "invalid"
    case overflow = "overflow"
    case quotationIssueRequired = "required"
    case stale = "stale"
    case unavailable = "unavailable"
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
    public let commandOrderCancel: CancelOrder?
    public let commandOrderConvert: ConvertQuotation?
    public let commandOrderDeliver: RecordOrderDelivery?
    public let commandOrderFulfillment: EditOrderFulfillment?
    public let commandOrderWorkComplete, commandOrderWorkStart: TransitionOrderWork?
    public let commandPartCategorySave: SavePartCategory?
    public let commandPartSave: SavePart?
    public let commandPricingPublish: PublishPrice?
    public let commandProductCategorySave: SaveProductCategory?
    public let commandProductSave: SaveProduct?
    public let commandQuotationAccept: AcceptQuotation?
    public let commandQuotationApprovalDecide: DecideDiscountApproval?
    public let commandQuotationApprovalRequest: RequestDiscountApproval?
    public let commandQuotationCancel: CancelQuotation?
    public let commandQuotationDraftCancel: CancelQuotationDraft?
    public let commandQuotationDraftCreate: CreateQuotationDraft?
    public let commandQuotationDraftUpdate: UpdateQuotationDraft?
    public let commandQuotationIssue: IssueQuotation?
    public let commandQuotationRevise, commandQuotationValidity: SetQuotationValidity?
    public let commandOutcomeFailed: ContractError?
    public let commandOutcomeSucceeded: [String: JSONAny]?
    public let commandResultCatalogImageImported: CatalogImageRef?
    public let commandResultConfigurationUpdated: ConfigSnapshot?
    public let commandResultCustomerCreated, commandResultCustomerUpdated: CustomerMutationResult?
    public let commandResultDesktopAccountCreated, commandResultDesktopAccountDeactivated, commandResultDesktopAccountUpdated: DesktopAccountSummary?
    public let commandResultDiscountApproval: DiscountApproval?
    public let commandResultFurnitureCategorySaved: FurnitureCategory?
    public let commandResultFurnitureSaved: Furniture?
    public let commandResultMaterialCategorySaved: MaterialCategory?
    public let commandResultMaterialSaved: Material?
    public let commandResultMaterialUnitSaved: MaterialUnit?
    public let commandResultOrder: OrderRecord?
    public let commandResultPartCategorySaved: PartCategory?
    public let commandResultPartSaved: Part?
    public let commandResultPricePublished: PublishedPrice?
    public let commandResultProductCategorySaved: ProductCategory?
    public let commandResultProductSaved: Product?
    public let commandResultQuotation: QuotationRecord?
    public let commandResultQuotationDraftCreated, commandResultQuotationDraftUpdated: QuotationDraft?
    public let commandResultRelationshipGranted, commandResultRelationshipRevoked: RelationshipMutationResult?
    public let eventAuthorizationPolicyChangedEvent: AuthorizationPolicyChangeNotice?
    public let eventConfigChangedEvent: ConfigSnapshot?
    public let eventCustomerChangedEvent: CustomerChangeNotice?
    public let eventFurnitureChangedEvent: FurnitureChangeNotice?
    public let eventMaterialChangedEvent: MaterialChangeNotice?
    public let eventOrderChangedEvent: OrderNotice?
    public let eventPartChangedEvent: PartChangeNotice?
    public let eventPermissionsChangedEvent: EffectivePermissions?
    public let eventPricingChangedEvent: PriceChangeNotice?
    public let eventProductChangedEvent: ProductChangeNotice?
    public let eventQuotationApprovalChangedEvent: DiscountApprovalNotice?
    public let eventQuotationChangedEvent: QuotationNotice?
    public let eventQuotationDraftChangedEvent: QuotationDraftChangeNotice?
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
    public let orderActionCancel: CancelOrder?
    public let orderActionCompleteWork: TransitionOrderWork?
    public let orderActionConvert: ConvertQuotation?
    public let orderActionDeliver: RecordOrderDelivery?
    public let orderActionEditFulfillment: EditOrderFulfillment?
    public let orderActionStartWork: TransitionOrderWork?
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
    public let queryOrderGet: GetOrder?
    public let queryOrderList: ListOrders?
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
    public let queryQuotationApprovalList: ListDiscountApprovals?
    public let queryQuotationDraftGet: GetQuotationDraft?
    public let queryQuotationDraftList: ListQuotationDrafts?
    public let queryQuotationEvaluate: EvaluateQuotation?
    public let queryQuotationList: ListQuotations?
    public let querySalesCatalogCheck: CheckSalesConfiguration?
    public let querySalesCatalogGet: GetSalesCatalogItem?
    public let querySalesCatalogList: ListSalesCatalog?
    public let queryWorkOrderList: ListWorkOrders?
    public let queryOutcomeFailed: ContractError?
    public let queryOutcomeSucceeded: [String: JSONAny]?
    public let queryResultCatalogImage: CatalogImageChunk?
    public let queryResultConfiguration: ConfigSnapshot?
    public let queryResultCustomer: Customer?
    public let queryResultCustomers: CustomerPage?
    public let queryResultDesktopAccounts: DesktopAccountPage?
    public let queryResultDiscountApprovals: DiscountApprovalPage?
    public let queryResultDiscountTotal: DiscountTotal?
    public let queryResultEffectivePermissions: EffectivePermissions?
    public let queryResultFurnitureCategories: FurnitureCategories?
    public let queryResultFurnitureReview: FurnitureReview?
    public let queryResultFurnitureRevision: Furniture?
    public let queryResultFurnitures: FurniturePage?
    public let queryResultFurnitureSelection: FurnitureSelection?
    public let queryResultMaterialReferences: MaterialReferences?
    public let queryResultMaterials: MaterialPage?
    public let queryResultOrders: OrderPage?
    public let queryResultPartCategories: PartCategories?
    public let queryResultPartComposition: Part?
    public let queryResultPartCost: PartCost?
    public let queryResultParts: PartPage?
    public let queryResultPriceReview: PriceReview?
    public let queryResultPrices: PricePage?
    public let queryResultProductCategories: ProductCategories?
    public let queryResultProductRevision: Product?
    public let queryResultProducts: ProductPage?
    public let queryResultQuotationDraft: QuotationDraft?
    public let queryResultQuotationDrafts: QuotationDraftPage?
    public let queryResultQuotationEvaluation: QuotationEvaluation?
    public let queryResultQuotations: QuotationPage?
    public let queryResultSalesCatalog: SalesCatalogPage?
    public let queryResultSalesCatalogItem: SalesCatalogDetails?
    public let queryResultSalesConfiguration: SalesConfiguration?
    public let queryResultScopeRelationships: RelationshipPage?
    public let queryResultSellingPrice: SellingPrice?
    public let queryResultWorkOrders: WorkOrderPage?
    public let subscriptionAuthorizationPolicyChangedSubscribe, subscriptionConfigChangedSubscribe, subscriptionCustomerChangedSubscribe, subscriptionFurnitureChangedSubscribe: [String: JSONAny]?
    public let subscriptionMaterialChangedSubscribe: [String: JSONAny]?
    public let subscriptionOrderChangedSubscribe: OrderChanges?
    public let subscriptionPartChangedSubscribe, subscriptionPermissionsChangedSubscribe, subscriptionPricingChangedSubscribe, subscriptionProductChangedSubscribe: [String: JSONAny]?
    public let subscriptionQuotationApprovalChangedSubscribe: DiscountApprovalChanges?
    public let subscriptionQuotationChangedSubscribe: QuotationChanges?
    public let subscriptionQuotationDraftChangedSubscribe: QuotationDraftChanges?
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
        case commandOrderCancel = "Command_OrderCancel"
        case commandOrderConvert = "Command_OrderConvert"
        case commandOrderDeliver = "Command_OrderDeliver"
        case commandOrderFulfillment = "Command_OrderFulfillment"
        case commandOrderWorkComplete = "Command_OrderWorkComplete"
        case commandOrderWorkStart = "Command_OrderWorkStart"
        case commandPartCategorySave = "Command_PartCategorySave"
        case commandPartSave = "Command_PartSave"
        case commandPricingPublish = "Command_PricingPublish"
        case commandProductCategorySave = "Command_ProductCategorySave"
        case commandProductSave = "Command_ProductSave"
        case commandQuotationAccept = "Command_QuotationAccept"
        case commandQuotationApprovalDecide = "Command_QuotationApprovalDecide"
        case commandQuotationApprovalRequest = "Command_QuotationApprovalRequest"
        case commandQuotationCancel = "Command_QuotationCancel"
        case commandQuotationDraftCancel = "Command_QuotationDraftCancel"
        case commandQuotationDraftCreate = "Command_QuotationDraftCreate"
        case commandQuotationDraftUpdate = "Command_QuotationDraftUpdate"
        case commandQuotationIssue = "Command_QuotationIssue"
        case commandQuotationRevise = "Command_QuotationRevise"
        case commandQuotationValidity = "Command_QuotationValidity"
        case commandOutcomeFailed = "CommandOutcome_Failed"
        case commandOutcomeSucceeded = "CommandOutcome_Succeeded"
        case commandResultCatalogImageImported = "CommandResult_CatalogImageImported"
        case commandResultConfigurationUpdated = "CommandResult_ConfigurationUpdated"
        case commandResultCustomerCreated = "CommandResult_CustomerCreated"
        case commandResultCustomerUpdated = "CommandResult_CustomerUpdated"
        case commandResultDesktopAccountCreated = "CommandResult_DesktopAccountCreated"
        case commandResultDesktopAccountDeactivated = "CommandResult_DesktopAccountDeactivated"
        case commandResultDesktopAccountUpdated = "CommandResult_DesktopAccountUpdated"
        case commandResultDiscountApproval = "CommandResult_DiscountApproval"
        case commandResultFurnitureCategorySaved = "CommandResult_FurnitureCategorySaved"
        case commandResultFurnitureSaved = "CommandResult_FurnitureSaved"
        case commandResultMaterialCategorySaved = "CommandResult_MaterialCategorySaved"
        case commandResultMaterialSaved = "CommandResult_MaterialSaved"
        case commandResultMaterialUnitSaved = "CommandResult_MaterialUnitSaved"
        case commandResultOrder = "CommandResult_Order"
        case commandResultPartCategorySaved = "CommandResult_PartCategorySaved"
        case commandResultPartSaved = "CommandResult_PartSaved"
        case commandResultPricePublished = "CommandResult_PricePublished"
        case commandResultProductCategorySaved = "CommandResult_ProductCategorySaved"
        case commandResultProductSaved = "CommandResult_ProductSaved"
        case commandResultQuotation = "CommandResult_Quotation"
        case commandResultQuotationDraftCreated = "CommandResult_QuotationDraftCreated"
        case commandResultQuotationDraftUpdated = "CommandResult_QuotationDraftUpdated"
        case commandResultRelationshipGranted = "CommandResult_RelationshipGranted"
        case commandResultRelationshipRevoked = "CommandResult_RelationshipRevoked"
        case eventAuthorizationPolicyChangedEvent = "Event_AuthorizationPolicyChangedEvent"
        case eventConfigChangedEvent = "Event_ConfigChangedEvent"
        case eventCustomerChangedEvent = "Event_CustomerChangedEvent"
        case eventFurnitureChangedEvent = "Event_FurnitureChangedEvent"
        case eventMaterialChangedEvent = "Event_MaterialChangedEvent"
        case eventOrderChangedEvent = "Event_OrderChangedEvent"
        case eventPartChangedEvent = "Event_PartChangedEvent"
        case eventPermissionsChangedEvent = "Event_PermissionsChangedEvent"
        case eventPricingChangedEvent = "Event_PricingChangedEvent"
        case eventProductChangedEvent = "Event_ProductChangedEvent"
        case eventQuotationApprovalChangedEvent = "Event_QuotationApprovalChangedEvent"
        case eventQuotationChangedEvent = "Event_QuotationChangedEvent"
        case eventQuotationDraftChangedEvent = "Event_QuotationDraftChangedEvent"
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
        case orderActionCancel = "OrderAction_Cancel"
        case orderActionCompleteWork = "OrderAction_CompleteWork"
        case orderActionConvert = "OrderAction_Convert"
        case orderActionDeliver = "OrderAction_Deliver"
        case orderActionEditFulfillment = "OrderAction_EditFulfillment"
        case orderActionStartWork = "OrderAction_StartWork"
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
        case queryOrderGet = "Query_OrderGet"
        case queryOrderList = "Query_OrderList"
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
        case queryQuotationApprovalList = "Query_QuotationApprovalList"
        case queryQuotationDraftGet = "Query_QuotationDraftGet"
        case queryQuotationDraftList = "Query_QuotationDraftList"
        case queryQuotationEvaluate = "Query_QuotationEvaluate"
        case queryQuotationList = "Query_QuotationList"
        case querySalesCatalogCheck = "Query_SalesCatalogCheck"
        case querySalesCatalogGet = "Query_SalesCatalogGet"
        case querySalesCatalogList = "Query_SalesCatalogList"
        case queryWorkOrderList = "Query_WorkOrderList"
        case queryOutcomeFailed = "QueryOutcome_Failed"
        case queryOutcomeSucceeded = "QueryOutcome_Succeeded"
        case queryResultCatalogImage = "QueryResult_CatalogImage"
        case queryResultConfiguration = "QueryResult_Configuration"
        case queryResultCustomer = "QueryResult_Customer"
        case queryResultCustomers = "QueryResult_Customers"
        case queryResultDesktopAccounts = "QueryResult_DesktopAccounts"
        case queryResultDiscountApprovals = "QueryResult_DiscountApprovals"
        case queryResultDiscountTotal = "QueryResult_DiscountTotal"
        case queryResultEffectivePermissions = "QueryResult_EffectivePermissions"
        case queryResultFurnitureCategories = "QueryResult_FurnitureCategories"
        case queryResultFurnitureReview = "QueryResult_FurnitureReview"
        case queryResultFurnitureRevision = "QueryResult_FurnitureRevision"
        case queryResultFurnitures = "QueryResult_Furnitures"
        case queryResultFurnitureSelection = "QueryResult_FurnitureSelection"
        case queryResultMaterialReferences = "QueryResult_MaterialReferences"
        case queryResultMaterials = "QueryResult_Materials"
        case queryResultOrders = "QueryResult_Orders"
        case queryResultPartCategories = "QueryResult_PartCategories"
        case queryResultPartComposition = "QueryResult_PartComposition"
        case queryResultPartCost = "QueryResult_PartCost"
        case queryResultParts = "QueryResult_Parts"
        case queryResultPriceReview = "QueryResult_PriceReview"
        case queryResultPrices = "QueryResult_Prices"
        case queryResultProductCategories = "QueryResult_ProductCategories"
        case queryResultProductRevision = "QueryResult_ProductRevision"
        case queryResultProducts = "QueryResult_Products"
        case queryResultQuotationDraft = "QueryResult_QuotationDraft"
        case queryResultQuotationDrafts = "QueryResult_QuotationDrafts"
        case queryResultQuotationEvaluation = "QueryResult_QuotationEvaluation"
        case queryResultQuotations = "QueryResult_Quotations"
        case queryResultSalesCatalog = "QueryResult_SalesCatalog"
        case queryResultSalesCatalogItem = "QueryResult_SalesCatalogItem"
        case queryResultSalesConfiguration = "QueryResult_SalesConfiguration"
        case queryResultScopeRelationships = "QueryResult_ScopeRelationships"
        case queryResultSellingPrice = "QueryResult_SellingPrice"
        case queryResultWorkOrders = "QueryResult_WorkOrders"
        case subscriptionAuthorizationPolicyChangedSubscribe = "Subscription_AuthorizationPolicyChangedSubscribe"
        case subscriptionConfigChangedSubscribe = "Subscription_ConfigChangedSubscribe"
        case subscriptionCustomerChangedSubscribe = "Subscription_CustomerChangedSubscribe"
        case subscriptionFurnitureChangedSubscribe = "Subscription_FurnitureChangedSubscribe"
        case subscriptionMaterialChangedSubscribe = "Subscription_MaterialChangedSubscribe"
        case subscriptionOrderChangedSubscribe = "Subscription_OrderChangedSubscribe"
        case subscriptionPartChangedSubscribe = "Subscription_PartChangedSubscribe"
        case subscriptionPermissionsChangedSubscribe = "Subscription_PermissionsChangedSubscribe"
        case subscriptionPricingChangedSubscribe = "Subscription_PricingChangedSubscribe"
        case subscriptionProductChangedSubscribe = "Subscription_ProductChangedSubscribe"
        case subscriptionQuotationApprovalChangedSubscribe = "Subscription_QuotationApprovalChangedSubscribe"
        case subscriptionQuotationChangedSubscribe = "Subscription_QuotationChangedSubscribe"
        case subscriptionQuotationDraftChangedSubscribe = "Subscription_QuotationDraftChangedSubscribe"
        case subscriptionOutcomeFailed = "SubscriptionOutcome_Failed"
        case subscriptionOutcomeSucceeded = "SubscriptionOutcome_Succeeded"
    }

    public init(commandAuthorizationRelationshipGrant: GrantScopeRelationship?, commandAuthorizationRelationshipRevoke: RevokeScopeRelationship?, commandCatalogImageImport: ImportCatalogImage?, commandConfigUpdate: UpdateConfiguration?, commandCustomerCreate: CreateCustomer?, commandCustomerUpdate: UpdateCustomer?, commandDesktopAccountCreate: CreateDesktopAccount?, commandDesktopAccountDeactivate: DeactivateDesktopAccount?, commandDesktopAccountUpdate: UpdateDesktopAccount?, commandFurnitureCategorySave: SaveFurnitureCategory?, commandFurnitureSave: SaveFurniture?, commandMaterialCategorySave: SaveMaterialCategory?, commandMaterialSave: SaveMaterial?, commandMaterialUnitSave: SaveMaterialUnit?, commandOrderCancel: CancelOrder?, commandOrderConvert: ConvertQuotation?, commandOrderDeliver: RecordOrderDelivery?, commandOrderFulfillment: EditOrderFulfillment?, commandOrderWorkComplete: TransitionOrderWork?, commandOrderWorkStart: TransitionOrderWork?, commandPartCategorySave: SavePartCategory?, commandPartSave: SavePart?, commandPricingPublish: PublishPrice?, commandProductCategorySave: SaveProductCategory?, commandProductSave: SaveProduct?, commandQuotationAccept: AcceptQuotation?, commandQuotationApprovalDecide: DecideDiscountApproval?, commandQuotationApprovalRequest: RequestDiscountApproval?, commandQuotationCancel: CancelQuotation?, commandQuotationDraftCancel: CancelQuotationDraft?, commandQuotationDraftCreate: CreateQuotationDraft?, commandQuotationDraftUpdate: UpdateQuotationDraft?, commandQuotationIssue: IssueQuotation?, commandQuotationRevise: SetQuotationValidity?, commandQuotationValidity: SetQuotationValidity?, commandOutcomeFailed: ContractError?, commandOutcomeSucceeded: [String: JSONAny]?, commandResultCatalogImageImported: CatalogImageRef?, commandResultConfigurationUpdated: ConfigSnapshot?, commandResultCustomerCreated: CustomerMutationResult?, commandResultCustomerUpdated: CustomerMutationResult?, commandResultDesktopAccountCreated: DesktopAccountSummary?, commandResultDesktopAccountDeactivated: DesktopAccountSummary?, commandResultDesktopAccountUpdated: DesktopAccountSummary?, commandResultDiscountApproval: DiscountApproval?, commandResultFurnitureCategorySaved: FurnitureCategory?, commandResultFurnitureSaved: Furniture?, commandResultMaterialCategorySaved: MaterialCategory?, commandResultMaterialSaved: Material?, commandResultMaterialUnitSaved: MaterialUnit?, commandResultOrder: OrderRecord?, commandResultPartCategorySaved: PartCategory?, commandResultPartSaved: Part?, commandResultPricePublished: PublishedPrice?, commandResultProductCategorySaved: ProductCategory?, commandResultProductSaved: Product?, commandResultQuotation: QuotationRecord?, commandResultQuotationDraftCreated: QuotationDraft?, commandResultQuotationDraftUpdated: QuotationDraft?, commandResultRelationshipGranted: RelationshipMutationResult?, commandResultRelationshipRevoked: RelationshipMutationResult?, eventAuthorizationPolicyChangedEvent: AuthorizationPolicyChangeNotice?, eventConfigChangedEvent: ConfigSnapshot?, eventCustomerChangedEvent: CustomerChangeNotice?, eventFurnitureChangedEvent: FurnitureChangeNotice?, eventMaterialChangedEvent: MaterialChangeNotice?, eventOrderChangedEvent: OrderNotice?, eventPartChangedEvent: PartChangeNotice?, eventPermissionsChangedEvent: EffectivePermissions?, eventPricingChangedEvent: PriceChangeNotice?, eventProductChangedEvent: ProductChangeNotice?, eventQuotationApprovalChangedEvent: DiscountApprovalNotice?, eventQuotationChangedEvent: QuotationNotice?, eventQuotationDraftChangedEvent: QuotationDraftChangeNotice?, handshakeOutcomeAccepted: HandshakeAccepted?, handshakeOutcomeRejected: HandshakeRejection?, ipcClientMessageIPCCommand: CommandEnvelope?, ipcClientMessageIPCDesktopSessionState: DesktopSessionRequest?, ipcClientMessageIPCDesktopSignIn: DesktopSignInRequest?, ipcClientMessageIPCDesktopSignOut: DesktopSessionRequest?, ipcClientMessageIPCHandshake: HandshakeRequest?, ipcClientMessageIPCQuery: QueryEnvelope?, ipcClientMessageIPCShutdown: ShutdownRequest?, ipcClientMessageIPCSubscribe: SubscriptionEnvelope?, ipcClientMessageIPCUnsubscribe: UnsubscribeRequest?, ipcServerMessageIPCCommandResponse: CommandResponseEnvelope?, ipcServerMessageIPCDesktopSessionResponse: DesktopSessionResponse?, ipcServerMessageIPCEvent: EventEnvelope?, ipcServerMessageIPCFailure: IPCFailureResponse?, ipcServerMessageIPCHandshakeResponse: HandshakeResponse?, ipcServerMessageIPCQueryResponse: QueryResponseEnvelope?, ipcServerMessageIPCShutdownResponse: ShutdownResponse?, ipcServerMessageIPCSubscribeResponse: SubscriptionResponseEnvelope?, ipcServerMessageIPCSubscriptionClosed: SubscriptionClosedEnvelope?, ipcServerMessageIPCUnsubscribeResponse: UnsubscribeResponse?, orderActionCancel: CancelOrder?, orderActionCompleteWork: TransitionOrderWork?, orderActionConvert: ConvertQuotation?, orderActionDeliver: RecordOrderDelivery?, orderActionEditFulfillment: EditOrderFulfillment?, orderActionStartWork: TransitionOrderWork?, priceTargetFurniture: FurnitureReference?, priceTargetProduct: ProductReference?, queryAuthorizationRelationshipsList: ListScopeRelationships?, queryCatalogImageGet: GetCatalogImage?, queryConfigGet: [String: JSONAny]?, queryCustomerGet: GetCustomer?, queryCustomerSearch: SearchCustomers?, queryDesktopAccountList: [String: JSONAny]?, queryFurnitureCategoryList: ListFurnitureCategories?, queryFurnitureList: ListFurnitures?, queryFurnitureReview: SaveFurniture?, queryFurnitureRevisionGet: GetFurnitureRevision?, queryFurnitureSelectionCheck: CheckFurnitureSelection?, queryMaterialList: ListMaterials?, queryMaterialReferenceList: [String: JSONAny]?, queryOrderGet: GetOrder?, queryOrderList: ListOrders?, queryPartCategoryList: ListPartCategories?, queryPartCompositionGet: GetPartComposition?, queryPartCost: CalculatePartCost?, queryPartList: ListParts?, queryPermissionsGetEffective: [String: JSONAny]?, queryPricingDiscount: CalculateDiscount?, queryPricingList: ListPrices?, queryPricingReview: ReviewPrice?, queryPricingSelection: PriceSelection?, queryProductCategoryList: ListProductCategories?, queryProductList: ListProducts?, queryProductRevisionGet: GetProductRevision?, queryQuotationApprovalList: ListDiscountApprovals?, queryQuotationDraftGet: GetQuotationDraft?, queryQuotationDraftList: ListQuotationDrafts?, queryQuotationEvaluate: EvaluateQuotation?, queryQuotationList: ListQuotations?, querySalesCatalogCheck: CheckSalesConfiguration?, querySalesCatalogGet: GetSalesCatalogItem?, querySalesCatalogList: ListSalesCatalog?, queryWorkOrderList: ListWorkOrders?, queryOutcomeFailed: ContractError?, queryOutcomeSucceeded: [String: JSONAny]?, queryResultCatalogImage: CatalogImageChunk?, queryResultConfiguration: ConfigSnapshot?, queryResultCustomer: Customer?, queryResultCustomers: CustomerPage?, queryResultDesktopAccounts: DesktopAccountPage?, queryResultDiscountApprovals: DiscountApprovalPage?, queryResultDiscountTotal: DiscountTotal?, queryResultEffectivePermissions: EffectivePermissions?, queryResultFurnitureCategories: FurnitureCategories?, queryResultFurnitureReview: FurnitureReview?, queryResultFurnitureRevision: Furniture?, queryResultFurnitures: FurniturePage?, queryResultFurnitureSelection: FurnitureSelection?, queryResultMaterialReferences: MaterialReferences?, queryResultMaterials: MaterialPage?, queryResultOrders: OrderPage?, queryResultPartCategories: PartCategories?, queryResultPartComposition: Part?, queryResultPartCost: PartCost?, queryResultParts: PartPage?, queryResultPriceReview: PriceReview?, queryResultPrices: PricePage?, queryResultProductCategories: ProductCategories?, queryResultProductRevision: Product?, queryResultProducts: ProductPage?, queryResultQuotationDraft: QuotationDraft?, queryResultQuotationDrafts: QuotationDraftPage?, queryResultQuotationEvaluation: QuotationEvaluation?, queryResultQuotations: QuotationPage?, queryResultSalesCatalog: SalesCatalogPage?, queryResultSalesCatalogItem: SalesCatalogDetails?, queryResultSalesConfiguration: SalesConfiguration?, queryResultScopeRelationships: RelationshipPage?, queryResultSellingPrice: SellingPrice?, queryResultWorkOrders: WorkOrderPage?, subscriptionAuthorizationPolicyChangedSubscribe: [String: JSONAny]?, subscriptionConfigChangedSubscribe: [String: JSONAny]?, subscriptionCustomerChangedSubscribe: [String: JSONAny]?, subscriptionFurnitureChangedSubscribe: [String: JSONAny]?, subscriptionMaterialChangedSubscribe: [String: JSONAny]?, subscriptionOrderChangedSubscribe: OrderChanges?, subscriptionPartChangedSubscribe: [String: JSONAny]?, subscriptionPermissionsChangedSubscribe: [String: JSONAny]?, subscriptionPricingChangedSubscribe: [String: JSONAny]?, subscriptionProductChangedSubscribe: [String: JSONAny]?, subscriptionQuotationApprovalChangedSubscribe: DiscountApprovalChanges?, subscriptionQuotationChangedSubscribe: QuotationChanges?, subscriptionQuotationDraftChangedSubscribe: QuotationDraftChanges?, subscriptionOutcomeFailed: ContractError?, subscriptionOutcomeSucceeded: SubscriptionAccepted?) {
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
        self.commandOrderCancel = commandOrderCancel
        self.commandOrderConvert = commandOrderConvert
        self.commandOrderDeliver = commandOrderDeliver
        self.commandOrderFulfillment = commandOrderFulfillment
        self.commandOrderWorkComplete = commandOrderWorkComplete
        self.commandOrderWorkStart = commandOrderWorkStart
        self.commandPartCategorySave = commandPartCategorySave
        self.commandPartSave = commandPartSave
        self.commandPricingPublish = commandPricingPublish
        self.commandProductCategorySave = commandProductCategorySave
        self.commandProductSave = commandProductSave
        self.commandQuotationAccept = commandQuotationAccept
        self.commandQuotationApprovalDecide = commandQuotationApprovalDecide
        self.commandQuotationApprovalRequest = commandQuotationApprovalRequest
        self.commandQuotationCancel = commandQuotationCancel
        self.commandQuotationDraftCancel = commandQuotationDraftCancel
        self.commandQuotationDraftCreate = commandQuotationDraftCreate
        self.commandQuotationDraftUpdate = commandQuotationDraftUpdate
        self.commandQuotationIssue = commandQuotationIssue
        self.commandQuotationRevise = commandQuotationRevise
        self.commandQuotationValidity = commandQuotationValidity
        self.commandOutcomeFailed = commandOutcomeFailed
        self.commandOutcomeSucceeded = commandOutcomeSucceeded
        self.commandResultCatalogImageImported = commandResultCatalogImageImported
        self.commandResultConfigurationUpdated = commandResultConfigurationUpdated
        self.commandResultCustomerCreated = commandResultCustomerCreated
        self.commandResultCustomerUpdated = commandResultCustomerUpdated
        self.commandResultDesktopAccountCreated = commandResultDesktopAccountCreated
        self.commandResultDesktopAccountDeactivated = commandResultDesktopAccountDeactivated
        self.commandResultDesktopAccountUpdated = commandResultDesktopAccountUpdated
        self.commandResultDiscountApproval = commandResultDiscountApproval
        self.commandResultFurnitureCategorySaved = commandResultFurnitureCategorySaved
        self.commandResultFurnitureSaved = commandResultFurnitureSaved
        self.commandResultMaterialCategorySaved = commandResultMaterialCategorySaved
        self.commandResultMaterialSaved = commandResultMaterialSaved
        self.commandResultMaterialUnitSaved = commandResultMaterialUnitSaved
        self.commandResultOrder = commandResultOrder
        self.commandResultPartCategorySaved = commandResultPartCategorySaved
        self.commandResultPartSaved = commandResultPartSaved
        self.commandResultPricePublished = commandResultPricePublished
        self.commandResultProductCategorySaved = commandResultProductCategorySaved
        self.commandResultProductSaved = commandResultProductSaved
        self.commandResultQuotation = commandResultQuotation
        self.commandResultQuotationDraftCreated = commandResultQuotationDraftCreated
        self.commandResultQuotationDraftUpdated = commandResultQuotationDraftUpdated
        self.commandResultRelationshipGranted = commandResultRelationshipGranted
        self.commandResultRelationshipRevoked = commandResultRelationshipRevoked
        self.eventAuthorizationPolicyChangedEvent = eventAuthorizationPolicyChangedEvent
        self.eventConfigChangedEvent = eventConfigChangedEvent
        self.eventCustomerChangedEvent = eventCustomerChangedEvent
        self.eventFurnitureChangedEvent = eventFurnitureChangedEvent
        self.eventMaterialChangedEvent = eventMaterialChangedEvent
        self.eventOrderChangedEvent = eventOrderChangedEvent
        self.eventPartChangedEvent = eventPartChangedEvent
        self.eventPermissionsChangedEvent = eventPermissionsChangedEvent
        self.eventPricingChangedEvent = eventPricingChangedEvent
        self.eventProductChangedEvent = eventProductChangedEvent
        self.eventQuotationApprovalChangedEvent = eventQuotationApprovalChangedEvent
        self.eventQuotationChangedEvent = eventQuotationChangedEvent
        self.eventQuotationDraftChangedEvent = eventQuotationDraftChangedEvent
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
        self.orderActionCancel = orderActionCancel
        self.orderActionCompleteWork = orderActionCompleteWork
        self.orderActionConvert = orderActionConvert
        self.orderActionDeliver = orderActionDeliver
        self.orderActionEditFulfillment = orderActionEditFulfillment
        self.orderActionStartWork = orderActionStartWork
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
        self.queryOrderGet = queryOrderGet
        self.queryOrderList = queryOrderList
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
        self.queryQuotationApprovalList = queryQuotationApprovalList
        self.queryQuotationDraftGet = queryQuotationDraftGet
        self.queryQuotationDraftList = queryQuotationDraftList
        self.queryQuotationEvaluate = queryQuotationEvaluate
        self.queryQuotationList = queryQuotationList
        self.querySalesCatalogCheck = querySalesCatalogCheck
        self.querySalesCatalogGet = querySalesCatalogGet
        self.querySalesCatalogList = querySalesCatalogList
        self.queryWorkOrderList = queryWorkOrderList
        self.queryOutcomeFailed = queryOutcomeFailed
        self.queryOutcomeSucceeded = queryOutcomeSucceeded
        self.queryResultCatalogImage = queryResultCatalogImage
        self.queryResultConfiguration = queryResultConfiguration
        self.queryResultCustomer = queryResultCustomer
        self.queryResultCustomers = queryResultCustomers
        self.queryResultDesktopAccounts = queryResultDesktopAccounts
        self.queryResultDiscountApprovals = queryResultDiscountApprovals
        self.queryResultDiscountTotal = queryResultDiscountTotal
        self.queryResultEffectivePermissions = queryResultEffectivePermissions
        self.queryResultFurnitureCategories = queryResultFurnitureCategories
        self.queryResultFurnitureReview = queryResultFurnitureReview
        self.queryResultFurnitureRevision = queryResultFurnitureRevision
        self.queryResultFurnitures = queryResultFurnitures
        self.queryResultFurnitureSelection = queryResultFurnitureSelection
        self.queryResultMaterialReferences = queryResultMaterialReferences
        self.queryResultMaterials = queryResultMaterials
        self.queryResultOrders = queryResultOrders
        self.queryResultPartCategories = queryResultPartCategories
        self.queryResultPartComposition = queryResultPartComposition
        self.queryResultPartCost = queryResultPartCost
        self.queryResultParts = queryResultParts
        self.queryResultPriceReview = queryResultPriceReview
        self.queryResultPrices = queryResultPrices
        self.queryResultProductCategories = queryResultProductCategories
        self.queryResultProductRevision = queryResultProductRevision
        self.queryResultProducts = queryResultProducts
        self.queryResultQuotationDraft = queryResultQuotationDraft
        self.queryResultQuotationDrafts = queryResultQuotationDrafts
        self.queryResultQuotationEvaluation = queryResultQuotationEvaluation
        self.queryResultQuotations = queryResultQuotations
        self.queryResultSalesCatalog = queryResultSalesCatalog
        self.queryResultSalesCatalogItem = queryResultSalesCatalogItem
        self.queryResultSalesConfiguration = queryResultSalesConfiguration
        self.queryResultScopeRelationships = queryResultScopeRelationships
        self.queryResultSellingPrice = queryResultSellingPrice
        self.queryResultWorkOrders = queryResultWorkOrders
        self.subscriptionAuthorizationPolicyChangedSubscribe = subscriptionAuthorizationPolicyChangedSubscribe
        self.subscriptionConfigChangedSubscribe = subscriptionConfigChangedSubscribe
        self.subscriptionCustomerChangedSubscribe = subscriptionCustomerChangedSubscribe
        self.subscriptionFurnitureChangedSubscribe = subscriptionFurnitureChangedSubscribe
        self.subscriptionMaterialChangedSubscribe = subscriptionMaterialChangedSubscribe
        self.subscriptionOrderChangedSubscribe = subscriptionOrderChangedSubscribe
        self.subscriptionPartChangedSubscribe = subscriptionPartChangedSubscribe
        self.subscriptionPermissionsChangedSubscribe = subscriptionPermissionsChangedSubscribe
        self.subscriptionPricingChangedSubscribe = subscriptionPricingChangedSubscribe
        self.subscriptionProductChangedSubscribe = subscriptionProductChangedSubscribe
        self.subscriptionQuotationApprovalChangedSubscribe = subscriptionQuotationApprovalChangedSubscribe
        self.subscriptionQuotationChangedSubscribe = subscriptionQuotationChangedSubscribe
        self.subscriptionQuotationDraftChangedSubscribe = subscriptionQuotationDraftChangedSubscribe
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

// MARK: - CancelOrder
public struct CancelOrder: Codable, Sendable {
    public let expectedRevision: Int
    public let orderID, reason: String

    public enum CodingKeys: String, CodingKey {
        case expectedRevision
        case orderID = "orderId"
        case reason
    }

    public init(expectedRevision: Int, orderID: String, reason: String) {
        self.expectedRevision = expectedRevision
        self.orderID = orderID
        self.reason = reason
    }
}

// MARK: - ConvertQuotation
public struct ConvertQuotation: Codable, Sendable {
    public let draftID: String
    public let expectedRevision: Int

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case expectedRevision
    }

    public init(draftID: String, expectedRevision: Int) {
        self.draftID = draftID
        self.expectedRevision = expectedRevision
    }
}

// MARK: - RecordOrderDelivery
public struct RecordOrderDelivery: Codable, Sendable {
    public let expectedRevision: Int
    public let method: AcceptanceMethod
    public let note: String?
    public let orderID, recipient: String

    public enum CodingKeys: String, CodingKey {
        case expectedRevision, method, note
        case orderID = "orderId"
        case recipient
    }

    public init(expectedRevision: Int, method: AcceptanceMethod, note: String?, orderID: String, recipient: String) {
        self.expectedRevision = expectedRevision
        self.method = method
        self.note = note
        self.orderID = orderID
        self.recipient = recipient
    }
}

public enum AcceptanceMethod: String, Codable, Sendable {
    case inPerson = "inPerson"
    case phone = "phone"
    case written = "written"
}

// MARK: - EditOrderFulfillment
public struct EditOrderFulfillment: Codable, Sendable {
    public let expectedRevision: Int
    public let note: String?
    public let orderID: String

    public enum CodingKeys: String, CodingKey {
        case expectedRevision, note
        case orderID = "orderId"
    }

    public init(expectedRevision: Int, note: String?, orderID: String) {
        self.expectedRevision = expectedRevision
        self.note = note
        self.orderID = orderID
    }
}

// MARK: - TransitionOrderWork
public struct TransitionOrderWork: Codable, Sendable {
    public let assignment: String?
    public let dueAt: Int?
    public let expectedRevision: Int
    public let orderID, workID: String

    public enum CodingKeys: String, CodingKey {
        case assignment, dueAt, expectedRevision
        case orderID = "orderId"
        case workID = "workId"
    }

    public init(assignment: String?, dueAt: Int?, expectedRevision: Int, orderID: String, workID: String) {
        self.assignment = assignment
        self.dueAt = dueAt
        self.expectedRevision = expectedRevision
        self.orderID = orderID
        self.workID = workID
    }
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

// MARK: - AcceptQuotation
public struct AcceptQuotation: Codable, Sendable {
    public let draftID: String
    public let expectedRevision: Int
    public let method: AcceptanceMethod
    public let note: String?

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case expectedRevision, method, note
    }

    public init(draftID: String, expectedRevision: Int, method: AcceptanceMethod, note: String?) {
        self.draftID = draftID
        self.expectedRevision = expectedRevision
        self.method = method
        self.note = note
    }
}

// MARK: - DecideDiscountApproval
public struct DecideDiscountApproval: Codable, Sendable {
    public let decision: DiscountDecision
    public let draftID: String
    public let expectedRevision: Int
    public let fingerprint: String
    public let quotationRevision: Int
    public let reason: String?
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case decision
        case draftID = "draftId"
        case expectedRevision, fingerprint, quotationRevision, reason
        case requestID = "requestId"
    }

    public init(decision: DiscountDecision, draftID: String, expectedRevision: Int, fingerprint: String, quotationRevision: Int, reason: String?, requestID: String) {
        self.decision = decision
        self.draftID = draftID
        self.expectedRevision = expectedRevision
        self.fingerprint = fingerprint
        self.quotationRevision = quotationRevision
        self.reason = reason
        self.requestID = requestID
    }
}

public enum DiscountDecision: String, Codable, Sendable {
    case approve = "approve"
    case reject = "reject"
}

// MARK: - RequestDiscountApproval
public struct RequestDiscountApproval: Codable, Sendable {
    public let draftID: String
    public let expectedRevision: Int

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case expectedRevision
    }

    public init(draftID: String, expectedRevision: Int) {
        self.draftID = draftID
        self.expectedRevision = expectedRevision
    }
}

// MARK: - CancelQuotation
public struct CancelQuotation: Codable, Sendable {
    public let draftID: String
    public let expectedRevision: Int
    public let reason: String

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case expectedRevision, reason
    }

    public init(draftID: String, expectedRevision: Int, reason: String) {
        self.draftID = draftID
        self.expectedRevision = expectedRevision
        self.reason = reason
    }
}

// MARK: - CancelQuotationDraft
public struct CancelQuotationDraft: Codable, Sendable {
    public let draftID: String
    public let expectedRevision: Int

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case expectedRevision
    }

    public init(draftID: String, expectedRevision: Int) {
        self.draftID = draftID
        self.expectedRevision = expectedRevision
    }
}

// MARK: - CreateQuotationDraft
public struct CreateQuotationDraft: Codable, Sendable {
    public let intent: EvaluateQuotation

    public init(intent: EvaluateQuotation) {
        self.intent = intent
    }
}

// MARK: - EvaluateQuotation
public struct EvaluateQuotation: Codable, Sendable {
    public let customer: QuotationCustomerIntent?
    public let discountBasisPoints: Int
    public let lines: [QuotationLineIntent]

    public init(customer: QuotationCustomerIntent?, discountBasisPoints: Int, lines: [QuotationLineIntent]) {
        self.customer = customer
        self.discountBasisPoints = discountBasisPoints
        self.lines = lines
    }
}

// MARK: - QuotationCustomerIntent
public struct QuotationCustomerIntent: Codable, Sendable {
    public let id: String
    public let revision: Int

    public init(id: String, revision: Int) {
        self.id = id
        self.revision = revision
    }
}

// MARK: - QuotationLineIntent
public struct QuotationLineIntent: Codable, Sendable {
    public let configuration: CheckSalesConfiguration
    public let id: String

    public init(configuration: CheckSalesConfiguration, id: String) {
        self.configuration = configuration
        self.id = id
    }
}

// MARK: - CheckSalesConfiguration
public struct CheckSalesConfiguration: Codable, Sendable {
    public let dimensions: FurnitureDimensions?
    public let selection: PriceSelection

    public init(dimensions: FurnitureDimensions?, selection: PriceSelection) {
        self.dimensions = dimensions
        self.selection = selection
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

// MARK: - UpdateQuotationDraft
public struct UpdateQuotationDraft: Codable, Sendable {
    public let draftID: String
    public let expectedRevision: Int
    public let intent: EvaluateQuotation

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case expectedRevision, intent
    }

    public init(draftID: String, expectedRevision: Int, intent: EvaluateQuotation) {
        self.draftID = draftID
        self.expectedRevision = expectedRevision
        self.intent = intent
    }
}

// MARK: - IssueQuotation
public struct IssueQuotation: Codable, Sendable {
    public let draftID: String
    public let expectedDraftRevision, expectedRevision: Int

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case expectedDraftRevision, expectedRevision
    }

    public init(draftID: String, expectedDraftRevision: Int, expectedRevision: Int) {
        self.draftID = draftID
        self.expectedDraftRevision = expectedDraftRevision
        self.expectedRevision = expectedRevision
    }
}

// MARK: - SetQuotationValidity
public struct SetQuotationValidity: Codable, Sendable {
    public let draftID: String
    public let expectedRevision, validityDays: Int

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case expectedRevision, validityDays
    }

    public init(draftID: String, expectedRevision: Int, validityDays: Int) {
        self.draftID = draftID
        self.expectedRevision = expectedRevision
        self.validityDays = validityDays
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
    public let syncState: SyncState
    public let updatedAt: Int

    public init(address: String?, id: String, name: String, notes: String?, phone: String, revision: Int, scope: ScopeRef, status: CustomerStatus, syncState: SyncState, updatedAt: Int) {
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

public enum SyncState: String, Codable, Sendable {
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

// MARK: - DiscountApproval
public struct DiscountApproval: Codable, Sendable {
    public let correlationID: String
    public let decidedAt: Int?
    public let decider: String?
    public let fingerprint, organizationID: String
    public let proposedValidUntil: Int
    public let quotation: QuotationDraftSnapshot
    public let reason: String?
    public let requestedAt: Int
    public let requester, requestID: String
    public let revision: Int
    public let scope: ScopeRef
    public let state: DiscountApprovalState
    public let validityDays: Int

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case decidedAt, decider, fingerprint
        case organizationID = "organizationId"
        case proposedValidUntil, quotation, reason, requestedAt, requester
        case requestID = "requestId"
        case revision, scope, state, validityDays
    }

    public init(correlationID: String, decidedAt: Int?, decider: String?, fingerprint: String, organizationID: String, proposedValidUntil: Int, quotation: QuotationDraftSnapshot, reason: String?, requestedAt: Int, requester: String, requestID: String, revision: Int, scope: ScopeRef, state: DiscountApprovalState, validityDays: Int) {
        self.correlationID = correlationID
        self.decidedAt = decidedAt
        self.decider = decider
        self.fingerprint = fingerprint
        self.organizationID = organizationID
        self.proposedValidUntil = proposedValidUntil
        self.quotation = quotation
        self.reason = reason
        self.requestedAt = requestedAt
        self.requester = requester
        self.requestID = requestID
        self.revision = revision
        self.scope = scope
        self.state = state
        self.validityDays = validityDays
    }
}

// MARK: - QuotationDraftSnapshot
public struct QuotationDraftSnapshot: Codable, Sendable {
    public let cancelled: Bool?
    public let evaluation: QuotationEvaluation
    public let id: String
    public let intent: EvaluateQuotation
    public let revision: Int

    public init(cancelled: Bool?, evaluation: QuotationEvaluation, id: String, intent: EvaluateQuotation, revision: Int) {
        self.cancelled = cancelled
        self.evaluation = evaluation
        self.id = id
        self.intent = intent
        self.revision = revision
    }
}

// MARK: - QuotationEvaluation
public struct QuotationEvaluation: Codable, Sendable {
    public let currency: String
    public let customer: QuotationCustomerSnapshot?
    public let discountBasisPoints: Int
    public let errors: [QuotationFieldError]
    public let lines: [EvaluatedQuotationLine]
    public let scope: ScopeRef
    /// Cache evaluation is never evidence that issuance can succeed online.
    public let serverAvailable: Bool
    /// Present only when every field and checked calculation is valid.
    public let totals: DiscountTotal?

    public init(currency: String, customer: QuotationCustomerSnapshot?, discountBasisPoints: Int, errors: [QuotationFieldError], lines: [EvaluatedQuotationLine], scope: ScopeRef, serverAvailable: Bool, totals: DiscountTotal?) {
        self.currency = currency
        self.customer = customer
        self.discountBasisPoints = discountBasisPoints
        self.errors = errors
        self.lines = lines
        self.scope = scope
        self.serverAvailable = serverAvailable
        self.totals = totals
    }
}

// MARK: - QuotationCustomerSnapshot
public struct QuotationCustomerSnapshot: Codable, Sendable {
    public let address: String?
    public let id, name, phone: String
    public let revision: Int

    public init(address: String?, id: String, name: String, phone: String, revision: Int) {
        self.address = address
        self.id = id
        self.name = name
        self.phone = phone
        self.revision = revision
    }
}

// MARK: - EvaluatedQuotationLine
public struct EvaluatedQuotationLine: Codable, Sendable {
    public let colorID, colorName: String?
    public let description: String
    public let dimensions: FurnitureDimensions?
    public let handleID, handleName: String?
    public let id, name: String
    public let price: SellingPrice
    public let quantity: Int
    public let variantName: String

    public enum CodingKeys: String, CodingKey {
        case colorID = "colorId"
        case colorName, description, dimensions
        case handleID = "handleId"
        case handleName, id, name, price, quantity, variantName
    }

    public init(colorID: String?, colorName: String?, description: String, dimensions: FurnitureDimensions?, handleID: String?, handleName: String?, id: String, name: String, price: SellingPrice, quantity: Int, variantName: String) {
        self.colorID = colorID
        self.colorName = colorName
        self.description = description
        self.dimensions = dimensions
        self.handleID = handleID
        self.handleName = handleName
        self.id = id
        self.name = name
        self.price = price
        self.quantity = quantity
        self.variantName = variantName
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

public enum DiscountApprovalState: String, Codable, Sendable {
    case approved = "approved"
    case invalidated = "invalidated"
    case pending = "pending"
    case rejected = "rejected"
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

// MARK: - OrderRecord
public struct OrderRecord: Codable, Sendable {
    public let cancellationReason: String?
    public let changedAt: Int
    public let changedBy: String
    public let createdAt: Int
    public let delivery: OrderDelivery?
    public let fulfillmentNote: String?
    public let id, number, organizationID: String
    public let permittedActions: [OrderPermittedAction]
    public let revision: Int
    public let scope: ScopeRef
    /// Exact accepted quotation, including its issued prices and customer snapshot.
    public let source: QuotationRecord
    public let state: OrderState
    public let work: [OrderWork]

    public enum CodingKeys: String, CodingKey {
        case cancellationReason, changedAt, changedBy, createdAt, delivery, fulfillmentNote, id, number
        case organizationID = "organizationId"
        case permittedActions, revision, scope, source, state, work
    }

    public init(cancellationReason: String?, changedAt: Int, changedBy: String, createdAt: Int, delivery: OrderDelivery?, fulfillmentNote: String?, id: String, number: String, organizationID: String, permittedActions: [OrderPermittedAction], revision: Int, scope: ScopeRef, source: QuotationRecord, state: OrderState, work: [OrderWork]) {
        self.cancellationReason = cancellationReason
        self.changedAt = changedAt
        self.changedBy = changedBy
        self.createdAt = createdAt
        self.delivery = delivery
        self.fulfillmentNote = fulfillmentNote
        self.id = id
        self.number = number
        self.organizationID = organizationID
        self.permittedActions = permittedActions
        self.revision = revision
        self.scope = scope
        self.source = source
        self.state = state
        self.work = work
    }
}

// MARK: - OrderDelivery
public struct OrderDelivery: Codable, Sendable {
    public let actor: String
    public let deliveredAt: Int
    public let id: String
    public let method: AcceptanceMethod
    public let note: String?
    public let recipient: String

    public init(actor: String, deliveredAt: Int, id: String, method: AcceptanceMethod, note: String?, recipient: String) {
        self.actor = actor
        self.deliveredAt = deliveredAt
        self.id = id
        self.method = method
        self.note = note
        self.recipient = recipient
    }
}

public enum OrderPermittedAction: String, Codable, Sendable {
    case cancel = "cancel"
    case completeWork = "completeWork"
    case deliver = "deliver"
    case editFulfillment = "editFulfillment"
    case startWork = "startWork"
}

/// Exact accepted quotation, including its issued prices and customer snapshot.
// MARK: - QuotationRecord
public struct QuotationRecord: Codable, Sendable {
    public let acceptance: QuotationAcceptance?
    public let approvalFingerprint, approvalRequestID, cancellationReason: String?
    public let changedAt: Int
    public let changedBy: String
    public let documentRevision: Int
    public let issuedAt: Int?
    public let number: String?
    public let organizationID: String
    /// Derived from the authenticated actor and current server state, never client role flags.
    public let permittedActions: [QuotationPermittedAction]
    public let quotation: QuotationDraftSnapshot
    public let revision: Int
    public let scope: ScopeRef
    public let state: QuotationState
    public let validityDays: Int
    public let validUntil: Int?

    public enum CodingKeys: String, CodingKey {
        case acceptance, approvalFingerprint
        case approvalRequestID = "approvalRequestId"
        case cancellationReason, changedAt, changedBy, documentRevision, issuedAt, number
        case organizationID = "organizationId"
        case permittedActions, quotation, revision, scope, state, validityDays, validUntil
    }

    public init(acceptance: QuotationAcceptance?, approvalFingerprint: String?, approvalRequestID: String?, cancellationReason: String?, changedAt: Int, changedBy: String, documentRevision: Int, issuedAt: Int?, number: String?, organizationID: String, permittedActions: [QuotationPermittedAction], quotation: QuotationDraftSnapshot, revision: Int, scope: ScopeRef, state: QuotationState, validityDays: Int, validUntil: Int?) {
        self.acceptance = acceptance
        self.approvalFingerprint = approvalFingerprint
        self.approvalRequestID = approvalRequestID
        self.cancellationReason = cancellationReason
        self.changedAt = changedAt
        self.changedBy = changedBy
        self.documentRevision = documentRevision
        self.issuedAt = issuedAt
        self.number = number
        self.organizationID = organizationID
        self.permittedActions = permittedActions
        self.quotation = quotation
        self.revision = revision
        self.scope = scope
        self.state = state
        self.validityDays = validityDays
        self.validUntil = validUntil
    }
}

// MARK: - QuotationAcceptance
public struct QuotationAcceptance: Codable, Sendable {
    public let acceptedAt: Int
    public let actor: String
    public let documentRevision: Int
    public let method: AcceptanceMethod
    public let note: String?

    public init(acceptedAt: Int, actor: String, documentRevision: Int, method: AcceptanceMethod, note: String?) {
        self.acceptedAt = acceptedAt
        self.actor = actor
        self.documentRevision = documentRevision
        self.method = method
        self.note = note
    }
}

public enum QuotationPermittedAction: String, Codable, Sendable {
    case accept = "accept"
    case cancel = "cancel"
    case convert = "convert"
    case edit = "edit"
    case issue = "issue"
    case manageValidity = "manageValidity"
    case print = "print"
    case requestApproval = "requestApproval"
    case revise = "revise"
}

public enum QuotationState: String, Codable, Sendable {
    case accepted = "accepted"
    case cancelled = "cancelled"
    case converted = "converted"
    case draft = "draft"
    case expired = "expired"
    case issued = "issued"
    case pendingApproval = "pendingApproval"
}

public enum OrderState: String, Codable, Sendable {
    case cancelled = "cancelled"
    case confirmed = "confirmed"
    case delivered = "delivered"
    case inProduction = "inProduction"
    case ready = "ready"
}

// MARK: - OrderWork
public struct OrderWork: Codable, Sendable {
    public let assignment: String?
    public let dueAt: Int?
    public let id: String
    public let lineIDS: [String]
    public let number: String
    public let state: WorkState

    public enum CodingKeys: String, CodingKey {
        case assignment, dueAt, id
        case lineIDS = "lineIds"
        case number, state
    }

    public init(assignment: String?, dueAt: Int?, id: String, lineIDS: [String], number: String, state: WorkState) {
        self.assignment = assignment
        self.dueAt = dueAt
        self.id = id
        self.lineIDS = lineIDS
        self.number = number
        self.state = state
    }
}

public enum WorkState: String, Codable, Sendable {
    case cancelled = "cancelled"
    case completed = "completed"
    case inProgress = "inProgress"
    case planned = "planned"
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

// MARK: - QuotationDraft
public struct QuotationDraft: Codable, Sendable {
    public let conflict: QuotationDraftConflict?
    public let permittedActions: [QuotationPermittedAction]?
    public let scope: ScopeRef
    public let snapshot: QuotationDraftSnapshot
    public let syncState: SyncState
    public let updatedAt: Int

    public init(conflict: QuotationDraftConflict?, permittedActions: [QuotationPermittedAction]?, scope: ScopeRef, snapshot: QuotationDraftSnapshot, syncState: SyncState, updatedAt: Int) {
        self.conflict = conflict
        self.permittedActions = permittedActions
        self.scope = scope
        self.snapshot = snapshot
        self.syncState = syncState
        self.updatedAt = updatedAt
    }
}

// MARK: - QuotationDraftConflict
public struct QuotationDraftConflict: Codable, Sendable {
    /// Retained authoritative input; the local snapshot remains visible.
    public let remote: ChangeRecord?
    public let serverConflictID: String?

    public enum CodingKeys: String, CodingKey {
        case remote
        case serverConflictID = "serverConflictId"
    }

    public init(remote: ChangeRecord?, serverConflictID: String?) {
        self.remote = remote
        self.serverConflictID = serverConflictID
    }
}

// MARK: - ChangeRecord
public struct ChangeRecord: Codable, Sendable {
    public let baseRevision: Int?
    public let changedAt: Int
    public let changeID, idempotencyKey: String
    public let merge: MergeMetadata?
    public let operation: ChangeOperation
    public let payload: EncodedDomainPayload?
    public let recordID: String
    public let revision: Int
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case baseRevision, changedAt
        case changeID = "changeId"
        case idempotencyKey, merge, operation, payload
        case recordID = "recordId"
        case revision, scope
    }

    public init(baseRevision: Int?, changedAt: Int, changeID: String, idempotencyKey: String, merge: MergeMetadata?, operation: ChangeOperation, payload: EncodedDomainPayload?, recordID: String, revision: Int, scope: ScopeRef) {
        self.baseRevision = baseRevision
        self.changedAt = changedAt
        self.changeID = changeID
        self.idempotencyKey = idempotencyKey
        self.merge = merge
        self.operation = operation
        self.payload = payload
        self.recordID = recordID
        self.revision = revision
        self.scope = scope
    }
}

// MARK: - MergeMetadata
public struct MergeMetadata: Codable, Sendable {
    public let commonAncestorRevision: Int?
    public let mergedAt: Int
    public let sourceChanges: [String]
    public let strategy: MergeStrategy

    public init(commonAncestorRevision: Int?, mergedAt: Int, sourceChanges: [String], strategy: MergeStrategy) {
        self.commonAncestorRevision = commonAncestorRevision
        self.mergedAt = mergedAt
        self.sourceChanges = sourceChanges
        self.strategy = strategy
    }
}

public enum MergeStrategy: String, Codable, Sendable {
    case domainMerge = "domainMerge"
    case keepLocal = "keepLocal"
    case keepRemote = "keepRemote"
}

public enum ChangeOperation: String, Codable, Sendable {
    case tombstone = "tombstone"
    case upsert = "upsert"
}

// MARK: - EncodedDomainPayload
public struct EncodedDomainPayload: Codable, Sendable {
    public let base64, schemaID: String
    public let schemaVersion: Int

    public enum CodingKeys: String, CodingKey {
        case base64
        case schemaID = "schemaId"
        case schemaVersion
    }

    public init(base64: String, schemaID: String, schemaVersion: Int) {
        self.base64 = base64
        self.schemaID = schemaID
        self.schemaVersion = schemaVersion
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

// MARK: - OrderNotice
public struct OrderNotice: Codable, Sendable {
    public let orderID: String
    public let revision: Int
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case orderID = "orderId"
        case revision, scope
    }

    public init(orderID: String, revision: Int, scope: ScopeRef) {
        self.orderID = orderID
        self.revision = revision
        self.scope = scope
    }
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

// MARK: - DiscountApprovalNotice
public struct DiscountApprovalNotice: Codable, Sendable {
    public let draftID: String
    public let revision: Int
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case revision, scope
    }

    public init(draftID: String, revision: Int, scope: ScopeRef) {
        self.draftID = draftID
        self.revision = revision
        self.scope = scope
    }
}

// MARK: - QuotationNotice
public struct QuotationNotice: Codable, Sendable {
    public let draftID: String
    public let revision: Int
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
        case revision, scope
    }

    public init(draftID: String, revision: Int, scope: ScopeRef) {
        self.draftID = draftID
        self.revision = revision
        self.scope = scope
    }
}

// MARK: - QuotationDraftChangeNotice
public struct QuotationDraftChangeNotice: Codable, Sendable {
    public let changedAt: Int
    public let changeID, draftID: String
    public let revision: Int
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case changedAt
        case changeID = "changeId"
        case draftID = "draftId"
        case revision, scope
    }

    public init(changedAt: Int, changeID: String, draftID: String, revision: Int, scope: ScopeRef) {
        self.changedAt = changedAt
        self.changeID = changeID
        self.draftID = draftID
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

// MARK: - GetOrder
public struct GetOrder: Codable, Sendable {
    public let orderID: String

    public enum CodingKeys: String, CodingKey {
        case orderID = "orderId"
    }

    public init(orderID: String) {
        self.orderID = orderID
    }
}

// MARK: - ListOrders
public struct ListOrders: Codable, Sendable {
    public let after: String?
    public let limit: Int

    public init(after: String?, limit: Int) {
        self.after = after
        self.limit = limit
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

// MARK: - ListDiscountApprovals
public struct ListDiscountApprovals: Codable, Sendable {
    public let after: String?
    public let limit: Int

    public init(after: String?, limit: Int) {
        self.after = after
        self.limit = limit
    }
}

// MARK: - GetQuotationDraft
public struct GetQuotationDraft: Codable, Sendable {
    public let draftID: String

    public enum CodingKeys: String, CodingKey {
        case draftID = "draftId"
    }

    public init(draftID: String) {
        self.draftID = draftID
    }
}

// MARK: - ListQuotationDrafts
public struct ListQuotationDrafts: Codable, Sendable {
    public let after: String?
    public let limit: Int

    public init(after: String?, limit: Int) {
        self.after = after
        self.limit = limit
    }
}

// MARK: - ListQuotations
public struct ListQuotations: Codable, Sendable {
    public let after: String?
    public let limit: Int

    public init(after: String?, limit: Int) {
        self.after = after
        self.limit = limit
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

// MARK: - DiscountApprovalPage
public struct DiscountApprovalPage: Codable, Sendable {
    public let items: [DiscountApproval]
    public let next: String?

    public init(items: [DiscountApproval], next: String?) {
        self.items = items
        self.next = next
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

// MARK: - OrderPage
public struct OrderPage: Codable, Sendable {
    public let items: [OrderRecord]
    public let next: String?
    public let pending: [OrderPending]?
    public let serverAvailable: Bool

    public init(items: [OrderRecord], next: String?, pending: [OrderPending]?, serverAvailable: Bool) {
        self.items = items
        self.next = next
        self.pending = pending
        self.serverAvailable = serverAvailable
    }
}

// MARK: - OrderPending
public struct OrderPending: Codable, Sendable {
    public let rejectedCode: String?
    public let request: ConfirmOrder

    public init(rejectedCode: String?, request: ConfirmOrder) {
        self.rejectedCode = rejectedCode
        self.request = request
    }
}

// MARK: - ConfirmOrder
public struct ConfirmOrder: Codable, Sendable {
    public let action: [String: JSONAny]
    public let idempotencyKey: String
    public let scope: ScopeRef

    public init(action: [String: JSONAny], idempotencyKey: String, scope: ScopeRef) {
        self.action = action
        self.idempotencyKey = idempotencyKey
        self.scope = scope
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

// MARK: - QuotationDraftPage
public struct QuotationDraftPage: Codable, Sendable {
    public let items: [QuotationDraft]
    public let next: String?

    public init(items: [QuotationDraft], next: String?) {
        self.items = items
        self.next = next
    }
}

// MARK: - QuotationPage
public struct QuotationPage: Codable, Sendable {
    public let items: [QuotationRecord]
    public let next: String?
    public let serverAvailable: Bool

    public init(items: [QuotationRecord], next: String?, serverAvailable: Bool) {
        self.items = items
        self.next = next
        self.serverAvailable = serverAvailable
    }
}

// MARK: - SalesCatalogPage
public struct SalesCatalogPage: Codable, Sendable {
    public let categories: [String]
    public let items: [CatalogEntry]
    public let next: String?
    public let serverAvailable: Bool

    public init(categories: [String], items: [CatalogEntry], next: String?, serverAvailable: Bool) {
        self.categories = categories
        self.items = items
        self.next = next
        self.serverAvailable = serverAvailable
    }
}

/// Server-confirmed sales projection. Private definitions and cost dependencies never occur
/// here.
// MARK: - CatalogEntry
public struct CatalogEntry: Codable, Sendable {
    public let categoryName: String
    public let colors: [FurnitureOption]
    public let customization: FurnitureCustomization?
    public let description: String
    public let dimensions: FurnitureDimensions?
    public let handles: [FurnitureOption]
    public let image: CatalogImageRef?
    public let name: String
    public let price: PublishedPrice
    public let variantName: String

    public init(categoryName: String, colors: [FurnitureOption], customization: FurnitureCustomization?, description: String, dimensions: FurnitureDimensions?, handles: [FurnitureOption], image: CatalogImageRef?, name: String, price: PublishedPrice, variantName: String) {
        self.categoryName = categoryName
        self.colors = colors
        self.customization = customization
        self.description = description
        self.dimensions = dimensions
        self.handles = handles
        self.image = image
        self.name = name
        self.price = price
        self.variantName = variantName
    }
}

// MARK: - SalesCatalogDetails
public struct SalesCatalogDetails: Codable, Sendable {
    public let serverAvailable: Bool
    public let variants: [CatalogEntry]

    public init(serverAvailable: Bool, variants: [CatalogEntry]) {
        self.serverAvailable = serverAvailable
        self.variants = variants
    }
}

// MARK: - SalesConfiguration
public struct SalesConfiguration: Codable, Sendable {
    public let additionsYer: Int
    public let dimensions: FurnitureDimensions?
    public let entry: CatalogEntry
    public let price: SellingPrice
    public let serverAvailable: Bool

    public init(additionsYer: Int, dimensions: FurnitureDimensions?, entry: CatalogEntry, price: SellingPrice, serverAvailable: Bool) {
        self.additionsYer = additionsYer
        self.dimensions = dimensions
        self.entry = entry
        self.price = price
        self.serverAvailable = serverAvailable
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

// MARK: - WorkOrderPage
public struct WorkOrderPage: Codable, Sendable {
    public let items: [WorkOrderRecord]
    public let next: String?
    public let pending: [OrderPending]
    public let serverAvailable: Bool

    public init(items: [WorkOrderRecord], next: String?, pending: [OrderPending], serverAvailable: Bool) {
        self.items = items
        self.next = next
        self.pending = pending
        self.serverAvailable = serverAvailable
    }
}

// MARK: - WorkOrderRecord
public struct WorkOrderRecord: Codable, Sendable {
    public let assignment: String?
    public let canComplete, canStart: Bool
    public let customer: String
    public let dueAt: Int?
    public let furniture: [WorkOrderFurniture]
    public let id: String
    public let note: String?
    public let number, orderID, orderNumber, organizationID: String
    /// Order aggregate revision used by all production commands.
    public let revision: Int
    public let scope: ScopeRef
    public let state: WorkState

    public enum CodingKeys: String, CodingKey {
        case assignment, canComplete, canStart, customer, dueAt, furniture, id, note, number
        case orderID = "orderId"
        case orderNumber
        case organizationID = "organizationId"
        case revision, scope, state
    }

    public init(assignment: String?, canComplete: Bool, canStart: Bool, customer: String, dueAt: Int?, furniture: [WorkOrderFurniture], id: String, note: String?, number: String, orderID: String, orderNumber: String, organizationID: String, revision: Int, scope: ScopeRef, state: WorkState) {
        self.assignment = assignment
        self.canComplete = canComplete
        self.canStart = canStart
        self.customer = customer
        self.dueAt = dueAt
        self.furniture = furniture
        self.id = id
        self.note = note
        self.number = number
        self.orderID = orderID
        self.orderNumber = orderNumber
        self.organizationID = organizationID
        self.revision = revision
        self.scope = scope
        self.state = state
    }
}

// MARK: - WorkOrderFurniture
public struct WorkOrderFurniture: Codable, Sendable {
    public let colorID, colorName: String?
    public let dimensions: FurnitureDimensions
    public let handleID, handleName: String?
    public let lineID, name: String
    public let parts: [WorkOrderPart]
    public let quantity: Int
    public let reference: FurnitureReference
    public let variantName: String

    public enum CodingKeys: String, CodingKey {
        case colorID = "colorId"
        case colorName, dimensions
        case handleID = "handleId"
        case handleName
        case lineID = "lineId"
        case name, parts, quantity, reference, variantName
    }

    public init(colorID: String?, colorName: String?, dimensions: FurnitureDimensions, handleID: String?, handleName: String?, lineID: String, name: String, parts: [WorkOrderPart], quantity: Int, reference: FurnitureReference, variantName: String) {
        self.colorID = colorID
        self.colorName = colorName
        self.dimensions = dimensions
        self.handleID = handleID
        self.handleName = handleName
        self.lineID = lineID
        self.name = name
        self.parts = parts
        self.quantity = quantity
        self.reference = reference
        self.variantName = variantName
    }
}

// MARK: - WorkOrderPart
public struct WorkOrderPart: Codable, Sendable {
    public let name: String
    /// Total count for this accepted Furniture line, calculated in Rust.
    public let quantity: Int
    public let reference: CompositionReference

    public init(name: String, quantity: Int, reference: CompositionReference) {
        self.name = name
        self.quantity = quantity
        self.reference = reference
    }
}

// MARK: - GetSalesCatalogItem
public struct GetSalesCatalogItem: Codable, Sendable {
    public let target: [String: JSONAny]

    public init(target: [String: JSONAny]) {
        self.target = target
    }
}

// MARK: - ListSalesCatalog
public struct ListSalesCatalog: Codable, Sendable {
    public let after, category: String?
    public let limit: Int
    public let term: String

    public init(after: String?, category: String?, limit: Int, term: String) {
        self.after = after
        self.category = category
        self.limit = limit
        self.term = term
    }
}

// MARK: - ListWorkOrders
public struct ListWorkOrders: Codable, Sendable {
    public let after: String?
    public let limit: Int
    public let orderID: String?

    public enum CodingKeys: String, CodingKey {
        case after, limit
        case orderID = "orderId"
    }

    public init(after: String?, limit: Int, orderID: String?) {
        self.after = after
        self.limit = limit
        self.orderID = orderID
    }
}

// MARK: - OrderChanges
public struct OrderChanges: Codable, Sendable {

    public init() {
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

// MARK: - DiscountApprovalChanges
public struct DiscountApprovalChanges: Codable, Sendable {

    public init() {
    }
}

// MARK: - QuotationChanges
public struct QuotationChanges: Codable, Sendable {

    public init() {
    }
}

// MARK: - QuotationDraftChanges
public struct QuotationDraftChanges: Codable, Sendable {

    public init() {
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
