//! Atomic server requests, decisions, invalidation, audit, replay, and scoped publication.
use crate::{
    DomainDescriptor, DomainSyncHandler, DomainValidationError, LocalOperationDraft, SyncIntent,
    database::tenant_transaction,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{
    identity::{PrincipalId, ScopeRef},
    quotation_approval::{
        ConfirmDiscountApproval, DiscountApproval, DiscountApprovalAction, DiscountApprovalPage,
        DiscountApprovalState, DiscountDecision, DiscountRequestId, ReadDiscountApprovals,
    },
    quotation_draft::{QuotationDraftId, QuotationDraftSnapshot},
    server::AuthenticatedServerSession,
    sync::{ChangeId, ChangeOperation, EncodedDomainPayload, RecordId, SyncMode},
    transport::{CorrelationId, SchemaId, UnixMillis},
};
use eitmad_pricing::{
    ApprovalError as E, DISCOUNT_APPROVAL_SCHEMA, QUOTATION_DRAFT_SCHEMA, approval_fingerprint,
    decide_approval, same_commercial_terms,
};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct QuotationApprovalServer {
    pool: PgPool,
}
impl QuotationApprovalServer {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    /// Opens a `PostgreSQL` notification listener before reading the durable event page.
    /// # Errors
    /// Fails closed when streaming authority is unavailable.
    pub async fn listener(&self) -> Result<sqlx::postgres::PgListener, E> {
        let mut listener = sqlx::postgres::PgListener::connect_with(&self.pool)
            .await
            .map_err(|_| E::Unavailable)?;
        listener
            .listen("eitmad_quotation_approvals")
            .await
            .map_err(|_| E::Unavailable)?;
        Ok(listener)
    }
    /// Reads the latest request state for every authorized quotation, with a bounded UUID cursor.
    /// # Errors
    /// Denies other organizations and branches without leaking counts.
    pub async fn list(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ReadDiscountApprovals,
    ) -> Result<DiscountApprovalPage, E> {
        if !(1..=100).contains(&input.query.limit) {
            return Err(E::Invalid);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        authorize(&mut tx, actor, &input.scope, false, false).await?;
        let rows: Vec<Vec<u8>> = sqlx::query_scalar("SELECT record_json FROM (SELECT DISTINCT ON(draft_id) draft_id,branch_id,organization_id,record_json FROM sync.quotation_approval_history WHERE tenant_id=$1 ORDER BY draft_id,revision DESC) latest WHERE (($2='branch' AND branch_id=$3) OR ($2='organization' AND organization_id=$3)) AND ($4::uuid IS NULL OR draft_id>$4) ORDER BY draft_id LIMIT $5")
            .bind(actor.tenant_id.value()).bind(input.scope.kind.as_str()).bind(input.scope.id.value()).bind(input.query.after.map(QuotationDraftId::value)).bind(i64::from(input.query.limit)+1).fetch_all(&mut *tx).await.map_err(|_|E::Unavailable)?;
        let more = rows.len() > input.query.limit as usize;
        let items: Vec<DiscountApproval> = rows
            .into_iter()
            .take(input.query.limit as usize)
            .map(|r| serde_json::from_slice(&r).map_err(|_| E::Unavailable))
            .collect::<Result<_, _>>()?;
        let next = if more {
            items.last().map(|r| r.quotation.id)
        } else {
            None
        };
        Ok(DiscountApprovalPage { items, next })
    }
    /// Serializes with draft transfer; commits state, immutable history, audit, receipt, and event together.
    /// # Errors
    /// Denies wrong roles/self-decision, stale terms, competing revisions, and reused keys.
    pub async fn transition(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ConfirmDiscountApproval,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<Option<DiscountApproval>, E> {
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
            .bind(actor.tenant_id.value())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| E::Unavailable)?;
        let deciding = matches!(input.action, DiscountApprovalAction::Decide(_));
        let permission = authorize(&mut tx, actor, &input.scope, !deciding, deciding).await;
        if let Err(e) = permission {
            evidence(&mut tx, actor, input, correlation, now, Err(e)).await?;
            tx.commit().await.map_err(|_| E::Unavailable)?;
            return Err(e);
        }
        let hash =
            Sha256::digest(serde_json::to_vec(&(actor.user_id, input)).map_err(|_| E::Invalid)?)
                .to_vec();
        let retry=sqlx::query("SELECT request_hash,response_json FROM sync.quotation_approval_receipts WHERE tenant_id=$1 AND idempotency_key=$2").bind(actor.tenant_id.value()).bind(input.idempotency_key.value()).fetch_optional(&mut *tx).await.map_err(|_|E::Unavailable)?;
        if let Some(row) = retry {
            if row.get::<Vec<u8>, _>("request_hash") != hash {
                evidence(&mut tx, actor, input, correlation, now, Err(E::Invalid)).await?;
                tx.commit().await.map_err(|_| E::Unavailable)?;
                return Err(E::Invalid);
            }
            return serde_json::from_slice(&row.get::<Vec<u8>, _>("response_json"))
                .map_err(|_| E::Unavailable);
        }
        let result = apply(&mut tx, actor, input, correlation, now).await;
        match result {
            Ok(value) => {
                evidence(&mut tx, actor, input, correlation, now, Ok(())).await?;
                sqlx::query("INSERT INTO sync.quotation_approval_receipts VALUES($1,$2,$3,$4)")
                    .bind(actor.tenant_id.value())
                    .bind(input.idempotency_key.value())
                    .bind(hash)
                    .bind(serde_json::to_vec(&value).map_err(|_| E::Unavailable)?)
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| E::Unavailable)?;
                tx.commit().await.map_err(|_| E::Unavailable)?;
                Ok(value)
            }
            Err(e) => {
                // Failed promotion, validation, or persistence never commits a partial transition.
                if e == E::Unavailable {
                    return Err(e);
                }
                tx.rollback().await.map_err(|_| E::Unavailable)?;
                let mut denied = tenant_transaction(&self.pool, actor.tenant_id)
                    .await
                    .map_err(|_| E::Unavailable)?;
                evidence(&mut denied, actor, input, correlation, now, Err(e)).await?;
                denied.commit().await.map_err(|_| E::Unavailable)?;
                Err(e)
            }
        }
    }
}

async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    request: bool,
    decide: bool,
) -> Result<Uuid, E> {
    if !matches!(scope.kind.as_str(), "branch" | "organization") {
        return Err(E::Denied);
    }
    let org: Option<Uuid> = sqlx::query_scalar(
        "SELECT organization_id FROM control.branches WHERE tenant_id=$1 AND branch_id=$2",
    )
    .bind(actor.tenant_id.value())
    .bind(scope.id.value())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| E::Unavailable)?;
    let organization = if scope.kind.as_str() == "branch" {
        org.ok_or(E::Denied)?
    } else {
        scope.id.value()
    };
    let manager = crate::pricing::manager_allowed(tx, actor, organization)
        .await
        .map_err(|_| E::Unavailable)?;
    let reception:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.relationship_tuples WHERE tenant_id=$1 AND subject_principal_id=$2 AND subject_kind='user' AND object_kind='branch' AND object_id=$3 AND relation='eitmad.relation.organization.receptionist.v1')").bind(actor.tenant_id.value()).bind(actor.user_id.value()).bind(scope.id.value()).fetch_one(&mut **tx).await.map_err(|_|E::Unavailable)?;
    let allowed = if request {
        reception && scope.kind.as_str() == "branch"
    } else if decide {
        manager
    } else {
        manager || reception && scope.kind.as_str() == "branch"
    };
    if allowed {
        Ok(organization)
    } else {
        Err(E::Denied)
    }
}

