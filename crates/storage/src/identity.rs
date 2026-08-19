use eitmad_contracts::{
    identity::{
        AccountId, DeviceId, OrganizationId, PrincipalId, PrincipalKind, SessionId, TenantId,
        UserId, WorkspaceId,
    },
    transport::UnixMillis,
};
use rusqlite::{OptionalExtension as _, params};

use crate::{AuthorityStore, StorageError, migrations::Migration};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::additive(
    5,
    "identity.foundation.v1",
    "identity",
    "CREATE TABLE identity_devices (
         device_id TEXT PRIMARY KEY,
         created_at INTEGER NOT NULL,
         last_seen_at INTEGER NOT NULL
     );
     CREATE TABLE identity_users (
         user_id TEXT PRIMARY KEY,
         created_at INTEGER NOT NULL
     );
     CREATE TABLE identity_tenants (
         tenant_id TEXT PRIMARY KEY,
         created_at INTEGER NOT NULL
     );
     CREATE TABLE identity_accounts (
         account_id TEXT PRIMARY KEY,
         user_id TEXT NOT NULL,
         tenant_id TEXT NOT NULL,
         created_at INTEGER NOT NULL,
         UNIQUE (account_id, user_id, tenant_id),
         FOREIGN KEY (user_id) REFERENCES identity_users(user_id),
         FOREIGN KEY (tenant_id) REFERENCES identity_tenants(tenant_id)
     );
     CREATE TABLE identity_organizations (
         organization_id TEXT PRIMARY KEY,
         tenant_id TEXT NOT NULL,
         created_at INTEGER NOT NULL,
         UNIQUE (organization_id, tenant_id),
         FOREIGN KEY (tenant_id) REFERENCES identity_tenants(tenant_id)
     );
     CREATE TABLE identity_workspaces (
         workspace_id TEXT PRIMARY KEY,
         tenant_id TEXT NOT NULL,
         organization_id TEXT,
         created_at INTEGER NOT NULL,
         UNIQUE (workspace_id, tenant_id),
         FOREIGN KEY (tenant_id) REFERENCES identity_tenants(tenant_id),
         FOREIGN KEY (organization_id, tenant_id)
             REFERENCES identity_organizations(organization_id, tenant_id)
     );
     CREATE TABLE identity_sessions (
         session_id TEXT PRIMARY KEY,
         principal_id TEXT NOT NULL,
         principal_kind TEXT NOT NULL,
         device_id TEXT NOT NULL,
         user_id TEXT NOT NULL,
         account_id TEXT NOT NULL,
         tenant_id TEXT NOT NULL,
         organization_id TEXT,
         workspace_id TEXT,
         issued_at INTEGER NOT NULL,
         expires_at INTEGER NOT NULL,
         last_seen_at INTEGER NOT NULL,
         offline INTEGER NOT NULL CHECK (offline IN (0, 1)),
         closed_at INTEGER,
         FOREIGN KEY (device_id) REFERENCES identity_devices(device_id),
         FOREIGN KEY (account_id, user_id, tenant_id)
             REFERENCES identity_accounts(account_id, user_id, tenant_id),
         FOREIGN KEY (organization_id, tenant_id)
             REFERENCES identity_organizations(organization_id, tenant_id),
         FOREIGN KEY (workspace_id, tenant_id)
             REFERENCES identity_workspaces(workspace_id, tenant_id),
         CHECK (expires_at > issued_at),
         CHECK (last_seen_at >= issued_at)
     );
     ALTER TABLE mutation_audit ADD COLUMN session_id TEXT;
     ALTER TABLE mutation_audit ADD COLUMN device_id TEXT;",
)];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub device_id: DeviceId,
    pub created_at: UnixMillis,
    pub last_seen_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityTopology {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub account_id: AccountId,
    pub organization_id: Option<OrganizationId>,
    pub workspace_id: Option<WorkspaceId>,
    pub created_at: UnixMillis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionConnectivity {
    Online,
    Offline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentSession {
    pub session_id: SessionId,
    pub principal_id: PrincipalId,
    pub principal_kind: PrincipalKind,
    pub device_id: DeviceId,
    pub user_id: UserId,
    pub account_id: AccountId,
    pub tenant_id: TenantId,
    pub organization_id: Option<OrganizationId>,
    pub workspace_id: Option<WorkspaceId>,
    pub issued_at: UnixMillis,
    pub expires_at: UnixMillis,
    pub last_seen_at: UnixMillis,
    pub connectivity: SessionConnectivity,
    pub closed_at: Option<UnixMillis>,
}

impl PersistentSession {
    #[must_use]
    pub fn is_locally_usable_at(&self, now: UnixMillis) -> bool {
        self.closed_at.is_none() && now.0 >= self.issued_at.0 && now.0 < self.expires_at.0
    }
}

impl AuthorityStore {
    /// Creates or refreshes one stable device identity.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for invalid timestamps or failed persistence.
    pub fn persist_device(&self, device: &DeviceIdentity) -> Result<(), StorageError> {
        if device.last_seen_at.0 < device.created_at.0 {
            return Err(StorageError);
        }
        self.write_transaction(|connection| {
            connection
                .execute(
                    "INSERT INTO identity_devices(device_id, created_at, last_seen_at)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(device_id) DO UPDATE SET
                       last_seen_at = MAX(last_seen_at, excluded.last_seen_at)",
                    params![
                        device.device_id.value().to_string(),
                        device.created_at.0,
                        device.last_seen_at.0
                    ],
                )
                .map_err(|_| StorageError)?;
            Ok(())
        })
    }

    /// Persists a tenant-scoped user/account and optional organization/workspace atomically.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for conflicting or cross-tenant topology.
    pub fn persist_identity_topology(
        &self,
        identity: &IdentityTopology,
    ) -> Result<(), StorageError> {
        self.write_transaction(|connection| {
            let tenant = identity.tenant_id.value().to_string();
            connection
                .execute(
                    "INSERT OR IGNORE INTO identity_tenants(tenant_id, created_at) VALUES (?1, ?2)",
                    params![tenant, identity.created_at.0],
                )
                .map_err(|_| StorageError)?;
            connection
                .execute(
                    "INSERT OR IGNORE INTO identity_users(user_id, created_at) VALUES (?1, ?2)",
                    params![identity.user_id.value().to_string(), identity.created_at.0],
                )
                .map_err(|_| StorageError)?;
            connection
                .execute(
                    "INSERT INTO identity_accounts(account_id, user_id, tenant_id, created_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(account_id) DO NOTHING",
                    params![
                        identity.account_id.value().to_string(),
                        identity.user_id.value().to_string(),
                        tenant,
                        identity.created_at.0
                    ],
                )
                .map_err(|_| StorageError)?;
            if let Some(organization_id) = identity.organization_id {
                connection
                    .execute(
                        "INSERT INTO identity_organizations
                         (organization_id, tenant_id, created_at) VALUES (?1, ?2, ?3)
                         ON CONFLICT(organization_id) DO NOTHING",
                        params![
                            organization_id.value().to_string(),
                            tenant,
                            identity.created_at.0
                        ],
                    )
                    .map_err(|_| StorageError)?;
            }
            if let Some(workspace_id) = identity.workspace_id {
                connection
                    .execute(
                        "INSERT INTO identity_workspaces
                         (workspace_id, tenant_id, organization_id, created_at)
                         VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT(workspace_id) DO NOTHING",
                        params![
                            workspace_id.value().to_string(),
                            tenant,
                            identity.organization_id.map(|id| id.value().to_string()),
                            identity.created_at.0
                        ],
                    )
                    .map_err(|_| StorageError)?;
            }
            verify_topology(connection, identity)
        })
    }

    /// Opens a durable session after every referenced identity is present in one tenant.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for invalid time bounds, missing identity,
    /// cross-tenant references, duplicate sessions, or failed persistence.
    pub fn persist_session(&self, session: &PersistentSession) -> Result<(), StorageError> {
        if session.expires_at.0 <= session.issued_at.0
            || session.last_seen_at.0 < session.issued_at.0
        {
            return Err(StorageError);
        }
        self.write_transaction(|connection| {
            connection
                .execute(
                    "INSERT INTO identity_sessions
                     (session_id, principal_id, principal_kind, device_id, user_id, account_id,
                      tenant_id, organization_id, workspace_id, issued_at, expires_at,
                      last_seen_at, offline, closed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        session.session_id.value().to_string(),
                        session.principal_id.value().to_string(),
                        serde_json::to_string(&session.principal_kind).map_err(|_| StorageError)?,
                        session.device_id.value().to_string(),
                        session.user_id.value().to_string(),
                        session.account_id.value().to_string(),
                        session.tenant_id.value().to_string(),
                        session.organization_id.map(|id| id.value().to_string()),
                        session.workspace_id.map(|id| id.value().to_string()),
                        session.issued_at.0,
                        session.expires_at.0,
                        session.last_seen_at.0,
                        i64::from(session.connectivity == SessionConnectivity::Offline),
                        session.closed_at.map(|value| value.0),
                    ],
                )
                .map_err(|_| StorageError)?;
            Ok(())
        })
    }

    /// Reads one session only through its tenant isolation key.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for unreadable or malformed session state.
    pub fn read_session(
        &self,
        tenant_id: TenantId,
        session_id: SessionId,
    ) -> Result<Option<PersistentSession>, StorageError> {
        self.read_transaction(|connection| {
            connection
                .query_row(
                    "SELECT principal_id, principal_kind, device_id, user_id, account_id,
                            organization_id, workspace_id, issued_at, expires_at, last_seen_at,
                            offline, closed_at
                     FROM identity_sessions WHERE tenant_id = ?1 AND session_id = ?2",
                    params![
                        tenant_id.value().to_string(),
                        session_id.value().to_string()
                    ],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, i64>(7)?,
                            row.get::<_, i64>(8)?,
                            row.get::<_, i64>(9)?,
                            row.get::<_, i64>(10)?,
                            row.get::<_, Option<i64>>(11)?,
                        ))
                    },
                )
                .optional()
                .map_err(|_| StorageError)?
                .map(|row| decode_session(tenant_id, session_id, &row))
                .transpose()
        })
    }

    /// Refreshes an active session and records whether the engine is currently offline.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error when the scoped update cannot complete.
    pub fn refresh_session(
        &self,
        tenant_id: TenantId,
        session_id: SessionId,
        seen_at: UnixMillis,
        connectivity: SessionConnectivity,
    ) -> Result<bool, StorageError> {
        self.write_transaction(|connection| {
            let changed = connection
                .execute(
                    "UPDATE identity_sessions SET last_seen_at = ?3, offline = ?4
                     WHERE tenant_id = ?1 AND session_id = ?2 AND closed_at IS NULL
                       AND last_seen_at <= ?3 AND ?3 < expires_at",
                    params![
                        tenant_id.value().to_string(),
                        session_id.value().to_string(),
                        seen_at.0,
                        i64::from(connectivity == SessionConnectivity::Offline)
                    ],
                )
                .map_err(|_| StorageError)?;
            Ok(changed == 1)
        })
    }

    /// Closes one tenant-scoped session. Repeated closure is a no-op.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error when the scoped update cannot complete.
    pub fn close_session(
        &self,
        tenant_id: TenantId,
        session_id: SessionId,
        closed_at: UnixMillis,
    ) -> Result<bool, StorageError> {
        self.write_transaction(|connection| {
            let changed = connection
                .execute(
                    "UPDATE identity_sessions SET closed_at = ?3
                     WHERE tenant_id = ?1 AND session_id = ?2 AND closed_at IS NULL
                       AND issued_at <= ?3",
                    params![
                        tenant_id.value().to_string(),
                        session_id.value().to_string(),
                        closed_at.0
                    ],
                )
                .map_err(|_| StorageError)?;
            Ok(changed == 1)
        })
    }
}

