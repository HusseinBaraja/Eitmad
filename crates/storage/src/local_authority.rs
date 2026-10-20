use eitmad_contracts::identity::{
    AccountId, AuthenticatedIdentity, AuthorizationContext, DeviceId, OrganizationId, PrincipalId,
    PrincipalKind, ScopeId, ScopeKind, ScopeRef, SessionId, TenantId, UserId,
};
use eitmad_contracts::transport::{CorrelationId, UnixMillis};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};
use uuid::Uuid;

use crate::{AuthorityStore, StorageError, migrations::Migration};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(
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

pub(crate) const BRANCH_MIGRATIONS: &[Migration] = &[Migration::new(
    13,
    "identity.local-desktop-branch.v1",
    "identity",
    "CREATE TABLE local_desktop_branch (
         singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
         tenant_id TEXT NOT NULL,
         branch_id TEXT NOT NULL UNIQUE,
         FOREIGN KEY (tenant_id) REFERENCES identity_tenants(tenant_id)
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
    /// Checks bootstrap state without creating an installation identity.
    /// # Errors
    /// Fails when durable identity state cannot be read.
    pub fn has_local_installation(&self) -> Result<bool, StorageError> {
        self.read_transaction(|connection| {
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM local_installation_authority)",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| StorageError)
        })
    }

    /// Initializes an empty installation from a server-verified tenant and registered device.
    /// The caller must authenticate the server account before calling this import boundary.
    /// Existing installations must match exactly; their identity and work are never replaced.
    ///
    /// # Errors
    /// Fails for invalid identifiers, a different existing identity, or failed atomic audit.
    pub fn initialize_server_installation(
        &self,
        tenant: TenantId,
        organization: OrganizationId,
        device: DeviceId,
        branch: ScopeId,
        now: UnixMillis,
    ) -> Result<AuthorizationContext, StorageError> {
        if now.0 < 0
            || [
                tenant.value(),
                organization.value(),
                device.value(),
                branch.value(),
            ]
            .iter()
            .any(Uuid::is_nil)
        {
            return Err(StorageError);
        }
        self.write_transaction(|connection| {
            let existing: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM local_installation_authority)", [], |r| r.get(0)
            ).map_err(|_| StorageError)?;
            if !existing {
                let authority = StoredLocalAuthority {
                    tenant, organization, device,
                    user: UserId::new(Uuid::new_v4()),
                    account: AccountId::new(Uuid::new_v4()),
                };
                persist_authority(connection, &authority, now)?;
                ensure_branch(connection, &authority, branch.value().to_string(), now)?;
            }
            let matches: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM local_installation_authority l
                 JOIN local_desktop_branch b ON b.tenant_id=l.tenant_id
                 WHERE l.tenant_id=?1 AND l.organization_id=?2 AND l.device_id=?3 AND b.branch_id=?4)",
                params![tenant.value().to_string(), organization.value().to_string(),
                    device.value().to_string(), branch.value().to_string()], |r| r.get(0)
            ).map_err(|_| StorageError)?;
            matches.then_some(()).ok_or(StorageError)
        })?;
        self.existing_local_authorization_context()
    }

    /// Reads the existing installation identity without bootstrap or repair.
    ///
    /// # Errors
    /// Fails closed when maintenance has no persisted installation authority.
    pub fn existing_local_authorization_context(
        &self,
    ) -> Result<AuthorizationContext, StorageError> {
        self.read_transaction(|connection| {
            let values = connection
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
                .map_err(|_| StorageError)?;
            let authority = decode(&values)?;
            verify(connection, &authority)?;
            authority.authorization_context()
        })
    }

    /// Returns the installation organization after local identity bootstrap.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error when bootstrap state is missing or malformed.
    pub fn local_organization_id(&self) -> Result<OrganizationId, StorageError> {
        self.read_transaction(|connection| {
            let value: String = connection
                .query_row(
                    "SELECT organization_id FROM local_installation_authority WHERE singleton = 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| StorageError)?;
            Ok(OrganizationId::new(
                Uuid::parse_str(&value).map_err(|_| StorageError)?,
            ))
        })
    }

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
            ensure_local_branch(connection, &authority, now)?;
            Ok(authority)
        })?;
        authority.authorization_context()
    }
}

