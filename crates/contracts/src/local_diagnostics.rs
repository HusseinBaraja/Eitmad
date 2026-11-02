//! Local support evidence. Every field is technical metadata; free-text errors are excluded.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LocalDiagnosticComponent {
    Desktop,
    Engine,
    Installer,
    Setup,
    Server,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalDiagnosticEvent {
    pub version: u8,
    pub utc_millis: u64,
    pub session: Uuid,
    pub sequence: u64,
    pub process_id: u32,
    pub component: LocalDiagnosticComponent,
    pub build: String,
    pub binary_sha256: String,
    pub os_version: String,
    pub phase: String,
    pub result: String,
    pub correlation: Option<Uuid>,
    pub operation: Option<String>,
    pub error_code: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub native_code: Option<i32>,
    pub http_status: Option<u16>,
    /// Code symbols and source basenames only; never exception messages or absolute paths.
    pub locations: Vec<String>,
    pub dropped: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalDiagnosticSource {
    pub component: LocalDiagnosticComponent,
    pub status: String,
    pub records: u64,
    pub rejected_records: u64,
    pub truncated_files: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalDiagnosticManifest {
    pub version: u8,
    pub exported_utc_millis: u64,
    pub sources: Vec<LocalDiagnosticSource>,
    pub services: Vec<LocalDiagnosticService>,
    pub capture_failures: u64,
    pub pending_dropped_events: u64,
    pub events_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalDiagnosticService {
    pub name: String,
    pub state: String,
    pub native_code: Option<i32>,
    pub service_exit_code: Option<u32>,
}

/// Shared bounds and allowlists consumed by native file adapters as generated data.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalDiagnosticPolicy {
    pub file_bytes: usize,
    pub files_per_component: usize,
    pub queue_capacity: usize,
    pub record_bytes: usize,
    pub max_locations: usize,
    pub symbol_pattern: &'static str,
    pub phases: Vec<&'static str>,
    pub results: Vec<&'static str>,
    pub error_codes: Vec<&'static str>,
    pub operations: Vec<&'static str>,
}

impl Default for LocalDiagnosticPolicy {
    fn default() -> Self {
        Self {
            file_bytes: 2 * 1024 * 1024,
            files_per_component: 4,
            queue_capacity: 256,
            record_bytes: 8192,
            max_locations: 12,
            symbol_pattern: r"\A[A-Za-z0-9_.:+<>`/{}-]{1,240}\z",
            phases: vec![
                "Process",
                "EngineProcess",
                "Crash",
                "Capture",
                "Export",
                "Activation",
                "Home",
                "Drafts",
                "Orders",
                "WorkOrders",
                "Customers",
                "Catalog",
                "AccountSwitch",
                "SignIn",
                "IpcRequest",
                "IpcLock",
                "IpcSend",
                "IpcReply",
                "DriverAdmission",
                "CredentialAdmission",
                "CredentialRefresh",
                "Dns",
                "Connect",
                "Tls",
                "Send",
                "Receive",
                "Confirmation",
                "PublisherTrust",
                "PayloadVerification",
                "PayloadExtraction",
                "Preflight",
                "Elevation",
                "PrepareWorkshop",
                "ApplyMigrations",
                "InitializeOwner",
                "StopPreparedDatabase",
                "InstallServices",
                "ConfigureFirewall",
                "VerifyHttps",
                "EnrollDesktop",
                "RegisterDesktop",
                "Complete",
                "ServerRequest",
            ],
            results: vec![
                "Started",
                "Succeeded",
                "GateBusy",
                "Poisoned",
                "Deadline",
                "Cancelled",
                "Network",
                "IoTimeout",
                "Denied",
                "Invalid",
                "Server",
                "Failed",
                "Stopped",
                "Running",
                "Blocked",
                "Unavailable",
            ],
            error_codes: crate::catalog::ERROR_CODES.to_vec(),
            operations: crate::commands::Command::IDS
                .iter()
                .chain(crate::queries::Query::IDS)
                .chain(crate::catalog::IPC_MESSAGES)
                .copied()
                .collect(),
        }
    }
}
