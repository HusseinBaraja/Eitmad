use eitmad_contracts::{
    identity::{
        AccountId, AuthorizationContext, OrganizationId, PrincipalKind, ScopeId, ScopeKind,
        ScopeRef, TenantId, UserId,
    },
    transport::{CorrelationId, UnixMillis},
};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};
use unicode_normalization::UnicodeNormalization as _;
use uuid::Uuid;

use crate::{AuthorityStore, PersistentSession, StorageError, migrations::Migration};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(
    10,
    "identity.desktop-credentials.v1",
    "identity",
    "CREATE TABLE desktop_accounts (
         account_id TEXT PRIMARY KEY,
         user_id TEXT NOT NULL,
         tenant_id TEXT NOT NULL,
         organization_id TEXT NOT NULL,
         canonical_username TEXT NOT NULL,
         password_hash TEXT NOT NULL,
         role TEXT NOT NULL CHECK (role IN ('manager', 'receptionist')),
         active INTEGER NOT NULL CHECK (active IN (0, 1)),
         UNIQUE (tenant_id, canonical_username),
         FOREIGN KEY (account_id, user_id, tenant_id)
             REFERENCES identity_accounts(account_id, user_id, tenant_id),
         FOREIGN KEY (organization_id, tenant_id)
             REFERENCES identity_organizations(organization_id, tenant_id)
     );",
)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopRole {
    Manager,
    Receptionist,
}

impl DesktopRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::Manager => "manager",
            Self::Receptionist => "receptionist",
        }
    }

    fn relation(self) -> &'static str {
        match self {
            Self::Manager => "eitmad.relation.organization.manager.v1",
            Self::Receptionist => "eitmad.relation.organization.receptionist.v1",
        }
    }
}

/// A Rust-only credential projection. Never serialize or log this value.
#[derive(Clone)]
pub struct DesktopAccount {
    pub account_id: AccountId,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub password_hash: String,
    pub role: DesktopRole,
}

