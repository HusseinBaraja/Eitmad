//! Unified local-first and server-authoritative synchronization protocol.

mod engine;

pub use engine::{
    CommandDraft, ConflictHook, ConflictResolution, DeliveryOutcome, LocalChangeDraft,
    LocalChangeOutcome, PendingCommandOutcome, SyncEngine, SyncEngineError,
};

impl std::fmt::Display for SyncEngineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Authorization(_) => "sync authorization failed",
            Self::StorageUnavailable => "sync storage is unavailable",
            Self::StorageConflict => "sync storage revision conflict",
            Self::CorruptState => "sync state is corrupt",
            Self::UnsupportedStateVersion { .. } => "sync state version is unsupported",
            Self::WrongMode => "sync operation is unavailable in this mode",
            Self::ScopeMismatch => "sync scope does not match",
            Self::InvalidChange => "sync change is invalid",
            Self::IdempotencyMismatch => "sync idempotency key was reused",
            Self::IncompatibleMode => "sync application modes are incompatible",
            Self::IncompatiblePeer(_) => "sync peer is incompatible",
            Self::Disconnected => "sync engine is offline",
            Self::StaleCache => "sync cache is stale",
        })
    }
}

impl std::error::Error for SyncEngineError {}

use eitmad_authorization::{AuthorizationGate, BoundaryAuditContext, BoundaryError, BoundaryKind};
use eitmad_contracts::{authorization::AuthorizationRequest, identity::AuthorizationContext};
use eitmad_observability_audit::RedactedAuditError;

#[derive(Clone, Debug)]
pub struct SyncAuthorization {
    gate: AuthorizationGate,
}

impl SyncAuthorization {
    #[must_use]
    pub const fn new(gate: AuthorizationGate) -> Self {
        Self { gate }
    }

    /// Authorizes one sync boundary and durably records a denial.
    ///
    /// # Errors
    ///
    /// Returns a deny-by-default boundary error; rejected sync work never runs.
    pub fn authorize(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
    ) -> Result<(), BoundaryError> {
        let mut audit = audit.clone();
        audit.kind = BoundaryKind::Sync;
        self.gate.authorize(actor, request, &audit)
    }

    /// Authorizes and audits one read-only sync operation.
    ///
    /// # Errors
    ///
    /// Returns a deny-by-default boundary error; rejected sync work never runs.
    pub fn execute_read<T>(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
        action: impl FnOnce() -> Result<T, RedactedAuditError>,
    ) -> Result<T, BoundaryError> {
        let mut audit = audit.clone();
        audit.kind = BoundaryKind::Sync;
        self.gate.execute_read(actor, request, &audit, action)
    }
}
