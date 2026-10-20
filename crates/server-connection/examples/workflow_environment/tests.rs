use super::*;
use eitmad_contracts::{
    identity::{AccountId, SessionId, UserId},
    server::{AuthenticatedServerSession, IssuedTokens},
};

#[test]
fn restart_import_preserves_role_credentials_sessions_and_deactivation() {
    let directory = tempfile::tempdir().unwrap();
    let journal = Journal {
        tenant: TenantId::new(Uuid::new_v4()),
        organization: OrganizationId::new(Uuid::new_v4()),
        branch: ScopeId::new(Uuid::new_v4()),
        owner_invite: None,
        reception_invite: None,
        seeded: true,
    };
    let authentication = AuthenticationResult {
        session: AuthenticatedServerSession {
            session_id: SessionId::new(Uuid::new_v4()),
            account_id: AccountId::new(Uuid::new_v4()),
            user_id: UserId::new(Uuid::new_v4()),
            device_id: DeviceId::new(Uuid::new_v4()),
            tenant_id: journal.tenant,
            issued_at: now(),
            expires_at: deadline(),
        },
        tokens: IssuedTokens {
            access_token: String::new(),
            refresh_token: String::new(),
            access_expires_at: deadline(),
            refresh_expires_at: deadline(),
        },
    };
    let (store, installer) = import_account(
        directory.path(),
        &journal,
        &authentication,
        "receptionist",
        "original-password-1",
        DesktopRole::Receptionist,
    )
    .unwrap();
    let mut account = store
        .desktop_account(journal.tenant, "receptionist")
        .unwrap()
        .unwrap();
    account.role = DesktopRole::Manager;
    store
        .provision_desktop_account(&installer, &account, "receptionist", now())
        .unwrap();
    let changed = store
        .list_desktop_accounts(journal.tenant)
        .unwrap()
        .accounts
        .remove(0);
    let (actor, branch) = sign_in(
        &store,
        installer.clone(),
        "receptionist",
        "original-password-1",
    )
    .unwrap();
    let hash = store
        .desktop_account(journal.tenant, "receptionist")
        .unwrap()
        .unwrap()
        .password_hash;
    let relationships = store.list_relationships(&branch.scope, None, 100).unwrap();

    let (reopened, _) = import_account(
        directory.path(),
        &journal,
        &authentication,
        "receptionist",
        "replacement-password-2",
        DesktopRole::Receptionist,
    )
    .unwrap();
    assert_eq!(
        reopened
            .desktop_account(journal.tenant, "receptionist")
            .unwrap()
            .unwrap()
            .password_hash,
        hash
    );
    assert_eq!(
        reopened
            .list_relationships(&branch.scope, None, 100)
            .unwrap(),
        relationships
    );
    assert!(
        DesktopAuthenticator::new(reopened.clone())
            .active(&actor, now())
            .unwrap()
    );
    assert!(
        reopened
            .list_desktop_accounts(journal.tenant)
            .unwrap()
            .accounts
            .contains(&changed)
    );
    store
        .revoke_connected_desktop_account(&actor, changed.revision, now())
        .unwrap();
    let disabled = store
        .list_desktop_accounts(journal.tenant)
        .unwrap()
        .accounts
        .remove(0);
    let relationships = store.list_relationships(&branch.scope, None, 100).unwrap();
    let (reopened, _) = import_account(
        directory.path(),
        &journal,
        &authentication,
        "receptionist",
        "replacement-password-3",
        DesktopRole::Receptionist,
    )
    .unwrap();
    assert!(
        reopened
            .list_desktop_accounts(journal.tenant)
            .unwrap()
            .accounts
            .contains(&disabled)
    );
    assert_eq!(
        reopened
            .list_relationships(&branch.scope, None, 100)
            .unwrap(),
        relationships
    );
    assert!(
        reopened
            .desktop_account(journal.tenant, "receptionist")
            .unwrap()
            .is_none()
    );
    assert!(
        !DesktopAuthenticator::new(reopened)
            .active(&actor, now())
            .unwrap()
    );
}
