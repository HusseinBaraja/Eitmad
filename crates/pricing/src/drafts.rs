//! Local-first quotation draft authority. Issuance is a separate server operation.
use crate::{PricingError, PricingService, quotation::evaluate_on};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_authorization::{
    AuthorizationError, AuthorizationService, MutationContext, QUOTATION_DRAFT_READ_PERMISSION,
    QUOTATION_DRAFT_WRITE_PERMISSION,
};
use eitmad_contracts::{
    events::Event,
    identity::AuthorizationContext,
    quotation::{EvaluateQuotation, QuotationFieldError},
    quotation_draft::{
        CreateQuotationDraft, GetQuotationDraft, ListQuotationDrafts, QuotationDraft,
        QuotationDraftChangeNotice, QuotationDraftId, QuotationDraftPage, QuotationDraftSnapshot,
        QuotationDraftSyncState, UpdateQuotationDraft,
    },
    sync::{ChangeId, ChangeOperation, ChangeRecord, ConflictId, EncodedDomainPayload, RecordId},
    transport::{CorrelationId, SchemaId, UnixMillis},
};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, DurableIdempotency, DurablePublication, QuotationDraftCommit, StorageError,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

pub const QUOTATION_DRAFT_SCHEMA: &str = "eitmad.schema.quotation-draft.v1";
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuotationDraftError {
    Denied,
    Invalid,
    NotFound,
    Unavailable,
    IdempotencyMismatch,
    Conflict {
        expected: Option<u64>,
        actual: Option<u64>,
    },
    Validation(Vec<QuotationFieldError>),
    UnresolvedConflict,
}
impl From<StorageError> for QuotationDraftError {
    fn from(_: StorageError) -> Self {
        Self::Unavailable
    }
}
impl From<PricingError> for QuotationDraftError {
    fn from(e: PricingError) -> Self {
        if e == PricingError::Denied {
            Self::Denied
        } else {
            Self::Unavailable
        }
    }
}