async fn latest(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    id: QuotationDraftId,
) -> Result<Option<DiscountApproval>, E> {
    let row:Option<Vec<u8>>=sqlx::query_scalar("SELECT record_json FROM sync.quotation_approval_history WHERE tenant_id=$1 AND draft_id=$2 ORDER BY revision DESC LIMIT 1").bind(actor.tenant_id.value()).bind(id.value()).fetch_optional(&mut **tx).await.map_err(|_|E::Unavailable)?;
    row.map(|r| serde_json::from_slice(&r).map_err(|_| E::Unavailable))
        .transpose()
}

pub(super) async fn invalidate_for_draft(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    snapshot: &QuotationDraftSnapshot,
    now: UnixMillis,
) -> Result<(), E> {
    let Some(mut a) = latest(tx, actor, snapshot.id).await? else {
        return Ok(());
    };
    if a.scope != *scope {
        return Err(E::Denied);
    }
    if matches!(
        a.state,
        DiscountApprovalState::Pending | DiscountApprovalState::Approved
    ) && !same_commercial_terms(&a.quotation, snapshot)
    {
        a.state = DiscountApprovalState::Invalidated;
        a.revision += 1;
        a.correlation_id = CorrelationId::new(Uuid::new_v4());
        persist(tx, actor, &a, now).await?;
        let input = ConfirmDiscountApproval {
            scope: scope.clone(),
            idempotency_key: eitmad_contracts::transport::IdempotencyKey::new(Uuid::new_v4()),
            action: DiscountApprovalAction::Refresh(snapshot.clone()),
        };
        evidence(tx, actor, &input, a.correlation_id, now, Ok(())).await?;
    }
    Ok(())
}

