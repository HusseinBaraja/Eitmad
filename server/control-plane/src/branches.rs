//! Tenant-owner branch registration for scoped product records.

use eitmad_contracts::{
    identity::{ScopeKind, ScopeRef},
    server::{AuthenticatedServerSession, RegisterBranchRequest, RegisteredBranch},
    transport::{CorrelationId, UnixMillis},
};
use eitmad_server_audit::{
    ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append as append_audit,
};
use sqlx::PgPool;

use crate::database::tenant_transaction;

const OWNER_RELATION: &str = "eitmad.relation.organization.owner.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BranchError {
    #[error("branch registration is denied")]
    Denied,
    #[error("branch registration is invalid")]
    Invalid,
    #[error("branch authority is unavailable")]
    Unavailable,
}

#[derive(Clone)]
pub struct BranchService {
    pool: PgPool,
}

impl BranchService {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Registers an engine-owned branch under one organization. An exact retry
    /// returns the same branch; reuse under another organization is denied.
    ///
    /// # Errors
    ///
    /// Denies a missing owner relationship or foreign organization and rolls
    /// back when the mandatory audit cannot commit.
    pub async fn register(
        &self,
        session: &AuthenticatedServerSession,
        request: &RegisterBranchRequest,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<RegisteredBranch, BranchError> {
        let scope = ScopeRef {
            kind: ScopeKind::parse("branch").expect("static branch scope"),
            id: request.branch_id,
        };
        let mut transaction = tenant_transaction(&self.pool, session.tenant_id)
            .await
            .map_err(|_| BranchError::Unavailable)?;
        let allowed = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1 FROM control.organizations o
                JOIN control.relationship_tuples r
                  ON r.tenant_id = o.tenant_id
                 AND r.object_kind = 'organization'
                 AND r.object_id = o.organization_id
                WHERE o.tenant_id = $1 AND o.organization_id = $2
                  AND r.subject_principal_id = $3 AND r.subject_kind = 'user'
                  AND r.relation = $4
            )",
        )
        .bind(session.tenant_id.value())
        .bind(request.organization_id.value())
        .bind(session.user_id.value())
        .bind(OWNER_RELATION)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| BranchError::Unavailable)?;
        if !allowed {
            append_audit(
                &mut transaction,
                &audit(
                    session,
                    scope,
                    request,
                    correlation_id,
                    now,
                    ServerAuditOutcome::Denied,
                ),
            )
            .await
            .map_err(|_| BranchError::Unavailable)?;
            transaction
                .commit()
                .await
                .map_err(|_| BranchError::Unavailable)?;
            return Err(BranchError::Denied);
        }
        let inserted = sqlx::query(
            "INSERT INTO control.branches (tenant_id, branch_id, organization_id, created_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (tenant_id, branch_id) DO NOTHING",
        )
        .bind(session.tenant_id.value())
        .bind(request.branch_id.value())
        .bind(request.organization_id.value())
        .bind(now.0)
        .execute(&mut *transaction)
        .await
        .map_err(|_| BranchError::Unavailable)?;
        if inserted.rows_affected() == 0 {
            let registered = sqlx::query_scalar::<_, uuid::Uuid>(
                "SELECT organization_id FROM control.branches
                 WHERE tenant_id = $1 AND branch_id = $2",
            )
            .bind(session.tenant_id.value())
            .bind(request.branch_id.value())
            .fetch_one(&mut *transaction)
            .await
            .map_err(|_| BranchError::Unavailable)?;
            if registered != request.organization_id.value() {
                append_audit(
                    &mut transaction,
                    &audit(
                        session,
                        scope,
                        request,
                        correlation_id,
                        now,
                        ServerAuditOutcome::Invalid,
                    ),
                )
                .await
                .map_err(|_| BranchError::Unavailable)?;
                transaction
                    .commit()
                    .await
                    .map_err(|_| BranchError::Unavailable)?;
                return Err(BranchError::Invalid);
            }
        } else {
            append_audit(
                &mut transaction,
                &audit(
                    session,
                    scope,
                    request,
                    correlation_id,
                    now,
                    ServerAuditOutcome::Succeeded,
                ),
            )
            .await
            .map_err(|_| BranchError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| BranchError::Unavailable)?;
        Ok(RegisteredBranch {
            tenant_id: session.tenant_id,
            organization_id: request.organization_id,
            branch_id: request.branch_id,
        })
    }
}

fn audit<'a>(
    session: &'a AuthenticatedServerSession,
    scope: ScopeRef,
    request: &'a RegisterBranchRequest,
    correlation_id: CorrelationId,
    now: UnixMillis,
    outcome: ServerAuditOutcome,
) -> ServerAuditEnvelope<'a> {
    ServerAuditEnvelope::from_session(
        session,
        scope,
        ServerAuditEvent {
            operation: "eitmad.server.branch.register.v1",
            outcome,
            target_kind: "branch",
            target_id: Some(request.branch_id.value()),
            correlation_id,
            causation_id: None,
            idempotency_key: None,
            redacted_error: None,
            occurred_at: now,
        },
    )
}
