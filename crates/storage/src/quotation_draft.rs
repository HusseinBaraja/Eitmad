//! Atomic quotation draft snapshots, replay results, and local-first publication work.
use crate::{
    AuthorityStore, DurableIdempotency, DurablePublication, PricingTransaction, StorageError,
    insert_audit, insert_idempotency, insert_publication, load_idempotency, migrations::Migration,
    scope_parts,
};
use eitmad_contracts::{
    identity::ScopeRef,
    quotation_draft::{
        QuotationDraft, QuotationDraftConflict, QuotationDraftId, QuotationDraftPage,
        QuotationDraftSyncState,
    },
    sync::ChangeRecord,
    transport::IdempotencyKey,
};
use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(24, "quotation.drafts.v1", "quotation",
    "CREATE TABLE quotation_drafts (
       scope_kind TEXT NOT NULL CHECK(scope_kind='branch'), scope_id TEXT NOT NULL,
       draft_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
       PRIMARY KEY(scope_kind,scope_id,draft_id));
     CREATE TABLE quotation_draft_outbox (
       change_id TEXT PRIMARY KEY, scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL,
       draft_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), change_json BLOB NOT NULL,
       FOREIGN KEY(scope_kind,scope_id,draft_id) REFERENCES quotation_drafts(scope_kind,scope_id,draft_id));
     CREATE INDEX quotation_draft_outbox_scope ON quotation_draft_outbox(scope_kind,scope_id,draft_id,revision);")];

pub struct QuotationDraftCommit<'a> {
    pub draft: &'a QuotationDraft,
    pub expected_revision: Option<u64>,
    pub operation: &'a str,
    pub idempotency: &'a DurableIdempotency,
    pub audit: &'a MutationAuditRecord,
    pub publication: &'a DurablePublication,
    pub change: &'a ChangeRecord,
}