async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmDiscountApproval,
    correlation: CorrelationId,
    now: UnixMillis,
) -> Result<Option<DiscountApproval>, E> {
    let organization = authorize(
        tx,
        actor,
        &input.scope,
        !matches!(input.action, DiscountApprovalAction::Decide(_)),
        matches!(input.action, DiscountApprovalAction::Decide(_)),
    )
    .await?;
    match &input.action {
        DiscountApprovalAction::Decide(command) => {
            apply_decision(tx, actor, input, command, organization, correlation, now).await
        }
        DiscountApprovalAction::Request(snapshot) | DiscountApprovalAction::Refresh(snapshot) => {
            if snapshot.evaluation.scope != input.scope {
                return Err(E::Denied);
            }
            let prior = load_draft(tx, actor, &input.scope, snapshot.id).await?;
            let expected = prior.as_ref().map(|r| r.revision);
            if let Some(prior) = &prior {
                if snapshot.revision == prior.revision && snapshot != prior {
                    return Err(E::Conflict);
                }
                if snapshot.revision != prior.revision
                    && Some(snapshot.revision) != prior.revision.checked_add(1)
                {
                    return Err(E::Conflict);
                }
            } else if snapshot.revision != 1 {
                return Err(E::Conflict);
            }
            let mut active = latest(tx, actor, snapshot.id).await?;
            if active
                .as_ref()
                .is_some_and(|a| a.scope != input.scope || a.organization_id != organization)
            {
                return Err(E::Denied);
            }
            let requesting = matches!(input.action, DiscountApprovalAction::Request(_));
            if requesting
                && snapshot
                    .evaluation
                    .totals
                    .as_ref()
                    .is_none_or(|t| !t.approval_required)
            {
                return Err(E::Invalid);
            }
            // The server independently verifies customer, catalog, options, quantities, and integer money.
            if prior.as_ref() != Some(snapshot) {
                let change = draft_change(input, snapshot, expected)?;
                crate::quotation_draft::snapshot(&change).map_err(|_| E::Invalid)?;
                crate::quotation_draft::retain_snapshot(tx, actor, &change, now)
                    .await
                    .map_err(map_operation)?;
                crate::operations::append_domain_change(
                    tx,
                    actor,
                    change,
                    SyncMode::LocalFirst,
                    now,
                )
                .await
                .map_err(map_operation)?;
                active = latest(tx, actor, snapshot.id).await?;
            }
            if !requesting {
                return Ok(active);
            }
            validate_current_prices(tx, actor, snapshot, organization).await?;
            if let Some(a) = &active {
                if matches!(
                    a.state,
                    DiscountApprovalState::Pending | DiscountApprovalState::Approved
                ) && same_commercial_terms(&a.quotation, snapshot)
                {
                    return Ok(active);
                }
            }
            let value = pending_request(
                actor,
                &input.scope,
                snapshot,
                organization,
                active.as_ref(),
                correlation,
                now,
            )?;
            persist(tx, actor, &value, now).await?;
            Ok(Some(value))
        }
    }
}

