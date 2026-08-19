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

    /// Authorizes and audits one invocation of one declared plugin capability.
    ///
    /// # Errors
    ///
    /// Returns a deny-by-default boundary error; rejected plugin code never runs.
    pub fn execute<T>(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
        action: impl FnOnce() -> Result<T, RedactedAuditError>,
    ) -> Result<T, BoundaryError> {
        let mut audit = audit.clone();
        audit.kind = BoundaryKind::PluginCapability;
        self.gate.execute(actor, request, &audit, action)
    }
}