fn verify_topology(
    connection: &rusqlite::Connection,
    identity: &IdentityTopology,
) -> Result<(), StorageError> {
    let stored = connection
        .query_row(
            "SELECT user_id, tenant_id FROM identity_accounts WHERE account_id = ?1",
            [identity.account_id.value().to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|_| StorageError)?;
    if stored
        != (
            identity.user_id.value().to_string(),
            identity.tenant_id.value().to_string(),
        )
    {
        return Err(StorageError);
    }
    if let Some(organization_id) = identity.organization_id {
        let count = connection
            .query_row(
                "SELECT COUNT(*) FROM identity_organizations
                 WHERE organization_id = ?1 AND tenant_id = ?2",
                params![
                    organization_id.value().to_string(),
                    identity.tenant_id.value().to_string()
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| StorageError)?;
        if count != 1 {
            return Err(StorageError);
        }
    }
    if let Some(workspace_id) = identity.workspace_id {
        let count = connection
            .query_row(
                "SELECT COUNT(*) FROM identity_workspaces
                 WHERE workspace_id = ?1 AND tenant_id = ?2",
                params![
                    workspace_id.value().to_string(),
                    identity.tenant_id.value().to_string()
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| StorageError)?;
        if count != 1 {
            return Err(StorageError);
        }
    }
    Ok(())
}

type StoredSessionRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    i64,
    i64,
    i64,
    i64,
    Option<i64>,
);

fn decode_session(
    tenant_id: TenantId,
    session_id: SessionId,
    row: &StoredSessionRow,
) -> Result<PersistentSession, StorageError> {
    let parse = |value: &str| uuid::Uuid::parse_str(value).map_err(|_| StorageError);
    Ok(PersistentSession {
        session_id,
        principal_id: PrincipalId::new(parse(&row.0)?),
        principal_kind: serde_json::from_str(&row.1).map_err(|_| StorageError)?,
        device_id: DeviceId::new(parse(&row.2)?),
        user_id: UserId::new(parse(&row.3)?),
        account_id: AccountId::new(parse(&row.4)?),
        tenant_id,
        organization_id: row
            .5
            .as_deref()
            .map(parse)
            .transpose()?
            .map(OrganizationId::new),
        workspace_id: row
            .6
            .as_deref()
            .map(parse)
            .transpose()?
            .map(WorkspaceId::new),
        issued_at: UnixMillis(row.7),
        expires_at: UnixMillis(row.8),
        last_seen_at: UnixMillis(row.9),
        connectivity: match row.10 {
            0 => SessionConnectivity::Online,
            1 => SessionConnectivity::Offline,
            _ => return Err(StorageError),
        },
        closed_at: row.11.map(UnixMillis),
    })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn topology(tenant: u128) -> IdentityTopology {
        IdentityTopology {
            tenant_id: TenantId::new(id(tenant)),
            user_id: UserId::new(id(10)),
            account_id: AccountId::new(id(11 + tenant)),
            organization_id: Some(OrganizationId::new(id(12 + tenant))),
            workspace_id: Some(WorkspaceId::new(id(13 + tenant))),
            created_at: UnixMillis(1),
        }
    }

    fn session(topology: &IdentityTopology) -> PersistentSession {
        PersistentSession {
            session_id: SessionId::new(id(20)),
            principal_id: PrincipalId::new(id(21)),
            principal_kind: PrincipalKind::User,
            device_id: DeviceId::new(id(22)),
            user_id: topology.user_id,
            account_id: topology.account_id,
            tenant_id: topology.tenant_id,
            organization_id: topology.organization_id,
            workspace_id: topology.workspace_id,
            issued_at: UnixMillis(100),
            expires_at: UnixMillis(1_000),
            last_seen_at: UnixMillis(100),
            connectivity: SessionConnectivity::Offline,
            closed_at: None,
        }
    }

    #[test]
    fn device_identity_and_offline_session_survive_reopen() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let topology = topology(1);
        store.persist_identity_topology(&topology).unwrap();
        store
            .persist_device(&DeviceIdentity {
                device_id: DeviceId::new(id(22)),
                created_at: UnixMillis(1),
                last_seen_at: UnixMillis(100),
            })
            .unwrap();
        let expected = session(&topology);
        store.persist_session(&expected).unwrap();
        drop(store);

        let reopened = AuthorityStore::open(directory.path()).unwrap();
        let actual = reopened
            .read_session(topology.tenant_id, expected.session_id)
            .unwrap()
            .unwrap();
        assert_eq!(actual, expected);
        assert!(actual.is_locally_usable_at(UnixMillis(500)));
        assert!(!actual.is_locally_usable_at(UnixMillis(1_000)));
        assert!(
            reopened
                .refresh_session(
                    topology.tenant_id,
                    expected.session_id,
                    UnixMillis(600),
                    SessionConnectivity::Online,
                )
                .unwrap()
        );
        assert_eq!(
            reopened
                .read_session(topology.tenant_id, expected.session_id)
                .unwrap()
                .unwrap()
                .connectivity,
            SessionConnectivity::Online
        );
        assert!(
            reopened
                .close_session(topology.tenant_id, expected.session_id, UnixMillis(700))
                .unwrap()
        );
        let closed = reopened
            .read_session(topology.tenant_id, expected.session_id)
            .unwrap()
            .unwrap();
        assert!(!closed.is_locally_usable_at(UnixMillis(701)));
        assert!(
            !reopened
                .refresh_session(
                    topology.tenant_id,
                    expected.session_id,
                    UnixMillis(702),
                    SessionConnectivity::Offline,
                )
                .unwrap()
        );
    }

    #[test]
    fn session_lookup_and_foreign_keys_enforce_tenant_isolation() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let first = topology(1);
        let second = topology(2);
        store.persist_identity_topology(&first).unwrap();
        store.persist_identity_topology(&second).unwrap();
        store
            .persist_device(&DeviceIdentity {
                device_id: DeviceId::new(id(22)),
                created_at: UnixMillis(1),
                last_seen_at: UnixMillis(100),
            })
            .unwrap();
        let session = session(&first);
        store.persist_session(&session).unwrap();

        assert!(
            store
                .read_session(second.tenant_id, session.session_id)
                .unwrap()
                .is_none()
        );
        let mut crossed = session.clone();
        crossed.session_id = SessionId::new(id(30));
        crossed.tenant_id = second.tenant_id;
        assert!(store.persist_session(&crossed).is_err());
    }
}