fn ensure_local_branch(
    connection: &rusqlite::Connection,
    authority: &StoredLocalAuthority,
    now: UnixMillis,
) -> Result<(), StorageError> {
    let existing: Option<String> = connection
        .query_row(
            "SELECT branch_id FROM local_desktop_branch WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| StorageError)?;
    if existing.is_some() {
        return Ok(());
    }
    ensure_branch(connection, authority, Uuid::new_v4().to_string(), now)
}

fn ensure_branch(
    connection: &rusqlite::Connection,
    authority: &StoredLocalAuthority,
    branch: String,
    now: UnixMillis,
) -> Result<(), StorageError> {
    connection.execute(
        "INSERT INTO authorization_scopes (scope_kind, scope_id, policy_schema_version, policy_version)
         VALUES ('branch', ?1, 1, 1)", [&branch],
    ).map_err(|_| StorageError)?;
    connection
        .execute(
            "INSERT INTO local_desktop_branch (singleton, tenant_id, branch_id) VALUES (1, ?1, ?2)",
            params![authority.tenant.value().to_string(), branch],
        )
        .map_err(|_| StorageError)?;
    connection
        .execute(
            "INSERT INTO scope_relationships
         (relationship_id, scope_kind, scope_id, principal_id, principal_kind, relation)
         SELECT lower(hex(randomblob(16))), 'branch', ?1, user_id, '\"user\"',
                'eitmad.relation.organization.' || role || '.v1'
         FROM desktop_accounts WHERE tenant_id = ?2 AND active = 1",
            params![branch, authority.tenant.value().to_string()],
        )
        .map_err(|_| StorageError)?;
    let authorization = authority.authorization_context()?;
    crate::insert_audit(
        connection,
        &MutationAuditRecord {
            audit_id: Uuid::new_v4(),
            occurred_at: now,
            principal_id: authorization.identity.principal_id,
            principal_kind: PrincipalKind::User,
            session_id: authorization.session_id,
            device_id: authorization.identity.device_id,
            tenant_id: authority.tenant,
            workspace_id: None,
            scope: authorization.scope,
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: None,
            operation: "eitmad.authorization.local-branch.bootstrap.v1".to_owned(),
            target: AuditTarget {
                kind: "branch".to_owned(),
                identifiers: vec![branch.clone()],
            },
            outcome: AuditOutcome::Succeeded,
            previous_revision: None,
            resulting_revision: Some(1),
            changed_identifiers: vec![branch],
            redacted_error: None,
            extension_points: Vec::new(),
        },
    )?;
    Ok(())
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
    persist_authority(connection, &authority, now)?;
    Ok(authority)
}

fn persist_authority(
    connection: &rusqlite::Connection,
    authority: &StoredLocalAuthority,
    now: UnixMillis,
) -> Result<(), StorageError> {
    insert_identity_rows(connection, authority, now)?;

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
    Ok(())
}

fn insert_identity_rows(
    connection: &rusqlite::Connection,
    authority: &StoredLocalAuthority,
    now: UnixMillis,
) -> Result<(), StorageError> {
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
                    now.0
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
                    now.0
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
                    now.0
                ],
            )
        })
        .map(|_| ())
        .map_err(|_| StorageError)
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
    fn server_installation_rejects_replacement_and_retains_identity_after_restart() {
        let directory = tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let tenant = TenantId::new(Uuid::new_v4());
        let organization = OrganizationId::new(Uuid::new_v4());
        let device = DeviceId::new(Uuid::new_v4());
        let branch = ScopeId::new(Uuid::new_v4());
        let original = store
            .initialize_server_installation(tenant, organization, device, branch, UnixMillis(100))
            .unwrap();
        for (t, o, d, b) in [
            (TenantId::new(Uuid::new_v4()), organization, device, branch),
            (tenant, OrganizationId::new(Uuid::new_v4()), device, branch),
            (tenant, organization, DeviceId::new(Uuid::new_v4()), branch),
            (tenant, organization, device, ScopeId::new(Uuid::new_v4())),
        ] {
            assert!(
                store
                    .initialize_server_installation(t, o, d, b, UnixMillis(200))
                    .is_err()
            );
        }
        let audit_count = |store: &AuthorityStore| {
            rusqlite::Connection::open(store.path())
                .unwrap()
                .query_row("SELECT COUNT(*) FROM mutation_audit", [], |r| {
                    r.get::<_, i64>(0)
                })
                .unwrap()
        };
        assert_eq!(audit_count(&store), 2);
        drop(store);
        let reopened = AuthorityStore::open(directory.path()).unwrap();
        let recovered = reopened
            .initialize_server_installation(tenant, organization, device, branch, UnixMillis(300))
            .unwrap();
        assert_eq!(recovered.identity, original.identity);
        assert_eq!(recovered.tenant_id, original.tenant_id);
        assert_eq!(reopened.local_organization_id().unwrap(), organization);
        assert_eq!(audit_count(&reopened), 2);
    }

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
