//! Explicit quotation-draft decisions; immutable snapshots never imply approval or issuance.
use crate::{LocalOperationDraft, SyncCoordinator, database::tenant_transaction};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{
    identity::ScopeRef,
    quotation_draft::{
        ConfirmQuotationDraftResolution, QuotationDraftConflictView, QuotationDraftId,
        QuotationDraftResolutionChoice, QuotationDraftResolutionSource, QuotationDraftReview,
        QuotationDraftSnapshot, ReadQuotationDraftRecovery, ReadQuotationDraftReview,
    },
    server::AuthenticatedServerSession,
    sync::{
        ChangeId, ChangeOperation, ChangeRecord, ConflictId, ConflictRecord, ConflictStatus,
        EncodedDomainPayload, MergeMetadata, MergeStrategy, RecordId,
    },
    transport::{CorrelationId, SchemaId, UnixMillis},
};
use eitmad_pricing::{QUOTATION_DRAFT_SCHEMA as SCHEMA, QuotationDraftError as E};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sha2::{Digest as _, Sha256};
use sqlx::{Postgres, Row as _, Transaction};
use uuid::Uuid;
type Tx<'a> = Transaction<'a, Postgres>;

impl SyncCoordinator {
    /// Reads the latest decision provenance before the current snapshot for interrupted clients.
    /// # Errors
    /// Requires ordinary branch draft-read authority; never grants decision rights.
    pub async fn quotation_draft_recovery(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ReadQuotationDraftRecovery,
    ) -> Result<Vec<ChangeRecord>, E> {
        use crate::DomainSyncHandler as _;
        if !crate::QuotationDraftSyncHandler::new(self.pool.clone())
            .authorize(actor, &input.scope, crate::SyncIntent::Read)
            .await
        {
            return Err(E::Denied);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        let current = load_current(&mut tx, actor, &input.scope, input.draft_id).await?;
        let decision: Option<serde_json::Value> = sqlx::query_scalar("SELECT result_json->'value' FROM sync.idempotency_results
            WHERE tenant_id=$1 AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3
              AND result_json->'value'->>'recordId'=$4 AND result_json->'value'->'merge' <> 'null'::jsonb
            ORDER BY (result_json->'value'->>'revision')::bigint DESC LIMIT 1")
            .bind(actor.tenant_id.value()).bind(input.scope.id.value()).bind(SCHEMA).bind(input.draft_id.value().to_string())
            .fetch_optional(&mut *tx).await.map_err(|_| E::Unavailable)?;
        let mut result = vec![];
        if let Some(decision) = decision {
            let decision: ChangeRecord =
                serde_json::from_value(decision).map_err(|_| E::Unavailable)?;
            if decision.revision < current.revision {
                result.push(decision);
            }
        }
        result.push(current);
        Ok(result)
    }

    /// Reads bounded preserved inputs and the latest authoritative draft.
    /// # Errors
    /// Rejects unauthorized scopes and unavailable history.
    pub async fn quotation_draft_review(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ReadQuotationDraftReview,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<QuotationDraftReview, E> {
        let scope = branch_scope(&input.scope, input.query.branch_id)?;
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        let can_resolve = authorize(&mut tx, actor, &input.scope, &scope).await?;
        if !can_resolve {
            evidence(
                &mut tx,
                actor,
                &scope,
                ResolutionAudit {
                    correlation,
                    now,
                    key: None,
                    target: input.query.draft_id.value(),
                    outcome: ServerAuditOutcome::Denied,
                },
            )
            .await?;
            tx.commit().await.map_err(|_| E::Unavailable)?;
            return Err(E::Denied);
        }
        let can_resolve = crate::quotation_lifecycle::latest(&mut tx, actor, input.query.draft_id).await
            .map_err(|_| E::Unavailable)?.is_none_or(|q| !q.quotation.cancelled && matches!(q.state,
                eitmad_contracts::quotation_lifecycle::QuotationState::Draft | eitmad_contracts::quotation_lifecycle::QuotationState::PendingApproval));
        let current = load_current(&mut tx, actor, &scope, input.query.draft_id).await?;
        let rows = sqlx::query("SELECT conflict_json FROM sync.conflicts WHERE tenant_id=$1
            AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3 AND record_id=$4
            AND status='open' AND ($5::uuid IS NULL OR conflict_id>$5) ORDER BY conflict_id LIMIT 2")
            .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(SCHEMA)
            .bind(input.query.draft_id.value()).bind(input.query.after.map(ConflictId::value))
            .fetch_all(&mut *tx).await.map_err(|_| E::Unavailable)?;
        // One maximum-sized conflict plus the current draft fits the bounded HTTP response.
        let more = rows.len() > 1;
        let items = rows
            .into_iter()
            .take(1)
            .map(|r| {
                let conflict: ConflictRecord =
                    serde_json::from_value(r.get("conflict_json")).map_err(|_| E::Unavailable)?;
                Ok(QuotationDraftConflictView {
                    local: decode(&conflict.local)?,
                    remote: decode(&conflict.remote)?,
                    conflict,
                })
            })
            .collect::<Result<Vec<_>, E>>()?;
        let next = more
            .then(|| items.last().map(|v| v.conflict.conflict_id))
            .flatten();
        Ok(QuotationDraftReview {
            current: decode(&current)?,
            items,
            next,
            pending_reviews: vec![],
            pending_resolution: None,
            can_resolve,
        })
    }

    /// Commits the decision, preserved inputs, dependency validation, audit and receipt atomically.
    /// # Errors
    /// Rejects stale reviews, changed retries, final documents and unauthorized scopes.
    pub async fn resolve_quotation_draft(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ConfirmQuotationDraftResolution,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<ChangeRecord, E> {
        let scope = branch_scope(&input.scope, input.command.branch_id)?;
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        // Same lock order as draft transfer and lifecycle transitions across replicas.
        sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
            .bind(actor.tenant_id.value())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| E::Unavailable)?;
        let audit = ResolutionAudit {
            correlation,
            now,
            key: Some(input.idempotency_key),
            target: input.command.draft_id.value(),
            outcome: ServerAuditOutcome::Denied,
        };
        if !authorize(&mut tx, actor, &input.scope, &scope).await? {
            evidence(&mut tx, actor, &scope, audit).await?;
            tx.commit().await.map_err(|_| E::Unavailable)?;
            return Err(E::Denied);
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
            serde_json::to_vec(&(actor.user_id, input)).map_err(|_| E::Unavailable)?,
        )
        .to_vec();
        let result = resolve_on(&mut tx, actor, &scope, input, audit, &fingerprint).await;
        match &result {
            Ok(_) => {}
            Err(E::Unavailable) => return result,
            Err(error) => {
                evidence(
                    &mut tx,
                    actor,
                    &scope,
                    ResolutionAudit {
                        outcome: match error {
                            E::Denied => ServerAuditOutcome::Denied,
                            E::Conflict { .. } | E::UnresolvedConflict => {
                                ServerAuditOutcome::Conflict
                            }
                            _ => ServerAuditOutcome::Invalid,
                        },
                        ..audit
                    },
                )
                .await?;
            }
        }
        tx.commit().await.map_err(|_| E::Unavailable)?;
        result
    }
}

async fn resolve_on(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    input: &ConfirmQuotationDraftResolution,
    audit: ResolutionAudit,
    fingerprint: &[u8],
) -> Result<ChangeRecord, E> {
    if let Some(result) = replay(tx, actor, scope, input, fingerprint).await? {
        return Ok(result);
    }
    let current = load_current(tx, actor, scope, input.command.draft_id).await?;
    if current.revision != input.command.expected_server_revision {
        return Err(E::Conflict {
            expected: Some(input.command.expected_server_revision),
            actual: Some(current.revision),
        });
    }
    if crate::quotation_lifecycle::latest(tx, actor, input.command.draft_id)
        .await
        .map_err(|_| E::Unavailable)?
        .is_some_and(|q| {
            q.quotation.cancelled
                || !matches!(
                    q.state,
                    eitmad_contracts::quotation_lifecycle::QuotationState::Draft
                        | eitmad_contracts::quotation_lifecycle::QuotationState::PendingApproval
                )
        })
    {
        return Err(E::UnresolvedConflict);
    }
    let mut conflict = resolution_input(tx, actor, scope, input, &current, audit.now).await?;
    if conflict.status != ConflictStatus::Open {
        return Err(E::UnresolvedConflict);
    }
    let current_snapshot = decode(&current)?;
    let source = decode(&conflict.local)?;
    if current_snapshot.cancelled {
        return Err(E::UnresolvedConflict);
    }
    // Both alternatives have been reviewed as immutable snapshots; never evaluate a new price here.
    let mut selected = match input.command.choice {
        QuotationDraftResolutionChoice::KeepCurrent => current_snapshot,
        QuotationDraftResolutionChoice::UseInput => source,
    };
    selected.revision = current
        .revision
        .checked_add(1)
        .filter(|r| i64::try_from(*r).is_ok())
        .ok_or(E::Unavailable)?;
    let draft = resolution_draft(&selected, &current, input.idempotency_key)?;
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
    // Validate before writing. A savepoint prevents dependency failures from committing partial domain work.
    sqlx::query("SAVEPOINT draft_resolution")
        .execute(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    if let Err(error) = crate::quotation_draft::retain_snapshot(tx, actor, &draft, audit.now).await
    {
        sqlx::query("ROLLBACK TO SAVEPOINT draft_resolution")
            .execute(&mut **tx)
            .await
            .map_err(|_| E::Unavailable)?;
        return Err(match error {
            crate::OperationError::Denied => E::Denied,
            crate::OperationError::Unavailable => E::Unavailable,
            _ => E::Invalid,
        });
    }
    crate::quotation_approval::invalidate_for_resolution(tx, actor, scope, &selected, audit.now)
        .await
        .map_err(|_| E::Unavailable)?;
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
    save_resolution(tx, actor, &conflict, audit.now).await?;
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

async fn resolution_input(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    input: &ConfirmQuotationDraftResolution,
    current: &ChangeRecord,
    now: UnixMillis,
) -> Result<ConflictRecord, E> {
    Ok(match &input.command.source {
        QuotationDraftResolutionSource::Conflict(id) => {
            let value: serde_json::Value = sqlx::query_scalar(
                "SELECT conflict_json FROM sync.conflicts
                WHERE tenant_id=$1 AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3
                AND conflict_id=$4 AND record_id=$5 FOR UPDATE",
            )
            .bind(actor.tenant_id.value())
            .bind(scope.id.value())
            .bind(SCHEMA)
            .bind(id.conflict_id.value())
            .bind(current.record_id.value())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| E::Unavailable)?
            .ok_or(E::NotFound)?;
            serde_json::from_value::<ConflictRecord>(value).map_err(|_| E::Unavailable)?
        }
        QuotationDraftResolutionSource::PendingReview(local) => {
            if local.scope != *scope || local.record_id != current.record_id {
                return Err(E::Denied);
            }
            decode(local)?;
            if local.base_revision.unwrap_or(0).checked_add(1) != Some(local.revision) {
                return Err(E::Invalid);
            }
            ConflictRecord {
                conflict_id: ConflictId::new(Uuid::new_v4()),
                scope: scope.clone(),
                record_id: current.record_id,
                local: *local.clone(),
                remote: current.clone(),
                detected_at: now,
                status: ConflictStatus::Open,
                resolution: None,
            }
        }
    })
}

fn resolution_draft(
    selected: &QuotationDraftSnapshot,
    current: &ChangeRecord,
    key: eitmad_contracts::transport::IdempotencyKey,
) -> Result<LocalOperationDraft, E> {
    Ok(LocalOperationDraft {
        change_id: ChangeId::new(Uuid::new_v4()),
        scope: current.scope.clone(),
        schema_id: SchemaId::parse(SCHEMA).map_err(|_| E::Unavailable)?,
        schema_version: 1,
        record_id: RecordId::new(selected.id.value()),
        operation: ChangeOperation::Upsert,
        base_revision: Some(current.revision),
        idempotency_key: key,
        payload: Some(EncodedDomainPayload {
            schema_id: SchemaId::parse(SCHEMA).map_err(|_| E::Unavailable)?,
            schema_version: 1,
            base64: STANDARD.encode(serde_json::to_vec(&selected).map_err(|_| E::Unavailable)?),
        }),
    })
}

async fn authorize(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    requested: &ScopeRef,
    scope: &ScopeRef,
) -> Result<bool, E> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM control.branches b
        JOIN control.relationship_tuples r ON r.tenant_id=b.tenant_id
          AND r.subject_principal_id=$2 AND r.subject_kind='user'
          AND r.relation='eitmad.relation.organization.manager.v1'
          AND r.object_kind='organization' AND r.object_id=b.organization_id
        WHERE b.tenant_id=$1 AND b.branch_id=$3
          AND ($4='branch' OR ($4='organization' AND b.organization_id=$5)))",
    )
    .bind(actor.tenant_id.value())
    .bind(actor.user_id.value())
    .bind(scope.id.value())
    .bind(requested.kind.as_str())
    .bind(requested.id.value())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| E::Unavailable)
}
fn branch_scope(
    scope: &ScopeRef,
    branch: eitmad_contracts::identity::ScopeId,
) -> Result<ScopeRef, E> {
    if scope.kind.as_str() == "organization"
        || scope.kind.as_str() == "branch" && scope.id == branch
    {
        Ok(ScopeRef {
            kind: eitmad_contracts::identity::ScopeKind::parse("branch").expect("scope"),
            id: branch,
        })
    } else {
        Err(E::Denied)
    }
}
async fn load_current(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    id: QuotationDraftId,
) -> Result<ChangeRecord, E> {
    let value: serde_json::Value = sqlx::query_scalar("SELECT change_json FROM sync.records WHERE tenant_id=$1
        AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3 AND record_id=$4 AND NOT tombstone")
        .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(SCHEMA).bind(id.value())
        .fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?.ok_or(E::NotFound)?;
    serde_json::from_value(value).map_err(|_| E::Unavailable)
}
fn decode(change: &ChangeRecord) -> Result<QuotationDraftSnapshot, E> {
    let draft = LocalOperationDraft {
        change_id: change.change_id,
        scope: change.scope.clone(),
        schema_id: SchemaId::parse(SCHEMA).map_err(|_| E::Unavailable)?,
        schema_version: 1,
        record_id: change.record_id,
        operation: change.operation,
        base_revision: change.base_revision,
        idempotency_key: change.idempotency_key,
        payload: change.payload.clone(),
    };
    crate::quotation_draft::snapshot(&draft).map_err(|_| E::Invalid)
}

async fn replay(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    input: &ConfirmQuotationDraftResolution,
    fingerprint: &[u8],
) -> Result<Option<ChangeRecord>, E> {
    let row = sqlx::query("SELECT request_fingerprint,result_json FROM sync.idempotency_results
        WHERE tenant_id=$1 AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3 AND idempotency_key=$4")
        .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(SCHEMA).bind(input.idempotency_key.value())
        .fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.get::<Vec<u8>, _>("request_fingerprint") != fingerprint {
        return Err(E::IdempotencyMismatch);
    }
    let value: serde_json::Value = row.get("result_json");
    serde_json::from_value(value.get("value").cloned().ok_or(E::Unavailable)?)
        .map(Some)
        .map_err(|_| E::Unavailable)
}

async fn save_resolution(
    tx: &mut Tx<'_>,
    actor: &AuthenticatedServerSession,
    conflict: &ConflictRecord,
    now: UnixMillis,
) -> Result<(), E> {
    sqlx::query("INSERT INTO sync.conflicts (tenant_id,scope_kind,scope_id,schema_id,conflict_id,record_id,conflict_json,status,detected_at,resolved_at)
        VALUES ($1,'branch',$2,$3,$4,$5,$6,'resolved',$7,$8) ON CONFLICT(tenant_id,conflict_id)
        DO UPDATE SET conflict_json=excluded.conflict_json,status='resolved',resolved_at=excluded.resolved_at")
        .bind(actor.tenant_id.value()).bind(conflict.scope.id.value()).bind(SCHEMA).bind(conflict.conflict_id.value())
        .bind(conflict.record_id.value()).bind(serde_json::to_value(conflict).map_err(|_| E::Unavailable)?)
        .bind(conflict.detected_at.0).bind(now.0).execute(&mut **tx).await.map_err(|_| E::Unavailable)?;
    Ok(())
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
    append(
        tx,
        &ServerAuditEnvelope::from_session(
            actor,
            scope.clone(),
            ServerAuditEvent {
                operation: "eitmad.quotation-draft.resolve.v1",
                outcome: audit.outcome,
                target_kind: "quotation-draft",
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
