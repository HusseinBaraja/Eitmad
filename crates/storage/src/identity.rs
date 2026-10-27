use eitmad_contracts::{
    identity::{
        AccountId, DeviceId, OrganizationId, PrincipalId, PrincipalKind, SessionId, TenantId,
        UserId, WorkspaceId,
    },
    transport::UnixMillis,
};
use rusqlite::{OptionalExtension as _, params};

use crate::{AuthorityStore, StorageError, migrations::Migration};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(
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
    use eitmad_contracts::transport::CorrelationId;
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn desktop_session_survives_reopen_and_keeps_tenant_and_audit_boundaries() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let authorization = store.local_authorization_context(UnixMillis(1)).unwrap();
        let account: String = store
            .read_transaction(|connection| {
                connection
                    .query_row(
                        "SELECT account_id FROM local_installation_authority",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|_| StorageError)
            })
            .unwrap();
        let expected = PersistentSession {
            session_id: authorization.session_id,
            principal_id: authorization.identity.principal_id,
            principal_kind: PrincipalKind::User,
            device_id: authorization.identity.device_id.unwrap(),
            user_id: UserId::new(authorization.identity.principal_id.value()),
            account_id: AccountId::new(Uuid::parse_str(&account).unwrap()),
            tenant_id: authorization.tenant_id,
            organization_id: Some(store.local_organization_id().unwrap()),
            workspace_id: None,
            issued_at: UnixMillis(100),
            expires_at: UnixMillis(1_000),
            last_seen_at: UnixMillis(100),
            connectivity: SessionConnectivity::Offline,
            closed_at: None,
        };
        store
            .persist_desktop_session(&expected, CorrelationId::new(Uuid::new_v4()))
            .unwrap();
        drop(store);

        let reopened = AuthorityStore::open(directory.path()).unwrap();
        let actual = reopened
            .read_session(expected.tenant_id, expected.session_id)
            .unwrap()
            .unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.is_locally_usable_at(UnixMillis(99)));
        assert!(actual.is_locally_usable_at(UnixMillis(500)));
        assert!(!actual.is_locally_usable_at(UnixMillis(1_000)));
        let other_tenant = TenantId::new(Uuid::new_v4());
        assert!(
            reopened
                .read_session(other_tenant, expected.session_id)
                .unwrap()
                .is_none()
        );
        let mut crossed = expected.clone();
        crossed.session_id = SessionId::new(Uuid::new_v4());
        crossed.tenant_id = other_tenant;
        assert!(
            reopened
                .persist_desktop_session(&crossed, CorrelationId::new(Uuid::new_v4()))
                .is_err()
        );

        for _ in 0..2 {
            reopened
                .close_desktop_session(
                    &authorization,
                    UnixMillis(700),
                    CorrelationId::new(Uuid::new_v4()),
                )
                .unwrap();
        }
        let closed = reopened
            .read_session(expected.tenant_id, expected.session_id)
            .unwrap()
            .unwrap();
        assert_eq!(closed.closed_at, Some(UnixMillis(700)));
        assert!(!closed.is_locally_usable_at(UnixMillis(701)));
        let audit_count: i64 = reopened.read_transaction(|connection| {
            connection.query_row("SELECT COUNT(*) FROM mutation_audit WHERE session_id = ?1 AND operation IN ('eitmad.desktop.session.sign-in.v1', 'eitmad.desktop.session.sign-out.v1')", [expected.session_id.value().to_string()], |row| row.get(0)).map_err(|_| StorageError)
        }).unwrap();
        assert_eq!(audit_count, 2);
    }
}
