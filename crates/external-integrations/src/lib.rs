//! Engine-owned boundary for explicitly named external providers.

use eitmad_authorization::{AuthorizationGate, BoundaryAuditContext, BoundaryError, BoundaryKind};
use eitmad_contracts::{authorization::AuthorizationRequest, identity::AuthorizationContext};
use eitmad_observability_audit::RedactedAuditError;

#[derive(Clone, Debug)]
pub struct ExternalActionAuthorization {
    gate: AuthorizationGate,
}

impl ExternalActionAuthorization {
    #[must_use]
    pub const fn new(gate: AuthorizationGate) -> Self {
        Self { gate }
    }

    /// Authorizes one provider boundary and durably records a denial.
    ///
    /// # Errors
    ///
    /// Returns a deny-by-default boundary error; rejected adapters are not called.
    pub fn authorize(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
    ) -> Result<(), BoundaryError> {
        let mut audit = audit.clone();
        audit.kind = BoundaryKind::ExternalAdapter;
        self.gate.authorize(actor, request, &audit)
    }

    /// Authorizes and audits one read-only external provider action.
    ///
    /// # Errors
    ///
    /// Returns a deny-by-default boundary error; rejected adapters are not called.
    pub fn execute_read<T>(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
        action: impl FnOnce() -> Result<T, RedactedAuditError>,
    ) -> Result<T, BoundaryError> {
        let mut audit = audit.clone();
        audit.kind = BoundaryKind::ExternalAdapter;
        self.gate.execute_read(actor, request, &audit, action)
    }
}
