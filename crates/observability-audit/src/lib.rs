//! Privacy-preserving diagnostics and mutation audit records.

use eitmad_contracts::{
    identity::{PrincipalId, PrincipalKind, ScopeRef},
    transport::{CausationId, CorrelationId, IdempotencyKey, UnixMillis},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditOutcome {
    Succeeded,
    Denied,
    Invalid,
    Conflict,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationAuditRecord {
    pub audit_id: Uuid,
    pub occurred_at: UnixMillis,
    pub principal_id: PrincipalId,
    pub principal_kind: PrincipalKind,
    pub scope: ScopeRef,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub operation: String,
    pub outcome: AuditOutcome,
    pub previous_revision: Option<u64>,
    pub resulting_revision: Option<u64>,
    pub changed_identifiers: Vec<String>,
    pub error_code: Option<String>,
}

impl MutationAuditRecord {
    #[must_use]
    pub fn with_outcome(mut self, outcome: AuditOutcome, error_code: Option<String>) -> Self {
        self.outcome = outcome;
        self.error_code = error_code;
        self
    }
}
