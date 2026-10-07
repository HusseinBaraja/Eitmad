//! Server-confirmed quotation history, expiry, CAS transitions, numbering, and receipts.
use crate::{database::tenant_transaction, quotation_approval as approvals};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{
    identity::{PrincipalId, ScopeRef},
    quotation_approval::{ConfirmDiscountApproval, DiscountApprovalAction, DiscountApprovalState},
    quotation_draft::{QuotationDraftId, QuotationDraftSnapshot},
    quotation_lifecycle::{
        ConfirmQuotation, QuotationAction, QuotationPage, QuotationPermittedAction as A,
        QuotationRecord, QuotationState as S, ReadQuotations,
    },
    server::AuthenticatedServerSession,
    sync::{ChangeId, ChangeOperation, EncodedDomainPayload, RecordId, SyncMode},
    transport::{CorrelationId, IdempotencyKey, SchemaId, UnixMillis},
};
use eitmad_pricing::{
    QUOTATION_LIFECYCLE_SCHEMA, QuotationError as E, issuance_snapshot, quotation_actions,
    quotation_expiry,
};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct QuotationLifecycleServer {
    pool: PgPool,
}
impl QuotationLifecycleServer {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Reads scoped records and commits overdue expiry before returning actions.
    /// # Errors
    /// Denies foreign scopes and unavailable authoritative reads.
    pub async fn list(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ReadQuotations,
        now: UnixMillis,
    ) -> Result<QuotationPage, E> {
        if !(1..=100).contains(&input.query.limit) {
            return Err(E::Invalid);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        lock(&mut tx, actor).await?;
        let organization = approvals::authorize(&mut tx, actor, &input.scope, false, false)
            .await
            .map_err(map_approval)?;
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT DISTINCT draft_id FROM sync.quotation_draft_revisions d JOIN control.branches b ON b.tenant_id=d.tenant_id AND b.branch_id=d.branch_id WHERE d.tenant_id=$1 AND (($2='branch' AND d.branch_id=$3) OR ($2='organization' AND b.organization_id=$3)) AND ($4::uuid IS NULL OR d.draft_id>$4) ORDER BY draft_id LIMIT $5")
            .bind(actor.tenant_id.value()).bind(input.scope.kind.as_str()).bind(input.scope.id.value()).bind(input.query.after.map(QuotationDraftId::value)).bind(i64::from(input.query.limit)+1).fetch_all(&mut *tx).await.map_err(|_| E::Unavailable)?;
        let more = ids.len() > input.query.limit as usize;
        let mut items = Vec::new();
        for id in ids.into_iter().take(input.query.limit as usize) {
            let mut value =
                load_or_initialize(&mut tx, actor, QuotationDraftId::new(id), organization, now)
                    .await?;
            if value.organization_id != organization {
                return Err(E::Denied);
            }
            expire(&mut tx, actor, &mut value, now).await?;
            project_actions(&mut tx, actor, &mut value, now).await?;
            items.push(value);
        }
        tx.commit().await.map_err(|_| E::Unavailable)?;
        let next = if more {
            items.last().map(|v| v.quotation.id)
        } else {
            None
        };
        Ok(QuotationPage {
            items,
            next,
            server_available: true,
        })
    }

    /// Commits expiry for the scoped authoritative records, including inactive UI pages.
    /// # Errors
    /// Denies foreign scopes and rolls back expiry if audit or publication fails.
    pub async fn expire_due(
        &self,
        actor: &AuthenticatedServerSession,
        scope: &ScopeRef,
        now: UnixMillis,
    ) -> Result<(), E> {
        let mut after = None;
        loop {
            let page = self
                .list(
                    actor,
                    &ReadQuotations {
                        scope: scope.clone(),
                        query: eitmad_contracts::quotation_lifecycle::ListQuotations {
                            after,
                            limit: 100,
                        },
                    },
                    now,
                )
                .await?;
            after = page.next;
            if after.is_none() {
                return Ok(());
            }
        }
    }

    /// Serializes with draft/approval/catalog writes and commits history, audit, receipt, and events.
    /// # Errors
    /// Rejects forbidden roles, stale input, invalid approvals, and competing transitions.
    pub async fn transition(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ConfirmQuotation,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<QuotationRecord, E> {
        self.expire_due(actor, &input.scope, now).await?;
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        lock(&mut tx, actor).await?;
        let mut value = latest(&mut tx, actor, input.action.draft_id())
            .await?
            .ok_or(E::Conflict)?;
        let result = self
            .authorized_transition(&mut tx, actor, input, &mut value, correlation, now)
            .await;
        match result {
            Ok(value) => {
                tx.commit().await.map_err(|_| E::Unavailable)?;
                Ok(value)
            }
            Err(E::Unavailable) => Err(E::Unavailable),
            Err(e) => {
                tx.rollback().await.map_err(|_| E::Unavailable)?;
                let mut failed = tenant_transaction(&self.pool, actor.tenant_id)
                    .await
                    .map_err(|_| E::Unavailable)?;
                evidence(&mut failed, actor, input, correlation, now, Err(e)).await?;
                failed.commit().await.map_err(|_| E::Unavailable)?;
                Err(e)
            }
        }
    }

    async fn authorized_transition(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: &AuthenticatedServerSession,
        input: &ConfirmQuotation,
        value: &mut QuotationRecord,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<QuotationRecord, E> {
        authorize_action(tx, actor, input, value).await?;
        let hash =
            Sha256::digest(serde_json::to_vec(&(actor.user_id, input)).map_err(|_| E::Invalid)?)
                .to_vec();
        if let Some(row) = sqlx::query("SELECT request_hash,response_json FROM sync.quotation_receipts WHERE tenant_id=$1 AND idempotency_key=$2")
            .bind(actor.tenant_id.value()).bind(input.idempotency_key.value()).fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)? {
            if row.get::<Vec<u8>, _>("request_hash") != hash { return Err(E::Invalid); }
            let mut result: QuotationRecord = serde_json::from_slice(&row.get::<Vec<u8>, _>("response_json")).map_err(|_| E::Unavailable)?;
            project_actions(tx, actor, &mut result, now).await?;
            return Ok(result);
        }
        if input.action.expected_revision() != value.revision {
            return Err(E::Conflict);
        }
        self.apply(tx, actor, input, value, now).await?;
        value.revision = value.revision.checked_add(1).ok_or(E::Unavailable)?;
        value.changed_at = now;
        value.changed_by = PrincipalId::new(actor.user_id.value());
        value.permitted_actions.clear();
        persist(tx, actor, value, now).await?;
        evidence(tx, actor, input, correlation, now, Ok(())).await?;
        sqlx::query("INSERT INTO sync.quotation_receipts VALUES($1,$2,$3,$4)")
            .bind(actor.tenant_id.value())
            .bind(input.idempotency_key.value())
            .bind(hash)
            .bind(serde_json::to_vec(value).map_err(|_| E::Unavailable)?)
            .execute(&mut **tx)
            .await
            .map_err(|_| E::Unavailable)?;
        project_actions(tx, actor, value, now).await?;
        Ok(value.clone())
    }

    async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: &AuthenticatedServerSession,
        input: &ConfirmQuotation,
        value: &mut QuotationRecord,
        now: UnixMillis,
    ) -> Result<(), E> {
        match &input.action {
            QuotationAction::Issue(command) => {
                let draft = approvals::load_draft(tx, actor, &value.scope, command.draft_id)
                    .await
                    .map_err(map_approval)?
                    .ok_or(E::Conflict)?;
                if draft.revision != command.expected_draft_revision {
                    return Err(E::Conflict);
                }
                let approval = approvals::latest(tx, actor, command.draft_id)
                    .await
                    .map_err(map_approval)?;
                let snapshot =
                    issuance_snapshot(actor.tenant_id, value, &draft, approval.as_ref(), now)?;
                approvals::validate_current_prices(tx, actor, &snapshot, value.organization_id)
                    .await
                    .map_err(|e| {
                        if e == eitmad_pricing::ApprovalError::Conflict {
                            E::StalePrice
                        } else {
                            map_approval(e)
                        }
                    })?;
                validate_active_customer(tx, actor, &snapshot).await?;
                value.quotation = snapshot;
                if value.number.is_none() {
                    value.number = Some(
                        self.reserve_number(actor, value.organization_id, now)
                            .await?,
                    );
                }
                value.state = S::Issued;
                value.issued_at = Some(now);
                value.valid_until = Some(quotation_expiry(now, value.validity_days)?);
                if value
                    .quotation
                    .evaluation
                    .totals
                    .as_ref()
                    .is_some_and(|t| t.approval_required)
                {
                    let a = approval.ok_or(E::ApprovalRequired)?;
                    value.approval_request_id = Some(a.request_id);
                    value.approval_fingerprint = Some(a.fingerprint);
                }
            }
            QuotationAction::SetValidity(c) => {
                if !matches!(value.state, S::Draft | S::PendingApproval) {
                    return Err(E::Conflict);
                }
                quotation_expiry(now, c.validity_days)?;
                value.validity_days = c.validity_days;
                value.state = S::Draft;
                approvals::invalidate(tx, actor, &value.scope, value.quotation.id, now)
                    .await
                    .map_err(map_approval)?;
            }
            QuotationAction::Revise(c) => {
                if !matches!(value.state, S::Issued | S::Expired) {
                    return Err(E::Conflict);
                }
                quotation_expiry(now, c.validity_days)?;
                value.validity_days = c.validity_days;
                value.state = S::Draft;
                value.document_revision = value
                    .document_revision
                    .checked_add(1)
                    .ok_or(E::Unavailable)?;
                value.issued_at = None;
                value.valid_until = None;
                value.approval_request_id = None;
                value.approval_fingerprint = None;
                approvals::invalidate(tx, actor, &value.scope, value.quotation.id, now)
                    .await
                    .map_err(map_approval)?;
                revise_draft(tx, actor, value, now).await?;
            }
            QuotationAction::Cancel(c) => {
                if value.state != S::Issued {
                    return Err(E::Conflict);
                }
                let reason = c.reason.trim();
                if reason.is_empty()
                    || reason.chars().count() > 240
                    || reason.chars().any(char::is_control)
                {
                    return Err(E::Invalid);
                }
                value.state = S::Cancelled;
                value.cancellation_reason = Some(reason.to_owned());
            }
        }
        Ok(())
    }

    async fn reserve_number(
        &self,
        actor: &AuthenticatedServerSession,
        organization: Uuid,
        now: UnixMillis,
    ) -> Result<String, E> {
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        let year: i32 = sqlx::query_scalar("SELECT extract(year FROM to_timestamp($1::double precision/1000) AT TIME ZONE 'Asia/Aden')::integer")
            .bind(now.0).fetch_one(&mut *tx).await.map_err(|_| E::Unavailable)?;
        let number: i64 = sqlx::query_scalar("INSERT INTO sync.quotation_numbers VALUES($1,$2,$3,1) ON CONFLICT(tenant_id,organization_id,calendar_year) DO UPDATE SET last_number=sync.quotation_numbers.last_number+1 RETURNING last_number")
            .bind(actor.tenant_id.value()).bind(organization).bind(year).fetch_one(&mut *tx).await.map_err(|_| E::Unavailable)?;
        tx.commit().await.map_err(|_| E::Unavailable)?;
        Ok(format!("QT-{year}-{number:05}"))
    }
}

async fn lock(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
) -> Result<(), E> {
    sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
        .bind(actor.tenant_id.value())
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    Ok(())
}
fn map_approval(e: eitmad_pricing::ApprovalError) -> E {
    match e {
        eitmad_pricing::ApprovalError::Denied => E::Denied,
        eitmad_pricing::ApprovalError::Invalid => E::Invalid,
        eitmad_pricing::ApprovalError::Conflict => E::Conflict,
        eitmad_pricing::ApprovalError::Unavailable => E::Unavailable,
    }
}
pub(super) async fn latest(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    id: QuotationDraftId,
) -> Result<Option<QuotationRecord>, E> {
    let row: Option<Vec<u8>> = sqlx::query_scalar("SELECT record_json FROM sync.quotation_history WHERE tenant_id=$1 AND draft_id=$2 ORDER BY revision DESC LIMIT 1")
        .bind(actor.tenant_id.value()).bind(id.value()).fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?;
    row.map(|r| serde_json::from_slice(&r).map_err(|_| E::Unavailable))
        .transpose()
}
// Existing retained drafts predate the lifecycle projection; their original revisions remain authority.
async fn load_or_initialize(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    id: QuotationDraftId,
    organization: Uuid,
    now: UnixMillis,
) -> Result<QuotationRecord, E> {
    if let Some(value) = latest(tx, actor, id).await? {
        return Ok(value);
    }
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT snapshot_json FROM sync.quotation_draft_revisions WHERE tenant_id=$1 AND draft_id=$2 ORDER BY revision DESC LIMIT 1")
        .bind(actor.tenant_id.value()).bind(id.value()).fetch_one(&mut **tx).await.map_err(|_| E::Unavailable)?;
    let snapshot: QuotationDraftSnapshot =
        serde_json::from_slice(&bytes).map_err(|_| E::Unavailable)?;
    draft_changed(
        tx,
        actor,
        &snapshot.evaluation.scope,
        &snapshot,
        organization,
        now,
    )
    .await?;
    let input = ConfirmQuotation {
        scope: snapshot.evaluation.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: QuotationAction::SetValidity(
            eitmad_contracts::quotation_lifecycle::SetQuotationValidity {
                draft_id: id,
                expected_revision: 0,
                validity_days: 30,
            },
        ),
    };
    evidence_operation(
        tx,
        actor,
        &input,
        CorrelationId::new(Uuid::new_v4()),
        now,
        Ok(()),
        "eitmad.quotation.initialize.v1",
    )
    .await?;
    latest(tx, actor, id).await?.ok_or(E::Unavailable)
}
async fn authorize_action(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmQuotation,
    value: &QuotationRecord,
) -> Result<(), E> {
    let issuing = matches!(input.action, QuotationAction::Issue(_));
    let org = approvals::authorize(tx, actor, &input.scope, issuing, !issuing)
        .await
        .map_err(map_approval)?;
    if org != value.organization_id
        || input.scope.kind.as_str() == "branch" && input.scope != value.scope
    {
        return Err(E::Denied);
    }
    Ok(())
}
async fn project_actions(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    value: &mut QuotationRecord,
    now: UnixMillis,
) -> Result<(), E> {
    let reception = approvals::authorize(tx, actor, &value.scope, true, false)
        .await
        .is_ok();
    let manager = approvals::authorize(tx, actor, &value.scope, false, true)
        .await
        .is_ok();
    if matches!(value.state, S::Draft | S::PendingApproval) {
        let a = approvals::latest(tx, actor, value.quotation.id)
            .await
            .map_err(map_approval)?;
        value.state = if a.as_ref().is_some_and(|v| {
            matches!(
                v.state,
                DiscountApprovalState::Pending | DiscountApprovalState::Approved
            )
        }) {
            S::PendingApproval
        } else {
            S::Draft
        };
        value.permitted_actions = quotation_actions(value, reception, manager);
        if reception
            && issuance_snapshot(actor.tenant_id, value, &value.quotation, a.as_ref(), now).is_ok()
            && !value.permitted_actions.contains(&A::Issue)
        {
            value.permitted_actions.push(A::Issue);
        }
    } else {
        value.permitted_actions = quotation_actions(value, reception, manager);
    }
    Ok(())
}

async fn validate_active_customer(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    draft: &QuotationDraftSnapshot,
) -> Result<(), E> {
    let id = draft.intent.customer.as_ref().ok_or(E::Invalid)?.id;
    let row: Option<serde_json::Value> = sqlx::query_scalar("SELECT change_json FROM sync.records WHERE tenant_id=$1 AND scope_kind='branch' AND scope_id=$2 AND schema_id='eitmad.schema.customer.v1' AND record_id=$3 AND NOT tombstone")
        .bind(actor.tenant_id.value()).bind(draft.evaluation.scope.id.value()).bind(id.value()).fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?;
    let change: eitmad_contracts::sync::ChangeRecord =
        serde_json::from_value(row.ok_or(E::Invalid)?).map_err(|_| E::Unavailable)?;
    let customer: eitmad_contracts::customer::CustomerSyncPayload = serde_json::from_slice(
        &STANDARD
            .decode(change.payload.ok_or(E::Invalid)?.base64)
            .map_err(|_| E::Unavailable)?,
    )
    .map_err(|_| E::Unavailable)?;
    if customer.customer_id != id {
        return Err(E::Invalid);
    }
    Ok(())
}

pub(super) async fn draft_changed(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    snapshot: &QuotationDraftSnapshot,
    organization: Uuid,
    now: UnixMillis,
) -> Result<(), E> {
    let mut value = if let Some(mut prior) = latest(tx, actor, snapshot.id).await? {
        if !matches!(prior.state, S::Draft | S::PendingApproval)
            || prior.scope != *scope
            || prior.quotation.cancelled
        {
            return Err(E::Conflict);
        }
        prior.revision = prior.revision.checked_add(1).ok_or(E::Unavailable)?;
        prior.quotation = snapshot.clone();
        prior
    } else {
        QuotationRecord {
            scope: scope.clone(),
            organization_id: organization,
            revision: 1,
            document_revision: 1,
            state: S::Draft,
            quotation: snapshot.clone(),
            number: None,
            validity_days: 30,
            issued_at: None,
            valid_until: None,
            approval_request_id: None,
            approval_fingerprint: None,
            changed_at: now,
            changed_by: PrincipalId::new(actor.user_id.value()),
            cancellation_reason: None,
            permitted_actions: vec![],
        }
    };
    value.state = if snapshot.cancelled {
        S::Cancelled
    } else {
        S::Draft
    };
    value.changed_at = now;
    value.changed_by = PrincipalId::new(actor.user_id.value());
    persist(tx, actor, &value, now).await
}
async fn revise_draft(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    value: &mut QuotationRecord,
    now: UnixMillis,
) -> Result<(), E> {
    let prior = approvals::load_draft(tx, actor, &value.scope, value.quotation.id)
        .await
        .map_err(map_approval)?
        .ok_or(E::Conflict)?;
    value.quotation.revision = prior.revision.checked_add(1).ok_or(E::Unavailable)?;
    let input = ConfirmDiscountApproval {
        scope: value.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: DiscountApprovalAction::Refresh(value.quotation.clone()),
    };
    let change = approvals::draft_change(&input, &value.quotation, Some(prior.revision))
        .map_err(map_approval)?;
    sqlx::query("INSERT INTO sync.quotation_draft_revisions VALUES($1,$2,$3,$4,$5,$6)")
        .bind(actor.tenant_id.value())
        .bind(value.scope.id.value())
        .bind(value.quotation.id.value())
        .bind(i64::try_from(value.quotation.revision).map_err(|_| E::Unavailable)?)
        .bind(
            value
                .quotation
                .intent
                .customer
                .as_ref()
                .ok_or(E::Invalid)?
                .id
                .value(),
        )
        .bind(serde_json::to_vec(&value.quotation).map_err(|_| E::Unavailable)?)
        .execute(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    crate::operations::append_domain_change(tx, actor, change, SyncMode::LocalFirst, now)
        .await
        .map_err(|_| E::Unavailable)?;
    Ok(())
}
async fn expire(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    value: &mut QuotationRecord,
    now: UnixMillis,
) -> Result<(), E> {
    if value.state != S::Issued || value.valid_until.is_none_or(|v| now.0 <= v.0) {
        return Ok(());
    }
    value.state = S::Expired;
    value.revision = value.revision.checked_add(1).ok_or(E::Unavailable)?;
    value.changed_at = now;
    value.changed_by = PrincipalId::new(actor.user_id.value());
    persist(tx, actor, value, now).await?;
    let input = ConfirmQuotation {
        scope: value.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: QuotationAction::Cancel(eitmad_contracts::quotation_lifecycle::CancelQuotation {
            draft_id: value.quotation.id,
            expected_revision: value.revision,
            reason: String::new(),
        }),
    };
    evidence_operation(
        tx,
        actor,
        &input,
        CorrelationId::new(Uuid::new_v4()),
        now,
        Ok(()),
        "eitmad.quotation.expire.v1",
    )
    .await
}
async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    value: &QuotationRecord,
    now: UnixMillis,
) -> Result<(), E> {
    let mut retained = value.clone();
    retained.permitted_actions.clear();
    let json = serde_json::to_vec(&retained).map_err(|_| E::Unavailable)?;
    sqlx::query("INSERT INTO sync.quotation_history VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(actor.tenant_id.value())
        .bind(value.organization_id)
        .bind(value.scope.id.value())
        .bind(value.quotation.id.value())
        .bind(i64::try_from(value.revision).map_err(|_| E::Unavailable)?)
        .bind(format!("{:?}", value.state))
        .bind(value.valid_until.map(|v| v.0))
        .bind(&json)
        .execute(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    for scope in [
        value.scope.clone(),
        ScopeRef {
            kind: eitmad_contracts::identity::ScopeKind::parse("organization").expect("scope"),
            id: eitmad_contracts::identity::ScopeId::new(value.organization_id),
        },
    ] {
        crate::operations::append_domain_change(
            tx,
            actor,
            crate::LocalOperationDraft {
                scope,
                schema_id: SchemaId::parse(QUOTATION_LIFECYCLE_SCHEMA).expect("schema"),
                schema_version: 1,
                record_id: RecordId::new(value.quotation.id.value()),
                operation: ChangeOperation::Upsert,
                change_id: ChangeId::new(Uuid::new_v4()),
                base_revision: None,
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                payload: Some(EncodedDomainPayload {
                    schema_id: SchemaId::parse(QUOTATION_LIFECYCLE_SCHEMA).expect("schema"),
                    schema_version: 1,
                    base64: STANDARD.encode(&json),
                }),
            },
            SyncMode::ServerAuthoritative,
            now,
        )
        .await
        .map_err(|_| E::Unavailable)?;
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
    input: &ConfirmQuotation,
    correlation: CorrelationId,
    now: UnixMillis,
    result: Result<(), E>,
) -> Result<(), E> {
    let operation = match input.action {
        QuotationAction::Issue(_) => "eitmad.quotation.issue.v1",
        QuotationAction::SetValidity(_) => "eitmad.quotation.validity.v1",
        QuotationAction::Revise(_) => "eitmad.quotation.revise.v1",
        QuotationAction::Cancel(_) => "eitmad.quotation.cancel.v1",
    };
    evidence_operation(tx, actor, input, correlation, now, result, operation).await
}
async fn evidence_operation(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmQuotation,
    correlation: CorrelationId,
    now: UnixMillis,
    result: Result<(), E>,
    operation: &'static str,
) -> Result<(), E> {
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
                    Err(E::Conflict | E::StalePrice | E::ApprovalRequired) => {
                        ServerAuditOutcome::Conflict
                    }
                    _ => ServerAuditOutcome::Invalid,
                },
                target_kind: "quotation",
                target_id: Some(input.action.draft_id().value()),
                correlation_id: correlation,
                causation_id: None,
                idempotency_key: Some(input.idempotency_key),
                redacted_error: result.err().map(eitmad_pricing::quotation_error_code),
                occurred_at: now,
            },
        ),
    )
    .await
    .map_err(|_| E::Unavailable)
}

#[async_trait::async_trait]
impl crate::DomainSyncHandler for QuotationLifecycleServer {
    fn descriptor(&self) -> crate::DomainDescriptor {
        crate::DomainDescriptor {
            schema_id: SchemaId::parse(QUOTATION_LIFECYCLE_SCHEMA).expect("schema"),
            minimum_schema_version: 1,
            maximum_schema_version: 1,
            mode: SyncMode::ServerAuthoritative,
        }
    }
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        scope: &ScopeRef,
        intent: crate::SyncIntent,
    ) -> bool {
        if intent != crate::SyncIntent::Read {
            return false;
        }
        let Ok(mut tx) = tenant_transaction(&self.pool, actor.tenant_id).await else {
            return false;
        };
        approvals::authorize(&mut tx, actor, scope, false, false)
            .await
            .is_ok()
    }
    fn validate_local(
        &self,
        _: &crate::LocalOperationDraft,
    ) -> Result<(), crate::DomainValidationError> {
        Err(crate::DomainValidationError::Denied)
    }
}
