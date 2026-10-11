use super::*;
use eitmad_contracts::{
    accounts::{DeactivateDesktopAccount, ListDesktopAccounts, UpdateDesktopAccount},
    identity::{AccountId, SessionId, UserId},
    server::{AuthenticatedServerSession, IssuedTokens},
};
use eitmad_engine_runtime::DesktopAccountService;

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
    let mut manager = store
        .desktop_account(journal.tenant, "receptionist")
        .unwrap()
        .unwrap();
    manager.account_id = AccountId::new(Uuid::new_v4());
    manager.user_id = UserId::new(Uuid::new_v4());
    manager.role = DesktopRole::Manager;
    store
        .provision_desktop_account(&installer, &manager, "manager", now())
        .unwrap();
    let (manager_actor, _) =
        sign_in(&store, installer.clone(), "manager", "original-password-1").unwrap();
    let service = DesktopAccountService::new(
        store.clone(),
        eitmad_authorization::AuthorizationService::new(store.clone()),
    );
    let changed = service
        .update(
            &mutation(&manager_actor, 501),
            &UpdateDesktopAccount {
                account_id: authentication.session.account_id,
                expected_revision: 1,
                display_name: "حساب اختبار معدل".into(),
                role: DesktopAccountRole::Manager,
            },
        )
        .unwrap();
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
        service
            .list(&manager_actor, &ListDesktopAccounts {})
            .unwrap()
            .accounts
            .contains(&changed)
    );

    let disabled = service
        .deactivate(
            &mutation(&manager_actor, 502),
            &DeactivateDesktopAccount {
                account_id: changed.account_id,
                expected_revision: changed.revision,
            },
        )
        .unwrap();
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
