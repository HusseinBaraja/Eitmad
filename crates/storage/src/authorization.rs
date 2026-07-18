use eitmad_contracts::{
    authorization::{
        RelationId, RelationshipId, RelationshipMutationResult, RelationshipSubject,
        ScopeRelationship,
    },
    identity::{PrincipalId, PrincipalKind, ScopeId, ScopeKind, ScopeRef},
};
use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};
use uuid::Uuid;

use crate::{
    AuthorityStore, DurableIdempotency, StorageError, insert_audit, insert_idempotency,
    load_idempotency, scope_parts,
};

const POLICY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelationshipCommitOutcome {
    Committed(RelationshipMutationResult),
    Replayed(RelationshipMutationResult),
    PolicyConflict { actual_version: u64 },
    LastProtectedRelationship,
    RelationshipNotFound,
    BootstrapUnavailable,
    IdempotencyMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationshipPageData {
    pub policy_version: u64,
    pub relationships: Vec<ScopeRelationship>,
    pub next_after: Option<RelationshipId>,
}

impl AuthorityStore {
    /// Reads direct relationships for one principal in one exact scope.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for unreadable or malformed state.
    pub fn relationships_for_subject(
        &self,
        scope: &ScopeRef,
        subject: &RelationshipSubject,
    ) -> Result<Vec<ScopeRelationship>, StorageError> {
        let connection = self.open_connection()?;
        let (scope_kind, scope_id) = scope_parts(scope);
        let principal_kind =
            serde_json::to_string(&subject.principal_kind).map_err(|_| StorageError)?;
        let mut statement = connection
            .prepare(
                "SELECT relationship_id, principal_id, principal_kind, relation
                 FROM scope_relationships
                 WHERE scope_kind = ?1 AND scope_id = ?2 AND principal_id = ?3
                   AND principal_kind = ?4
                 ORDER BY relationship_id",
            )
            .map_err(|_| StorageError)?;
        let rows = statement
            .query_map(
                params![
                    scope_kind,
                    scope_id,
                    subject.principal_id.value().to_string(),
                    principal_kind,
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .map_err(|_| StorageError)?;
        rows.map(|row| decode_relationship(scope, row.map_err(|_| StorageError)?))
            .collect()
    }

    /// Returns the current policy revision for one exact scope.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for unreadable state.
    pub fn policy_version(&self, scope: &ScopeRef) -> Result<u64, StorageError> {
        let connection = self.open_connection()?;
        let (scope_kind, scope_id) = scope_parts(scope);
        let version = connection
            .query_row(
                "SELECT policy_version FROM authorization_scopes
                 WHERE scope_kind = ?1 AND scope_id = ?2",
                (scope_kind, scope_id),
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|_| StorageError)?
            .unwrap_or_default();
        u64::try_from(version).map_err(|_| StorageError)
    }

    /// Lists a stable bounded page of relationships for one exact scope.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for unreadable or malformed state.
    pub fn list_relationships(
        &self,
        scope: &ScopeRef,
        after: Option<RelationshipId>,
        limit: u32,
    ) -> Result<RelationshipPageData, StorageError> {
        if !(1..=500).contains(&limit) {
            return Err(StorageError);
        }
        let connection = self.open_connection()?;
        let policy_version = self.policy_version(scope)?;
        let (scope_kind, scope_id) = scope_parts(scope);
        let after = after.map(|id| id.value().to_string()).unwrap_or_default();
        let fetch = limit.checked_add(1).ok_or(StorageError)?;
        let mut statement = connection
            .prepare(
                "SELECT relationship_id, principal_id, principal_kind, relation
                 FROM scope_relationships
                 WHERE scope_kind = ?1 AND scope_id = ?2 AND relationship_id > ?3
                 ORDER BY relationship_id LIMIT ?4",
            )
            .map_err(|_| StorageError)?;
        let rows = statement
            .query_map(params![scope_kind, scope_id, after, fetch], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| StorageError)?;
        let mut relationships = Vec::new();
        for row in rows {
            relationships.push(decode_relationship(scope, row.map_err(|_| StorageError)?)?);
        }
        let has_more = relationships.len() > usize::try_from(limit).map_err(|_| StorageError)?;
        if has_more {
            relationships.pop();
        }
        let next_after = has_more
            .then(|| relationships.last().map(|value| value.relationship_id))
            .flatten();
        Ok(RelationshipPageData {
            policy_version,
            relationships,
            next_after,
        })
    }

    /// Atomically grants a direct principal-to-scope relationship.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error if the transaction cannot commit.
    #[allow(clippy::too_many_arguments)]
    pub fn grant_relationship(
        &self,
        scope: &ScopeRef,
        expected_policy_version: u64,
        relationship_id: RelationshipId,
        subject: &RelationshipSubject,
        relation: &RelationId,
        operation: &str,
        idempotency: &DurableIdempotency,
        audit: &MutationAuditRecord,
    ) -> Result<RelationshipCommitOutcome, StorageError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction().map_err(|_| StorageError)?;
        if let Some(outcome) = replay_relationship(&transaction, scope, idempotency, audit)? {
            transaction.commit().map_err(|_| StorageError)?;
            return Ok(outcome);
        }
        let actual = policy_version_on(&transaction, scope)?;
        if actual != expected_policy_version {
            record_policy_conflict(&transaction, audit, actual)?;
            transaction.commit().map_err(|_| StorageError)?;
            return Ok(RelationshipCommitOutcome::PolicyConflict {
                actual_version: actual,
            });
        }
        let (scope_kind, scope_id) = scope_parts(scope);
        let principal_kind =
            serde_json::to_string(&subject.principal_kind).map_err(|_| StorageError)?;
        let existing = transaction
            .query_row(
                "SELECT relationship_id FROM scope_relationships
                 WHERE scope_kind = ?1 AND scope_id = ?2 AND principal_id = ?3
                   AND principal_kind = ?4 AND relation = ?5",
                params![
                    scope_kind,
                    scope_id,
                    subject.principal_id.value().to_string(),
                    principal_kind,
                    relation.as_str(),
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| StorageError)?;
        let changed = existing.is_none();
        let resulting_version = actual + u64::from(changed);
        let result_relationship = ScopeRelationship {
            relationship_id: existing
                .map(|value| parse_relationship_id(&value))
                .transpose()?
                .unwrap_or(relationship_id),
            subject: subject.clone(),
            relation: relation.clone(),
            scope: scope.clone(),
        };
        if changed {
            ensure_authorization_scope(&transaction, scope, resulting_version)?;
            transaction
                .execute(
                    "INSERT INTO scope_relationships
                     (relationship_id, scope_kind, scope_id, principal_id, principal_kind, relation)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        relationship_id.value().to_string(),
                        scope_kind,
                        scope_id,
                        subject.principal_id.value().to_string(),
                        principal_kind,
                        relation.as_str(),
                    ],
                )
                .map_err(|_| StorageError)?;
        }
        let result = RelationshipMutationResult {
            policy_version: resulting_version,
            relationship: result_relationship,
            changed,
        };
        finish_relationship_mutation(
            &transaction,
            scope,
            operation,
            idempotency,
            audit,
            actual,
            &result,
        )?;
        transaction.commit().map_err(|_| StorageError)?;
        Ok(RelationshipCommitOutcome::Committed(result))
    }

    /// Atomically revokes a relationship while preserving a protected minimum.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error if the transaction cannot commit.
    #[allow(clippy::too_many_arguments)]
    pub fn revoke_relationship(
        &self,
        scope: &ScopeRef,
        expected_policy_version: u64,
        relationship_id: RelationshipId,
        protected_relation: &RelationId,
        operation: &str,
        idempotency: &DurableIdempotency,
        audit: &MutationAuditRecord,
    ) -> Result<RelationshipCommitOutcome, StorageError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction().map_err(|_| StorageError)?;
        if let Some(outcome) = replay_relationship(&transaction, scope, idempotency, audit)? {
            transaction.commit().map_err(|_| StorageError)?;
            return Ok(outcome);
        }
        let actual = policy_version_on(&transaction, scope)?;
        if actual != expected_policy_version {
            record_policy_conflict(&transaction, audit, actual)?;
            transaction.commit().map_err(|_| StorageError)?;
            return Ok(RelationshipCommitOutcome::PolicyConflict {
                actual_version: actual,
            });
        }
        let relationship = find_relationship_on(&transaction, scope, relationship_id)?;
        let Some(relationship) = relationship else {
            let invalid = audit.clone().with_outcome(
                AuditOutcome::Invalid,
                Some("eitmad.error.authorization-relation-invalid.v1".to_owned()),
            );
            insert_audit(&transaction, &invalid)?;
            transaction.commit().map_err(|_| StorageError)?;
            return Ok(RelationshipCommitOutcome::RelationshipNotFound);
        };
        if relationship.relation == *protected_relation {
            let (scope_kind, scope_id) = scope_parts(scope);
            let count: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM scope_relationships
                     WHERE scope_kind = ?1 AND scope_id = ?2 AND relation = ?3",
                    params![scope_kind, scope_id, protected_relation.as_str()],
                    |row| row.get(0),
                )
                .map_err(|_| StorageError)?;
            if count <= 1 {
                let denied = audit.clone().with_outcome(
                    AuditOutcome::Denied,
                    Some("eitmad.error.authorization-last-owner.v1".to_owned()),
                );
                insert_audit(&transaction, &denied)?;
                transaction.commit().map_err(|_| StorageError)?;
                return Ok(RelationshipCommitOutcome::LastProtectedRelationship);
            }
        }
        transaction
            .execute(
                "DELETE FROM scope_relationships WHERE relationship_id = ?1",
                [relationship_id.value().to_string()],
            )
            .map_err(|_| StorageError)?;
        let resulting_version = actual.checked_add(1).ok_or(StorageError)?;
        ensure_authorization_scope(&transaction, scope, resulting_version)?;
        let result = RelationshipMutationResult {
            policy_version: resulting_version,
            relationship,
            changed: true,
        };
        finish_relationship_mutation(
            &transaction,
            scope,
            operation,
            idempotency,
            audit,
            actual,
            &result,
        )?;
        transaction.commit().map_err(|_| StorageError)?;
        Ok(RelationshipCommitOutcome::Committed(result))
    }

    /// Inserts the first protected relationship for an empty scope.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error if the transaction cannot commit.
    pub fn bootstrap_relationship(
        &self,
        scope: &ScopeRef,
        relationship_id: RelationshipId,
        subject: &RelationshipSubject,
        relation: &RelationId,
        audit: &MutationAuditRecord,
    ) -> Result<RelationshipCommitOutcome, StorageError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction().map_err(|_| StorageError)?;
        let (scope_kind, scope_id) = scope_parts(scope);
        let count: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM scope_relationships
                 WHERE scope_kind = ?1 AND scope_id = ?2 AND relation = ?3",
                params![scope_kind, scope_id, relation.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| StorageError)?;
        if count != 0 {
            return Ok(RelationshipCommitOutcome::BootstrapUnavailable);
        }
        let actual = policy_version_on(&transaction, scope)?;
        let resulting_version = actual.checked_add(1).ok_or(StorageError)?;
        ensure_authorization_scope(&transaction, scope, resulting_version)?;
        let principal_kind =
            serde_json::to_string(&subject.principal_kind).map_err(|_| StorageError)?;
        transaction
            .execute(
                "INSERT INTO scope_relationships
                 (relationship_id, scope_kind, scope_id, principal_id, principal_kind, relation)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    relationship_id.value().to_string(),
                    scope_kind,
                    scope_id,
                    subject.principal_id.value().to_string(),
                    principal_kind,
                    relation.as_str(),
                ],
            )
            .map_err(|_| StorageError)?;
        let relationship = ScopeRelationship {
            relationship_id,
            subject: subject.clone(),
            relation: relation.clone(),
            scope: scope.clone(),
        };
        let result = RelationshipMutationResult {
            policy_version: resulting_version,
            relationship,
            changed: true,
        };
        let mut success = audit.clone();
        success.outcome = AuditOutcome::Succeeded;
        success.previous_revision = Some(actual);
        success.resulting_revision = Some(resulting_version);
        insert_audit(&transaction, &success)?;
        transaction.commit().map_err(|_| StorageError)?;
        Ok(RelationshipCommitOutcome::Committed(result))
    }
}