async fn apply_decision(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmDiscountApproval,
    command: &eitmad_contracts::quotation_approval::DecideDiscountApproval,
    organization: Uuid,
    correlation: CorrelationId,
    now: UnixMillis,
) -> Result<Option<DiscountApproval>, E> {
    let mut current = latest(tx, actor, command.draft_id)
        .await?
        .ok_or(E::Conflict)?;
    if current.organization_id != organization
        || input.scope.kind.as_str() == "branch" && current.scope != input.scope
    {
        return Err(E::Denied);
    }
    let draft = load_draft(tx, actor, &current.scope, command.draft_id)
        .await?
        .ok_or(E::Conflict)?;
    current = decide_approval(
        &current,
        &draft,
        PrincipalId::new(actor.user_id.value()),
        command,
        now,
    )?;
    current.correlation_id = correlation;
    persist(tx, actor, &current, now).await?;
    Ok(Some(current))
}

fn pending_request(
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    snapshot: &QuotationDraftSnapshot,
    organization: Uuid,
    active: Option<&DiscountApproval>,
    correlation: CorrelationId,
    now: UnixMillis,
) -> Result<DiscountApproval, E> {
    let validity_days = 30;
    // Asia/Aden has a fixed UTC+03:00 offset; proposed expiry is the end of the calendar day.
    let valid_until = UnixMillis(
        ((now.0 + 10_800_000).div_euclid(86_400_000) + i64::from(validity_days) + 1) * 86_400_000
            - 10_800_000
            - 1,
    );
    let value = DiscountApproval {
        scope: scope.clone(),
        organization_id: organization,
        request_id: DiscountRequestId::new(Uuid::new_v4()),
        revision: active.map_or(Ok(1), |a| a.revision.checked_add(1).ok_or(E::Unavailable))?,
        quotation: snapshot.clone(),
        fingerprint: approval_fingerprint(
            actor.tenant_id,
            organization,
            snapshot,
            validity_days,
            valid_until,
        )?,
        validity_days,
        proposed_valid_until: valid_until,
        requester: PrincipalId::new(actor.user_id.value()),
        requested_at: now,
        state: DiscountApprovalState::Pending,
        decider: None,
        decided_at: None,
        reason: None,
        correlation_id: correlation,
    };
    Ok(value)
}

