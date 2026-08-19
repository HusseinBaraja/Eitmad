//! Permission-gated extension contracts and host isolation seams.

use eitmad_authorization::{AuthorizationGate, BoundaryAuditContext, BoundaryError, BoundaryKind};
use eitmad_contracts::{authorization::AuthorizationRequest, identity::AuthorizationContext};
use eitmad_observability_audit::RedactedAuditError;

#[derive(Clone, Debug)]
pub struct PluginCapabilityAuthorization {
    gate: AuthorizationGate,
}

impl PluginCapabilityAuthorization {
    #[must_use]
    pub const fn new(gate: AuthorizationGate) -> Self {
        Self { gate }
    }

    /// Authorizes one plugin boundary and durably records a denial.
    ///
    /// # Errors
    ///
    /// Returns a deny-by-default boundary error; rejected plugin code never runs.
    pub fn authorize(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
    ) -> Result<(), BoundaryError> {
        let mut audit = audit.clone();
        audit.kind = BoundaryKind::PluginCapability;
        self.gate.authorize(actor, request, &audit)
    }

    /// Authorizes and audits one read-only plugin capability invocation.
    ///
    /// # Errors
    ///
    /// Returns a deny-by-default boundary error; rejected plugin code never runs.
    pub fn execute_read<T>(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
        action: impl FnOnce() -> Result<T, RedactedAuditError>,
    ) -> Result<T, BoundaryError> {
        let mut audit = audit.clone();
        audit.kind = BoundaryKind::PluginCapability;
        self.gate.execute_read(actor, request, &audit, action)
    }
}
