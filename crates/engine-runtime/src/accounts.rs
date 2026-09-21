//! Manager-only desktop account lifecycle authority.

use argon2::{Argon2, PasswordHasher as _, password_hash::SaltString};
use eitmad_authorization::{
    AuthorizationError, AuthorizationService, DESKTOP_ACCOUNTS_MANAGE_PERMISSION, MutationContext,
};
use eitmad_contracts::{
    accounts::{
        CreateDesktopAccount, DeactivateDesktopAccount, DesktopAccountPage, DesktopAccountSummary,
        ListDesktopAccounts, UpdateDesktopAccount,
    },
    identity::AuthorizationContext,
};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, DesktopAccountCommitOutcome, DesktopAccountMutation, DurableIdempotency,
    canonical_desktop_username,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

const CREATE_OPERATION: &str = "eitmad.desktop-account.create.v1";
const UPDATE_OPERATION: &str = "eitmad.desktop-account.update.v1";
const DEACTIVATE_OPERATION: &str = "eitmad.desktop-account.deactivate.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopAccountError {
    Denied,
    Invalid,
    RevisionConflict { expected: u64, actual: u64 },
    LastUsableManager,
    IdempotencyMismatch,
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct DesktopAccountService {
    store: AuthorityStore,
    authorization: AuthorizationService,
}