fn map_operation(e: crate::OperationError) -> E {
    match e {
        crate::OperationError::Unavailable => E::Unavailable,
        crate::OperationError::Denied => E::Denied,
        _ => E::Invalid,
    }
}
fn draft_change(
    input: &ConfirmDiscountApproval,
    s: &QuotationDraftSnapshot,
    base: Option<u64>,
) -> Result<LocalOperationDraft, E> {
    Ok(LocalOperationDraft {
        change_id: ChangeId::new(Uuid::new_v4()),
        scope: input.scope.clone(),
        schema_id: SchemaId::parse(QUOTATION_DRAFT_SCHEMA).expect("schema"),
        schema_version: 1,
        record_id: RecordId::new(s.id.value()),
        operation: ChangeOperation::Upsert,
        base_revision: base,
        idempotency_key: eitmad_contracts::transport::IdempotencyKey::new(Uuid::new_v4()),
        payload: Some(EncodedDomainPayload {
            schema_id: SchemaId::parse(QUOTATION_DRAFT_SCHEMA).expect("schema"),
            schema_version: 1,
            base64: STANDARD.encode(serde_json::to_vec(s).map_err(|_| E::Invalid)?),
        }),
    })
}
async fn load_draft(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    id: QuotationDraftId,
) -> Result<Option<QuotationDraftSnapshot>, E> {
    let row:Option<Vec<u8>>=sqlx::query_scalar("SELECT snapshot_json FROM sync.quotation_draft_revisions WHERE tenant_id=$1 AND branch_id=$2 AND draft_id=$3 ORDER BY revision DESC LIMIT 1").bind(actor.tenant_id.value()).bind(scope.id.value()).bind(id.value()).fetch_optional(&mut **tx).await.map_err(|_|E::Unavailable)?;
    row.map(|b| serde_json::from_slice(&b).map_err(|_| E::Unavailable))
        .transpose()
}
async fn validate_current_prices(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    s: &QuotationDraftSnapshot,
    organization: Uuid,
) -> Result<(), E> {
    for line in &s.intent.lines {
        let selection = &line.configuration.selection;
        let (kind, id, variant) = selection.target.identity();
        let public: Option<serde_json::Value> = sqlx::query_scalar("SELECT change_json FROM sync.records WHERE tenant_id=$1 AND scope_kind='organization' AND scope_id=$2 AND schema_id=$3 AND record_id=$4 AND NOT tombstone")
            .bind(actor.tenant_id.value()).bind(organization).bind(crate::catalog_sync::PUBLIC_SCHEMA)
            .bind(eitmad_pricing::catalog_record_id(&format!("{kind}:{id}:{variant}"))).fetch_optional(&mut **tx).await.map_err(|_|E::Unavailable)?;
        let change: eitmad_contracts::sync::ChangeRecord =
            serde_json::from_value(public.ok_or(E::Conflict)?).map_err(|_| E::Unavailable)?;
        let entry: eitmad_contracts::catalog_revision::CatalogEntry = serde_json::from_slice(
            &STANDARD
                .decode(&change.payload.ok_or(E::Unavailable)?.base64)
                .map_err(|_| E::Unavailable)?,
        )
        .map_err(|_| E::Unavailable)?;
        if entry.price.target != selection.target
            || entry.price.revision != selection.price_revision
        {
            return Err(E::Conflict);
        }
        let current:Option<Vec<u8>>=sqlx::query_scalar("SELECT record_json FROM sync.price_revisions WHERE tenant_id=$1 AND organization_id=$2 AND kind=$3 AND entry_id=$4 AND variant_id=$5 ORDER BY revision DESC LIMIT 1").bind(actor.tenant_id.value()).bind(organization).bind(kind).bind(id).bind(variant).fetch_optional(&mut **tx).await.map_err(|_|E::Unavailable)?;
        let current: eitmad_contracts::pricing::PublishedPrice =
            serde_json::from_slice(&current.ok_or(E::Conflict)?).map_err(|_| E::Unavailable)?;
        if current.target != selection.target || current.revision != selection.price_revision {
            return Err(E::Conflict);
        }
    }
    Ok(())
}
async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    a: &DiscountApproval,
    now: UnixMillis,
) -> Result<(), E> {
    sqlx::query("INSERT INTO sync.quotation_approval_history VALUES($1,$2,$3,$4,$5,$6)")
        .bind(actor.tenant_id.value())
        .bind(a.organization_id)
        .bind(a.scope.id.value())
        .bind(a.quotation.id.value())
        .bind(i64::try_from(a.revision).map_err(|_| E::Unavailable)?)
        .bind(serde_json::to_vec(a).map_err(|_| E::Unavailable)?)
        .execute(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    for scope in [
        a.scope.clone(),
        ScopeRef {
            kind: eitmad_contracts::identity::ScopeKind::parse("organization").expect("scope"),
            id: eitmad_contracts::identity::ScopeId::new(a.organization_id),
        },
    ] {
        crate::operations::append_domain_change(
            tx,
            actor,
            LocalOperationDraft {
                scope,
                schema_id: SchemaId::parse(DISCOUNT_APPROVAL_SCHEMA).expect("schema"),
                schema_version: 1,
                record_id: RecordId::new(a.quotation.id.value()),
                operation: ChangeOperation::Upsert,
                change_id: ChangeId::new(Uuid::new_v4()),
                base_revision: None,
                idempotency_key: eitmad_contracts::transport::IdempotencyKey::new(Uuid::new_v4()),
                payload: Some(EncodedDomainPayload {
                    schema_id: SchemaId::parse(DISCOUNT_APPROVAL_SCHEMA).expect("schema"),
                    schema_version: 1,
                    base64: STANDARD.encode(serde_json::to_vec(a).map_err(|_| E::Unavailable)?),
                }),
            },
            SyncMode::ServerAuthoritative,
            now,
        )
        .await
        .map_err(map_operation)?;
    }
    sqlx::query("SELECT pg_notify('eitmad_quotation_approvals','changed')")
        .execute(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    Ok(())
}
async fn evidence(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmDiscountApproval,
    correlation: CorrelationId,
    now: UnixMillis,
    result: Result<(), E>,
) -> Result<(), E> {
    let (id, operation) = match &input.action {
        DiscountApprovalAction::Request(s) => (s.id, "eitmad.quotation-approval.request.v1"),
        DiscountApprovalAction::Refresh(s) => (s.id, "eitmad.quotation-approval.refresh.v1"),
        DiscountApprovalAction::Decide(c) => (
            c.draft_id,
            match c.decision {
                DiscountDecision::Approve => "eitmad.quotation-approval.approve.v1",
                DiscountDecision::Reject => "eitmad.quotation-approval.reject.v1",
            },
        ),
    };
    append(
        tx,
        &ServerAuditEnvelope::from_session(
            actor,
            input.scope.clone(),
            ServerAuditEvent {
                operation,
                outcome: match result {
                    Ok(()) => ServerAuditOutcome::Succeeded,
                    Err(E::Denied) => ServerAuditOutcome::Denied,
                    Err(E::Conflict) => ServerAuditOutcome::Conflict,
                    _ => ServerAuditOutcome::Invalid,
                },
                target_kind: "quotation-approval",
                target_id: Some(id.value()),
                correlation_id: correlation,
                causation_id: None,
                idempotency_key: Some(input.idempotency_key),
                redacted_error: result.err().map(eitmad_pricing::approval_error_code),
                occurred_at: now,
            },
        ),
    )
    .await
    .map_err(|_| E::Unavailable)
}

#[async_trait::async_trait]
impl DomainSyncHandler for QuotationApprovalServer {
    fn descriptor(&self) -> DomainDescriptor {
        DomainDescriptor {
            schema_id: SchemaId::parse(DISCOUNT_APPROVAL_SCHEMA).expect("schema"),
            minimum_schema_version: 1,
            maximum_schema_version: 1,
            mode: SyncMode::ServerAuthoritative,
        }
    }
    async fn authorize(
        &self,
        session: &AuthenticatedServerSession,
        scope: &ScopeRef,
        intent: SyncIntent,
    ) -> bool {
        if intent != SyncIntent::Read {
            return false;
        }
        let Ok(mut tx) = tenant_transaction(&self.pool, session.tenant_id).await else {
            return false;
        };
        authorize(&mut tx, session, scope, false, false)
            .await
            .is_ok()
    }
    fn validate_local(&self, _: &LocalOperationDraft) -> Result<(), DomainValidationError> {
        Err(DomainValidationError::Denied)
    }
}
