use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use eitmad_contracts::{
    PROTOCOL_VERSION,
    administration::{BackupStatus, DiagnosticSummary, MigrationStatus, SupportWorkflow},
    catalog::ProtocolCatalog,
    config::{ConfigReadValue, ConfigSnapshot},
    errors::{
        ContractError, ErrorCode, ErrorParameter, ErrorParameterName, ErrorParameterValue,
        MessageId, RetryDisposition,
    },
    identity::{
        AuthenticatedIdentity, AuthorizationContext, DeviceId, PrincipalId, PrincipalKind, ScopeId,
        ScopeKind, ScopeRef, SessionId, TenantId,
    },
    ipc::{IpcClientMessage, IpcServerMessage},
    observability::{
        ComponentId, DataClassification, ObservationEventId, ObservationFieldName,
        ObservationSeverity, ObservationValueKind,
    },
    permissions::EffectivePermissions,
    queries::{GetConfiguration, Query, QueryResult},
    relay::{RelayFailureReport, RelayHealth, RelaySessionMetadata},
    runtime::{DiagnosticReport, LifecycleSnapshot},
    server::{RegisterBranchRequest, RegisteredBranch, ServerClientMessage, ServerMessage},
    sync::{SyncMessage, SyncStatus},
    sync_transport::SyncTransportFrame,
    transport::{
        CausationId, CommandEnvelope, CommandResponseEnvelope, CorrelationId, EventEnvelope,
        QueryEnvelope, QueryOutcome, QueryResponseEnvelope, RequestId, SubscriptionEnvelope,
        UnixMillis,
    },
    updates::{SignedUpdateManifest, UpdateCheckOutcome, UpdateClientProfile},
    versioning::{NegotiationOutcome, PeerHello},
};
use schemars::{JsonSchema, generate::SchemaSettings};
use serde::Serialize;
use uuid::Uuid;