impl AuthorityStore {
    pub fn persist_desktop_session(
        &self,
        session: &PersistentSession,
        correlation_id: CorrelationId,
    ) -> Result<(), StorageError> {
        if session.principal_kind != PrincipalKind::User
            || session.principal_id.value() != session.user_id.value()
            || session.expires_at.0 <= session.issued_at.0
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL)",
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
                        i64::from(session.connectivity == crate::SessionConnectivity::Offline),
                    ],
                )
                .map_err(|_| StorageError)?;
            crate::insert_audit(
                connection,
                &desktop_session_audit(
                    session,
                    correlation_id,
                    session.issued_at,
                    "eitmad.desktop.session.sign-in.v1",
                ),
            )?;
            Ok(())
        })
    }

    pub fn close_desktop_session(
        &self,
        authorization: &AuthorizationContext,
        closed_at: UnixMillis,
        correlation_id: CorrelationId,
    ) -> Result<(), StorageError> {
        if authorization.identity.principal_kind != PrincipalKind::User
            || authorization.workspace_id.is_some()
            || authorization.scope.kind.as_str() != "organization"
            || authorization.scope.id.value() != authorization.tenant_id.value()
        {
            return Err(StorageError);
        }
        let device_id = authorization.identity.device_id.ok_or(StorageError)?;
        self.write_transaction(|connection| {
            let changed = connection
                .execute(
                    "UPDATE identity_sessions SET closed_at = ?3
                 WHERE tenant_id = ?1 AND session_id = ?2 AND closed_at IS NULL
                   AND principal_id = ?4 AND device_id = ?5 AND issued_at <= ?3",
                    params![
                        authorization.tenant_id.value().to_string(),
                        authorization.session_id.value().to_string(),
                        closed_at.0,
                        authorization.identity.principal_id.value().to_string(),
                        device_id.value().to_string(),
                    ],
                )
                .map_err(|_| StorageError)?;
            if changed == 1 {
                crate::insert_audit(
                    connection,
                    &desktop_authorization_audit(
                        authorization,
                        correlation_id,
                        closed_at,
                        "eitmad.desktop.session.sign-out.v1",
                    ),
                )?;
            }
            Ok(())
        })
    }

    /// Provisions a separate user from an already verified server account.
    /// Only the installation owner may import a credential verifier. The
    /// password hash uses the same Argon2 PHC format as the control plane.
    pub fn provision_desktop_account(
        &self,
        installer: &AuthorizationContext,
        account: &DesktopAccount,
        username: &str,
        created_at: UnixMillis,
    ) -> Result<(), StorageError> {
        let username = canonical_username(username).ok_or(StorageError)?;
        if !account.password_hash.starts_with("$argon2id$")
            || account.password_hash.len() > 1_024
            || account.user_id.value() == installer.identity.principal_id.value()
            || account.tenant_id != installer.tenant_id
            || installer.identity.principal_kind != PrincipalKind::User
        {
            return Err(StorageError);
        }
        self.write_transaction(|connection| {
            let owner: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM local_installation_authority l
                 JOIN scope_relationships r ON r.scope_kind = 'organization'
                   AND r.scope_id = l.tenant_id AND r.principal_id = l.user_id
                   AND r.relation = 'eitmad.relation.organization.owner.v1'
                 WHERE l.singleton = 1 AND l.tenant_id = ?1 AND l.user_id = ?2
                   AND l.device_id = ?3 AND l.organization_id = ?4)",
                params![
                    installer.tenant_id.value().to_string(),
                    installer.identity.principal_id.value().to_string(),
                    installer.identity.device_id.ok_or(StorageError)?.value().to_string(),
                    account.organization_id.value().to_string(),
                ],
                |row| row.get(0),
            ).map_err(|_| StorageError)?;
            if !owner { return Err(StorageError); }
            let tenant = account.tenant_id.value().to_string();
            let user = account.user_id.value().to_string();
            let account_id = account.account_id.value().to_string();
            connection.execute("INSERT OR IGNORE INTO identity_users(user_id, created_at) VALUES (?1, ?2)", params![user, created_at.0]).map_err(|_| StorageError)?;
            connection.execute("INSERT INTO identity_accounts(account_id, user_id, tenant_id, created_at)
                VALUES (?1, ?2, ?3, ?4) ON CONFLICT(account_id) DO NOTHING",
                params![account_id, user, tenant, created_at.0]).map_err(|_| StorageError)?;
            let changed = connection.execute("INSERT INTO desktop_accounts(account_id, user_id, tenant_id, organization_id, canonical_username, password_hash, role, active)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)
                ON CONFLICT(account_id) DO UPDATE SET password_hash = excluded.password_hash,
                    active = 1 WHERE desktop_accounts.user_id = excluded.user_id
                    AND desktop_accounts.tenant_id = excluded.tenant_id
                    AND desktop_accounts.organization_id = excluded.organization_id
                    AND desktop_accounts.canonical_username = excluded.canonical_username
                    AND desktop_accounts.role = excluded.role",
                params![account_id, user, tenant, account.organization_id.value().to_string(), username, account.password_hash, account.role.as_str()]
            ).map_err(|_| StorageError)?;
            if changed != 1 { return Err(StorageError); }
            connection.execute("UPDATE identity_sessions SET closed_at = ?3
                WHERE tenant_id = ?1 AND account_id = ?2 AND closed_at IS NULL",
                params![tenant, account_id, created_at.0]).map_err(|_| StorageError)?;
            let relation_changed = connection.execute("INSERT OR IGNORE INTO scope_relationships
                (relationship_id, scope_kind, scope_id, principal_id, principal_kind, relation)
                VALUES (?1, 'organization', ?2, ?3, ?4, ?5)",
                params![Uuid::new_v4().to_string(), tenant, user,
                    serde_json::to_string(&PrincipalKind::User).map_err(|_| StorageError)?, account.role.relation()]
            ).map_err(|_| StorageError)?;
            if relation_changed == 1 {
                connection.execute("UPDATE authorization_scopes SET policy_version = policy_version + 1
                    WHERE scope_kind = 'organization' AND scope_id = ?1", params![tenant])
                    .map_err(|_| StorageError)?;
            }
            let audit = MutationAuditRecord {
                audit_id: Uuid::new_v4(), occurred_at: created_at,
                principal_id: installer.identity.principal_id,
                principal_kind: installer.identity.principal_kind,
                session_id: installer.session_id,
                device_id: installer.identity.device_id,
                tenant_id: installer.tenant_id,
                workspace_id: None,
                scope: installer.scope.clone(),
                correlation_id: CorrelationId::new(Uuid::new_v4()),
                causation_id: None, idempotency_key: None,
                operation: "eitmad.desktop.account.provision.v1".to_owned(),
                target: AuditTarget { kind: "desktop-account".to_owned(), identifiers: vec![account_id] },
                outcome: AuditOutcome::Succeeded, previous_revision: None,
                resulting_revision: None, changed_identifiers: vec![user],
                redacted_error: None, extension_points: Vec::new(),
            };
            crate::insert_audit(connection, &audit)?;
            Ok(())
        })
    }

    pub fn desktop_account(
        &self,
        tenant_id: TenantId,
        username: &str,
    ) -> Result<Option<DesktopAccount>, StorageError> {
        let Some(username) = canonical_username(username) else {
            return Ok(None);
        };
        self.read_transaction(|connection| {
            connection.query_row(
                "SELECT account_id, user_id, organization_id, password_hash, role
                 FROM desktop_accounts WHERE tenant_id = ?1 AND canonical_username = ?2 AND active = 1",
                params![tenant_id.value().to_string(), username],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
            ).optional().map_err(|_| StorageError)?.map(|(account, user, organization, password_hash, role)| {
                Ok(DesktopAccount {
                    account_id: AccountId::new(Uuid::parse_str(&account).map_err(|_| StorageError)?),
                    user_id: UserId::new(Uuid::parse_str(&user).map_err(|_| StorageError)?),
                    tenant_id,
                    organization_id: OrganizationId::new(Uuid::parse_str(&organization).map_err(|_| StorageError)?),
                    password_hash,
                    role: match role.as_str() { "manager" => DesktopRole::Manager, "receptionist" => DesktopRole::Receptionist, _ => return Err(StorageError) },
                })
            }).transpose()
        })
    }

    pub fn desktop_account_active(
        &self,
        tenant_id: TenantId,
        account_id: AccountId,
        user_id: UserId,
    ) -> Result<bool, StorageError> {
        self.read_transaction(|connection| {
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM desktop_accounts WHERE tenant_id = ?1
             AND account_id = ?2 AND user_id = ?3 AND active = 1)",
                    params![
                        tenant_id.value().to_string(),
                        account_id.value().to_string(),
                        user_id.value().to_string()
                    ],
                    |row| row.get(0),
                )
                .map_err(|_| StorageError)
        })
    }
}

