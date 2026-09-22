//! Durable customer contact state and bounded local-first sync work.

use eitmad_contracts::{
    customer::{
        Customer, CustomerAddress, CustomerId, CustomerMutationResult, CustomerName, CustomerNotes,
        CustomerPage, CustomerPhone, CustomerStatus, CustomerSyncState,
    },
    identity::ScopeRef,
    sync::{ChangeId, ChangeRecord},
    transport::UnixMillis,
};
use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};

use crate::{
    AuthorityStore, DurableIdempotency, DurablePublication, StorageError, insert_audit,
    insert_idempotency, insert_publication, load_idempotency, migrations::Migration, scope_parts,
};

pub const MAX_CUSTOMER_SYNC_BATCH: u32 = 50;
const MAX_DUPLICATE_ADVISORIES: u32 = 10;

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(
    12,
    "customer.initial.v1",
    "customer",
    "CREATE TABLE customers (
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         customer_id TEXT NOT NULL,
         name TEXT NOT NULL,
         normalized_name TEXT NOT NULL,
         phone TEXT NOT NULL,
         normalized_phone TEXT NOT NULL,
         address TEXT,
         notes TEXT,
         status TEXT NOT NULL CHECK (status IN ('active', 'archived', 'merged')),
         revision INTEGER NOT NULL CHECK (revision > 0),
         updated_at INTEGER NOT NULL,
         sync_state TEXT NOT NULL CHECK (sync_state IN ('pending', 'confirmed')),
         PRIMARY KEY (scope_kind, scope_id, customer_id)
     );
     CREATE INDEX customers_search_name
         ON customers(scope_kind, scope_id, normalized_name, customer_id);
     CREATE INDEX customers_search_phone
         ON customers(scope_kind, scope_id, normalized_phone, customer_id);
     CREATE TABLE customer_sync_outbox (
         change_id TEXT PRIMARY KEY,
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         customer_id TEXT NOT NULL,
         change_json BLOB NOT NULL,
         FOREIGN KEY (scope_kind, scope_id, customer_id)
             REFERENCES customers(scope_kind, scope_id, customer_id) ON DELETE CASCADE
     );",
)];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomerCommitOutcome {
    Committed(CustomerMutationResult),
    Replayed { response_json: Vec<u8> },
    RevisionConflict { actual_revision: Option<u64> },
    IdempotencyMismatch,
}

pub struct CustomerCommit<'a> {
    pub customer: &'a Customer,
    pub normalized_name: &'a str,
    pub normalized_phone: &'a str,
    pub expected_revision: Option<u64>,
    pub operation: &'a str,
    pub idempotency: &'a DurableIdempotency,
    pub audit: &'a MutationAuditRecord,
    pub publication: &'a DurablePublication,
    pub change: &'a ChangeRecord,
    pub conflict_error_code: &'a str,
}

impl AuthorityStore {
    /// Gets one customer from an exact scope.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error for malformed or unavailable storage.
    pub fn get_customer(
        &self,
        scope: &ScopeRef,
        customer_id: CustomerId,
    ) -> Result<Option<Customer>, StorageError> {
        self.read_transaction(|connection| {
            let (scope_kind, scope_id) = scope_parts(scope);
            connection
                .query_row(
                    "SELECT customer_id, name, phone, address, notes, status, revision,
                            updated_at, sync_state
                     FROM customers
                     WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3",
                    params![scope_kind, scope_id, customer_id.value().to_string()],
                    customer_row,
                )
                .optional()
                .map_err(|_| StorageError)?
                .map(|row| decode_customer(scope, row))
                .transpose()
        })
    }