fn replay_relationship(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    idempotency: &DurableIdempotency,
    audit: &MutationAuditRecord,
) -> Result<Option<RelationshipCommitOutcome>, StorageError> {
    let Some((hash, response)) = load_idempotency(connection, scope, idempotency.key)? else {
        return Ok(None);
    };
    if hash == idempotency.request_hash {
        let mut result: RelationshipMutationResult =
            serde_json::from_slice(&response).map_err(|_| StorageError)?;
        result.changed = false;
        return Ok(Some(RelationshipCommitOutcome::Replayed(result)));
    }
    let invalid = audit.clone().with_outcome(
        AuditOutcome::Invalid,
        Some("eitmad.error.contract-invalid.v1".to_owned()),
    );
    insert_audit(connection, &invalid)?;
    Ok(Some(RelationshipCommitOutcome::IdempotencyMismatch))
}

fn finish_relationship_mutation(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    operation: &str,
    idempotency: &DurableIdempotency,
    audit: &MutationAuditRecord,
    previous_version: u64,
    result: &RelationshipMutationResult,
) -> Result<(), StorageError> {
    let mut success = audit.clone();
    success.outcome = AuditOutcome::Succeeded;
    success.previous_revision = Some(previous_version);
    success.resulting_revision = Some(result.policy_version);
    insert_audit(connection, &success)?;
    let mut durable = idempotency.clone();
    durable.response_json = serde_json::to_vec(result).map_err(|_| StorageError)?;
    insert_idempotency(connection, scope, operation, &durable)
}

