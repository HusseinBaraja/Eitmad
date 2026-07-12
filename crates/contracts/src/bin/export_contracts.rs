use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use eitmad_contracts::{
    PROTOCOL_VERSION,
    catalog::ProtocolCatalog,
    config::{ConfigReadValue, ConfigSnapshot},
    errors::{
        ContractError, ErrorCode, ErrorParameter, ErrorParameterName, ErrorParameterValue,
        MessageId, RetryDisposition,
    },
    identity::{
        AuthenticatedIdentity, AuthorizationContext, DeviceId, PrincipalId, PrincipalKind, ScopeId,
        ScopeKind, ScopeRef, SessionId,
    },
    ipc::{IpcClientMessage, IpcServerMessage},
    permissions::EffectivePermissions,
    queries::{GetConfiguration, Query, QueryResult},
    runtime::{DiagnosticReport, LifecycleSnapshot},
    sync::{SyncMessage, SyncStatus},
    transport::{
        CausationId, CommandEnvelope, CommandResponseEnvelope, CorrelationId, EventEnvelope,
        QueryEnvelope, QueryOutcome, QueryResponseEnvelope, RequestId, SubscriptionEnvelope,
        UnixMillis,
    },
    updates::UpdateState,
    versioning::{NegotiationOutcome, PeerHello},
};
use schemars::{JsonSchema, generate::SchemaSettings};
use serde::Serialize;
use uuid::Uuid;

#[derive(JsonSchema)]
#[allow(dead_code)]
struct ContractSchemaRoot {
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
    update_state: UpdateState,
    sync_status: SyncStatus,
    effective_permissions: EffectivePermissions,
    lifecycle_snapshot: LifecycleSnapshot,
    diagnostic_report: DiagnosticReport,
    catalog: ProtocolCatalog,
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
            value: ErrorParameterValue::Text("ملف عرض السعر Quote-١٢.pdf".to_owned()),
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
         last_verified: \"2026-07-12\"\n\
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
        ("Commands", &catalog.commands),
        ("Queries", &catalog.queries),
        ("Subscriptions", &catalog.subscriptions),
        ("Events", &catalog.events),
        ("Sync messages", &catalog.sync_messages),
        ("Capabilities", &catalog.capabilities),
        ("Permissions", &catalog.permissions),
        ("Configuration keys", &catalog.config_keys),
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