    /// Searches one exact scope using bounded normalized name and phone terms.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error for invalid bounds or unavailable storage.
    pub fn search_customers(
        &self,
        scope: &ScopeRef,
        normalized_name: &str,
        normalized_phone: Option<&str>,
        after: Option<CustomerId>,
        limit: u32,
    ) -> Result<CustomerPage, StorageError> {
        self.read_transaction(|connection| {
            let (scope_kind, scope_id) = scope_parts(scope);
            let after = after.map(|id| id.value().to_string()).unwrap_or_default();
            let fetch = i64::from(limit.checked_add(1).ok_or(StorageError)?);
            let name_pattern = contains_pattern(normalized_name);
            let phone_pattern = normalized_phone.map(contains_pattern);
            let mut statement = connection
                .prepare(
                    "SELECT customer_id, name, phone, address, notes, status, revision,
                            updated_at, sync_state
                     FROM customers
                     WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id > ?3
                       AND (?4 = '' OR normalized_name LIKE ?5 ESCAPE '\\'
                            OR (?6 IS NOT NULL AND normalized_phone LIKE ?6 ESCAPE '\\'))
                     ORDER BY customer_id LIMIT ?7",
                )
                .map_err(|_| StorageError)?;
            let rows = statement
                .query_map(
                    params![
                        scope_kind,
                        scope_id,
                        after,
                        normalized_name,
                        name_pattern,
                        phone_pattern,
                        fetch
                    ],
                    customer_row,
                )
                .map_err(|_| StorageError)?;
            let mut items = rows
                .map(|row| decode_customer(scope, row.map_err(|_| StorageError)?))
                .collect::<Result<Vec<_>, _>>()?;
            let has_more = items.len() > usize::try_from(limit).map_err(|_| StorageError)?;
            if has_more {
                items.pop();
            }
            let next = has_more.then(|| items.last().map(|item| item.id)).flatten();
            Ok(CustomerPage { items, next })
        })
    }

    /// Atomically writes customer state, audit, idempotency, event, and sync work.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error if the transaction cannot commit.
    pub fn commit_customer(
        &self,
        commit: &CustomerCommit<'_>,
    ) -> Result<CustomerCommitOutcome, StorageError> {
        self.write_transaction(|transaction| commit_customer_on(transaction, commit))
    }