fn record_policy_conflict(
    connection: &rusqlite::Connection,
    audit: &MutationAuditRecord,
    actual: u64,
) -> Result<(), StorageError> {
    let mut conflict = audit.clone().with_outcome(
        AuditOutcome::Conflict,
        Some("eitmad.error.authorization-policy-conflict.v1".to_owned()),
    );
    conflict.previous_revision = Some(actual);
    conflict.resulting_revision = Some(actual);
    insert_audit(connection, &conflict)
}

fn policy_version_on(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
) -> Result<u64, StorageError> {
    let (scope_kind, scope_id) = scope_parts(scope);
    let version = connection
        .query_row(
            "SELECT policy_version FROM authorization_scopes
             WHERE scope_kind = ?1 AND scope_id = ?2",
            (scope_kind, scope_id),
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| StorageError)?
        .unwrap_or_default();
    u64::try_from(version).map_err(|_| StorageError)
}

fn ensure_authorization_scope(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    policy_version: u64,
) -> Result<(), StorageError> {
    let (scope_kind, scope_id) = scope_parts(scope);
    let policy_version = i64::try_from(policy_version).map_err(|_| StorageError)?;
    connection
        .execute(
            "INSERT INTO authorization_scopes
             (scope_kind, scope_id, policy_schema_version, policy_version)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(scope_kind, scope_id) DO UPDATE SET
               policy_schema_version = excluded.policy_schema_version,
               policy_version = excluded.policy_version",
            params![scope_kind, scope_id, POLICY_SCHEMA_VERSION, policy_version],
        )
        .map_err(|_| StorageError)?;
    Ok(())
}

