use eitmad_contracts::{
    accounts::{DesktopAccountPage, DesktopAccountRole, DesktopAccountSummary},
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

use crate::{AuthorityStore, PersistentSession, StorageError, insert_audit, migrations::Migration};

pub(crate) const MIGRATIONS: &[Migration] = &[
    Migration::new(
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
    ),
    Migration::new(
        11,
        "identity.desktop-account-management.v1",
        "identity",
        "ALTER TABLE desktop_accounts ADD COLUMN display_name TEXT NOT NULL DEFAULT '';
         ALTER TABLE desktop_accounts ADD COLUMN revision INTEGER NOT NULL DEFAULT 1
             CHECK (revision > 0);
         UPDATE desktop_accounts SET display_name = canonical_username
             WHERE display_name = '';",
    ),
];

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

impl From<DesktopRole> for DesktopAccountRole {
    fn from(value: DesktopRole) -> Self {
        match value {
            DesktopRole::Manager => Self::Manager,
            DesktopRole::Receptionist => Self::Receptionist,
        }
    }
}

impl From<DesktopAccountRole> for DesktopRole {
    fn from(value: DesktopAccountRole) -> Self {
        match value {
            DesktopAccountRole::Manager => Self::Manager,
            DesktopAccountRole::Receptionist => Self::Receptionist,
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
    /// Removes imported access after a confirmed server denial. Retains records, caches, and authorship.
    /// # Errors
    /// Fails atomically if identity, grant, session, or audit storage is unavailable.
    pub fn revoke_connected_desktop_account(
        &self,
        actor: &AuthorizationContext,
        expected_revision: u64,
        now: UnixMillis,
    ) -> Result<bool, StorageError> {
        let observer = self.existing_local_authorization_context()?;
        self.write_transaction(|connection| {
            let tenant = actor.tenant_id.value().to_string();
            let user = actor.identity.principal_id.value().to_string();
            let changed = connection.execute("UPDATE desktop_accounts SET active=0
                WHERE tenant_id=?1 AND user_id=?2 AND active=1 AND revision=?3", params![tenant,user,i64::try_from(expected_revision).map_err(|_| StorageError)?]).map_err(|_| StorageError)?;
            if changed == 0 { return Ok(false); }
            connection.execute("UPDATE identity_sessions SET closed_at=?3 WHERE tenant_id=?1 AND user_id=?2 AND closed_at IS NULL",
                params![tenant,user,now.0]).map_err(|_| StorageError)?;
            connection.execute("DELETE FROM scope_relationships WHERE principal_id=?1 AND
                (scope_kind='organization' AND scope_id=?2 OR scope_kind='branch' AND scope_id IN
                (SELECT branch_id FROM local_desktop_branch WHERE tenant_id=?2))",params![user,tenant]).map_err(|_| StorageError)?;
            connection.execute("UPDATE authorization_scopes SET policy_version=policy_version+1 WHERE
                scope_kind='organization' AND scope_id=?1 OR scope_kind='branch' AND scope_id IN
                (SELECT branch_id FROM local_desktop_branch WHERE tenant_id=?1)",[tenant]).map_err(|_| StorageError)?;
            insert_audit(connection, &MutationAuditRecord::from_authorization(&observer, now,
                CorrelationId::new(Uuid::new_v4()), "eitmad.desktop.account.server-revoked.v1", AuditTarget {
                    kind: "desktop-account".to_owned(), identifiers: vec![user],
                }))?;
            Ok(true)
        })
    }

    /// Persists one authenticated desktop session and its audit record.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for invalid session identity or failed persistence.
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

    /// Closes an exact desktop session and records the transition.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for invalid authorization or failed persistence.
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
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for invalid input, denied import, or failed persistence.
    pub fn provision_desktop_account(
        &self,
        installer: &AuthorizationContext,
        account: &DesktopAccount,
        username: &str,
        created_at: UnixMillis,
    ) -> Result<(), StorageError> {
        let username = canonical_desktop_username(username).ok_or(StorageError)?;
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
            let changed = connection.execute("INSERT INTO desktop_accounts(account_id, user_id, tenant_id, organization_id, canonical_username, password_hash, role, active, display_name, revision)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?5, 1)
                ON CONFLICT(account_id) DO UPDATE SET password_hash = excluded.password_hash,
                    active = 1, role = excluded.role, revision = desktop_accounts.revision + 1
                    WHERE desktop_accounts.user_id = excluded.user_id
                    AND desktop_accounts.tenant_id = excluded.tenant_id
                    AND desktop_accounts.organization_id = excluded.organization_id
                    AND desktop_accounts.canonical_username = excluded.canonical_username",
                params![account_id, user, tenant, account.organization_id.value().to_string(), username, account.password_hash, account.role.as_str()]
            ).map_err(|_| StorageError)?;
            if changed != 1 { return Err(StorageError); }
            connection.execute("UPDATE identity_sessions SET closed_at = ?3
                WHERE tenant_id = ?1 AND account_id = ?2 AND closed_at IS NULL",
                params![tenant, account_id, created_at.0]).map_err(|_| StorageError)?;
            connection.execute("DELETE FROM scope_relationships WHERE principal_id=?1
                AND relation IN ('eitmad.relation.organization.manager.v1','eitmad.relation.organization.receptionist.v1')",
                [&user]).map_err(|_| StorageError)?;
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
            grant_branch_access(connection, &tenant, &user, account.role)?;
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

    /// Resolves one active account by canonical username.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for malformed durable state or a failed read.
    pub fn desktop_account(
        &self,
        tenant_id: TenantId,
        username: &str,
    ) -> Result<Option<DesktopAccount>, StorageError> {
        let Some(username) = canonical_desktop_username(username) else {
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

    /// Checks whether one exact account and user pair remains active.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error when the state cannot be read.
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

    /// Returns the one local branch only when the active user has an exact branch relation.
    ///
    /// # Errors
    /// Returns a sanitized storage error when the branch state cannot be read.
    pub fn desktop_customer_branch(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<Option<ScopeRef>, StorageError> {
        self.read_transaction(|connection| {
            let branch: Option<String> = connection
                .query_row(
                    "SELECT b.branch_id FROM local_desktop_branch b
                 JOIN scope_relationships r ON r.scope_kind = 'branch' AND r.scope_id = b.branch_id
                 JOIN desktop_accounts a ON a.user_id = r.principal_id AND a.tenant_id = b.tenant_id
                 WHERE b.singleton = 1 AND b.tenant_id = ?1 AND a.user_id = ?2 AND a.active = 1
                   AND r.principal_kind = '\"user\"'
                   AND r.relation = 'eitmad.relation.organization.' || a.role || '.v1'",
                    params![tenant_id.value().to_string(), user_id.value().to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| StorageError)?;
            branch
                .map(|value| {
                    Ok(ScopeRef {
                        kind: ScopeKind::parse("branch").map_err(|_| StorageError)?,
                        id: ScopeId::new(Uuid::parse_str(&value).map_err(|_| StorageError)?),
                    })
                })
                .transpose()
        })
    }

    /// Returns the current role for one exact active desktop account and user pair.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error when durable state is malformed or cannot be read.
    pub fn desktop_account_role(
        &self,
        tenant_id: TenantId,
        account_id: AccountId,
        user_id: UserId,
    ) -> Result<Option<DesktopAccountRole>, StorageError> {
        self.read_transaction(|connection| {
            connection
                .query_row(
                    "SELECT role FROM desktop_accounts WHERE tenant_id = ?1
                     AND account_id = ?2 AND user_id = ?3 AND active = 1",
                    params![
                        tenant_id.value().to_string(),
                        account_id.value().to_string(),
                        user_id.value().to_string()
                    ],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|_| StorageError)?
                .map(|role| match role.as_str() {
                    "manager" => Ok(DesktopAccountRole::Manager),
                    "receptionist" => Ok(DesktopAccountRole::Receptionist),
                    _ => Err(StorageError),
                })
                .transpose()
        })
    }

    /// Lists the bounded account projection for one tenant.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for malformed durable state or a failed read.
    pub fn list_desktop_accounts(
        &self,
        tenant_id: TenantId,
    ) -> Result<DesktopAccountPage, StorageError> {
        self.read_transaction(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT account_id, user_id, display_name, canonical_username, role, active, revision
                     FROM desktop_accounts WHERE tenant_id = ?1
                     ORDER BY display_name, canonical_username LIMIT 501",
                )
                .map_err(|_| StorageError)?;
            let rows = statement
                .query_map([tenant_id.value().to_string()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, bool>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                })
                .map_err(|_| StorageError)?;
            let accounts = rows
                .map(|row| decode_account_summary(row.map_err(|_| StorageError)?))
                .collect::<Result<Vec<_>, _>>()?;
            if accounts.len() > 500 {
                return Err(StorageError);
            }
            Ok(DesktopAccountPage { accounts })
        })
    }
}

fn grant_branch_access(
    connection: &rusqlite::Connection,
    tenant: &str,
    user: &str,
    role: DesktopRole,
) -> Result<(), StorageError> {
    let branch: String = connection
        .query_row(
            "SELECT branch_id FROM local_desktop_branch WHERE singleton = 1 AND tenant_id = ?1",
            [tenant],
            |row| row.get(0),
        )
        .map_err(|_| StorageError)?;
    connection
        .execute(
            "INSERT OR IGNORE INTO scope_relationships
         (relationship_id, scope_kind, scope_id, principal_id, principal_kind, relation)
         VALUES (?1, 'branch', ?2, ?3, ?4, ?5)",
            params![
                Uuid::new_v4().to_string(),
                branch,
                user,
                serde_json::to_string(&PrincipalKind::User).map_err(|_| StorageError)?,
                role.relation()
            ],
        )
        .map_err(|_| StorageError)?;
    connection
        .execute(
            "UPDATE authorization_scopes SET policy_version = policy_version + 1
         WHERE scope_kind = 'branch' AND scope_id = ?1",
            [branch],
        )
        .map_err(|_| StorageError)?;
    Ok(())
}

fn decode_account_summary(
    row: (String, String, String, String, String, bool, i64),
) -> Result<DesktopAccountSummary, StorageError> {
    Ok(DesktopAccountSummary {
        account_id: AccountId::new(Uuid::parse_str(&row.0).map_err(|_| StorageError)?),
        user_id: UserId::new(Uuid::parse_str(&row.1).map_err(|_| StorageError)?),
        display_name: row.2,
        username: row.3,
        role: Some(match row.4.as_str() {
            "manager" => DesktopAccountRole::Manager,
            "receptionist" => DesktopAccountRole::Receptionist,
            _ => return Err(StorageError),
        }),
        active: row.5,
        revision: u64::try_from(row.6).map_err(|_| StorageError)?,
    })
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

#[must_use]
pub fn canonical_desktop_username(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !(1..=128).contains(&trimmed.chars().count())
        || trimmed.chars().any(|ch| ch.is_control() || matches!(ch, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
    { return None; }
    Some(trimmed.nfkc().flat_map(char::to_lowercase).collect())
}
