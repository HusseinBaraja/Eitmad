//! Unified local-first and server-authoritative synchronization protocol.

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

    /// Authorizes and audits one sync negotiation, pull, push, or acknowledgement.
    ///
    /// # Errors
    ///
    /// Returns a deny-by-default boundary error; rejected sync work never runs.
    pub fn execute<T>(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
        action: impl FnOnce() -> Result<T, RedactedAuditError>,
    ) -> Result<T, BoundaryError> {
        let mut audit = audit.clone();
        audit.kind = BoundaryKind::Sync;
        self.gate.execute(actor, request, &audit, action)
    }
}
