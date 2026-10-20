//! Durable uncertain decisions and held revisions for explicit Manager review.
use crate::{AuthorityStore, StorageError, insert_audit, migrations::Migration, scope_parts};
use eitmad_contracts::{
    identity::{AuthorizationContext, ScopeRef},
    quotation_draft::QuotationDraftId,
    sync::ChangeRecord,
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};
pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(30, "quotation.draft-resolution.v1", "quotation",
    "CREATE TABLE quotation_draft_pending_reviews (
        change_id TEXT PRIMARY KEY, scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL,
        draft_id TEXT NOT NULL, change_json BLOB NOT NULL, held_by TEXT NOT NULL, reviewed_by TEXT);
     CREATE INDEX quotation_draft_review_scope ON quotation_draft_pending_reviews(scope_kind,scope_id,draft_id);
     CREATE TABLE quotation_draft_resolution_intents (
        scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, quotation_draft_id TEXT NOT NULL,
        branch_id TEXT NOT NULL, principal_id TEXT NOT NULL, request_json BLOB NOT NULL,
        PRIMARY KEY(scope_kind,scope_id,branch_id,quotation_draft_id));
     CREATE TABLE quotation_draft_resolution_projections (
        change_id TEXT NOT NULL, scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL,
        PRIMARY KEY(change_id,scope_kind,scope_id));")];

pub(crate) const RECOVERY_MIGRATIONS: &[Migration] = &[Migration::new(
    31,
    "resolution.reconciliation.v1",
    "sync",
    "CREATE TABLE resolution_reconciliations (
        audit_id TEXT PRIMARY KEY, schema_id TEXT NOT NULL,
        scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, branch_id TEXT NOT NULL,
        record_id TEXT NOT NULL, principal_id TEXT NOT NULL, request_json BLOB NOT NULL,
        receipt_json BLOB, reconciled_by TEXT NOT NULL);",
)];