    /// Loads one bounded customer sync batch without deleting it.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid limit or malformed durable work.
    pub fn customer_sync_batch(
        &self,
        scope: &ScopeRef,
        limit: u32,
    ) -> Result<Vec<ChangeRecord>, StorageError> {
        if !(1..=MAX_CUSTOMER_SYNC_BATCH).contains(&limit) {
            return Err(StorageError);
        }
        let connection = self.open_connection()?;
        let (scope_kind, scope_id) = scope_parts(scope);
        let mut statement = connection
            .prepare(
                "SELECT change_json FROM customer_sync_outbox
                 WHERE scope_kind = ?1 AND scope_id = ?2 ORDER BY rowid LIMIT ?3",
            )
            .map_err(|_| StorageError)?;
        statement
            .query_map(params![scope_kind, scope_id, limit], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .map_err(|_| StorageError)?
            .map(|row| {
                serde_json::from_slice(&row.map_err(|_| StorageError)?).map_err(|_| StorageError)
            })
            .collect()
    }

    /// Confirms one delivered customer change and marks its record current when
    /// no later change remains queued.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error when the exact scoped change is unavailable.
    pub fn confirm_customer_sync(
        &self,
        scope: &ScopeRef,
        change_id: ChangeId,
    ) -> Result<(), StorageError> {
        self.write_transaction(|transaction| {
            let (scope_kind, scope_id) = scope_parts(scope);
            let customer_id = transaction
                .query_row(
                    "SELECT customer_id FROM customer_sync_outbox
                     WHERE change_id = ?1 AND scope_kind = ?2 AND scope_id = ?3",
                    params![change_id.value().to_string(), scope_kind, scope_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|_| StorageError)?
                .ok_or(StorageError)?;
            transaction
                .execute(
                    "DELETE FROM customer_sync_outbox WHERE change_id = ?1",
                    [change_id.value().to_string()],
                )
                .map_err(|_| StorageError)?;
            let has_later_change = transaction
                .query_row(
                    "SELECT 1 FROM customer_sync_outbox
                     WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3 LIMIT 1",
                    params![scope_kind, scope_id, customer_id],
                    |_| Ok(()),
                )
                .optional()
                .map_err(|_| StorageError)?
                .is_some();
            if !has_later_change {
                transaction
                    .execute(
                        "UPDATE customers SET sync_state = 'confirmed'
                         WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3",
                        params![scope_kind, scope_id, customer_id],
                    )
                    .map_err(|_| StorageError)?;
            }
            Ok(())
        })
    }
}

fn commit_customer_on(
    transaction: &rusqlite::Connection,
    commit: &CustomerCommit<'_>,
) -> Result<CustomerCommitOutcome, StorageError> {
    let scope = &commit.customer.scope;
    if let Some((stored_hash, response_json)) =
        load_idempotency(transaction, scope, commit.idempotency.key)?
    {
        if stored_hash == commit.idempotency.request_hash {
            return Ok(CustomerCommitOutcome::Replayed { response_json });
        }
        insert_audit(
            transaction,
            &commit.audit.clone().with_outcome(
                AuditOutcome::Invalid,
                Some("eitmad.error.contract-invalid.v1".to_owned()),
            ),
        )?;
        return Ok(CustomerCommitOutcome::IdempotencyMismatch);
    }

    let (scope_kind, scope_id) = scope_parts(scope);
    let customer_id = commit.customer.id.value().to_string();
    let actual_revision = transaction
        .query_row(
            "SELECT revision FROM customers
             WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3",
            params![scope_kind, scope_id, customer_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| StorageError)?
        .map(u64::try_from)
        .transpose()
        .map_err(|_| StorageError)?;
    if actual_revision != commit.expected_revision {
        let mut conflict = commit.audit.clone().with_outcome(
            AuditOutcome::Conflict,
            Some(commit.conflict_error_code.to_owned()),
        );
        conflict.previous_revision = actual_revision;
        conflict.resulting_revision = actual_revision;
        insert_audit(transaction, &conflict)?;
        return Ok(CustomerCommitOutcome::RevisionConflict { actual_revision });
    }

    let potential_duplicate_ids = duplicate_candidates_on(
        transaction,
        scope,
        commit.normalized_phone,
        Some(commit.customer.id),
    )?;

    persist_customer_change_on(transaction, commit, scope_kind, &scope_id, &customer_id)?;

    let mut success = commit.audit.clone();
    success.outcome = AuditOutcome::Succeeded;
    success.previous_revision = actual_revision;
    success.resulting_revision = Some(commit.customer.revision);
    insert_audit(transaction, &success)?;
    let result = CustomerMutationResult {
        customer: commit.customer.clone(),
        potential_duplicate_ids,
    };
    let mut idempotency = commit.idempotency.clone();
    idempotency.response_json = serde_json::to_vec(&result).map_err(|_| StorageError)?;
    insert_idempotency(transaction, scope, commit.operation, &idempotency)?;
    insert_publication(
        transaction,
        scope,
        commit.idempotency.key,
        commit.publication,
    )?;
    Ok(CustomerCommitOutcome::Committed(result))
}

fn persist_customer_change_on(
    transaction: &rusqlite::Connection,
    commit: &CustomerCommit<'_>,
    scope_kind: &str,
    scope_id: &str,
    customer_id: &str,
) -> Result<(), StorageError> {
    transaction
        .execute(
            "INSERT INTO customers
                 (scope_kind, scope_id, customer_id, name, normalized_name, phone,
                  normalized_phone, address, notes, status, revision, updated_at, sync_state)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'pending')
             ON CONFLICT(scope_kind, scope_id, customer_id) DO UPDATE SET
               name = excluded.name,
               normalized_name = excluded.normalized_name,
               phone = excluded.phone,
               normalized_phone = excluded.normalized_phone,
               address = excluded.address,
               notes = excluded.notes,
               status = excluded.status,
               revision = excluded.revision,
               updated_at = excluded.updated_at,
               sync_state = excluded.sync_state",
            params![
                scope_kind,
                scope_id,
                customer_id,
                commit.customer.name.as_str(),
                commit.normalized_name,
                commit.customer.phone.as_str(),
                commit.normalized_phone,
                commit
                    .customer
                    .address
                    .as_ref()
                    .map(CustomerAddress::as_str),
                commit.customer.notes.as_ref().map(CustomerNotes::as_str),
                match commit.customer.status {
                    CustomerStatus::Active => "active",
                    CustomerStatus::Archived => "archived",
                    CustomerStatus::Merged => "merged",
                },
                i64::try_from(commit.customer.revision).map_err(|_| StorageError)?,
                commit.customer.updated_at.0,
            ],
        )
        .map_err(|_| StorageError)?;
    let encoded_change = serde_json::to_vec(commit.change).map_err(|_| StorageError)?;
    transaction
        .execute(
            "INSERT INTO customer_sync_outbox
                 (change_id, scope_kind, scope_id, customer_id, change_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                commit.change.change_id.value().to_string(),
                scope_kind,
                scope_id,
                customer_id,
                encoded_change,
            ],
        )
        .map_err(|_| StorageError)?;

    Ok(())
}

fn duplicate_candidates_on(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    normalized_phone: &str,
    excluding: Option<CustomerId>,
) -> Result<Vec<CustomerId>, StorageError> {
    let (scope_kind, scope_id) = scope_parts(scope);
    let excluding = excluding
        .map(|id| id.value().to_string())
        .unwrap_or_default();
    let mut statement = connection
        .prepare(
            "SELECT customer_id FROM customers
             WHERE scope_kind = ?1 AND scope_id = ?2 AND normalized_phone = ?3
               AND customer_id != ?4
             ORDER BY customer_id LIMIT ?5",
        )
        .map_err(|_| StorageError)?;
    statement
        .query_map(
            params![
                scope_kind,
                scope_id,
                normalized_phone,
                excluding,
                MAX_DUPLICATE_ADVISORIES
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| StorageError)?
        .map(|row| {
            Ok(CustomerId::new(
                uuid::Uuid::parse_str(&row.map_err(|_| StorageError)?).map_err(|_| StorageError)?,
            ))
        })
        .collect()
}

type CustomerRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    i64,
    i64,
    String,
);

fn customer_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CustomerRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
    ))
}

fn decode_customer(scope: &ScopeRef, row: CustomerRow) -> Result<Customer, StorageError> {
    Ok(Customer {
        id: CustomerId::new(uuid::Uuid::parse_str(&row.0).map_err(|_| StorageError)?),
        scope: scope.clone(),
        name: CustomerName::parse(row.1).map_err(|_| StorageError)?,
        phone: CustomerPhone::parse(row.2).map_err(|_| StorageError)?,
        address: row
            .3
            .map(CustomerAddress::parse)
            .transpose()
            .map_err(|_| StorageError)?,
        notes: row
            .4
            .map(CustomerNotes::parse)
            .transpose()
            .map_err(|_| StorageError)?,
        status: match row.5.as_str() {
            "active" => CustomerStatus::Active,
            "archived" => CustomerStatus::Archived,
            "merged" => CustomerStatus::Merged,
            _ => return Err(StorageError),
        },
        revision: u64::try_from(row.6).map_err(|_| StorageError)?,
        updated_at: UnixMillis(row.7),
        sync_state: match row.8.as_str() {
            "pending" => CustomerSyncState::Pending,
            "confirmed" => CustomerSyncState::Confirmed,
            _ => return Err(StorageError),
        },
    })
}

fn contains_pattern(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}