fn desktop_session_audit(
    session: &PersistentSession,
    correlation_id: CorrelationId,
    occurred_at: UnixMillis,
    operation: &str,
) -> MutationAuditRecord {
    MutationAuditRecord {
        audit_id: Uuid::new_v4(),
        occurred_at,
        principal_id: session.principal_id,
        principal_kind: session.principal_kind,
        session_id: session.session_id,
        device_id: Some(session.device_id),
        tenant_id: session.tenant_id,
        workspace_id: session.workspace_id,
        scope: ScopeRef {
            kind: ScopeKind::parse("organization").expect("static scope kind is valid"),
            id: ScopeId::new(session.tenant_id.value()),
        },
        correlation_id,
        causation_id: None,
        idempotency_key: None,
        operation: operation.to_owned(),
        target: AuditTarget {
            kind: "desktop-session".to_owned(),
            identifiers: vec![session.session_id.value().to_string()],
        },
        outcome: AuditOutcome::Succeeded,
        previous_revision: None,
        resulting_revision: None,
        changed_identifiers: vec![session.session_id.value().to_string()],
        redacted_error: None,
        extension_points: Vec::new(),
    }
}

fn desktop_authorization_audit(
    authorization: &AuthorizationContext,
    correlation_id: CorrelationId,
    occurred_at: UnixMillis,
    operation: &str,
) -> MutationAuditRecord {
    MutationAuditRecord {
        audit_id: Uuid::new_v4(),
        occurred_at,
        principal_id: authorization.identity.principal_id,
        principal_kind: authorization.identity.principal_kind,
        session_id: authorization.session_id,
        device_id: authorization.identity.device_id,
        tenant_id: authorization.tenant_id,
        workspace_id: authorization.workspace_id,
        scope: authorization.scope.clone(),
        correlation_id,
        causation_id: None,
        idempotency_key: None,
        operation: operation.to_owned(),
        target: AuditTarget {
            kind: "desktop-session".to_owned(),
            identifiers: vec![authorization.session_id.value().to_string()],
        },
        outcome: AuditOutcome::Succeeded,
        previous_revision: None,
        resulting_revision: None,
        changed_identifiers: vec![authorization.session_id.value().to_string()],
        redacted_error: None,
        extension_points: Vec::new(),
    }
}

fn canonical_username(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !(1..=128).contains(&trimmed.chars().count())
        || trimmed.chars().any(|ch| ch.is_control() || matches!(ch, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
    { return None; }
    Some(trimmed.nfkc().flat_map(char::to_lowercase).collect())
}