#[derive(JsonSchema)]
#[allow(dead_code)]
struct ContractSchemaRoot {
    guided_windows_setup: eitmad_contracts::setup::GuidedWindowsSetup,
    windows_setup_bundle: eitmad_contracts::setup::WindowsSetupBundle,
    server_backup_configuration: eitmad_contracts::backup::ServerBackupConfiguration,
    server_backup_status: eitmad_contracts::backup::ServerBackupStatus,
    manage_server_backup: eitmad_contracts::backup::ManageServerBackup,
    server_backup_management: eitmad_contracts::backup::ServerBackupManagement,
    open_pairing: eitmad_contracts::pairing::OpenPairing,
    opened_pairing: eitmad_contracts::pairing::OpenedPairing,
    start_pairing: eitmad_contracts::pairing::StartPairing,
    pairing_challenge: eitmad_contracts::pairing::PairingChallenge,
    complete_pairing: eitmad_contracts::pairing::CompletePairing,
    pairing_receipt: eitmad_contracts::pairing::PairingReceipt,
    install_windows: eitmad_contracts::setup::InstallWindows,
    install_windows_server: eitmad_contracts::setup::InstallWindowsServer,
    install_windows_client: eitmad_contracts::setup::InstallWindowsClient,
    windows_install_progress: eitmad_contracts::setup::WindowsInstallProgress,
    read_windows_installation: eitmad_contracts::setup::ReadWindowsInstallation,
    windows_installation_state: eitmad_contracts::setup::WindowsInstallationState,
    prepare_windows_workshop: eitmad_contracts::setup::PrepareWindowsWorkshop,
    prepared_windows_workshop: eitmad_contracts::setup::PreparedWindowsWorkshop,
    initialize_workshop: eitmad_contracts::setup::InitializeWorkshop,
    initialized_workshop: eitmad_contracts::setup::InitializedWorkshop,
    reconcile_customer_resolution: eitmad_contracts::sync::ReconcileResolution<
        eitmad_contracts::customer::ConfirmCustomerResolution,
    >,
    reconcile_quotation_draft_resolution: eitmad_contracts::sync::ReconcileResolution<
        eitmad_contracts::quotation_draft::ConfirmQuotationDraftResolution,
    >,
    read_quotation_draft_recovery: eitmad_contracts::quotation_draft::ReadQuotationDraftRecovery,
    read_quotation_draft_review: eitmad_contracts::quotation_draft::ReadQuotationDraftReview,
    confirm_quotation_draft_resolution:
        eitmad_contracts::quotation_draft::ConfirmQuotationDraftResolution,
    read_customers: eitmad_contracts::customer::ReadCustomers,
    read_customer_conflicts: eitmad_contracts::customer::ReadCustomerConflicts,
    confirm_customer_resolution: eitmad_contracts::customer::ConfirmCustomerResolution,
    watch_server_accounts: eitmad_contracts::server_accounts::WatchServerAccounts,
    server_account_watch: eitmad_contracts::server_accounts::ServerAccountWatch,
    change_server_account: eitmad_contracts::server_accounts::ChangeServerAccount,
    list_server_accounts: eitmad_contracts::server_accounts::ListServerAccounts,
    server_account_page: eitmad_contracts::server_accounts::ServerAccountPage,
    server_account_result: eitmad_contracts::server_accounts::ServerAccountResult,
    activate_account_request: eitmad_contracts::server::ActivateAccountRequest,
    authentication_result: eitmad_contracts::server::AuthenticationResult,
    desktop_enrollment_profile: eitmad_contracts::server::DesktopEnrollmentProfile,
    confirm_order: eitmad_contracts::order::ConfirmOrder,
    read_orders: eitmad_contracts::order::ReadOrders,
    read_work_orders: eitmad_contracts::work_order::ReadWorkOrders,
    confirm_quotation: eitmad_contracts::quotation_lifecycle::ConfirmQuotation,
    read_quotations: eitmad_contracts::quotation_lifecycle::ReadQuotations,
    confirm_discount_approval: eitmad_contracts::quotation_approval::ConfirmDiscountApproval,
    read_discount_approvals: eitmad_contracts::quotation_approval::ReadDiscountApprovals,
    catalog_entry: eitmad_contracts::catalog_revision::CatalogEntry,
    synchronize_catalog_revisions: eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions,
    confirm_price: eitmad_contracts::pricing::ConfirmPrice,
    read_published_prices: eitmad_contracts::pricing::ReadPublishedPrices,
    published_price_page: eitmad_contracts::pricing::PublishedPricePage,
    ipc_client_message: IpcClientMessage,
    ipc_server_message: IpcServerMessage,
    command_request: CommandEnvelope,
    query_request: QueryEnvelope,
    subscription_request: SubscriptionEnvelope,
    command_response: CommandResponseEnvelope,
    query_response: QueryResponseEnvelope,
    event: EventEnvelope,
    peer_hello: PeerHello,
    negotiation: NegotiationOutcome,
    sync_message: SyncMessage,
    sync_transport_frame: SyncTransportFrame,
    server_client_message: ServerClientMessage,
    server_message: ServerMessage,
    upload_catalog_image: eitmad_contracts::catalog_image::UploadCatalogImage,
    download_catalog_image: eitmad_contracts::catalog_image::DownloadCatalogImage,
    register_branch_request: RegisterBranchRequest,
    registered_branch: RegisteredBranch,
    relay_session: RelaySessionMetadata,
    relay_failure: RelayFailureReport,
    relay_health: RelayHealth,
    signed_update_manifest: SignedUpdateManifest,
    update_client_profile: UpdateClientProfile,
    update_check_outcome: UpdateCheckOutcome,
    administration_diagnostics: DiagnosticSummary,
    administration_backup_status: BackupStatus,
    administration_migration_status: MigrationStatus,
    administration_support_workflow: SupportWorkflow,
    sync_status: SyncStatus,
    effective_permissions: EffectivePermissions,
    lifecycle_snapshot: LifecycleSnapshot,
    diagnostic_report: DiagnosticReport,
    observation_event_id: ObservationEventId,
    observation_field_name: ObservationFieldName,
    observation_component_id: ComponentId,
    observation_severity: ObservationSeverity,
    observation_classification: DataClassification,
    observation_value_kind: ObservationValueKind,
    catalog: ProtocolCatalog,
}

