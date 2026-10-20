//! Explicit Manager decisions over immutable customer conflict inputs.
use crate::{LocalOperationDraft, SyncCoordinator, SyncIntent, database::tenant_transaction};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{
    customer::{
        ConfirmCustomerResolution, Customer, CustomerConflictPage, CustomerConflictView,
        CustomerResolutionSource, CustomerStatus, CustomerSyncPayload, CustomerSyncState,
        ReadCustomerConflicts,
    },
    identity::ScopeRef,
    server::AuthenticatedServerSession,
    sync::{
        ChangeId, ChangeOperation, ChangeRecord, ConflictId, ConflictRecord, ConflictStatus,
        EncodedDomainPayload, MergeMetadata, MergeStrategy, RecordId,
    },
    transport::{CorrelationId, SchemaId, UnixMillis},
};
use eitmad_customer::CustomerError as E;
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sha2::{Digest as _, Sha256};
use sqlx::{Postgres, Row as _, Transaction};
use uuid::Uuid;

const SCHEMA: &str = "eitmad.schema.customer.v1";
type Tx<'a> = Transaction<'a, Postgres>;

impl SyncCoordinator {
    /// Returns the original receipt or closes its unused key under resolution locks.
    /// # Errors
    /// Requires the caller's current Manager authority; never executes the earlier decision.
    pub async fn reconcile_customer_resolution(
        &self,
        actor: &AuthenticatedServerSession,
        recovery: &eitmad_contracts::sync::ReconcileResolution<ConfirmCustomerResolution>,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<Option<ChangeRecord>, E> {
        let input = &recovery.request;
        let scope = branch_scope(&input.scope, input.command.customer.branch_id)?;
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
            .bind(actor.tenant_id.value())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| E::Unavailable)?;
        if let Err(error) =
            authorize_customer_resolution(&mut tx, actor, &input.scope, &scope, correlation, now)
                .await
        {
            tx.commit().await.map_err(|_| E::Unavailable)?;
            return Err(error);
        }
        crate::operations::lock_scope(
            &mut tx,
            actor.tenant_id,
            &scope,
            &SchemaId::parse(SCHEMA).map_err(|_| E::Unavailable)?,
        )
        .await
        .map_err(|_| E::Unavailable)?;
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(recovery.original_user, input)).map_err(|_| E::Unavailable)?,
        );
        let result = crate::operations::resolution::reconcile(
            &mut tx,
            actor.tenant_id,
            &scope,
            SCHEMA,
            input.idempotency_key,
            &fingerprint,
            now,
        )
        .await
        .map_err(|error| match error {
            crate::OperationError::IdempotencyMismatch => E::IdempotencyMismatch,
            _ => E::Unavailable,
        })?;
        evidence_for(
            &mut tx,
            actor,
            &scope,
            ResolutionAudit {
                correlation,
                now,
                key: Some(input.idempotency_key),
                target: input.command.customer.customer_id.value(),
                outcome: ServerAuditOutcome::Succeeded,
            },
            "eitmad.customer.reconcile-resolution.v1",
        )
        .await?;
        tx.commit().await.map_err(|_| E::Unavailable)?;
        Ok(result)
    }

    /// Reads a bounded page of preserved inputs and the current authoritative contact.
    /// # Errors
    /// Denies non-Managers and unrelated branches before any contact read.
    pub async fn customer_conflicts(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ReadCustomerConflicts,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<CustomerConflictPage, E> {
        let scope = branch_scope(&input.scope, input.query.customer.branch_id)?;
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        if let Err(error) =
            authorize_customer_resolution(&mut tx, actor, &input.scope, &scope, correlation, now)
                .await
        {
            tx.commit().await.map_err(|_| E::Unavailable)?;
            return Err(error);
        }
        let current = load_current(
            &mut tx,
            actor,
            &scope,
            input.query.customer.customer_id.value(),
        )
        .await?;
        let rows = sqlx::query("SELECT conflict_json FROM sync.conflicts WHERE tenant_id=$1
            AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3 AND record_id=$4
            AND status='open' AND ($5::uuid IS NULL OR conflict_id>$5) ORDER BY conflict_id LIMIT 21")
            .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(SCHEMA)
            .bind(input.query.customer.customer_id.value()).bind(input.query.after.map(ConflictId::value))
            .fetch_all(&mut *tx).await.map_err(|_| E::Unavailable)?;
        let more = rows.len() > 20;
        let items = rows
            .into_iter()
            .take(20)
            .map(|r| {
                let conflict: ConflictRecord =
                    serde_json::from_value(r.get("conflict_json")).map_err(|_| E::Unavailable)?;
                Ok(CustomerConflictView {
                    local: decode(&conflict.local)?,
                    remote: decode(&conflict.remote)?,
                    conflict,
                })
            })
            .collect::<Result<Vec<_>, E>>()?;
        let next = more
            .then(|| items.last().map(|v| v.conflict.conflict_id))
            .flatten();
        Ok(CustomerConflictPage {
            current: contact(&current)?,
            items,
            next,
            pending_reviews: vec![],
            pending_resolution: None,
        })
    }

    /// Commits one explicit decision, both inputs, history, audit and replay result atomically.
    /// # Errors
    /// Rejects stale reviews, changed retries and unauthorized scopes without applying a prefix.
    pub async fn resolve_customer(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ConfirmCustomerResolution,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<ChangeRecord, E> {
        let scope = branch_scope(&input.scope, input.command.customer.branch_id)?;
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        // Match ordinary customer submissions' lock order, including changes from other replicas.
        sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
            .bind(actor.tenant_id.value())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| E::Unavailable)?;
        if let Err(error) =
            authorize_customer_resolution(&mut tx, actor, &input.scope, &scope, correlation, now)
                .await
        {
            tx.commit().await.map_err(|_| E::Unavailable)?;
            return Err(error);
        }
        let schema = SchemaId::parse(SCHEMA).map_err(|_| E::Unavailable)?;
        crate::operations::lock_scope(&mut tx, actor.tenant_id, &scope, &schema)
            .await
            .map_err(|_| E::Unavailable)?;
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(actor.user_id, input)).map_err(|_| E::Unavailable)?,
        );
        match crate::operations::resolution::replay(
            &mut tx,
            actor.tenant_id,
            &scope,
            SCHEMA,
            input.idempotency_key,
            &fingerprint,
        )
        .await
        {
            Ok(Some(result)) => return Ok(result),
            Ok(None) => {}
            Err(crate::OperationError::IdempotencyMismatch) => {
                evidence(
                    &mut tx,
                    actor,
                    &scope,
                    ResolutionAudit {
                        correlation,
                        now,
                        key: Some(input.idempotency_key),
                        target: input.command.customer.customer_id.value(),
                        outcome: ServerAuditOutcome::Invalid,
                    },
                )
                .await?;
                tx.commit().await.map_err(|_| E::Unavailable)?;
                return Err(E::IdempotencyMismatch);
            }
            Err(_) => return Err(E::Unavailable),
        }
        let current = load_current(
            &mut tx,
            actor,
            &scope,
            input.command.customer.customer_id.value(),
        )
        .await?;
        let audit = ResolutionAudit {
            correlation,
            now,
            key: Some(input.idempotency_key),
            target: current.record_id.value(),
            outcome: ServerAuditOutcome::Conflict,
        };
        if current.revision != input.command.expected_server_revision {
            evidence(&mut tx, actor, &scope, audit).await?;
            tx.commit().await.map_err(|_| E::Unavailable)?;
            return Err(E::RevisionConflict {
                expected_revision: Some(input.command.expected_server_revision),
                actual_revision: Some(current.revision),
            });
        }
        let change =
            commit_resolution(&mut tx, actor, &scope, input, &current, audit, &fingerprint).await?;
        tx.commit().await.map_err(|_| E::Unavailable)?;
        Ok(change)
    }
}

