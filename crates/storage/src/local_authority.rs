use eitmad_contracts::identity::{
    AccountId, AuthenticatedIdentity, AuthorizationContext, DeviceId, OrganizationId, PrincipalId,
    PrincipalKind, ScopeId, ScopeKind, ScopeRef, SessionId, TenantId, UserId,
};
use eitmad_contracts::transport::{CorrelationId, UnixMillis};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};
use uuid::Uuid;

use crate::{AuthorityStore, StorageError, migrations::Migration};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::additive(
    9,
    "identity.local-authority.v1",
    "identity",
    "CREATE TABLE local_installation_authority (
         singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
         tenant_id TEXT NOT NULL,
         user_id TEXT NOT NULL,
         account_id TEXT NOT NULL,
         organization_id TEXT NOT NULL,
         device_id TEXT NOT NULL,
         created_at INTEGER NOT NULL,
         FOREIGN KEY (tenant_id) REFERENCES identity_tenants(tenant_id),
         FOREIGN KEY (account_id, user_id, tenant_id)
             REFERENCES identity_accounts(account_id, user_id, tenant_id),
         FOREIGN KEY (organization_id, tenant_id)
             REFERENCES identity_organizations(organization_id, tenant_id),
         FOREIGN KEY (device_id) REFERENCES identity_devices(device_id)
     );",
)];

struct StoredLocalAuthority {
    tenant: TenantId,
    user: UserId,
    account: AccountId,
    organization: OrganizationId,
    device: DeviceId,
}

impl StoredLocalAuthority {
    fn authorization_context(&self) -> Result<AuthorizationContext, StorageError> {
        Ok(AuthorizationContext {
            session_id: SessionId::new(Uuid::new_v4()),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(self.user.value()),
                principal_kind: PrincipalKind::User,
                device_id: Some(self.device),
                service_id: None,
            },
            tenant_id: self.tenant,
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").map_err(|_| StorageError)?,
                id: ScopeId::new(self.tenant.value()),
            },
        })
    }
}

impl AuthorityStore {
    /// Loads the installation identity or creates it with its durable owner relation.
    ///
    /// The native shell receives only the returned session projection. It never
    /// chooses the principal, tenant, scope, or relationship.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error if bootstrap or integrity verification fails.
    pub fn local_authorization_context(
        &self,
        now: UnixMillis,
    ) -> Result<AuthorizationContext, StorageError> {
        if now.0 < 0 {
            return Err(StorageError);
        }
        let authority = self.write_transaction(|connection| {
            let stored = connection
                .query_row(
                    "SELECT tenant_id, user_id, account_id, organization_id, device_id
                     FROM local_installation_authority WHERE singleton = 1",
                    [],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                        ))
                    },
                )
                .optional()
                .map_err(|_| StorageError)?;
            let authority = match stored {
                Some(values) => decode(&values)?,
                None => create(connection, now)?,
            };
            verify(connection, &authority)?;
            Ok(authority)
        })?;
        authority.authorization_context()
    }
}

fn create(
    connection: &rusqlite::Connection,
    now: UnixMillis,
) -> Result<StoredLocalAuthority, StorageError> {
    let authority = StoredLocalAuthority {
        tenant: TenantId::new(Uuid::new_v4()),
        user: UserId::new(Uuid::new_v4()),
        account: AccountId::new(Uuid::new_v4()),
        organization: OrganizationId::new(Uuid::new_v4()),
        device: DeviceId::new(Uuid::new_v4()),
    };
    connection
        .execute(
            "INSERT INTO identity_tenants(tenant_id, created_at) VALUES (?1, ?2)",
            params![authority.tenant.value().to_string(), now.0],
        )
        .and_then(|_| {
            connection.execute(
                "INSERT INTO identity_users(user_id, created_at) VALUES (?1, ?2)",
                params![authority.user.value().to_string(), now.0],
            )
        })
        .and_then(|_| {
            connection.execute(
                "INSERT INTO identity_accounts
                 (account_id, user_id, tenant_id, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    authority.account.value().to_string(),
                    authority.user.value().to_string(),
                    authority.tenant.value().to_string(),
                    now.0,
                ],
            )
        })
        .and_then(|_| {
            connection.execute(
                "INSERT INTO identity_organizations
                 (organization_id, tenant_id, created_at) VALUES (?1, ?2, ?3)",
                params![
                    authority.organization.value().to_string(),
                    authority.tenant.value().to_string(),
                    now.0,
                ],
            )
        })
        .and_then(|_| {
            connection.execute(
                "INSERT INTO identity_devices(device_id, created_at, last_seen_at)
                 VALUES (?1, ?2, ?2)",
                params![authority.device.value().to_string(), now.0],
            )
        })
        .and_then(|_| {
            connection.execute(
                "INSERT INTO local_installation_authority
                 (singleton, tenant_id, user_id, account_id, organization_id, device_id, created_at)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    authority.tenant.value().to_string(),
                    authority.user.value().to_string(),
                    authority.account.value().to_string(),
                    authority.organization.value().to_string(),
                    authority.device.value().to_string(),
                    now.0,
                ],
            )
        })
        .map_err(|_| StorageError)?;

    let scope_id = authority.tenant.value().to_string();
    let relationship_id = Uuid::new_v4();
    connection
        .execute(
            "INSERT INTO authorization_scopes
             (scope_kind, scope_id, policy_schema_version, policy_version)
             VALUES ('organization', ?1, 1, 1)",
            [&scope_id],
        )
        .and_then(|_| {
            connection.execute(
                "INSERT INTO scope_relationships
                 (relationship_id, scope_kind, scope_id, principal_id, principal_kind, relation)
                 VALUES (?1, 'organization', ?2, ?3, ?4, ?5)",
                params![
                    relationship_id.to_string(),
                    scope_id,
                    authority.user.value().to_string(),
                    serde_json::to_string(&PrincipalKind::User)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    "eitmad.relation.organization.owner.v1",
                ],
            )
        })
        .and_then(|_| {
            let authorization = authority
                .authorization_context()
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let audit = MutationAuditRecord {
                audit_id: Uuid::new_v4(),
                occurred_at: now,
                principal_id: authorization.identity.principal_id,
                principal_kind: authorization.identity.principal_kind,
                session_id: authorization.session_id,
                device_id: authorization.identity.device_id,
                tenant_id: authorization.tenant_id,
                workspace_id: authorization.workspace_id,
                scope: authorization.scope,
                correlation_id: CorrelationId::new(Uuid::new_v4()),
                causation_id: None,
                idempotency_key: None,
                operation: "eitmad.authorization.relationship.bootstrap.v1".to_owned(),
                target: AuditTarget {
                    kind: "authorization-relationship".to_owned(),
                    identifiers: vec![relationship_id.to_string()],
                },
                outcome: AuditOutcome::Succeeded,
                previous_revision: Some(0),
                resulting_revision: Some(1),
                changed_identifiers: vec![relationship_id.to_string()],
                redacted_error: None,
                extension_points: Vec::new(),
            };
            crate::insert_audit(connection, &audit).map_err(|_| rusqlite::Error::InvalidQuery)
        })
        .map_err(|_| StorageError)?;
    Ok(authority)
}