fn get_on(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    id: QuotationDraftId,
) -> Result<Option<QuotationDraft>, StorageError> {
    let (kind, scope_id) = scope_parts(scope);
    let bytes: Option<Vec<u8>> = connection.query_row("SELECT record_json FROM quotation_drafts WHERE scope_kind=?1 AND scope_id=?2 AND draft_id=?3", params![kind, scope_id, id.value().to_string()], |row| row.get(0)).optional().map_err(|_| StorageError)?;
    bytes
        .map(|b| serde_json::from_slice(&b).map_err(|_| StorageError))
        .transpose()
}
fn put_on(connection: &rusqlite::Connection, draft: &QuotationDraft) -> Result<(), StorageError> {
    let (kind, scope_id) = scope_parts(&draft.scope);
    connection.execute("INSERT INTO quotation_drafts(scope_kind,scope_id,draft_id,revision,record_json) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(scope_kind,scope_id,draft_id) DO UPDATE SET revision=excluded.revision,record_json=excluded.record_json", params![kind,scope_id,draft.snapshot.id.value().to_string(),i64::try_from(draft.snapshot.revision).map_err(|_| StorageError)?,serde_json::to_vec(draft).map_err(|_| StorageError)?]).map_err(|_| StorageError)?;
    Ok(())
}
impl PricingTransaction<'_> {
    /// Loads exact retry state inside the draft write transaction.
    /// # Errors
    /// Fails closed on unavailable durable replay state.
    pub fn quotation_draft_replay(
        &self,
        scope: &ScopeRef,
        key: IdempotencyKey,
    ) -> Result<Option<crate::StoredIdempotency>, StorageError> {
        load_idempotency(self.connection, scope, key)
    }
    /// Reads one scoped draft within the evaluator snapshot.
    /// # Errors
    /// Rejects unavailable or malformed data.
    pub fn quotation_draft(
        &self,
        scope: &ScopeRef,
        id: QuotationDraftId,
    ) -> Result<Option<QuotationDraft>, StorageError> {
        get_on(self.connection, scope, id)
    }
    /// Commits evaluated content and all required side effects in this transaction.
    /// # Errors
    /// Rolls back the caller transaction if any required write fails.
    pub fn commit_quotation_draft(
        &self,
        commit: &QuotationDraftCommit<'_>,
    ) -> Result<(), StorageError> {
        let draft = commit.draft;
        let current = get_on(self.connection, &draft.scope, draft.snapshot.id)?;
        if current.as_ref().map(|v| v.snapshot.revision) != commit.expected_revision {
            return Err(StorageError);
        }
        let (kind, scope_id) = scope_parts(&draft.scope);
        let count: i64 = self
            .connection
            .query_row(
                "SELECT count(*) FROM quotation_draft_outbox WHERE scope_kind=?1 AND scope_id=?2",
                params![kind, scope_id],
                |r| r.get(0),
            )
            .map_err(|_| StorageError)?;
        if count >= 2048 {
            return Err(StorageError);
        }
        put_on(self.connection, draft)?;
        self.connection.execute("INSERT INTO quotation_draft_outbox(change_id,scope_kind,scope_id,draft_id,revision,change_json) VALUES(?1,?2,?3,?4,?5,?6)", params![commit.change.change_id.value().to_string(),kind,scope_id,draft.snapshot.id.value().to_string(),i64::try_from(draft.snapshot.revision).map_err(|_| StorageError)?,serde_json::to_vec(commit.change).map_err(|_| StorageError)?]).map_err(|_| StorageError)?;
        insert_audit(self.connection, commit.audit)?;
        insert_idempotency(
            self.connection,
            &draft.scope,
            commit.operation,
            commit.idempotency,
        )?;
        insert_publication(
            self.connection,
            &draft.scope,
            commit.idempotency.key,
            commit.publication,
        )
    }
}
impl AuthorityStore {
    /// Reads a scoped draft without evaluating or repricing its historical lines.
    /// # Errors
    /// Rejects unavailable durable state.
    pub fn get_quotation_draft(
        &self,
        scope: &ScopeRef,
        id: QuotationDraftId,
    ) -> Result<Option<QuotationDraft>, StorageError> {
        self.read_transaction(|tx| get_on(tx, scope, id))
    }
    /// Lists an exact branch using a bounded stable UUID cursor.
    /// # Errors
    /// Rejects invalid limits or malformed records.
    pub fn list_quotation_drafts(
        &self,
        scope: &ScopeRef,
        after: Option<QuotationDraftId>,
        limit: u32,
    ) -> Result<QuotationDraftPage, StorageError> {
        if !(1..=100).contains(&limit) {
            return Err(StorageError);
        }
        self.read_transaction(|tx| {
            let (kind,scope_id) = scope_parts(scope);
            let mut query = tx.prepare("SELECT record_json FROM quotation_drafts WHERE scope_kind=?1 AND scope_id=?2 AND (?3 IS NULL OR draft_id>?3) ORDER BY draft_id LIMIT ?4").map_err(|_| StorageError)?;
            let mut items: Vec<QuotationDraft> = query.query_map(params![kind,scope_id,after.map(|v| v.value().to_string()),limit+1], |r| r.get::<_,Vec<u8>>(0)).map_err(|_| StorageError)?.map(|r| serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError)).collect::<Result<_,_>>()?;
            let more = items.len() > usize::try_from(limit).map_err(|_| StorageError)?;
            if more { items.pop(); }
            let next = if more { items.last().map(|v| v.snapshot.id) } else { None };
            Ok(QuotationDraftPage { items,next })
        })
    }
    /// Retains queued work until exact authoritative acknowledgement.
    /// # Errors
    /// Rejects unavailable storage or invalid limits.
    pub fn quotation_draft_sync_batch(
        &self,
        scope: &ScopeRef,
        limit: u32,
    ) -> Result<Vec<ChangeRecord>, StorageError> {
        if !(1..=50).contains(&limit) {
            return Err(StorageError);
        }
        self.read_transaction(|tx| {
            let (kind,id)=scope_parts(scope);
            let mut q=tx.prepare("SELECT o.change_json FROM quotation_draft_outbox o JOIN quotation_drafts d USING(scope_kind,scope_id,draft_id) WHERE o.scope_kind=?1 AND o.scope_id=?2 AND json_extract(CAST(d.record_json AS TEXT),'$.syncState')='pending' ORDER BY o.rowid LIMIT ?3").map_err(|_| StorageError)?;
            q.query_map(params![kind,id,limit], |r| r.get::<_,Vec<u8>>(0)).map_err(|_| StorageError)?.map(|r| serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError)).collect()
        })
    }
    /// Projects before checkpoint advancement; preserves any conflicting local content.
    /// # Errors
    /// Rolls back projection, audit, and publication together.
    pub fn project_quotation_draft(
        &self,
        incoming: &QuotationDraft,
        change: &ChangeRecord,
        audit: &MutationAuditRecord,
        publication: &DurablePublication,
    ) -> Result<bool, StorageError> {
        self.write_transaction(|tx| {
            let (kind,id)=scope_parts(&incoming.scope);
            let draft_id=incoming.snapshot.id.value().to_string();
            let removed=tx.execute("DELETE FROM quotation_draft_outbox WHERE change_id=?1 AND scope_kind=?2 AND scope_id=?3 AND draft_id=?4", params![change.change_id.value().to_string(),kind,id,draft_id]).map_err(|_| StorageError)?;
            let pending: Option<Vec<u8>>=tx.query_row("SELECT change_json FROM quotation_draft_outbox WHERE scope_kind=?1 AND scope_id=?2 AND draft_id=?3 ORDER BY revision LIMIT 1", params![kind,id,draft_id], |r| r.get(0)).optional().map_err(|_| StorageError)?;
            let current=get_on(tx,&incoming.scope,incoming.snapshot.id)?;
            let mut target=incoming.clone();
            if let Some(mut local)=current {
                if let Some(bytes)=pending {
                    let queued: ChangeRecord=serde_json::from_slice(&bytes).map_err(|_| StorageError)?;
                    if removed == 0 && change.revision > queued.base_revision.unwrap_or(0) {
                        local.sync_state=QuotationDraftSyncState::Conflicted;
                        let conflict=local.conflict.get_or_insert(QuotationDraftConflict { server_conflict_id:None,remote:None });
                        if conflict.remote.as_ref().is_some_and(|r| r.revision >= change.revision) { return Ok(false); }
                        conflict.remote=Some(change.clone());
                    } else if removed == 0 { return Ok(false); }
                    target=local;
                } else if local.snapshot.revision > incoming.snapshot.revision || (removed == 0 && local == *incoming) { return Ok(false); }
            }
            put_on(tx,&target)?;
            insert_audit(tx,audit)?;
            insert_publication(tx,&incoming.scope,IdempotencyKey::new(uuid::Uuid::new_v4()),publication)?;
            Ok(true)
        })
    }
    /// Saves a terminal submission result without deleting its local input.
    /// # Errors
    /// Rejects a missing record or failed atomic audit.
    pub fn mark_quotation_draft_exception(
        &self,
        scope: &ScopeRef,
        id: QuotationDraftId,
        state: QuotationDraftSyncState,
        conflict_id: Option<eitmad_contracts::sync::ConflictId>,
        audit: &MutationAuditRecord,
        publication: &DurablePublication,
    ) -> Result<(), StorageError> {
        self.write_transaction(|tx| {
            let mut draft = get_on(tx, scope, id)?.ok_or(StorageError)?;
            if draft.sync_state == state
                && draft.conflict.as_ref().and_then(|c| c.server_conflict_id) == conflict_id
            {
                return Ok(());
            }
            draft.sync_state = state;
            if state == QuotationDraftSyncState::Conflicted {
                draft
                    .conflict
                    .get_or_insert(QuotationDraftConflict {
                        server_conflict_id: None,
                        remote: None,
                    })
                    .server_conflict_id = conflict_id;
            }
            put_on(tx, &draft)?;
            insert_audit(
                tx,
                &audit.clone().with_outcome(AuditOutcome::Succeeded, None),
            )?;
            insert_publication(
                tx,
                scope,
                IdempotencyKey::new(uuid::Uuid::new_v4()),
                publication,
            )
        })
    }
}