async fn authorize_customer_resolution(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    requested_scope: &ScopeRef,
    scope: &ScopeRef,
    correlation: CorrelationId,
    now: UnixMillis,
) -> Result<(), E> {
    let contained = requested_scope.kind.as_str() == "branch" || sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM control.branches WHERE tenant_id=$1 AND branch_id=$2 AND organization_id=$3)")
            .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(requested_scope.id.value())
            .fetch_one(&mut **tx).await.map_err(|_| E::Unavailable)?;
    if contained
        && crate::customer::branch_authorized(tx, actor, scope, SyncIntent::ResolveConflict).await
    {
        return Ok(());
    }
    evidence(
        tx,
        actor,
        scope,
        ResolutionAudit {
            correlation,
            now,
            key: None,
            target: scope.id.value(),
            outcome: ServerAuditOutcome::Denied,
        },
    )
    .await?;
    Err(E::Denied)
}

async fn commit_resolution(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    input: &ConfirmCustomerResolution,
    current: &ChangeRecord,
    audit: ResolutionAudit,
    fingerprint: &[u8],
) -> Result<ChangeRecord, E> {
    let mut conflict = resolution_input(tx, actor, scope, input, current, audit.now).await?;
    if conflict.status != ConflictStatus::Open {
        return Err(E::RevisionConflict {
            expected_revision: Some(input.command.expected_server_revision),
            actual_revision: Some(current.revision),
        });
    }
    let merge = MergeMetadata {
        strategy: MergeStrategy::DomainMerge,
        common_ancestor_revision: conflict.local.base_revision,
        source_changes: vec![
            conflict.local.change_id,
            conflict.remote.change_id,
            current.change_id,
        ],
        merged_at: audit.now,
    };
    let draft = resolution_draft(input, scope, current.revision)?;
    let change = crate::operations::commit_change(
        tx,
        actor.tenant_id,
        &draft,
        current.revision,
        fingerprint,
        audit.now,
        Some(merge.clone()),
    )
    .await
    .map_err(|_| E::Unavailable)?;
    conflict.status = ConflictStatus::Resolved;
    conflict.resolution = Some(merge);
    crate::operations::resolution::save(tx, actor.tenant_id, SCHEMA, &conflict, audit.now)
        .await
        .map_err(|_| E::Unavailable)?;
    evidence(
        tx,
        actor,
        scope,
        ResolutionAudit {
            outcome: ServerAuditOutcome::Succeeded,
            ..audit
        },
    )
    .await?;
    Ok(change)
}