fn verify(
    connection: &rusqlite::Connection,
    authority: &StoredLocalAuthority,
) -> Result<(), StorageError> {
    let count = connection
        .query_row(
            "SELECT COUNT(*)
             FROM local_installation_authority l
             JOIN identity_accounts a
               ON a.account_id = l.account_id AND a.user_id = l.user_id
              AND a.tenant_id = l.tenant_id
             JOIN identity_organizations o
               ON o.organization_id = l.organization_id AND o.tenant_id = l.tenant_id
             JOIN identity_devices d ON d.device_id = l.device_id
             JOIN authorization_scopes s
               ON s.scope_kind = 'organization' AND s.scope_id = l.tenant_id
             JOIN scope_relationships r
               ON r.scope_kind = s.scope_kind AND r.scope_id = s.scope_id
              AND r.principal_id = l.user_id AND r.principal_kind = ?6
              AND r.relation = 'eitmad.relation.organization.owner.v1'
             WHERE l.singleton = 1 AND l.tenant_id = ?1 AND l.user_id = ?2
               AND l.account_id = ?3 AND l.organization_id = ?4 AND l.device_id = ?5",
            params![
                authority.tenant.value().to_string(),
                authority.user.value().to_string(),
                authority.account.value().to_string(),
                authority.organization.value().to_string(),
                authority.device.value().to_string(),
                serde_json::to_string(&PrincipalKind::User).map_err(|_| StorageError)?,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| StorageError)?;
    (count == 1).then_some(()).ok_or(StorageError)
}

fn decode(
    values: &(String, String, String, String, String),
) -> Result<StoredLocalAuthority, StorageError> {
    Ok(StoredLocalAuthority {
        tenant: TenantId::new(Uuid::parse_str(&values.0).map_err(|_| StorageError)?),
        user: UserId::new(Uuid::parse_str(&values.1).map_err(|_| StorageError)?),
        account: AccountId::new(Uuid::parse_str(&values.2).map_err(|_| StorageError)?),
        organization: OrganizationId::new(Uuid::parse_str(&values.3).map_err(|_| StorageError)?),
        device: DeviceId::new(Uuid::parse_str(&values.4).map_err(|_| StorageError)?),
    })
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::authorization::RelationshipSubject;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn bootstrap_is_stable_and_persists_a_real_owner_relation() {
        let directory = tempdir().unwrap();
        let first_store = AuthorityStore::open(directory.path()).unwrap();
        let first = first_store
            .local_authorization_context(UnixMillis(100))
            .unwrap();
        let second = first_store
            .local_authorization_context(UnixMillis(200))
            .unwrap();
        assert_ne!(first.session_id, second.session_id);
        assert_eq!(first.identity, second.identity);
        assert_eq!(first.tenant_id, second.tenant_id);
        assert_eq!(first.scope, second.scope);
        let relationships = first_store
            .relationships_for_subject(
                &first.scope,
                &RelationshipSubject {
                    principal_id: first.identity.principal_id,
                    principal_kind: first.identity.principal_kind,
                },
            )
            .unwrap();
        assert_eq!(relationships.len(), 1);
        assert_eq!(
            relationships[0].relation.as_str(),
            "eitmad.relation.organization.owner.v1"
        );
        let audit_count = rusqlite::Connection::open(first_store.path())
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM mutation_audit
                 WHERE operation = 'eitmad.authorization.relationship.bootstrap.v1'
                   AND outcome = '\"succeeded\"' AND scope_id = ?1
                   AND target LIKE '%' || ?2 || '%'",
                params![
                    first.scope.id.value().to_string(),
                    relationships[0].relationship_id.value().to_string(),
                ],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        assert_eq!(audit_count, 1);

        let reopened = AuthorityStore::open(directory.path()).unwrap();
        let recovered = reopened
            .local_authorization_context(UnixMillis(300))
            .unwrap();
        assert_eq!(first.identity, recovered.identity);
        assert_eq!(first.tenant_id, recovered.tenant_id);
        assert_eq!(first.scope, recovered.scope);
    }
}
