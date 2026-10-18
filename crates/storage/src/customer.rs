use eitmad_contracts::{
    customer::{
        Customer, CustomerAddress, CustomerId, CustomerMutationResult, CustomerName, CustomerNotes,
        CustomerPage, CustomerPhone, CustomerStatus, CustomerSyncState,
    },
    identity::{AuthorizationContext, ScopeRef},
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

pub(crate) const SYNC_MIGRATIONS: &[Migration] = &[Migration::new(
    14,
    "customer.sync-exceptions.v1",
    "customer",
    "CREATE TABLE customer_sync_exceptions (
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         customer_id TEXT NOT NULL,
         state TEXT NOT NULL CHECK (state IN ('rejected', 'conflicted')),
         PRIMARY KEY (scope_kind, scope_id, customer_id),
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

pub struct CustomerSyncProjection<'a> {
    pub customer: &'a Customer,
    pub normalized_name: &'a str,
    pub normalized_phone: &'a str,
    pub source_change_id: ChangeId,
    pub audit: &'a MutationAuditRecord,
    pub publication: &'a DurablePublication,
}

impl AuthorityStore {
    /// Recovers original customer authors from the audit committed with each outbox entry.
    /// Returns distinct author contexts only for successful customer create/update audits.
    /// Excludes customers with sync exceptions; the caller must check current authority.
    /// # Errors
    /// Returns an error if storage or matched audit evidence cannot be read or decoded.
    pub fn pending_customer_actors(&self) -> Result<Vec<AuthorizationContext>, StorageError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT DISTINCT json_object(
                'sessionId', a.session_id, 'tenantId', a.tenant_id,
                'workspaceId', a.workspace_id,
                'identity', json_object('principalId', a.principal_id,
                    'principalKind', json(a.principal_kind), 'deviceId', a.device_id,
                    'serviceId', NULL),
                'scope', json_object('kind', a.scope_kind, 'id', a.scope_id))
             FROM customer_sync_outbox o JOIN mutation_audit a
               ON a.scope_kind = o.scope_kind AND a.scope_id = o.scope_id
               AND a.idempotency_key = json_extract(CAST(o.change_json AS TEXT), '$.idempotencyKey')
               AND a.operation IN ('eitmad.customer.create.v1', 'eitmad.customer.update.v1')
               AND a.outcome = '\"succeeded\"'
             WHERE NOT EXISTS (SELECT 1 FROM customer_sync_exceptions e
               WHERE e.scope_kind = o.scope_kind AND e.scope_id = o.scope_id
                 AND e.customer_id = o.customer_id)",
            )
            .map_err(|_| StorageError)?;
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| StorageError)?
            .map(|row| {
                serde_json::from_str(&row.map_err(|_| StorageError)?).map_err(|_| StorageError)
            })
            .collect()
    }

    /// Loads pending changes for their original author, including work retained after sign-out.
    /// Matches the scope and successful create/update audit's tenant, principal ID/kind, and device
    /// exactly (including an absent device), without requiring the original session ID.
    /// Excludes customers with sync exceptions and selects only each customer's earliest
    /// pending entry, in insertion order, with at most 50 records. The caller authorizes access.
    /// # Errors
    /// Returns an error for malformed durable work.
    pub fn customer_sync_batch_for_author(
        &self,
        actor: &AuthorizationContext,
    ) -> Result<Vec<ChangeRecord>, StorageError> {
        let connection = self.open_connection()?;
        let (kind, id) = scope_parts(&actor.scope);
        let principal_kind =
            serde_json::to_string(&actor.identity.principal_kind).map_err(|_| StorageError)?;
        let mut statement = connection.prepare(
            "SELECT o.change_json FROM customer_sync_outbox o
             WHERE o.scope_kind = ?1 AND o.scope_id = ?2
               AND NOT EXISTS (SELECT 1 FROM customer_sync_outbox earlier
                 WHERE earlier.scope_kind = o.scope_kind AND earlier.scope_id = o.scope_id
                   AND earlier.customer_id = o.customer_id AND earlier.rowid < o.rowid)
               AND NOT EXISTS (SELECT 1 FROM customer_sync_exceptions e
                 WHERE e.scope_kind = o.scope_kind AND e.scope_id = o.scope_id
                   AND e.customer_id = o.customer_id)
               AND EXISTS (SELECT 1 FROM mutation_audit a
                 WHERE a.scope_kind = o.scope_kind AND a.scope_id = o.scope_id
                   AND a.idempotency_key = json_extract(CAST(o.change_json AS TEXT), '$.idempotencyKey')
                   AND a.operation IN ('eitmad.customer.create.v1', 'eitmad.customer.update.v1')
                   AND a.outcome = '\"succeeded\"'
                   AND a.tenant_id = ?3 AND a.principal_id = ?4
                   AND a.principal_kind = ?5 AND a.device_id IS ?6)
             ORDER BY o.rowid LIMIT 50"
        ).map_err(|_| StorageError)?;
        statement
            .query_map(
                params![
                    kind,
                    id,
                    actor.tenant_id.value().to_string(),
                    actor.identity.principal_id.value().to_string(),
                    principal_kind,
                    actor.identity.device_id.map(|id| id.value().to_string())
                ],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(|_| StorageError)?
            .map(|row| {
                serde_json::from_slice(&row.map_err(|_| StorageError)?).map_err(|_| StorageError)
            })
            .collect()
    }

    /// Projects one server-confirmed contact without overwriting a later local edit.
    /// This is idempotent so a shared-sync checkpoint can be replayed after a crash.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error if projection or its audit cannot commit.
    pub fn project_customer_sync(
        &self,
        projection: &CustomerSyncProjection<'_>,
    ) -> Result<bool, StorageError> {
        self.write_transaction(|transaction| {
            let customer = projection.customer;
            let (scope_kind, scope_id) = scope_parts(&customer.scope);
            let customer_id = customer.id.value().to_string();
            let removed = transaction
                .execute(
                    "DELETE FROM customer_sync_outbox
                     WHERE change_id = ?1 AND scope_kind = ?2 AND scope_id = ?3 AND customer_id = ?4",
                    params![projection.source_change_id.value().to_string(), scope_kind, scope_id, customer_id],
                )
                .map_err(|_| StorageError)?;
            let has_pending = transaction
                .query_row(
                    "SELECT 1 FROM customer_sync_outbox
                     WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3 LIMIT 1",
                    params![scope_kind, scope_id, customer_id],
                    |_| Ok(()),
                )
                .optional()
                .map_err(|_| StorageError)?
                .is_some();
            if has_pending {
                if removed != 0 {
                    insert_audit(transaction, projection.audit)?;
                    insert_publication(transaction, &customer.scope, eitmad_contracts::transport::IdempotencyKey::new(uuid::Uuid::new_v4()), projection.publication)?;
                }
                return Ok(removed != 0);
            }
            let current: Option<(i64, String)> = transaction
                .query_row(
                    "SELECT revision, sync_state FROM customers WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3",
                    params![scope_kind, scope_id, customer_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|_| StorageError)?;
            let target_revision = i64::try_from(customer.revision).map_err(|_| StorageError)?;
            if current.as_ref().is_some_and(|(revision, _)| *revision > target_revision) {
                if removed != 0 {
                    insert_audit(transaction, projection.audit)?;
                }
                return Ok(removed != 0);
            }
            let has_exception = transaction.query_row(
                "SELECT 1 FROM customer_sync_exceptions WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3",
                params![scope_kind, scope_id, customer_id], |_| Ok(()),
            ).optional().map_err(|_| StorageError)?.is_some();
            if removed == 0 && !has_exception && current.as_ref().is_some_and(|(revision, state)| *revision == target_revision && state == "confirmed") {
                return Ok(false);
            }
            transaction.execute(
                "INSERT INTO customers
                   (scope_kind, scope_id, customer_id, name, normalized_name, phone, normalized_phone,
                    address, notes, status, revision, updated_at, sync_state)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'active', ?10, ?11, 'confirmed')
                 ON CONFLICT(scope_kind, scope_id, customer_id) DO UPDATE SET
                   name = excluded.name, normalized_name = excluded.normalized_name,
                   phone = excluded.phone, normalized_phone = excluded.normalized_phone,
                   address = excluded.address, notes = excluded.notes,
                   revision = excluded.revision, updated_at = excluded.updated_at,
                   sync_state = 'confirmed'",
                params![scope_kind, scope_id, customer_id, customer.name.as_str(), projection.normalized_name,
                    customer.phone.as_str(), projection.normalized_phone,
                    customer.address.as_ref().map(CustomerAddress::as_str),
                    customer.notes.as_ref().map(CustomerNotes::as_str),
                    i64::try_from(customer.revision).map_err(|_| StorageError)?, customer.updated_at.0],
            ).map_err(|_| StorageError)?;
            let cleared = transaction.execute(
                "DELETE FROM customer_sync_exceptions WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3",
                params![scope_kind, scope_id, customer_id],
            ).map_err(|_| StorageError)?;
            debug_assert_eq!(cleared != 0, has_exception);
            insert_audit(transaction, projection.audit)?;
            insert_publication(transaction, &customer.scope, eitmad_contracts::transport::IdempotencyKey::new(uuid::Uuid::new_v4()), projection.publication)?;
            Ok(true)
        })
    }

    /// Marks local contact delivery as rejected or conflicted, retaining its outbox input.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error if the status and audit cannot commit.
    pub fn mark_customer_sync_exception(
        &self,
        scope: &ScopeRef,
        customer_id: CustomerId,
        state: CustomerSyncState,
        audit: &MutationAuditRecord,
        publication: &DurablePublication,
    ) -> Result<(), StorageError> {
        let state = match state {
            CustomerSyncState::Rejected => "rejected",
            CustomerSyncState::Conflicted => "conflicted",
            _ => return Err(StorageError),
        };
        self.write_transaction(|transaction| {
            let (scope_kind, scope_id) = scope_parts(scope);
            let prior: Option<String> = transaction.query_row(
                "SELECT state FROM customer_sync_exceptions WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3",
                params![scope_kind, scope_id, customer_id.value().to_string()],
                |row| row.get(0),
            ).optional().map_err(|_| StorageError)?;
            if prior.as_deref() == Some(state) { return Ok(()); }
            let exists = transaction.query_row(
                "SELECT 1 FROM customers WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3",
                params![scope_kind, scope_id, customer_id.value().to_string()],
                |_| Ok(()),
            ).optional().map_err(|_| StorageError)?.is_some();
            if !exists { return Err(StorageError); }
            transaction.execute(
                "INSERT INTO customer_sync_exceptions (scope_kind, scope_id, customer_id, state)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(scope_kind, scope_id, customer_id) DO UPDATE SET state = excluded.state",
                params![scope_kind, scope_id, customer_id.value().to_string(), state],
            ).map_err(|_| StorageError)?;
            insert_audit(transaction, audit)?;
            insert_publication(transaction, scope, eitmad_contracts::transport::IdempotencyKey::new(uuid::Uuid::new_v4()), publication)
        })
    }
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
        self.read_transaction(|connection| get_customer_on(connection, scope, customer_id))
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
                            updated_at, COALESCE((SELECT state FROM customer_sync_exceptions e
                              WHERE e.scope_kind = customers.scope_kind AND e.scope_id = customers.scope_id
                                AND e.customer_id = customers.customer_id), sync_state)
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
            let next = has_more.then(|| items.last().map(|item| eitmad_contracts::customer::CustomerCursor {
                customer_id: item.id, branch_id: item.scope.id,
            })).flatten();
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
                "SELECT o.change_json FROM customer_sync_outbox o
                 WHERE o.scope_kind = ?1 AND o.scope_id = ?2
                   AND NOT EXISTS (SELECT 1 FROM customer_sync_exceptions e
                     WHERE e.scope_kind = o.scope_kind AND e.scope_id = o.scope_id
                       AND e.customer_id = o.customer_id)
                 ORDER BY o.rowid LIMIT ?3",
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
            "rejected" => CustomerSyncState::Rejected,
            "conflicted" => CustomerSyncState::Conflicted,
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

/// Reads a customer in the caller's consistent read transaction.
pub(crate) fn get_customer_on(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    customer_id: CustomerId,
) -> Result<Option<Customer>, StorageError> {
    let (scope_kind, scope_id) = scope_parts(scope);
    connection
        .query_row(
            "SELECT customer_id, name, phone, address, notes, status, revision,
                updated_at, COALESCE((SELECT state FROM customer_sync_exceptions e
                  WHERE e.scope_kind = customers.scope_kind AND e.scope_id = customers.scope_id
                    AND e.customer_id = customers.customer_id), sync_state)
         FROM customers
         WHERE scope_kind = ?1 AND scope_id = ?2 AND customer_id = ?3",
            params![scope_kind, scope_id, customer_id.value().to_string()],
            customer_row,
        )
        .optional()
        .map_err(|_| StorageError)?
        .map(|row| decode_customer(scope, row))
        .transpose()
}