fn find_relationship_on(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    relationship_id: RelationshipId,
) -> Result<Option<ScopeRelationship>, StorageError> {
    let (scope_kind, scope_id) = scope_parts(scope);
    connection
        .query_row(
            "SELECT relationship_id, principal_id, principal_kind, relation
             FROM scope_relationships
             WHERE scope_kind = ?1 AND scope_id = ?2 AND relationship_id = ?3",
            params![scope_kind, scope_id, relationship_id.value().to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|_| StorageError)?
        .map(|stored| decode_relationship(scope, stored))
        .transpose()
}

fn decode_relationship(
    scope: &ScopeRef,
    stored: (String, String, String, String),
) -> Result<ScopeRelationship, StorageError> {
    let (relationship_id, principal_id, principal_kind, relation) = stored;
    Ok(ScopeRelationship {
        relationship_id: parse_relationship_id(&relationship_id)?,
        subject: RelationshipSubject {
            principal_id: PrincipalId::new(
                Uuid::parse_str(&principal_id).map_err(|_| StorageError)?,
            ),
            principal_kind: serde_json::from_str::<PrincipalKind>(&principal_kind)
                .map_err(|_| StorageError)?,
        },
        relation: RelationId::parse(relation).map_err(|_| StorageError)?,
        scope: ScopeRef {
            kind: ScopeKind::parse(scope.kind.as_str()).map_err(|_| StorageError)?,
            id: ScopeId::new(scope.id.value()),
        },
    })
}

fn parse_relationship_id(value: &str) -> Result<RelationshipId, StorageError> {
    Ok(RelationshipId::new(
        Uuid::parse_str(value).map_err(|_| StorageError)?,
    ))
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::{
        identity::{PrincipalKind, ScopeId, ScopeKind},
        transport::{CorrelationId, IdempotencyKey, UnixMillis},
    };
    use tempfile::TempDir;

    use super::*;

    fn scope() -> ScopeRef {
        ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(Uuid::from_u128(1)),
        }
    }

    fn subject(id: u128) -> RelationshipSubject {
        RelationshipSubject {
            principal_id: PrincipalId::new(Uuid::from_u128(id)),
            principal_kind: PrincipalKind::User,
        }
    }

    fn audit(key: IdempotencyKey) -> MutationAuditRecord {
        MutationAuditRecord {
            audit_id: Uuid::new_v4(),
            occurred_at: UnixMillis(1),
            principal_id: PrincipalId::new(Uuid::from_u128(2)),
            principal_kind: PrincipalKind::User,
            scope: scope(),
            correlation_id: CorrelationId::new(Uuid::from_u128(3)),
            causation_id: None,
            idempotency_key: Some(key),
            operation: "relationship".to_owned(),
            outcome: AuditOutcome::Succeeded,
            previous_revision: None,
            resulting_revision: None,
            changed_identifiers: Vec::new(),
            error_code: None,
        }
    }

    #[test]
    fn grants_pages_and_replays_relationships() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let key = IdempotencyKey::new(Uuid::from_u128(10));
        let relation = RelationId::parse("eitmad.relation.organization.owner.v1").unwrap();
        let durable = DurableIdempotency {
            key,
            request_hash: [4; 32],
            response_json: Vec::new(),
        };
        let first = store
            .grant_relationship(
                &scope(),
                0,
                RelationshipId::new(Uuid::from_u128(20)),
                &subject(30),
                &relation,
                "grant",
                &durable,
                &audit(key),
            )
            .unwrap();
        assert!(matches!(first, RelationshipCommitOutcome::Committed(_)));
        let replay = store
            .grant_relationship(
                &scope(),
                0,
                RelationshipId::new(Uuid::from_u128(99)),
                &subject(30),
                &relation,
                "grant",
                &durable,
                &audit(key),
            )
            .unwrap();
        assert!(matches!(replay, RelationshipCommitOutcome::Replayed(_)));
        let page = store.list_relationships(&scope(), None, 10).unwrap();
        assert_eq!(page.policy_version, 1);
        assert_eq!(page.relationships.len(), 1);
    }

    #[test]
    fn protects_the_last_owner() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let relation = RelationId::parse("eitmad.relation.organization.owner.v1").unwrap();
        let bootstrap = store
            .bootstrap_relationship(
                &scope(),
                RelationshipId::new(Uuid::from_u128(20)),
                &subject(30),
                &relation,
                &audit(IdempotencyKey::new(Uuid::from_u128(10))),
            )
            .unwrap();
        assert!(matches!(bootstrap, RelationshipCommitOutcome::Committed(_)));
        let key = IdempotencyKey::new(Uuid::from_u128(11));
        let outcome = store
            .revoke_relationship(
                &scope(),
                1,
                RelationshipId::new(Uuid::from_u128(20)),
                &relation,
                "revoke",
                &DurableIdempotency {
                    key,
                    request_hash: [5; 32],
                    response_json: Vec::new(),
                },
                &audit(key),
            )
            .unwrap();
        assert_eq!(
            outcome,
            RelationshipCommitOutcome::LastProtectedRelationship
        );
    }
}