#[derive(JsonSchema)]
#[allow(dead_code)]
struct NativeShellSchemaRoot {
    windows_setup_role: eitmad_contracts::setup::WindowsSetupRole,
    guided_windows_setup: eitmad_contracts::setup::GuidedWindowsSetup,
    manage_server_backup: eitmad_contracts::backup::ManageServerBackup,
    server_backup_management: eitmad_contracts::backup::ServerBackupManagement,
    windows_installation_state: eitmad_contracts::setup::WindowsInstallationState,
    ipc_client_message: IpcClientMessage,
    ipc_server_message: IpcServerMessage,
    lifecycle_snapshot: LifecycleSnapshot,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConformanceFixture {
    query: QueryEnvelope,
    query_response: QueryResponseEnvelope,
    structured_error: ContractError,
    mixed_direction_samples: Vec<&'static str>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: export_contracts <output-directory>")?;

    write_json(
        &output.join("contract-v1.schema.json"),
        &SchemaSettings::draft07()
            .into_generator()
            .into_root_schema_for::<ContractSchemaRoot>(),
    )?;
    write_json(
        &output.join("protocol-v1.json"),
        &ProtocolCatalog::current(),
    )?;
    write_json(
        &output.join("native-shell-v1.schema.json"),
        &SchemaSettings::draft07()
            .into_generator()
            .into_root_schema_for::<NativeShellSchemaRoot>(),
    )?;
    write_json(&output.join("protocol-v1.fixture.json"), &fixture()?)?;
    write_text(
        &output.join("contracts-v1.md"),
        &render_reference(&ProtocolCatalog::current()),
    )?;
    Ok(())
}

fn fixture() -> Result<ConformanceFixture, Box<dyn std::error::Error>> {
    let scope = ScopeRef {
        kind: ScopeKind::parse("organization")?,
        id: ScopeId::new(uuid(1)),
    };
    let authorization = AuthorizationContext {
        session_id: SessionId::new(uuid(2)),
        identity: AuthenticatedIdentity {
            principal_id: PrincipalId::new(uuid(3)),
            principal_kind: PrincipalKind::User,
            device_id: Some(DeviceId::new(uuid(4))),
            service_id: None,
        },
        tenant_id: TenantId::new(uuid(8)),
        workspace_id: None,
        scope: scope.clone(),
    };
    let correlation_id = CorrelationId::new(uuid(5));
    let query = QueryEnvelope {
        protocol_version: PROTOCOL_VERSION,
        request_id: RequestId::new(uuid(6)),
        correlation_id,
        causation_id: Some(CausationId::new(uuid(7))),
        authorization,
        deadline: UnixMillis(1_800_000_000_000),
        query: Query::Configuration(GetConfiguration {}),
    };
    let snapshot = ConfigSnapshot {
        schema_version: 1,
        revision: 7,
        scope,
        entries: vec![eitmad_contracts::config::ConfigEntry {
            key: eitmad_contracts::config::ConfigKey::parse("eitmad.config.locale.primary.v1")?,
            value: ConfigReadValue::Text("ar-YE".to_owned()),
            sensitivity: eitmad_contracts::config::ConfigSensitivity::Public,
            restart_requirement: eitmad_contracts::config::RestartRequirement::Application,
        }],
    };
    let query_response = QueryResponseEnvelope {
        request_id: query.request_id,
        correlation_id,
        outcome: QueryOutcome::Succeeded(QueryResult::Configuration(snapshot)),
    };
    let structured_error = ContractError {
        code: ErrorCode::parse("eitmad.error.config-revision-conflict.v1")?,
        message_id: MessageId::parse("eitmad.message.config-revision-conflict.v1")?,
        parameters: vec![ErrorParameter {
            name: ErrorParameterName::parse("expected-revision")?,
            value: ErrorParameterValue::Integer(6),
        }],
        retry: RetryDisposition::Never,
        correlation_id,
        detail: Some(eitmad_contracts::errors::ErrorDetail::RevisionConflict {
            expected: 6,
            actual: 7,
        }),
    };

    Ok(ConformanceFixture {
        query,
        query_response,
        structured_error,
        mixed_direction_samples: vec![
            "خزانة Wardrobe 120 cm - فرع صنعاء",
            "ملف عرض السعر Quote-١٢.pdf",
        ],
    })
}

const fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    write_text(path, &format!("{}\n", serde_json::to_string_pretty(value)?))
}

fn write_text(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content.replace("\r\n", "\n"))?;
    Ok(())
}

fn render_reference(catalog: &ProtocolCatalog) -> String {
    let mut output = String::from(
        "---\n\
         title: \"Protocol v1 identifier reference\"\n\
         description: \"Generated commands, queries, subscriptions, events, capabilities, permissions, errors, and other stable protocol identifiers.\"\n\
         audience: \"api\"\n\
         page_type: \"reference\"\n\
         status: \"active\"\n\
         owner: \"Rust contract maintainers\"\n\
         last_verified: \"2026-07-18\"\n\
         review_triggers:\n\
           - \"the Rust protocol catalog or generator changes\"\n\
         keywords:\n\
           - \"protocol identifiers\"\n\
           - \"eitmad.error.protocol-incompatible.v1\"\n\
         ---\n\n\
         <!-- Generated by crates/contracts/src/bin/export_contracts.rs. Do not edit. -->\n\n\
         # Protocol v1 identifier reference\n\n\
         Regenerate with `npm run contracts:generate --prefix crates/contracts/codegen`.\n",
    );
    for (title, identifiers) in [
        ("IPC messages", &catalog.ipc_messages),
        ("Commands", &catalog.commands),
        ("Queries", &catalog.queries),
        ("Subscriptions", &catalog.subscriptions),
        ("Events", &catalog.events),
        ("Sync messages", &catalog.sync_messages),
        ("Server messages", &catalog.server_messages),
        ("Capabilities", &catalog.capabilities),
        ("Permissions", &catalog.permissions),
        ("Configuration keys", &catalog.config_keys),
        ("Relationships", &catalog.relations),
        ("Schema identifiers", &catalog.schema_ids),
        ("Error codes", &catalog.error_codes),
        ("Localization message identifiers", &catalog.message_ids),
        ("Error parameter names", &catalog.error_parameter_names),
    ] {
        writeln!(&mut output, "\n## {title}\n").expect("writing to a String cannot fail");
        if identifiers.is_empty() {
            output.push_str("No identifiers are registered.\n");
        } else {
            for identifier in identifiers {
                writeln!(&mut output, "- `{identifier}`").expect("writing to a String cannot fail");
            }
        }
    }
    output
}
