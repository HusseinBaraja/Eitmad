//! Authorized, manual maintenance while the normal engine is stopped.

use std::path::{Path, PathBuf};

use eitmad_authorization::{AuthorizationError, AuthorizationService, STORAGE_RECOVERY_PERMISSION};
use eitmad_contracts::{
    identity::{AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind},
    runtime::EngineMode,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::AuthorityStore;
use uuid::Uuid;

use crate::{
    DesktopAuthenticationError, DesktopAuthenticator, RuntimeBuilder,
    authority::{AuthorityLock, AuthorityLockError},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryError {
    AlreadyRunning,
    Denied,
    UnsafeSource,
    Unavailable,
}

pub struct ManualRecovery {
    _authority: AuthorityLock,
    directory: PathBuf,
    store: AuthorityStore,
}

impl ManualRecovery {
    /// Acquires the same exclusive lock as normal engine startup before opening existing storage.
    ///
    /// # Errors
    /// Rejects an active engine, missing storage, or unsafe storage. Never bootstraps a database.
    pub fn acquire(directory: impl AsRef<Path>) -> Result<Self, RecoveryError> {
        let directory = directory
            .as_ref()
            .canonicalize()
            .map_err(|_| RecoveryError::Unavailable)?;
        let identity = RuntimeBuilder::new(EngineMode::Headless, &directory)
            .build()
            .snapshot()
            .identity
            .clone();
        let authority =
            AuthorityLock::acquire(&directory, &identity).map_err(|error| match error {
                AuthorityLockError::AlreadyRunning => RecoveryError::AlreadyRunning,
                AuthorityLockError::Unavailable => RecoveryError::Unavailable,
            })?;
        let store =
            AuthorityStore::open_existing(&directory).map_err(|_| RecoveryError::Unavailable)?;
        Ok(Self {
            _authority: authority,
            directory,
            store,
        })
    }

    /// Authenticates a maintenance operator against existing local desktop credentials.
    ///
    /// # Errors
    /// Returns sanitized authentication or storage failures.
    pub fn sign_in(
        &self,
        username: &str,
        password: &str,
        now: UnixMillis,
    ) -> Result<AuthorizationContext, RecoveryError> {
        let mut process = self
            .store
            .existing_local_authorization_context()
            .map_err(|_| RecoveryError::Unavailable)?;
        let device = process
            .identity
            .device_id
            .ok_or(RecoveryError::Unavailable)?;
        process.identity = AuthenticatedIdentity {
            principal_id: PrincipalId::new(device.value()),
            principal_kind: PrincipalKind::Device,
            device_id: Some(device),
            service_id: None,
        };
        DesktopAuthenticator::new(self.store.clone())
            .sign_in(
                &process,
                username,
                password,
                CorrelationId::new(Uuid::new_v4()),
                now,
            )
            .map_err(|error| match error {
                DesktopAuthenticationError::Failed => RecoveryError::Denied,
                DesktopAuthenticationError::Unavailable => RecoveryError::Unavailable,
            })?
            .authorization
            .ok_or(RecoveryError::Denied)
    }

    /// Creates a validated private backup at a new destination.
    ///
    /// # Errors
    /// Denies non-managers and returns sanitized failures without overwriting a destination.
    pub fn backup(
        &self,
        actor: &AuthorizationContext,
        destination: impl AsRef<Path>,
        now: UnixMillis,
    ) -> Result<(), RecoveryError> {
        let audit = self.authorize(actor, "eitmad.storage.backup.v1", now)?;
        self.store
            .backup_to(destination)
            .map_err(|_| self.failed(&audit))?;
        self.store
            .append_audit(&audit)
            .map_err(|_| RecoveryError::Unavailable)
    }

    /// Restores the same installation and preserves the previous database family.
    /// The lock stays held through staging, audit, replacement, and normal reopen.
    ///
    /// # Errors
    /// Denies non-managers, foreign identities, and unsafe or invalid sources.
    pub fn restore(
        &mut self,
        actor: &AuthorizationContext,
        source: impl AsRef<Path>,
        now: UnixMillis,
    ) -> Result<(), RecoveryError> {
        let audit = self.authorize(actor, "eitmad.storage.restore.v1", now)?;
        let source = source.as_ref();
        let result = (|| {
            AuthorityStore::validate_backup(source).map_err(|_| RecoveryError::UnsafeSource)?;
            if !self
                .store
                .backup_matches_authority(source)
                .map_err(|_| RecoveryError::UnsafeSource)?
            {
                return Err(RecoveryError::UnsafeSource);
            }
            let restored =
                AuthorityStore::restore_from_backup_with_audit(&self.directory, source, &audit)
                    .map_err(|_| RecoveryError::Unavailable)?;
            self.store = restored.store;
            Ok(())
        })();
        if let Err(error) = result {
            self.failed(&audit);
            return Err(error);
        }
        Ok(())
    }

    fn authorize(
        &self,
        actor: &AuthorizationContext,
        operation: &str,
        now: UnixMillis,
    ) -> Result<MutationAuditRecord, RecoveryError> {
        let audit = MutationAuditRecord::from_authorization(
            actor,
            now,
            CorrelationId::new(Uuid::new_v4()),
            operation,
            AuditTarget {
                kind: "local-authority".into(),
                identifiers: Vec::new(),
            },
        );
        let authorization = AuthorizationService::new(self.store.clone());
        let result = (|| {
            authorization.authorize(actor, STORAGE_RECOVERY_PERMISSION)?;
            for tenant in self
                .store
                .recovery_tenants()
                .map_err(|_| AuthorizationError::Unavailable)?
            {
                let mut root = actor.clone();
                root.tenant_id = tenant;
                root.scope.id = eitmad_contracts::identity::ScopeId::new(tenant.value());
                authorization.authorize(&root, STORAGE_RECOVERY_PERMISSION)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => Ok(audit),
            Err(AuthorizationError::Denied) => {
                self.store
                    .append_audit(&audit.with_outcome(
                        AuditOutcome::Denied,
                        Some("eitmad.error.authorization-denied.v1".into()),
                    ))
                    .map_err(|_| RecoveryError::Unavailable)?;
                Err(RecoveryError::Denied)
            }
            Err(_) => Err(RecoveryError::Unavailable),
        }
    }

    fn failed(&self, audit: &MutationAuditRecord) -> RecoveryError {
        let _ = self.store.append_audit(&audit.clone().with_outcome(
            AuditOutcome::Failed,
            Some("eitmad.error.storage-recovery-failed.v1".into()),
        ));
        RecoveryError::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_authorization::MutationContext;
    use eitmad_contracts::{
        authorization::RelationshipSubject,
        identity::{ScopeId, TenantId},
        transport::IdempotencyKey,
    };

    fn provision_operator(store: &AuthorityStore) {
        use argon2::{Argon2, PasswordHasher as _, password_hash::SaltString};
        use eitmad_contracts::identity::{AccountId, UserId};
        use eitmad_storage::{DesktopAccount, DesktopRole};

        let owner = store.local_authorization_context(UnixMillis(1)).unwrap();
        let salt = SaltString::encode_b64(b"synthetic-recovery").unwrap();
        let account = DesktopAccount {
            account_id: AccountId::new(Uuid::new_v4()),
            user_id: UserId::new(Uuid::new_v4()),
            tenant_id: owner.tenant_id,
            organization_id: store.local_organization_id().unwrap(),
            password_hash: Argon2::default()
                .hash_password(b"synthetic recovery password", &salt)
                .unwrap()
                .to_string(),
            role: DesktopRole::Manager,
        };
        store
            .provision_desktop_account(&owner, &account, "operator", UnixMillis(2))
            .unwrap();
    }

    #[test]
    fn recovery_sign_in_distinguishes_denial_from_account_read_and_session_write_failures() {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        provision_operator(&store);
        let recovery = ManualRecovery::acquire(directory.path()).unwrap();
        assert_eq!(
            recovery.sign_in("operator", "incorrect password", UnixMillis(3)),
            Err(RecoveryError::Denied)
        );
        let connection = rusqlite::Connection::open(store.path()).unwrap();
        connection.execute_batch("CREATE TRIGGER fail_recovery_session BEFORE INSERT ON identity_sessions BEGIN SELECT RAISE(ABORT, 'synthetic session failure'); END;").unwrap();
        assert_eq!(
            recovery.sign_in("operator", "synthetic recovery password", UnixMillis(3)),
            Err(RecoveryError::Unavailable)
        );
        connection.execute_batch("DROP TRIGGER fail_recovery_session; ALTER TABLE desktop_accounts RENAME TO unavailable_accounts;").unwrap();
        assert_eq!(
            recovery.sign_in("operator", "synthetic recovery password", UnixMillis(3)),
            Err(RecoveryError::Unavailable)
        );
    }

    #[test]
    fn maintenance_does_not_initialize_or_migrate_storage() {
        let directory = tempfile::tempdir().unwrap();
        assert!(matches!(
            ManualRecovery::acquire(directory.path()),
            Err(RecoveryError::Unavailable)
        ));
        assert!(
            !directory
                .path()
                .join(eitmad_storage::DATABASE_FILE_NAME)
                .exists()
        );
        let store = AuthorityStore::open(directory.path()).unwrap();
        let connection = rusqlite::Connection::open(store.path()).unwrap();
        connection.execute_batch("DROP TABLE work_order_confirmed_history; DELETE FROM schema_migrations WHERE version=28;").unwrap();
        drop(connection);
        let before = std::fs::read(store.path()).unwrap();
        assert!(matches!(
            ManualRecovery::acquire(directory.path()),
            Err(RecoveryError::Unavailable)
        ));
        assert!(before == std::fs::read(store.path()).unwrap());
        assert!(
            AuthorityStore::recovery_artifacts(directory.path())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn whole_database_backup_requires_permission_for_every_tenant_root() {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let first = store.local_authorization_context(UnixMillis(1)).unwrap();
        let mut foreign = first.clone();
        foreign.tenant_id = TenantId::new(Uuid::new_v4());
        foreign.scope.id = ScopeId::new(foreign.tenant_id.value());
        foreign.identity.principal_id = PrincipalId::new(Uuid::new_v4());
        let context = MutationContext {
            authorization: foreign.clone(),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            occurred_at: UnixMillis(2),
        };
        let subject = RelationshipSubject {
            principal_id: foreign.identity.principal_id,
            principal_kind: PrincipalKind::User,
        };
        let authorization = AuthorizationService::new(store.clone());
        authorization.bootstrap_owner(&context, &subject).unwrap();
        // The first owner has no relationship to the second root.
        let backup = directory.path().join("must-not-exist.sqlite3");
        let recovery = ManualRecovery::acquire(directory.path()).unwrap();
        assert_eq!(
            recovery.backup(&first, &backup, UnixMillis(3)),
            Err(RecoveryError::Denied)
        );
        assert!(!backup.exists());
    }
}