#[derive(Clone)]
pub struct QuotationDraftService {
    store: AuthorityStore,
    authorization: AuthorizationService,
    pricing: PricingService,
}
impl QuotationDraftService {
    #[must_use]
    pub fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            pricing: PricingService::new(store.clone(), authorization.clone()),
            store,
            authorization,
        }
    }
    /// Creates an unnumbered evaluated branch draft with an engine-assigned UUID.
    /// # Errors
    /// Rejects unauthorized, invalid, or unavailable durable work.
    pub fn create(
        &self,
        context: &MutationContext,
        command: &CreateQuotationDraft,
    ) -> Result<QuotationDraft, QuotationDraftError> {
        self.save(
            context,
            "eitmad.quotation-draft.create.v1",
            QuotationDraftId::new(Uuid::new_v4()),
            None,
            &command.intent,
            command,
        )
    }
    /// Updates against the expected revision without replacing a competing edit.
    /// # Errors
    /// Reports validation, stale revisions, retained conflicts, and denied mutations.
    pub fn update(
        &self,
        context: &MutationContext,
        command: &UpdateQuotationDraft,
    ) -> Result<QuotationDraft, QuotationDraftError> {
        self.save(
            context,
            "eitmad.quotation-draft.update.v1",
            command.draft_id,
            Some(command.expected_revision),
            &command.intent,
            command,
        )
    }
    fn save(
        &self,
        context: &MutationContext,
        operation: &str,
        id: QuotationDraftId,
        expected: Option<u64>,
        intent: &EvaluateQuotation,
        command: &impl Serialize,
    ) -> Result<QuotationDraft, QuotationDraftError> {
        if let Err(e) = self.require(&context.authorization, QUOTATION_DRAFT_WRITE_PERMISSION) {
            self.store
                .append_audit(&audit(context, operation, None).with_outcome(
                    AuditOutcome::Denied,
                    Some("eitmad.error.authorization-denied.v1".into()),
                ))?;
            return Err(e);
        }
        let hash: [u8; 32] = Sha256::digest(
            serde_json::to_vec(&(
                1,
                operation,
                context.authorization.identity.principal_id,
                command,
            ))
            .map_err(|_| QuotationDraftError::Unavailable)?,
        )
        .into();
        let result = self.store.transact_pricing(true, |tx| {
            self.require(&context.authorization, QUOTATION_DRAFT_WRITE_PERMISSION)?;
            if let Some((retained, response)) =
                tx.quotation_draft_replay(&context.authorization.scope, context.idempotency_key)?
            {
                return if retained == hash {
                    serde_json::from_slice(&response).map_err(|_| QuotationDraftError::Unavailable)
                } else {
                    Err(QuotationDraftError::IdempotencyMismatch)
                };
            }
            let current = tx.quotation_draft(&context.authorization.scope, id)?;
            let actual = current.as_ref().map(|d| d.snapshot.revision);
            if actual != expected {
                return Err(QuotationDraftError::Conflict { expected, actual });
            }
            if current.is_some_and(|d| {
                matches!(
                    d.sync_state,
                    QuotationDraftSyncState::Conflicted | QuotationDraftSyncState::Rejected
                )
            }) {
                return Err(QuotationDraftError::UnresolvedConflict);
            }
            let catalog_actor = self.pricing.authorize_quotation(&context.authorization)?;
            if intent
                .lines
                .iter()
                .any(|l| l.configuration.selection.target.scope() != &catalog_actor.scope)
            {
                return Err(QuotationDraftError::Denied);
            }
            let evaluation = evaluate_on(tx, &context.authorization, &catalog_actor, intent)?;
            if !evaluation.errors.is_empty() || evaluation.totals.is_none() {
                return Err(QuotationDraftError::Validation(evaluation.errors));
            }
            let revision = expected
                .unwrap_or(0)
                .checked_add(1)
                .filter(|v| i64::try_from(*v).is_ok())
                .ok_or(QuotationDraftError::Unavailable)?;
            let draft = QuotationDraft {
                scope: context.authorization.scope.clone(),
                snapshot: QuotationDraftSnapshot {
                    id,
                    revision,
                    intent: intent.clone(),
                    evaluation,
                },
                updated_at: context.occurred_at,
                sync_state: QuotationDraftSyncState::Pending,
                conflict: None,
            };
            Self::commit_evaluated(tx, context, operation, expected, hash, &draft)?;
            Ok(draft)
        });
        if let Err(e) = &result {
            let (outcome, code) = match e {
                QuotationDraftError::Unavailable => return result,
                QuotationDraftError::Denied => {
                    (AuditOutcome::Denied, "eitmad.error.authorization-denied.v1")
                }
                QuotationDraftError::Conflict { .. } | QuotationDraftError::UnresolvedConflict => (
                    AuditOutcome::Conflict,
                    "eitmad.error.quotation-draft-conflict.v1",
                ),
                _ => (
                    AuditOutcome::Invalid,
                    "eitmad.error.quotation-draft-invalid.v1",
                ),
            };
            self.store.append_audit(
                &audit(context, operation, Some(id)).with_outcome(outcome, Some(code.into())),
            )?;
        }
        result
    }
    fn commit_evaluated(
        tx: &eitmad_storage::PricingTransaction<'_>,
        context: &MutationContext,
        operation: &str,
        expected: Option<u64>,
        hash: [u8; 32],
        draft: &QuotationDraft,
    ) -> Result<(), QuotationDraftError> {
        let change = ChangeRecord {
            change_id: ChangeId::new(Uuid::new_v4()),
            record_id: RecordId::new(draft.snapshot.id.value()),
            scope: draft.scope.clone(),
            operation: ChangeOperation::Upsert,
            base_revision: expected,
            revision: draft.snapshot.revision,
            changed_at: context.occurred_at,
            idempotency_key: context.idempotency_key,
            payload: Some(EncodedDomainPayload {
                schema_id: SchemaId::parse(QUOTATION_DRAFT_SCHEMA).expect("static schema"),
                schema_version: 1,
                base64: STANDARD.encode(
                    serde_json::to_vec(&draft.snapshot)
                        .map_err(|_| QuotationDraftError::Unavailable)?,
                ),
            }),
            merge: None,
        };
        let mut record = audit(context, operation, Some(draft.snapshot.id));
        record.outcome = AuditOutcome::Succeeded;
        record.previous_revision = expected;
        record.resulting_revision = Some(draft.snapshot.revision);
        tx.commit_quotation_draft(&QuotationDraftCommit {
            draft,
            expected_revision: expected,
            operation,
            idempotency: &DurableIdempotency {
                key: context.idempotency_key,
                request_hash: hash,
                response_json: serde_json::to_vec(draft)
                    .map_err(|_| QuotationDraftError::Unavailable)?,
            },
            audit: &record,
            publication: &publication(draft, change.change_id),
            change: &change,
        })?;
        Ok(())
    }
    /// Reads the saved snapshot; catalog changes never reprice this query.
    /// # Errors
    /// Denies foreign scopes and reports missing or unavailable data.
    pub fn get(
        &self,
        actor: &AuthorizationContext,
        query: &GetQuotationDraft,
    ) -> Result<QuotationDraft, QuotationDraftError> {
        self.require(actor, QUOTATION_DRAFT_READ_PERMISSION)?;
        self.store
            .get_quotation_draft(&actor.scope, query.draft_id)?
            .ok_or(QuotationDraftError::NotFound)
    }
    /// Lists only the exact authorized branch.
    /// # Errors
    /// Denies unauthorized reads and invalid page limits.
    pub fn list(
        &self,
        actor: &AuthorizationContext,
        query: &ListQuotationDrafts,
    ) -> Result<QuotationDraftPage, QuotationDraftError> {
        self.require(actor, QUOTATION_DRAFT_READ_PERMISSION)?;
        if !(1..=100).contains(&query.limit) {
            return Err(QuotationDraftError::Invalid);
        }
        self.store
            .list_quotation_drafts(&actor.scope, query.after, query.limit)
            .map_err(Into::into)
    }
    /// Loads a bounded authorized publication batch.
    /// # Errors
    /// Denies mutation access after role revocation.
    pub fn sync_batch(
        &self,
        actor: &AuthorizationContext,
        limit: u32,
    ) -> Result<Vec<ChangeRecord>, QuotationDraftError> {
        self.require(actor, QUOTATION_DRAFT_WRITE_PERMISSION)?;
        self.store
            .quotation_draft_sync_batch(&actor.scope, limit)
            .map_err(Into::into)
    }
    /// Projects an authenticated server change before its checkpoint is stored.
    /// # Errors
    /// Rejects malformed records or revoked read access.
    pub fn project_confirmed(
        &self,
        actor: &AuthorizationContext,
        change: &ChangeRecord,
        correlation_id: CorrelationId,
    ) -> Result<bool, QuotationDraftError> {
        self.require(actor, QUOTATION_DRAFT_READ_PERMISSION)?;
        if change.scope != actor.scope || change.operation != ChangeOperation::Upsert {
            return Err(QuotationDraftError::Denied);
        }
        let payload = change
            .payload
            .as_ref()
            .ok_or(QuotationDraftError::Unavailable)?;
        if payload.schema_id.as_str() != QUOTATION_DRAFT_SCHEMA || payload.schema_version != 1 {
            return Err(QuotationDraftError::Unavailable);
        }
        let snapshot: QuotationDraftSnapshot = serde_json::from_slice(
            &STANDARD
                .decode(&payload.base64)
                .map_err(|_| QuotationDraftError::Unavailable)?,
        )
        .map_err(|_| QuotationDraftError::Unavailable)?;
        if snapshot.id.value() != change.record_id.value()
            || snapshot.revision != change.revision
            || snapshot.evaluation.scope != actor.scope
        {
            return Err(QuotationDraftError::Unavailable);
        }
        let draft = QuotationDraft {
            scope: actor.scope.clone(),
            snapshot,
            updated_at: change.changed_at,
            sync_state: QuotationDraftSyncState::Confirmed,
            conflict: None,
        };
        let context = MutationContext {
            authorization: actor.clone(),
            correlation_id,
            causation_id: None,
            idempotency_key: change.idempotency_key,
            occurred_at: change.changed_at,
        };
        self.store
            .project_quotation_draft(
                &draft,
                change,
                &audit(
                    &context,
                    "eitmad.quotation-draft.sync-project.v1",
                    Some(draft.snapshot.id),
                )
                .with_outcome(AuditOutcome::Succeeded, None),
                &publication(&draft, change.change_id),
            )
            .map_err(Into::into)
    }
    /// Keeps rejected/conflicted content and records the server conflict identity.
    /// # Errors
    /// Denies unauthorized state changes and unavailable atomic storage.
    pub fn mark_sync_exception(
        &self,
        actor: &AuthorizationContext,
        id: QuotationDraftId,
        state: QuotationDraftSyncState,
        conflict: Option<ConflictId>,
        correlation_id: CorrelationId,
        occurred_at: UnixMillis,
    ) -> Result<(), QuotationDraftError> {
        let draft = self.get(actor, &GetQuotationDraft { draft_id: id })?;
        let change_id = ChangeId::new(Uuid::new_v4());
        let context = MutationContext {
            authorization: actor.clone(),
            correlation_id,
            causation_id: None,
            idempotency_key: eitmad_contracts::transport::IdempotencyKey::new(Uuid::new_v4()),
            occurred_at,
        };
        self.store
            .mark_quotation_draft_exception(
                &actor.scope,
                id,
                state,
                conflict,
                &audit(&context, "eitmad.quotation-draft.sync-state.v1", Some(id)),
                &publication(&draft, change_id),
            )
            .map_err(Into::into)
    }
    fn require(
        &self,
        actor: &AuthorizationContext,
        permission: &str,
    ) -> Result<(), QuotationDraftError> {
        if actor.scope.kind.as_str() != "branch" {
            return Err(QuotationDraftError::Denied);
        }
        self.authorization
            .authorize(actor, permission)
            .map_err(|e| {
                if matches!(
                    e,
                    AuthorizationError::Denied | AuthorizationError::UnsupportedScope
                ) {
                    QuotationDraftError::Denied
                } else {
                    QuotationDraftError::Unavailable
                }
            })
    }
}
fn audit(
    context: &MutationContext,
    operation: &str,
    id: Option<QuotationDraftId>,
) -> MutationAuditRecord {
    let mut a = MutationAuditRecord::from_authorization(
        &context.authorization,
        context.occurred_at,
        context.correlation_id,
        operation,
        AuditTarget {
            kind: "quotation-draft".into(),
            identifiers: id.map(|v| vec![v.value().to_string()]).unwrap_or_default(),
        },
    );
    a.idempotency_key = Some(context.idempotency_key);
    a.causation_id = context.causation_id;
    a.changed_identifiers = vec!["snapshot".into()];
    a
}
fn publication(draft: &QuotationDraft, change_id: ChangeId) -> DurablePublication {
    DurablePublication {
        event: Event::QuotationDraftChanged(QuotationDraftChangeNotice {
            draft_id: draft.snapshot.id,
            scope: draft.scope.clone(),
            revision: draft.snapshot.revision,
            changed_at: draft.updated_at,
            change_id,
        }),
        policy_changed: false,
    }
}