impl DesktopAccountService {
    #[must_use]
    pub const fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            store,
            authorization,
        }
    }

    /// Lists the desktop accounts visible to the authorized manager.
    ///
    /// # Errors
    ///
    /// Returns an authorization error for non-managers or an availability error when storage
    /// cannot be read.
    pub fn list(
        &self,
        authorization: &AuthorizationContext,
        _query: &ListDesktopAccounts,
    ) -> Result<DesktopAccountPage, DesktopAccountError> {
        self.authorize(authorization)?;
        self.store
            .list_desktop_accounts(authorization.tenant_id)
            .map_err(|_| DesktopAccountError::Unavailable)
    }

    /// Creates an active desktop account with a locally usable credential.
    ///
    /// # Errors
    ///
    /// Returns an authorization, validation, idempotency, or availability error when the mutation
    /// cannot be committed.
    pub fn create(
        &self,
        context: &MutationContext,
        command: &CreateDesktopAccount,
    ) -> Result<DesktopAccountSummary, DesktopAccountError> {
        self.authorize_mutation(context, CREATE_OPERATION, None)?;
        let display_name = validate_display_name(&command.display_name)
            .ok_or_else(|| self.invalid(context, CREATE_OPERATION, None))?;
        let username = canonical_desktop_username(&command.username)
            .ok_or_else(|| self.invalid(context, CREATE_OPERATION, None))?;
        validate_password(command.password.expose_secret())
            .then_some(())
            .ok_or_else(|| self.invalid(context, CREATE_OPERATION, None))?;
        let salt_uuid = Uuid::new_v4();
        let salt = SaltString::encode_b64(salt_uuid.as_bytes())
            .map_err(|_| DesktopAccountError::Unavailable)?;
        let password_hash = Argon2::default()
            .hash_password(command.password.expose_secret().as_bytes(), &salt)
            .map_err(|_| DesktopAccountError::Unavailable)?
            .to_string();
        let summary = DesktopAccountSummary {
            account_id: eitmad_contracts::identity::AccountId::new(Uuid::new_v4()),
            user_id: eitmad_contracts::identity::UserId::new(Uuid::new_v4()),
            display_name,
            username,
            role: command.role,
            active: true,
            revision: 1,
        };
        self.commit(
            context,
            CREATE_OPERATION,
            command,
            &summary,
            Some(&password_hash),
            None,
        )
    }

    /// Updates the display name and role of an existing desktop account.
    ///
    /// # Errors
    ///
    /// Returns an authorization, validation, revision, last-manager, idempotency, or availability
    /// error when the mutation cannot be committed.
    pub fn update(
        &self,
        context: &MutationContext,
        command: &UpdateDesktopAccount,
    ) -> Result<DesktopAccountSummary, DesktopAccountError> {
        self.authorize_mutation(context, UPDATE_OPERATION, Some(command.account_id.value()))?;
        let display_name = validate_display_name(&command.display_name).ok_or_else(|| {
            self.invalid(context, UPDATE_OPERATION, Some(command.account_id.value()))
        })?;
        let current = self
            .find(context.authorization.tenant_id, command.account_id)
            .map_err(|error| match error {
                DesktopAccountError::Invalid => {
                    self.invalid(context, UPDATE_OPERATION, Some(command.account_id.value()))
                }
                other => other,
            })?;
        let summary = DesktopAccountSummary {
            display_name,
            role: command.role,
            revision: command
                .expected_revision
                .checked_add(1)
                .ok_or(DesktopAccountError::Unavailable)?,
            ..current
        };
        self.commit(
            context,
            UPDATE_OPERATION,
            command,
            &summary,
            None,
            Some(command.expected_revision),
        )
    }

    /// Deactivates a desktop account and revokes its active sessions.
    ///
    /// # Errors
    ///
    /// Returns an authorization, validation, revision, last-manager, idempotency, or availability
    /// error when the mutation cannot be committed.
    pub fn deactivate(
        &self,
        context: &MutationContext,
        command: &DeactivateDesktopAccount,
    ) -> Result<DesktopAccountSummary, DesktopAccountError> {
        self.authorize_mutation(
            context,
            DEACTIVATE_OPERATION,
            Some(command.account_id.value()),
        )?;
        let current = self
            .find(context.authorization.tenant_id, command.account_id)
            .map_err(|error| match error {
                DesktopAccountError::Invalid => self.invalid(
                    context,
                    DEACTIVATE_OPERATION,
                    Some(command.account_id.value()),
                ),
                other => other,
            })?;
        if !current.active {
            return Err(self.invalid(
                context,
                DEACTIVATE_OPERATION,
                Some(command.account_id.value()),
            ));
        }
        let summary = DesktopAccountSummary {
            active: false,
            revision: command
                .expected_revision
                .checked_add(1)
                .ok_or(DesktopAccountError::Unavailable)?,
            ..current
        };
        self.commit(
            context,
            DEACTIVATE_OPERATION,
            command,
            &summary,
            None,
            Some(command.expected_revision),
        )
    }

    fn find(
        &self,
        tenant_id: eitmad_contracts::identity::TenantId,
        account_id: eitmad_contracts::identity::AccountId,
    ) -> Result<DesktopAccountSummary, DesktopAccountError> {
        self.store
            .list_desktop_accounts(tenant_id)
            .map_err(|_| DesktopAccountError::Unavailable)?
            .accounts
            .into_iter()
            .find(|account| account.account_id == account_id)
            .ok_or(DesktopAccountError::Invalid)
    }

    fn commit<T: Serialize>(
        &self,
        context: &MutationContext,
        operation: &'static str,
        command: &T,
        account: &DesktopAccountSummary,
        password_hash: Option<&str>,
        expected_revision: Option<u64>,
    ) -> Result<DesktopAccountSummary, DesktopAccountError> {
        let response_json =
            serde_json::to_vec(&account).map_err(|_| DesktopAccountError::Unavailable)?;
        let encoded = serde_json::to_vec(&(operation, command))
            .map_err(|_| DesktopAccountError::Unavailable)?;
        let idempotency = DurableIdempotency {
            key: context.idempotency_key,
            request_hash: Sha256::digest(encoded).into(),
            response_json,
        };
        let audit = account_audit(context, operation, Some(account.account_id.value()));
        match self.store.commit_desktop_account(
            context.authorization.tenant_id,
            &DesktopAccountMutation {
                account,
                password_hash,
                expected_revision,
                operation,
                idempotency: &idempotency,
                audit: &audit,
            },
        ) {
            Ok(DesktopAccountCommitOutcome::Committed(account)) => Ok(account),
            Ok(DesktopAccountCommitOutcome::Replayed(response)) => {
                serde_json::from_slice(&response).map_err(|_| DesktopAccountError::Unavailable)
            }
            Ok(DesktopAccountCommitOutcome::RevisionConflict {
                actual_revision: Some(actual),
            }) => Err(DesktopAccountError::RevisionConflict {
                expected: expected_revision.unwrap_or_default(),
                actual,
            }),
            Ok(DesktopAccountCommitOutcome::LastUsableManager) => {
                Err(DesktopAccountError::LastUsableManager)
            }
            Ok(DesktopAccountCommitOutcome::IdempotencyMismatch) => {
                Err(DesktopAccountError::IdempotencyMismatch)
            }
            Ok(
                DesktopAccountCommitOutcome::DuplicateUsername
                | DesktopAccountCommitOutcome::NotFound
                | DesktopAccountCommitOutcome::RevisionConflict {
                    actual_revision: None,
                },
            ) => Err(DesktopAccountError::Invalid),
            Err(_) => Err(DesktopAccountError::Unavailable),
        }
    }

    fn authorize(&self, authorization: &AuthorizationContext) -> Result<(), DesktopAccountError> {
        self.authorization
            .authorize(authorization, DESKTOP_ACCOUNTS_MANAGE_PERMISSION)
            .map_err(|error| match error {
                AuthorizationError::Denied => DesktopAccountError::Denied,
                _ => DesktopAccountError::Unavailable,
            })
    }

    fn authorize_mutation(
        &self,
        context: &MutationContext,
        operation: &str,
        target: Option<Uuid>,
    ) -> Result<(), DesktopAccountError> {
        match self.authorize(&context.authorization) {
            Ok(()) => Ok(()),
            Err(DesktopAccountError::Denied) => {
                self.store
                    .append_audit(&account_audit(context, operation, target).with_outcome(
                        AuditOutcome::Denied,
                        Some("eitmad.error.authorization-denied.v1".to_owned()),
                    ))
                    .map_err(|_| DesktopAccountError::Unavailable)?;
                Err(DesktopAccountError::Denied)
            }
            Err(error) => Err(error),
        }
    }

    fn invalid(
        &self,
        context: &MutationContext,
        operation: &str,
        target: Option<Uuid>,
    ) -> DesktopAccountError {
        if self
            .store
            .append_audit(&account_audit(context, operation, target).with_outcome(
                AuditOutcome::Invalid,
                Some("eitmad.error.desktop-account-invalid.v1".to_owned()),
            ))
            .is_err()
        {
            DesktopAccountError::Unavailable
        } else {
            DesktopAccountError::Invalid
        }
    }
}

