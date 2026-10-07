//! Scoped durable confirmed quotation history; cached permissions are never authority.
use crate::{
    AuthorityStore, PricingTransaction, StorageError, insert_audit, migrations::Migration,
    scope_parts,
};
use eitmad_contracts::{
    identity::ScopeRef,
    quotation_draft::QuotationDraftId,
    quotation_lifecycle::{QuotationPage, QuotationRecord},
    transport::UnixMillis,
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(
    26,
    "quotation.lifecycle-cache.v1",
    "quotation",
    "CREATE TABLE quotation_confirmed_history (
     scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, draft_id TEXT NOT NULL,
     revision INTEGER NOT NULL, record_json BLOB NOT NULL,
     PRIMARY KEY(scope_kind,scope_id,draft_id,revision));",
)];

impl PricingTransaction<'_> {
    /// # Errors
    /// Rejects unavailable confirmed state.
    pub fn confirmed_quotation(
        &self,
        scope: &ScopeRef,
        id: QuotationDraftId,
    ) -> Result<Option<QuotationRecord>, StorageError> {
        get(self.connection, scope, id)
    }
    /// # Errors
    /// Commits the confirmed projection with its redacted audit, or rolls back both.
    pub fn cache_quotation(
        &self,
        scope: &ScopeRef,
        record: &QuotationRecord,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        let (kind, id) = scope_parts(scope);
        let mut retained = record.clone();
        retained.permitted_actions.clear();
        self.connection
            .execute(
                "INSERT OR IGNORE INTO quotation_confirmed_history VALUES(?1,?2,?3,?4,?5)",
                params![
                    kind,
                    id,
                    record.quotation.id.value().to_string(),
                    i64::try_from(record.revision).map_err(|_| StorageError)?,
                    serde_json::to_vec(&retained).map_err(|_| StorageError)?
                ],
            )
            .map_err(|_| StorageError)?;
        insert_audit(self.connection, audit)
    }
}
fn get(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    id: QuotationDraftId,
) -> Result<Option<QuotationRecord>, StorageError> {
    let (kind, scope_id) = scope_parts(scope);
    let bytes: Option<Vec<u8>> = connection.query_row("SELECT record_json FROM quotation_confirmed_history WHERE scope_kind=?1 AND scope_id=?2 AND draft_id=?3 ORDER BY revision DESC LIMIT 1", params![kind,scope_id,id.value().to_string()], |row| row.get(0)).optional().map_err(|_| StorageError)?;
    bytes
        .map(|r| serde_json::from_slice(&r).map_err(|_| StorageError))
        .transpose()
}
impl AuthorityStore {
    /// # Errors
    /// Reads only confirmed records previously obtained under this exact scope.
    pub fn cached_quotations(
        &self,
        scope: &ScopeRef,
        after: Option<QuotationDraftId>,
        limit: u32,
        _now: UnixMillis,
    ) -> Result<QuotationPage, StorageError> {
        if !(1..=100).contains(&limit) {
            return Err(StorageError);
        }
        self.read_transaction(|tx| {
            let (kind, scope_id) = scope_parts(scope);
            let mut statement = tx.prepare("SELECT DISTINCT draft_id FROM quotation_confirmed_history WHERE scope_kind=?1 AND scope_id=?2 AND (?3 IS NULL OR draft_id>?3) ORDER BY draft_id LIMIT ?4").map_err(|_| StorageError)?;
            let ids = statement.query_map(params![kind,scope_id,after.map(|v| v.value().to_string()),i64::from(limit)+1], |r| r.get::<_, String>(0)).map_err(|_| StorageError)?.collect::<Result<Vec<_>,_>>().map_err(|_| StorageError)?;
            let more = ids.len() > limit as usize;
            let mut items = Vec::new();
            for id in ids.into_iter().take(limit as usize) {
                let id = QuotationDraftId::new(uuid::Uuid::parse_str(&id).map_err(|_| StorageError)?);
                items.push(get(tx,scope,id)?.ok_or(StorageError)?);
            }
            let next = if more { items.last().map(|v| v.quotation.id) } else { None };
            Ok(QuotationPage { items, next, server_available: false })
        })
    }
}
