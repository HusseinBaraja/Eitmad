//! Local user sessions anchored to imported, server-verified accounts.

use argon2::{Argon2, PasswordHash, PasswordVerifier as _};
use eitmad_contracts::{
    identity::{
        AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind, ScopeId,
        ScopeKind, ScopeRef, SessionId, UserId,
    },
    ipc::DesktopSessionState,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_storage::{AuthorityStore, PersistentSession, SessionConnectivity};
use uuid::Uuid;

const DESKTOP_SESSION_TTL_MS: i64 = 8 * 60 * 60 * 1_000;

#[derive(Clone)]
pub struct DesktopAuthenticator {
    store: AuthorityStore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopAuthenticationError {
    Failed,
    Unavailable,
}

impl DesktopAuthenticator {
    #[must_use]
    pub const fn new(store: AuthorityStore) -> Self {
        Self { store }
    }

    pub fn sign_in(
        &self,
        process: &AuthorizationContext,
        username: &str,
        password: &str,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<DesktopSessionState, DesktopAuthenticationError> {
        if !(12..=256).contains(&password.chars().count()) || password.len() > 1_024 {
            return Err(DesktopAuthenticationError::Failed);
        }
        let device_id = process
            .identity
            .device_id
            .filter(|device_id| {
                process.identity.principal_kind == PrincipalKind::Device
                    && process.identity.principal_id.value() == device_id.value()
                    && process.workspace_id.is_none()
                    && process.scope.kind.as_str() == "organization"
                    && process.scope.id.value() == process.tenant_id.value()
            })
            .ok_or(DesktopAuthenticationError::Failed)?;
        let account = self
            .store
            .desktop_account(process.tenant_id, username)
            .map_err(|_| DesktopAuthenticationError::Unavailable)?
            .ok_or(DesktopAuthenticationError::Failed)?;
        let hash = PasswordHash::new(&account.password_hash)
            .map_err(|_| DesktopAuthenticationError::Failed)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .map_err(|_| DesktopAuthenticationError::Failed)?;
        let expires_at = UnixMillis(
            now.0
                .checked_add(DESKTOP_SESSION_TTL_MS)
                .ok_or(DesktopAuthenticationError::Unavailable)?,
        );
        let session_id = SessionId::new(Uuid::new_v4());
        self.store
            .persist_desktop_session(
                &PersistentSession {
                    session_id,
                    principal_id: PrincipalId::new(account.user_id.value()),
                    principal_kind: PrincipalKind::User,
                    device_id,
                    user_id: account.user_id,
                    account_id: account.account_id,
                    tenant_id: account.tenant_id,
                    organization_id: Some(account.organization_id),
                    workspace_id: None,
                    issued_at: now,
                    expires_at,
                    last_seen_at: now,
                    connectivity: SessionConnectivity::Offline,
                    closed_at: None,
                },
                correlation_id,
            )
            .map_err(|_| DesktopAuthenticationError::Unavailable)?;
        Ok(DesktopSessionState {
            authorization: Some(AuthorizationContext {
                session_id,
                identity: AuthenticatedIdentity {
                    principal_id: PrincipalId::new(account.user_id.value()),
                    principal_kind: PrincipalKind::User,
                    device_id: Some(device_id),
                    service_id: None,
                },
                tenant_id: account.tenant_id,
                workspace_id: None,
                scope: ScopeRef {
                    kind: ScopeKind::parse("organization")
                        .map_err(|_| DesktopAuthenticationError::Unavailable)?,
                    id: ScopeId::new(account.tenant_id.value()),
                },
            }),
            expires_at: Some(expires_at),
        })
    }

    pub fn active(
        &self,
        authorization: &AuthorizationContext,
        now: UnixMillis,
    ) -> Result<bool, DesktopAuthenticationError> {
        let session = self
            .store
            .read_session(authorization.tenant_id, authorization.session_id)
            .map_err(|_| DesktopAuthenticationError::Unavailable)?;
        let Some(session) = session else {
            return Ok(false);
        };
        if !session.is_locally_usable_at(now)
            || session.principal_kind != PrincipalKind::User
            || session.principal_id != authorization.identity.principal_id
            || session.device_id
                != authorization
                    .identity
                    .device_id
                    .ok_or(DesktopAuthenticationError::Failed)?
            || session.user_id.value() != authorization.identity.principal_id.value()
            || session.tenant_id != authorization.tenant_id
            || authorization.workspace_id.is_some()
            || authorization.scope.kind.as_str() != "organization"
            || authorization.scope.id.value() != authorization.tenant_id.value()
        {
            return Ok(false);
        }
        self.store
            .desktop_account_active(
                session.tenant_id,
                session.account_id,
                UserId::new(session.user_id.value()),
            )
            .map_err(|_| DesktopAuthenticationError::Unavailable)
    }

    pub fn session_state(
        &self,
        authorization: &AuthorizationContext,
    ) -> Result<DesktopSessionState, DesktopAuthenticationError> {
        let session = self
            .store
            .read_session(authorization.tenant_id, authorization.session_id)
            .map_err(|_| DesktopAuthenticationError::Unavailable)?
            .ok_or(DesktopAuthenticationError::Failed)?;
        Ok(DesktopSessionState {
            authorization: Some(authorization.clone()),
            expires_at: Some(session.expires_at),
        })
    }

    pub fn sign_out(
        &self,
        authorization: &AuthorizationContext,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), DesktopAuthenticationError> {
        self.store
            .close_desktop_session(authorization, now, correlation_id)
            .map_err(|_| DesktopAuthenticationError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use argon2::{PasswordHasher as _, password_hash::SaltString};
    use eitmad_contracts::identity::{
        AccountId, AuthenticatedIdentity, DeviceId, PrincipalKind, UserId,
    };
    use eitmad_storage::{DesktopAccount, DesktopRole};
    use tempfile::tempdir;

    use super::*;

    fn correlation(value: u128) -> CorrelationId {
        CorrelationId::new(Uuid::from_u128(value))
    }

    fn account(
        tenant_id: eitmad_contracts::identity::TenantId,
        organization_id: eitmad_contracts::identity::OrganizationId,
        id: u128,
        role: DesktopRole,
    ) -> DesktopAccount {
        let salt = SaltString::encode_b64(&[id as u8; 16]).unwrap();
        let password_hash = Argon2::default()
            .hash_password(b"correct horse battery", &salt)
            .unwrap()
            .to_string();
        DesktopAccount {
            account_id: AccountId::new(Uuid::from_u128(id + 10)),
            user_id: UserId::new(Uuid::from_u128(id)),
            tenant_id,
            organization_id,
            password_hash,
            role,
        }
    }

    #[test]
    fn distinct_user_sessions_reject_wrong_password_expiry_and_revocation() {
        let directory = tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let owner = store.local_authorization_context(UnixMillis(100)).unwrap();
        let device_id = owner.identity.device_id.unwrap();
        let mut process = owner.clone();
        process.identity = AuthenticatedIdentity {
            principal_id: PrincipalId::new(device_id.value()),
            principal_kind: PrincipalKind::Device,
            device_id: Some(DeviceId::new(device_id.value())),
            service_id: None,
        };
        let organization = store.local_organization_id().unwrap();
        let manager = account(owner.tenant_id, organization, 201, DesktopRole::Manager);
        let receptionist = account(
            owner.tenant_id,
            organization,
            301,
            DesktopRole::Receptionist,
        );
        assert!(
            store
                .provision_desktop_account(
                    &owner,
                    &DesktopAccount {
                        user_id: UserId::new(owner.identity.principal_id.value()),
                        ..manager.clone()
                    },
                    "owner",
                    UnixMillis(100)
                )
                .is_err()
        );
        store
            .provision_desktop_account(&owner, &manager, "مدير", UnixMillis(100))
            .unwrap();
        store
            .provision_desktop_account(&owner, &receptionist, "استقبال", UnixMillis(100))
            .unwrap();
        let export = directory.path().join("tenant-export.json");
        store
            .export_tenant_data(owner.tenant_id, &export, UnixMillis(100))
            .unwrap();
        let exported = std::fs::read_to_string(export).unwrap();
        assert!(!exported.contains(&manager.password_hash));
        assert!(!exported.contains("مدير"));
        let auth = DesktopAuthenticator::new(store.clone());
        assert!(matches!(
            auth.sign_in(
                &process,
                "مدير",
                "incorrect passphrase",
                correlation(1),
                UnixMillis(101)
            ),
            Err(DesktopAuthenticationError::Failed)
        ));
        assert!(matches!(
            auth.sign_in(
                &owner,
                "مدير",
                "correct horse battery",
                correlation(1),
                UnixMillis(101)
            ),
            Err(DesktopAuthenticationError::Failed)
        ));
        let manager_session = auth
            .sign_in(
                &process,
                "مدير",
                "correct horse battery",
                correlation(2),
                UnixMillis(102),
            )
            .unwrap();
        let receptionist_session = auth
            .sign_in(
                &process,
                "استقبال",
                "correct horse battery",
                correlation(3),
                UnixMillis(103),
            )
            .unwrap();
        let manager_identity = manager_session.authorization.unwrap();
        let receptionist_identity = receptionist_session.authorization.unwrap();
        assert_ne!(
            manager_identity.identity.principal_id,
            receptionist_identity.identity.principal_id
        );
        assert_ne!(
            manager_identity.identity.principal_id,
            owner.identity.principal_id
        );
        assert_eq!(auth.active(&manager_identity, UnixMillis(104)), Ok(true));
        assert_eq!(
            auth.active(&manager_identity, manager_session.expires_at.unwrap()),
            Ok(false)
        );
        auth.sign_out(&receptionist_identity, correlation(4), UnixMillis(105))
            .unwrap();
        assert_eq!(
            auth.active(&receptionist_identity, UnixMillis(106)),
            Ok(false)
        );
        let database = rusqlite::Connection::open(store.path()).unwrap();
        let signed_in: i64 = database
            .query_row(
                "SELECT COUNT(*) FROM mutation_audit
                 WHERE operation = 'eitmad.desktop.session.sign-in.v1'
                   AND principal_id IN (?1, ?2)",
                [
                    manager.user_id.value().to_string(),
                    receptionist.user_id.value().to_string(),
                ],
                |row| row.get(0),
            )
            .unwrap();
        let signed_out: i64 = database
            .query_row(
                "SELECT COUNT(*) FROM mutation_audit
                 WHERE operation = 'eitmad.desktop.session.sign-out.v1'
                   AND principal_id = ?1",
                [receptionist.user_id.value().to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(signed_in, 2);
        assert_eq!(signed_out, 1);
    }
}