fn account_audit(
    context: &MutationContext,
    operation: &str,
    target: Option<Uuid>,
) -> MutationAuditRecord {
    let mut record = MutationAuditRecord::from_authorization(
        &context.authorization,
        context.occurred_at,
        context.correlation_id,
        operation,
        AuditTarget {
            kind: "desktop-account".to_owned(),
            identifiers: target.into_iter().map(|id| id.to_string()).collect(),
        },
    );
    record.causation_id = context.causation_id;
    record.idempotency_key = Some(context.idempotency_key);
    record.changed_identifiers = vec![
        "display-name".to_owned(),
        "role".to_owned(),
        "active".to_owned(),
    ];
    record
}

fn validate_display_name(value: &str) -> Option<String> {
    let value = value.trim();
    ((1..=128).contains(&value.chars().count())
        && !value.chars().any(|character| character.is_control()
            || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')))
        .then(|| value.to_owned())
}

fn validate_password(value: &str) -> bool {
    (12..=256).contains(&value.chars().count()) && value.len() <= 1_024
}

#[cfg(test)]
mod tests {
    use argon2::password_hash::SaltString;
    use eitmad_contracts::{
        accounts::{AccountPassword, DesktopAccountRole},
        identity::{
            AccountId, AuthenticatedIdentity, DeviceId, PrincipalId, PrincipalKind, SessionId,
            UserId,
        },
        transport::{CorrelationId, IdempotencyKey, UnixMillis},
    };
    use eitmad_storage::{DesktopAccount, DesktopRole};
    use tempfile::tempdir;

    use crate::DesktopAuthenticator;

    use super::*;

    fn mutation(authorization: AuthorizationContext, value: u128) -> MutationContext {
        MutationContext {
            authorization,
            correlation_id: CorrelationId::new(Uuid::from_u128(value + 100)),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(value + 200)),
            occurred_at: UnixMillis(i64::try_from(value).unwrap() + 1_000),
        }
    }

    fn hash(password: &str, value: u8) -> String {
        let salt = SaltString::encode_b64(&[value; 16]).unwrap();
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    fn user_context(
        owner: &AuthorizationContext,
        user_id: UserId,
        value: u128,
    ) -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(value)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(user_id.value()),
                principal_kind: PrincipalKind::User,
                device_id: owner.identity.device_id,
                service_id: None,
            },
            tenant_id: owner.tenant_id,
            workspace_id: None,
            scope: owner.scope.clone(),
        }
    }

    fn assert_receptionist_session_lifecycle(
        store: &AuthorityStore,
        owner: &AuthorizationContext,
        manager_context: &AuthorizationContext,
        service: &DesktopAccountService,
        receptionist: &DesktopAccountSummary,
    ) {
        let mut process = owner.clone();
        let device_id = owner.identity.device_id.unwrap();
        process.identity = AuthenticatedIdentity {
            principal_id: PrincipalId::new(device_id.value()),
            principal_kind: PrincipalKind::Device,
            device_id: Some(DeviceId::new(device_id.value())),
            service_id: None,
        };
        let authenticator = DesktopAuthenticator::new(store.clone());
        let signed_in = authenticator
            .sign_in(
                &process,
                "reception",
                "reception-password-1",
                CorrelationId::new(Uuid::from_u128(40)),
                UnixMillis(2_000),
            )
            .unwrap()
            .authorization
            .unwrap();
        assert_eq!(
            authenticator.active(&signed_in, UnixMillis(2_001)),
            Ok(true)
        );

        let promoted = service
            .update(
                &mutation(manager_context.clone(), 3),
                &UpdateDesktopAccount {
                    account_id: receptionist.account_id,
                    expected_revision: receptionist.revision,
                    display_name: receptionist.display_name.clone(),
                    role: DesktopAccountRole::Manager,
                },
            )
            .unwrap();
        assert_eq!(promoted.revision, 2);
        assert_eq!(
            authenticator.active(&signed_in, UnixMillis(2_002)),
            Ok(false)
        );

        let signed_in = authenticator
            .sign_in(
                &process,
                "reception",
                "reception-password-1",
                CorrelationId::new(Uuid::from_u128(41)),
                UnixMillis(2_003),
            )
            .unwrap()
            .authorization
            .unwrap();
        let deactivated = service
            .deactivate(
                &mutation(manager_context.clone(), 4),
                &DeactivateDesktopAccount {
                    account_id: promoted.account_id,
                    expected_revision: promoted.revision,
                },
            )
            .unwrap();
        assert!(!deactivated.active);
        assert_eq!(deactivated.revision, 3);
        assert_eq!(
            authenticator.active(&signed_in, UnixMillis(2_004)),
            Ok(false)
        );
        assert!(
            authenticator
                .sign_in(
                    &process,
                    "reception",
                    "reception-password-1",
                    CorrelationId::new(Uuid::from_u128(42)),
                    UnixMillis(2_005),
                )
                .is_err()
        );
    }

    #[test]
    fn manager_provisions_and_revokes_receptionist_while_receptionist_is_denied() {
        let directory = tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let owner = store.local_authorization_context(UnixMillis(10)).unwrap();
        let organization_id = store.local_organization_id().unwrap();
        let manager = DesktopAccount {
            account_id: AccountId::new(Uuid::from_u128(10)),
            user_id: UserId::new(Uuid::from_u128(11)),
            tenant_id: owner.tenant_id,
            organization_id,
            password_hash: hash("manager-password-1", 1),
            role: DesktopRole::Manager,
        };
        store
            .provision_desktop_account(&owner, &manager, "manager", UnixMillis(20))
            .unwrap();
        let manager_context = user_context(&owner, manager.user_id, 20);
        let service =
            DesktopAccountService::new(store.clone(), AuthorizationService::new(store.clone()));

        let receptionist = service
            .create(
                &mutation(manager_context.clone(), 1),
                &CreateDesktopAccount {
                    display_name: "سارة أحمد".to_owned(),
                    username: "reception".to_owned(),
                    password: AccountPassword::new("reception-password-1"),
                    role: DesktopAccountRole::Receptionist,
                },
            )
            .unwrap();
        assert!(receptionist.active);
        assert_eq!(receptionist.revision, 1);

        let receptionist_context = user_context(&owner, receptionist.user_id, 30);
        assert_eq!(
            service.list(&receptionist_context, &ListDesktopAccounts {}),
            Err(DesktopAccountError::Denied)
        );
        assert_eq!(
            service.create(
                &mutation(receptionist_context.clone(), 2),
                &CreateDesktopAccount {
                    display_name: "مرفوض".to_owned(),
                    username: "denied".to_owned(),
                    password: AccountPassword::new("denied-password-1"),
                    role: DesktopAccountRole::Receptionist,
                },
            ),
            Err(DesktopAccountError::Denied)
        );

        assert_receptionist_session_lifecycle(
            &store,
            &owner,
            &manager_context,
            &service,
            &receptionist,
        );

        let database = rusqlite::Connection::open(store.path()).unwrap();
        let audited: i64 = database
            .query_row(
                "SELECT COUNT(*) FROM mutation_audit WHERE operation IN
             ('eitmad.desktop-account.create.v1', 'eitmad.desktop-account.deactivate.v1')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audited, 3);
    }

    #[test]
    fn revision_and_last_usable_manager_are_protected() {
        let directory = tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let owner = store.local_authorization_context(UnixMillis(10)).unwrap();
        let manager = DesktopAccount {
            account_id: AccountId::new(Uuid::from_u128(50)),
            user_id: UserId::new(Uuid::from_u128(51)),
            tenant_id: owner.tenant_id,
            organization_id: store.local_organization_id().unwrap(),
            password_hash: hash("manager-password-2", 2),
            role: DesktopRole::Manager,
        };
        store
            .provision_desktop_account(&owner, &manager, "manager", UnixMillis(20))
            .unwrap();
        let manager_context = user_context(&owner, manager.user_id, 60);
        let service = DesktopAccountService::new(store.clone(), AuthorizationService::new(store));
        let current = service
            .list(&manager_context, &ListDesktopAccounts {})
            .unwrap()
            .accounts
            .remove(0);

        assert_eq!(
            service.deactivate(
                &mutation(manager_context.clone(), 10),
                &DeactivateDesktopAccount {
                    account_id: current.account_id,
                    expected_revision: current.revision
                }
            ),
            Err(DesktopAccountError::LastUsableManager)
        );
        assert_eq!(
            service.update(
                &mutation(manager_context, 11),
                &UpdateDesktopAccount {
                    account_id: current.account_id,
                    expected_revision: current.revision + 1,
                    display_name: "مدير".to_owned(),
                    role: DesktopAccountRole::Manager,
                }
            ),
            Err(DesktopAccountError::RevisionConflict {
                expected: current.revision + 1,
                actual: current.revision
            })
        );
    }
}