/// Holds all work based on superseded state; later revisions never automatically rebase.
/// Runs inside projection so archive, projection, audit and checkpoint recovery remain atomic.
pub(crate) fn hold_resolution_inputs(
    tx: &rusqlite::Connection,
    change: &ChangeRecord,
) -> Result<bool, StorageError> {
    let Some(merge) = &change.merge else {
        return Ok(false);
    };
    let source = merge
        .source_changes
        .first()
        .ok_or(StorageError)?
        .value()
        .to_string();
    let (kind, scope) = scope_parts(&change.scope);
    let draft = change.record_id.value().to_string();
    let resolution = change.change_id.value().to_string();
    let revision = i64::try_from(change.revision).map_err(|_| StorageError)?;
    let first = tx
        .execute(
            "INSERT OR IGNORE INTO quotation_draft_resolution_projections
        (change_id,scope_kind,scope_id) VALUES (?1,?2,?3)",
            params![resolution, kind, scope],
        )
        .map_err(|_| StorageError)?;
    if first == 0 {
        return Ok(false);
    }
    let held = tx
        .execute(
            "INSERT OR IGNORE INTO quotation_draft_pending_reviews
        (change_id,scope_kind,scope_id,draft_id,change_json,held_by,reviewed_by)
        SELECT change_id,scope_kind,scope_id,draft_id,change_json,?4,
            CASE WHEN change_id=?5 THEN ?4 ELSE NULL END FROM quotation_draft_outbox
        WHERE scope_kind=?1 AND scope_id=?2 AND draft_id=?3
          AND EXISTS (SELECT 1 FROM quotation_draft_outbox first
            WHERE first.scope_kind=?1 AND first.scope_id=?2 AND first.draft_id=?3
              AND COALESCE(json_extract(CAST(first.change_json AS TEXT),'$.baseRevision'),0) < ?6)",
            params![kind, scope, draft, resolution, source, revision],
        )
        .map_err(|_| StorageError)?;
    tx.execute(
        "DELETE FROM quotation_draft_outbox WHERE scope_kind=?1 AND scope_id=?2 AND draft_id=?3
        AND change_id IN (SELECT change_id FROM quotation_draft_pending_reviews)",
        params![kind, scope, draft],
    )
    .map_err(|_| StorageError)?;
    let reviewed = tx.execute("UPDATE quotation_draft_pending_reviews SET reviewed_by=?1
        WHERE change_id=?2 AND scope_kind=?3 AND scope_id=?4 AND draft_id=?5 AND reviewed_by IS NULL",
        params![resolution,source,kind,scope,draft]).map_err(|_| StorageError)?;
    Ok(held > 0 || reviewed > 0)
}
impl AuthorityStore {
    /// Freezes uncertain intent before network I/O. A changed request cannot replace it.
    /// # Errors
    /// Returns an error for a different outstanding request or an audit/storage failure.
    pub fn stage_quotation_draft_resolution(
        &self,
        actor: &AuthorizationContext,
        input: &eitmad_contracts::quotation_draft::ConfirmQuotationDraftResolution,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        self.write_transaction(|tx| {
            let (kind,id) = scope_parts(&actor.scope);
            let bytes = serde_json::to_vec(input).map_err(|_| StorageError)?;
            let branch = input.command.branch_id.value().to_string();
            let prior: Option<(Vec<u8>,String)> = tx.query_row("SELECT request_json,principal_id FROM quotation_draft_resolution_intents
                WHERE scope_kind=?1 AND scope_id=?2 AND quotation_draft_id=?3 AND branch_id=?4", params![kind,id,input.command.draft_id.value().to_string(),branch], |r| Ok((r.get(0)?,r.get(1)?)))
                .optional().map_err(|_| StorageError)?;
            if let Some((prior,principal)) = prior { return if prior == bytes && principal == actor.identity.principal_id.value().to_string() { Ok(()) } else { Err(StorageError) }; }
            tx.execute("INSERT INTO quotation_draft_resolution_intents(scope_kind,scope_id,quotation_draft_id,principal_id,request_json,branch_id)
                VALUES (?1,?2,?3,?4,?5,?6)", params![kind,id,input.command.draft_id.value().to_string(),actor.identity.principal_id.value().to_string(),bytes,branch]).map_err(|_| StorageError)?;
            insert_audit(tx,audit)
        })
    }

    /// Reads the record's original author and frozen request for authorized reconciliation.
    /// # Errors
    /// The capability must authorize current Manager access before reading this evidence.
    pub fn pending_quotation_draft_resolution(
        &self,
        actor: &AuthorizationContext,
        record: QuotationDraftId,
        branch: eitmad_contracts::identity::ScopeId,
    ) -> Result<
        Option<
            eitmad_contracts::sync::ReconcileResolution<
                eitmad_contracts::quotation_draft::ConfirmQuotationDraftResolution,
            >,
        >,
        StorageError,
    > {
        self.read_transaction(|tx| {
            let (kind, id) = scope_parts(&actor.scope);
            let row: Option<(String, Vec<u8>)> = tx
                .query_row(
                    "SELECT principal_id,request_json FROM quotation_draft_resolution_intents
                WHERE scope_kind=?1 AND scope_id=?2 AND quotation_draft_id=?3 AND branch_id=?4",
                    params![
                        kind,
                        id,
                        record.value().to_string(),
                        branch.value().to_string()
                    ],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|_| StorageError)?;
            row.map(|(principal, bytes)| {
                Ok(eitmad_contracts::sync::ReconcileResolution {
                    original_user: eitmad_contracts::identity::UserId::new(
                        principal.parse().map_err(|_| StorageError)?,
                    ),
                    request: serde_json::from_slice(&bytes).map_err(|_| StorageError)?,
                })
            })
            .transpose()
        })
    }

    /// Archives exact original evidence and the reconciled receipt before unlocking the record.
    /// # Errors
    /// Requires a server receipt or permanent key closure and current Manager authorization.
    pub fn reconcile_quotation_draft_resolution(
        &self,
        actor: &AuthorizationContext,
        recovery: &eitmad_contracts::sync::ReconcileResolution<
            eitmad_contracts::quotation_draft::ConfirmQuotationDraftResolution,
        >,
        receipt: Option<&ChangeRecord>,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        self.write_transaction(|tx| {
            let (kind,id) = scope_parts(&actor.scope);
            let bytes = serde_json::to_vec(&recovery.request).map_err(|_| StorageError)?;
            let original = recovery.original_user.value().to_string();
            let archived = tx.execute("INSERT INTO resolution_reconciliations
                (audit_id,schema_id,scope_kind,scope_id,branch_id,record_id,principal_id,request_json,receipt_json,reconciled_by)
                SELECT ?1,'eitmad.schema.quotation-draft.v1',scope_kind,scope_id,branch_id,quotation_draft_id,principal_id,request_json,?2,?3
                FROM quotation_draft_resolution_intents WHERE scope_kind=?4 AND scope_id=?5 AND principal_id=?6 AND request_json=?7",
                params![audit.audit_id.to_string(),receipt.map(serde_json::to_vec).transpose().map_err(|_| StorageError)?,
                    actor.identity.principal_id.value().to_string(),kind,id,original,bytes]).map_err(|_| StorageError)?;
            if archived > 0 {
                tx.execute("DELETE FROM quotation_draft_resolution_intents WHERE scope_kind=?1 AND scope_id=?2 AND principal_id=?3 AND request_json=?4",
                    params![kind,id,original,bytes]).map_err(|_| StorageError)?;
                insert_audit(tx,audit)?;
            }
            Ok(())
        })
    }

    /// # Errors
    /// Returns an error if the current actor's durable intent cannot be decoded.
    pub fn quotation_draft_resolution_intent(
        &self,
        actor: &AuthorizationContext,
        quotation_draft: QuotationDraftId,
        branch: eitmad_contracts::identity::ScopeId,
    ) -> Result<
        Option<eitmad_contracts::quotation_draft::ConfirmQuotationDraftResolution>,
        StorageError,
    > {
        self.read_transaction(|tx| {
            let (kind,id) = scope_parts(&actor.scope);
            let bytes: Option<Vec<u8>> = tx.query_row("SELECT request_json FROM quotation_draft_resolution_intents
                WHERE scope_kind=?1 AND scope_id=?2 AND quotation_draft_id=?3 AND principal_id=?4 AND branch_id=?5",
                params![kind,id,quotation_draft.value().to_string(),actor.identity.principal_id.value().to_string(),branch.value().to_string()], |r| r.get(0)).optional().map_err(|_| StorageError)?;
            bytes.map(|v| serde_json::from_slice(&v).map_err(|_| StorageError)).transpose()
        })
    }

    /// # Errors
    /// Removes only this actor's completed or definitively failed intent, with audit.
    pub fn finish_quotation_draft_resolution(
        &self,
        actor: &AuthorizationContext,
        quotation_draft: QuotationDraftId,
        branch: eitmad_contracts::identity::ScopeId,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        self.write_transaction(|tx| {
            let (kind,id) = scope_parts(&actor.scope);
            let removed = tx.execute("DELETE FROM quotation_draft_resolution_intents WHERE scope_kind=?1 AND scope_id=?2 AND quotation_draft_id=?3 AND principal_id=?4 AND branch_id=?5",
                params![kind,id,quotation_draft.value().to_string(),actor.identity.principal_id.value().to_string(),branch.value().to_string()]).map_err(|_| StorageError)?;
            if removed > 0 { insert_audit(tx,audit)?; }
            Ok(())
        })
    }

    /// Returns the next bounded set of unreviewed immutable revisions.
    /// # Errors
    /// Rejects unavailable or malformed storage.
    pub fn quotation_draft_pending_reviews(
        &self,
        scope: &ScopeRef,
        draft: QuotationDraftId,
    ) -> Result<Vec<ChangeRecord>, StorageError> {
        self.read_transaction(|tx| {
            let (kind,id)=scope_parts(scope);
            let mut q=tx.prepare("SELECT change_json FROM (
                SELECT change_json, rowid AS position FROM quotation_draft_pending_reviews
                  WHERE scope_kind=?1 AND scope_id=?2 AND draft_id=?3 AND reviewed_by IS NULL
                UNION ALL
                SELECT o.change_json,o.rowid AS position FROM quotation_draft_outbox o
                  JOIN quotation_drafts d USING(scope_kind,scope_id,draft_id)
                  WHERE o.scope_kind=?1 AND o.scope_id=?2 AND o.draft_id=?3
                    AND json_extract(CAST(d.record_json AS TEXT),'$.syncState') IN ('conflicted','rejected')
                ) ORDER BY position LIMIT 50").map_err(|_| StorageError)?;
            q.query_map(params![kind,id,draft.value().to_string()], |r| r.get::<_,Vec<u8>>(0)).map_err(|_| StorageError)?
                .map(|r| serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError)).collect()
        })
    }
    /// Checks immutable review evidence, including completed inputs needed for exact retries.
    /// # Errors
    /// Rejects unavailable or malformed storage.
    pub fn has_quotation_draft_review_input(
        &self,
        scope: &ScopeRef,
        change: &ChangeRecord,
    ) -> Result<bool, StorageError> {
        self.read_transaction(|tx| {
            let (kind,id)=scope_parts(scope);
            let bytes:Option<Vec<u8>>=tx.query_row("SELECT change_json FROM quotation_draft_pending_reviews
                WHERE scope_kind=?1 AND scope_id=?2 AND draft_id=?3 AND change_id=?4
                UNION ALL SELECT o.change_json FROM quotation_draft_outbox o
                  JOIN quotation_drafts d USING(scope_kind,scope_id,draft_id)
                  WHERE o.scope_kind=?1 AND o.scope_id=?2 AND o.draft_id=?3 AND o.change_id=?4
                    AND json_extract(CAST(d.record_json AS TEXT),'$.syncState') IN ('conflicted','rejected') LIMIT 1",
                params![kind,id,change.record_id.value().to_string(),change.change_id.value().to_string()],|r| r.get(0))
                .optional().map_err(|_| StorageError)?;
            bytes.map(|v| serde_json::from_slice::<ChangeRecord>(&v).map(|v|v==*change).map_err(|_|StorageError)).transpose().map(|v|v.unwrap_or(false))
        })
    }
}