fn branch_scope(
    scope: &ScopeRef,
    branch: Option<eitmad_contracts::identity::ScopeId>,
) -> Result<ScopeRef, E> {
    match (scope.kind.as_str(), branch) {
        ("organization", Some(id)) => Ok(ScopeRef {
            kind: eitmad_contracts::identity::ScopeKind::parse("branch").expect("scope"),
            id,
        }),
        ("branch", None) => Ok(scope.clone()),
        ("branch", Some(id)) if id == scope.id => Ok(scope.clone()),
        _ => Err(E::Denied),
    }
}

async fn load_current(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    id: Uuid,
) -> Result<ChangeRecord, E> {
    let value: serde_json::Value = sqlx::query_scalar("SELECT change_json FROM sync.records WHERE tenant_id=$1
        AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3 AND record_id=$4 AND NOT tombstone")
        .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(SCHEMA).bind(id)
        .fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?.ok_or(E::NotFound)?;
    serde_json::from_value(value).map_err(|_| E::Unavailable)
}

async fn resolution_input(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    input: &ConfirmCustomerResolution,
    current: &ChangeRecord,
    now: UnixMillis,
) -> Result<ConflictRecord, E> {
    match &input.command.source {
        CustomerResolutionSource::Conflict(id) => {
            let value: serde_json::Value = sqlx::query_scalar("SELECT conflict_json FROM sync.conflicts
                WHERE tenant_id=$1 AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3 AND conflict_id=$4 AND record_id=$5 FOR UPDATE")
                .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(SCHEMA).bind(id.conflict_id.value()).bind(current.record_id.value())
                .fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?.ok_or(E::NotFound)?;
            serde_json::from_value(value).map_err(|_| E::Unavailable)
        }
        CustomerResolutionSource::PendingReview(local) => {
            if local.scope != *scope || local.record_id != current.record_id {
                return Err(E::Denied);
            }
            decode(local)?;
            if local.base_revision.unwrap_or(0).checked_add(1) != Some(local.revision) {
                return Err(E::IdempotencyMismatch);
            }
            Ok(ConflictRecord {
                conflict_id: ConflictId::new(Uuid::new_v4()),
                scope: scope.clone(),
                record_id: current.record_id,
                local: local.as_ref().clone(),
                remote: current.clone(),
                detected_at: now,
                status: ConflictStatus::Open,
                resolution: None,
            })
        }
    }
}

fn resolution_draft(
    input: &ConfirmCustomerResolution,
    scope: &ScopeRef,
    revision: u64,
) -> Result<LocalOperationDraft, E> {
    let fields = &input.command.contact;
    let payload = CustomerSyncPayload {
        customer_id: input.command.customer.customer_id,
        name: fields.name.clone(),
        phone: fields.phone.clone(),
        address: fields.address.clone(),
        notes: fields.notes.clone(),
        revision: revision
            .checked_add(1)
            .filter(|r| i64::try_from(*r).is_ok())
            .ok_or(E::Unavailable)?,
    };
    if eitmad_customer::normalize_phone(payload.phone.as_str()).is_none() {
        return Err(E::Unavailable);
    }
    Ok(LocalOperationDraft {
        change_id: ChangeId::new(Uuid::new_v4()),
        scope: scope.clone(),
        schema_id: SchemaId::parse(SCHEMA).expect("schema"),
        schema_version: 1,
        record_id: RecordId::new(payload.customer_id.value()),
        operation: ChangeOperation::Upsert,
        base_revision: Some(revision),
        idempotency_key: input.idempotency_key,
        payload: Some(EncodedDomainPayload {
            schema_id: SchemaId::parse(SCHEMA).expect("schema"),
            schema_version: 1,
            base64: STANDARD.encode(serde_json::to_vec(&payload).map_err(|_| E::Unavailable)?),
        }),
    })
}

fn decode(change: &ChangeRecord) -> Result<CustomerSyncPayload, E> {
    let payload = change.payload.as_ref().ok_or(E::Unavailable)?;
    if payload.schema_id.as_str() != SCHEMA
        || payload.schema_version != 1
        || change.operation != ChangeOperation::Upsert
    {
        return Err(E::Unavailable);
    }
    let contact: CustomerSyncPayload = serde_json::from_slice(
        &STANDARD
            .decode(&payload.base64)
            .map_err(|_| E::Unavailable)?,
    )
    .map_err(|_| E::Unavailable)?;
    if contact.customer_id.value() != change.record_id.value()
        || contact.revision != change.revision
    {
        return Err(E::Unavailable);
    }
    Ok(contact)
}

pub(super) fn contact(change: &ChangeRecord) -> Result<Customer, E> {
    let value = decode(change)?;
    Ok(Customer {
        id: value.customer_id,
        scope: change.scope.clone(),
        name: value.name,
        phone: value.phone,
        address: value.address,
        notes: value.notes,
        status: CustomerStatus::Active,
        revision: value.revision,
        updated_at: change.changed_at,
        sync_state: CustomerSyncState::Confirmed,
    })
}

#[derive(Clone, Copy)]
struct ResolutionAudit {
    correlation: CorrelationId,
    now: UnixMillis,
    key: Option<eitmad_contracts::transport::IdempotencyKey>,
    target: Uuid,
    outcome: ServerAuditOutcome,
}
async fn evidence(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    audit: ResolutionAudit,
) -> Result<(), E> {
    evidence_for(
        tx,
        actor,
        scope,
        audit,
        "eitmad.customer.resolve-conflict.v1",
    )
    .await
}
async fn evidence_for(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    audit: ResolutionAudit,
    operation: &'static str,
) -> Result<(), E> {
    append(
        tx,
        &ServerAuditEnvelope::from_session(
            actor,
            scope.clone(),
            ServerAuditEvent {
                operation,
                outcome: audit.outcome,
                target_kind: "customer",
                target_id: Some(audit.target),
                correlation_id: audit.correlation,
                causation_id: None,
                idempotency_key: audit.key,
                redacted_error: None,
                occurred_at: audit.now,
            },
        ),
    )
    .await
    .map_err(|_| E::Unavailable)
}
