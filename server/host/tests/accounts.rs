//! Real `PostgreSQL` and authenticated HTTP handlers; provisioning uses supported account routes.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer as _, SigningKey};
use eitmad_contracts::{
    accounts::DesktopAccountRole,
    identity::{AccountId, DeviceId, OrganizationId, ScopeId},
    server::{
        ActivateAccountRequest, AuthenticationResult, DeviceProof, DevicePublicKey, LoginRequest,
        RefreshRequest, TenantCode,
    },
    server_accounts::{
        ChangeServerAccount, ListServerAccounts, ServerAccountChange, ServerAccountPage,
        ServerAccountResult, ServerRoleAssignment,
    },
    transport::{CorrelationId, IdempotencyKey},
    versioning::{PeerHello, PeerKind, SupportedProtocol},
};
use eitmad_control_plane::{
    BootstrapInput, ControlDatabase, ControlPlane, TokenKey, unix_millis_now,
};
use eitmad_postgres_support::{self as sqlx, Row as _};
use eitmad_server::{ServerState, router};
use eitmad_server_audit::AuditDatabase;
use eitmad_sync_plane::{DomainRegistry, SyncCoordinator, SyncDatabase};
use serde::Serialize;
use serde_json::{Value, json};
use tower::ServiceExt as _;
use uuid::Uuid;

struct Client {
    auth: AuthenticationResult,
    key: SigningKey,
}
struct Fixture {
    app: Router,
    control: ControlPlane,
    pool: sqlx::PgPool,
    owner: Client,
    organization: OrganizationId,
    branches: [ScopeId; 2],
}

fn correlation() -> CorrelationId {
    CorrelationId::new(Uuid::new_v4())
}
fn command(change: ServerAccountChange) -> ChangeServerAccount {
    ChangeServerAccount {
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        change,
    }
}
fn assignment(role: DesktopAccountRole, branch: ScopeId) -> Vec<ServerRoleAssignment> {
    vec![ServerRoleAssignment {
        role,
        branch_ids: vec![branch],
    }]
}

impl Client {
    fn proof(&self) -> DeviceProof {
        device_proof(&self.key, self.auth.session.device_id)
    }
}
fn device_proof(key: &SigningKey, device: DeviceId) -> DeviceProof {
    let issued_at = unix_millis_now();
    let nonce = Uuid::new_v4().to_string();
    let bytes = format!("{}\n{}\n{nonce}", device.value(), issued_at.0);
    DeviceProof {
        device_id: device,
        nonce,
        issued_at,
        signature_base64: URL_SAFE_NO_PAD.encode(key.sign(bytes.as_bytes()).to_bytes()),
    }
}

async fn post<T: Serialize>(
    app: &Router,
    path: &str,
    input: &T,
    actor: Option<&Client>,
) -> (StatusCode, Value) {
    let mut builder = Request::post(path).header("content-type", "application/json");
    if let Some(actor) = actor {
        let peer = PeerHello {
            product_version: eitmad_contracts::updates::ReleaseVersion::new(semver::Version::new(
                0, 0, 0,
            )),
            peer_kind: PeerKind::Engine,
            protocols: vec![SupportedProtocol {
                major: 1,
                minimum_minor: 5,
                maximum_minor: eitmad_contracts::PROTOCOL_VERSION.minor,
            }],
            capabilities: vec![
                eitmad_contracts::transport::CapabilityId::parse(
                    "eitmad.capability.server-administration.v1",
                )
                .unwrap(),
                eitmad_contracts::transport::CapabilityId::parse(
                    "eitmad.capability.shared-accounts.v1",
                )
                .unwrap(),
            ],
            required_capabilities: vec![],
            schemas: vec![],
        };
        builder = builder
            .header(
                "authorization",
                format!("Bearer {}", actor.auth.tokens.access_token),
            )
            .header(
                "x-eitmad-device-proof",
                URL_SAFE_NO_PAD.encode(serde_json::to_vec(&actor.proof()).unwrap()),
            )
            .header(
                "x-eitmad-peer-hello",
                URL_SAFE_NO_PAD.encode(serde_json::to_vec(&peer).unwrap()),
            );
    }
    let response = app
        .clone()
        .oneshot(
            builder
                .body(Body::from(serde_json::to_vec(input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

fn activation(token: String, seed: u8) -> (ActivateAccountRequest, SigningKey) {
    let key = SigningKey::from_bytes(&[seed; 32]);
    (
        ActivateAccountRequest {
            invite_token: token,
            password: "synthetic-account-password".into(),
            device_id: DeviceId::new(Uuid::new_v4()),
            device_label: "جهاز اختباري".into(),
            device_public_key: DevicePublicKey {
                algorithm: "ed25519".into(),
                base64: URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()),
            },
        },
        key,
    )
}

async fn activate(app: &Router, request: &ActivateAccountRequest, key: SigningKey) -> Client {
    let (status, value) = post(app, "/v1/auth/activate", request, None).await;
    assert_eq!(status, StatusCode::OK);
    Client {
        auth: serde_json::from_value(value).unwrap(),
        key,
    }
}

async fn fixture() -> Fixture {
    let url = std::env::var("EITMAD_DIRECT_TEST_DATABASE_URL")
        .expect("set a fresh disposable PostgreSQL database URL");
    let db = ControlDatabase::connect(&url, 6).await.unwrap();
    db.migrate().await.unwrap();
    let pool = db.pool();
    let elevated: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname=current_user",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!elevated, "test role must enforce RLS");
    let sync = SyncDatabase::connect(&url, 4).await.unwrap();
    sync.migrate().await.unwrap();
    eitmad_admin_plane::AdminDatabase::connect(&url, 2)
        .await
        .unwrap()
        .migrate()
        .await
        .unwrap();
    AuditDatabase::from_pool(pool.clone())
        .migrate()
        .await
        .unwrap();
    let control = ControlPlane::new(pool.clone(), TokenKey::new([41; 32]));
    let bootstrap = control
        .identity
        .bootstrap(
            &BootstrapInput {
                tenant_code: TenantCode::parse("accounts-test").unwrap(),
                tenant_display_name: "مصنع اختباري".into(),
                organization_display_name: "الاعتماد".into(),
                owner_username: "administrator".into(),
            },
            correlation(),
            unix_millis_now(),
        )
        .await
        .unwrap();
    let app = router(ServerState::new(
        control.clone(),
        SyncCoordinator::new(&sync, DomainRegistry::new(std::iter::empty()).unwrap()),
    ));
    let (request, key) = activation(bootstrap.invite_token, 51);
    let owner = activate(&app, &request, key).await;
    let organization = bootstrap.organization_id;
    let branches = [ScopeId::new(Uuid::new_v4()), ScopeId::new(Uuid::new_v4())];
    for branch in branches {
        assert_eq!(
            post(
                &app,
                "/v1/customer-branches",
                &json!({"organizationId":organization,"branchId":branch}),
                Some(&owner)
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    Fixture {
        app,
        control,
        pool,
        owner,
        organization,
        branches,
    }
}

#[tokio::test]
#[ignore = "requires a fresh disposable PostgreSQL database with enforced RLS"]
async fn workshop_setup_is_atomic_and_creates_a_usable_owner() {
    use eitmad_contracts::{accounts::AccountPassword, setup::InitializeWorkshop};
    use eitmad_control_plane::IdentityError;
    let url = std::env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let db = ControlDatabase::connect(&url, 6).await.unwrap();
    db.migrate().await.unwrap();
    let pool = db.pool();
    let elevated: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname=current_user",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!elevated, "setup proof must enforce RLS");
    let sync = SyncDatabase::connect(&url, 2).await.unwrap();
    sync.migrate().await.unwrap();
    eitmad_admin_plane::AdminDatabase::connect(&url, 2)
        .await
        .unwrap()
        .migrate()
        .await
        .unwrap();
    AuditDatabase::from_pool(pool.clone())
        .migrate()
        .await
        .unwrap();
    let control = ControlPlane::new(pool.clone(), TokenKey::new([42; 32]));
    let request = InitializeWorkshop {
        version: 1,
        workshop_name: "ورشة اختبار الإعداد".into(),
        owner_username: "workshop-owner".into(),
        owner_password: AccountPassword::new("synthetic-setup-owner-password"),
    };
    assert_setup_rollback(&control, &pool, &request).await;
    let (one, two) = tokio::join!(
        control
            .identity
            .initialize_workshop(&request, correlation(), unix_millis_now()),
        control
            .identity
            .initialize_workshop(&request, correlation(), unix_millis_now()),
    );
    let ((Ok(initialized), Err(IdentityError::AlreadyBootstrapped))
    | (Err(IdentityError::AlreadyBootstrapped), Ok(initialized))) = (one, two)
    else {
        panic!("only one competing initialization may commit");
    };
    assert_initialized_owner(sync, control, pool, request, initialized).await;
}

async fn assert_initialized_owner(
    sync: SyncDatabase,
    control: ControlPlane,
    pool: sqlx::PgPool,
    request: eitmad_contracts::setup::InitializeWorkshop,
    initialized: eitmad_contracts::setup::InitializedWorkshop,
) {
    use eitmad_contracts::accounts::AccountPassword;
    let app = router(ServerState::new(
        control.clone(),
        SyncCoordinator::new(&sync, DomainRegistry::new(std::iter::empty()).unwrap()),
    ));
    let (activation, key) = activation(String::new(), 61);
    let login = LoginRequest {
        tenant_code: initialized.tenant_code.clone(),
        username: request.owner_username.clone(),
        password: request.owner_password.expose_secret().into(),
        device_id: activation.device_id,
        device_label: activation.device_label,
        device_public_key: activation.device_public_key,
        device_proof: Some(device_proof(&key, activation.device_id)),
    };
    control
        .pairing
        .enroll_first_desktop(&login, correlation(), unix_millis_now())
        .await
        .unwrap();
    let (status, body) = post(&app, "/v1/auth/login", &login, None).await;
    assert_eq!(status, StatusCode::OK);
    let owner = Client {
        auth: serde_json::from_value(body).unwrap(),
        key,
    };
    assert_eq!(owner.auth.session.account_id, initialized.owner_account_id);
    let f = Fixture {
        app,
        control,
        pool,
        owner,
        organization: initialized.organization_id,
        branches: [initialized.branch_id, initialized.branch_id],
    };
    let accounts = list(&f).await.accounts;
    assert_eq!(accounts.len(), 1);
    assert!(accounts[0].organizations[0].owner);
    assert_eq!(
        accounts[0].organizations[0].assignments[0].role,
        DesktopAccountRole::Manager
    );
    let (status, profile) = post(
        &f.app,
        "/v1/auth/desktop-profile",
        &json!({
            "organizationId": initialized.organization_id, "branchId": initialized.branch_id,
        }),
        Some(&f.owner),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(profile["role"], "manager");
    let staff = change(
        &f,
        &command(ServerAccountChange::CreateDesktop {
            organization_id: f.organization,
            branch_id: initialized.branch_id,
            account: eitmad_contracts::accounts::CreateDesktopAccount {
                display_name: "موظف اختبار".into(),
                username: "setup-receptionist".into(),
                password: AccountPassword::new("synthetic-staff-password"),
                role: DesktopAccountRole::Receptionist,
            },
        }),
    )
    .await;
    assert_eq!(
        staff.receipt.account.organizations[0].assignments[0].role,
        DesktopAccountRole::Receptionist
    );
    assert!(
        f.control
            .identity
            .initialize_workshop(&request, correlation(), unix_millis_now())
            .await
            .is_err()
    );
    assert_eq!(list(&f).await.accounts.len(), 2);
}

async fn assert_setup_rollback(
    control: &ControlPlane,
    pool: &sqlx::PgPool,
    request: &eitmad_contracts::setup::InitializeWorkshop,
) {
    use eitmad_contracts::accounts::AccountPassword;
    use eitmad_control_plane::IdentityError;
    for invalid in [
        eitmad_contracts::setup::InitializeWorkshop {
            version: 2,
            ..request.clone()
        },
        eitmad_contracts::setup::InitializeWorkshop {
            workshop_name: "\n".into(),
            ..request.clone()
        },
        eitmad_contracts::setup::InitializeWorkshop {
            owner_password: AccountPassword::new("short"),
            ..request.clone()
        },
    ] {
        assert!(matches!(
            control
                .identity
                .initialize_workshop(&invalid, correlation(), unix_millis_now())
                .await,
            Err(IdentityError::InvalidInput)
        ));
    }
    sqlx::raw_sql("CREATE FUNCTION audit.reject_setup_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic setup audit failure'; END $$; CREATE TRIGGER setup_test_failure BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION audit.reject_setup_test();")
        .execute(pool).await.unwrap();
    assert!(matches!(
        control
            .identity
            .initialize_workshop(request, correlation(), unix_millis_now())
            .await,
        Err(IdentityError::Unavailable)
    ));
    sqlx::raw_sql("DROP TRIGGER setup_test_failure ON audit.server_records; DROP FUNCTION audit.reject_setup_test();")
        .execute(pool).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM control.tenants")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(
        count, 0,
        "failed initialization must leave no tenant or partial owner"
    );
}

async fn change(f: &Fixture, request: &ChangeServerAccount) -> ServerAccountResult {
    let (status, value) = post(&f.app, "/v1/accounts/transition", request, Some(&f.owner)).await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_value(value).unwrap()
}

async fn invite(f: &Fixture, username: &str, role: DesktopAccountRole) -> ServerAccountResult {
    change(
        f,
        &command(ServerAccountChange::Invite {
            organization_id: f.organization,
            username: username.into(),
            assignments: assignment(role, f.branches[0]),
        }),
    )
    .await
}

async fn list(f: &Fixture) -> ServerAccountPage {
    let (status, value) = post(
        &f.app,
        "/v1/accounts/list",
        &ListServerAccounts {
            after: None,
            limit: 100,
        },
        Some(&f.owner),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_value(value).unwrap()
}

async fn login(f: &Fixture, client: &Client, username: &str) -> Client {
    let input = LoginRequest {
        tenant_code: TenantCode::parse("accounts-test").unwrap(),
        username: username.into(),
        password: "synthetic-account-password".into(),
        device_id: client.auth.session.device_id,
        device_label: "جهاز اختباري".into(),
        device_public_key: DevicePublicKey {
            algorithm: "ed25519".into(),
            base64: URL_SAFE_NO_PAD.encode(client.key.verifying_key().as_bytes()),
        },
        device_proof: Some(client.proof()),
    };
    let (status, value) = post(&f.app, "/v1/auth/login", &input, None).await;
    assert_eq!(status, StatusCode::OK);
    Client {
        auth: serde_json::from_value(value).unwrap(),
        key: client.key.clone(),
    }
}

async fn assert_revoked(f: &Fixture, client: &Client) {
    assert_eq!(
        post(
            &f.app,
            "/v1/accounts/list",
            &ListServerAccounts {
                after: None,
                limit: 10
            },
            Some(client)
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let refresh = RefreshRequest {
        refresh_token: client.auth.tokens.refresh_token.clone(),
        device_proof: client.proof(),
    };
    assert_eq!(
        post(&f.app, "/v1/auth/refresh", &refresh, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert!(
        f.control
            .authentication
            .revalidate_access(
                &client.auth.tokens.access_token,
                &client.auth.session,
                unix_millis_now()
            )
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires fresh disposable recovery database, superuser and RLS application URLs"]
async fn local_owner_password_recovery_preserves_scope_and_revokes_sessions() {
    let admin_url = std::env::var("EITMAD_RECOVERY_TEST_ADMIN_URL").unwrap();
    let admin = ControlDatabase::connect(&admin_url, 2)
        .await
        .unwrap()
        .pool();
    let name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&admin)
        .await
        .unwrap();
    assert!(
        name.starts_with("eitmad_owner_recovery_"),
        "disposable recovery database required"
    );
    let f = fixture().await;
    let tenant = TenantCode::parse("accounts-test").unwrap();
    let invited = invite(&f, "non-owner", DesktopAccountRole::Manager).await;
    let (request, key) = activation(invited.activation_token.unwrap(), 72);
    let non_owner = activate(&f.app, &request, key).await;
    assert_recovery_denials(&f, &admin, &tenant).await;
    let before = recovery_snapshot(&admin).await;
    sqlx::raw_sql("CREATE FUNCTION public.fail_recovery_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operation='eitmad.server.authentication.owner-password-recovered.v1' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fail_recovery_audit BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION public.fail_recovery_audit()")
        .execute(&admin).await.unwrap();
    assert_eq!(
        eitmad_control_plane::recover_owner_password(
            &admin,
            &tenant,
            "administrator",
            "synthetic-new-password",
            correlation(),
            unix_millis_now()
        )
        .await,
        Err(eitmad_control_plane::IdentityError::Unavailable)
    );
    assert_eq!(
        recovery_snapshot(&admin).await,
        before,
        "audit failure rolls back all recovery state"
    );
    sqlx::raw_sql("DROP TRIGGER fail_recovery_audit ON audit.server_records; DROP FUNCTION public.fail_recovery_audit()")
        .execute(&admin).await.unwrap();
    let audit_correlation = correlation();
    eitmad_control_plane::recover_owner_password(
        &admin,
        &tenant,
        "administrator",
        "synthetic-new-password",
        audit_correlation,
        unix_millis_now(),
    )
    .await
    .unwrap();
    assert_revoked(&f, &f.owner).await;
    // Another account's password, roles, and sessions are retained.
    let _ = login(&f, &non_owner, "non-owner").await;
    assert_recovered_login(&f).await;
    let audit: Value = sqlx::query_scalar("SELECT jsonb_build_object('actor',actor_kind,'outcome',outcome,'target',target_id,'error',redacted_error) FROM audit.server_records WHERE correlation_id=$1")
        .bind(audit_correlation.value()).fetch_one(&admin).await.unwrap();
    assert_eq!(
        audit,
        json!({"actor":"system","outcome":"succeeded","target":f.owner.auth.session.account_id,"error":null})
    );
    let identities: Value = sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_object('account',account_id,'user',user_id,'tenant',tenant_id) ORDER BY account_id) FROM control.accounts")
        .fetch_one(&admin).await.unwrap();
    assert_eq!(identities, before["identities"]);
    admin.close().await;
}

async fn assert_recovery_denials(f: &Fixture, admin: &sqlx::PgPool, tenant: &TenantCode) {
    for (pool, code, username, password, expected) in [
        (
            &f.pool,
            tenant,
            "administrator",
            "synthetic-new-password",
            eitmad_control_plane::IdentityError::Denied,
        ),
        (
            admin,
            tenant,
            "non-owner",
            "synthetic-new-password",
            eitmad_control_plane::IdentityError::Denied,
        ),
        (
            admin,
            &TenantCode::parse("foreign-tenant").unwrap(),
            "administrator",
            "synthetic-new-password",
            eitmad_control_plane::IdentityError::Denied,
        ),
        (
            admin,
            tenant,
            "administrator",
            "short",
            eitmad_control_plane::IdentityError::InvalidInput,
        ),
    ] {
        assert_eq!(
            eitmad_control_plane::recover_owner_password(
                pool,
                code,
                username,
                password,
                correlation(),
                unix_millis_now()
            )
            .await,
            Err(expected)
        );
    }
}

async fn recovery_snapshot(pool: &sqlx::PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('identities',(SELECT jsonb_agg(jsonb_build_object('account',account_id,'user',user_id,'tenant',tenant_id) ORDER BY account_id) FROM control.accounts),'accounts',(SELECT jsonb_agg(to_jsonb(a) ORDER BY account_id) FROM control.accounts a),'sessions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY session_id) FROM control.sessions s),'families',(SELECT jsonb_agg(to_jsonb(f) ORDER BY token_family_id) FROM control.token_families f),'revision',(SELECT account_revision FROM control.tenants WHERE tenant_code='accounts-test'))")
        .fetch_one(pool).await.unwrap()
}

async fn assert_recovered_login(f: &Fixture) {
    let input = |password: &str| LoginRequest {
        tenant_code: TenantCode::parse("accounts-test").unwrap(),
        username: "administrator".into(),
        password: password.into(),
        device_id: f.owner.auth.session.device_id,
        device_label: "Synthetic recovery device".into(),
        device_public_key: DevicePublicKey {
            algorithm: "ed25519".into(),
            base64: URL_SAFE_NO_PAD.encode(f.owner.key.verifying_key().as_bytes()),
        },
        device_proof: Some(f.owner.proof()),
    };
    assert_eq!(
        post(
            &f.app,
            "/v1/auth/login",
            &input("synthetic-account-password"),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, body) = post(
        &f.app,
        "/v1/auth/login",
        &input("synthetic-new-password"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let auth: AuthenticationResult = serde_json::from_value(body).unwrap();
    assert_eq!(auth.session.account_id, f.owner.auth.session.account_id);
    assert_eq!(auth.session.user_id, f.owner.auth.session.user_id);
}

#[tokio::test]
#[ignore = "requires a fresh disposable PostgreSQL database with enforced RLS"]
async fn server_accounts_work_through_authenticated_routes() {
    let f = fixture().await;
    let request = command(ServerAccountChange::Invite {
        organization_id: f.organization,
        username: "موظفة الاستقبال".into(),
        assignments: assignment(DesktopAccountRole::Receptionist, f.branches[0]),
    });
    let (one, two) = tokio::join!(change(&f, &request), change(&f, &request));
    assert_eq!(one.receipt, two.receipt);
    assert!(one.activation_token == two.activation_token);
    let account = one.receipt.account.account_id;
    let accounts = list(&f).await.accounts;
    assert_eq!(accounts.len(), 2);
    let owner = accounts
        .iter()
        .find(|a| a.account_id == f.owner.auth.session.account_id)
        .unwrap();
    let invited = accounts.iter().find(|a| a.account_id == account).unwrap();
    assert_eq!(
        [
            owner.display_name.as_str(),
            one.receipt.account.display_name.as_str(),
            invited.display_name.as_str(),
        ],
        ["administrator", "موظفة الاستقبال", "موظفة الاستقبال"]
    );
    let changed = ChangeServerAccount {
        change: ServerAccountChange::Invite {
            organization_id: f.organization,
            username: "different".into(),
            assignments: vec![],
        },
        ..request.clone()
    };
    assert_eq!(
        post(&f.app, "/v1/accounts/transition", &changed, Some(&f.owner))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let (activation, key) = activation(one.activation_token.unwrap(), 52);
    let reception = activate(&f.app, &activation, key).await;
    let (status, retry) = post(&f.app, "/v1/auth/activate", &activation, None).await;
    assert_eq!(status, StatusCode::OK);
    let retried: AuthenticationResult = serde_json::from_value(retry).unwrap();
    assert!(retried == reception.auth);
    let mut mismatch = activation.clone();
    mismatch.password = "synthetic-other-password".into();
    assert_eq!(
        post(&f.app, "/v1/auth/activate", &mismatch, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_denied_roles_and_scope(&f, &reception).await;
    assert_revision_assignment_and_revocation(&f, account, &reception, &activation).await;
    assert_audit_rollback(&f).await;
    assert_role_replacement_and_conflict_retry(&f).await;
    assert_activation_rotation(&f).await;
    assert_pending_deactivation(&f).await;
    assert_pagination_and_audit(&f, request.idempotency_key).await;
    assert_owner_family_revocation(&f).await;
}

async fn assert_owner_family_revocation(f: &Fixture) {
    let request = RefreshRequest {
        refresh_token: f.owner.auth.tokens.refresh_token.clone(),
        device_proof: f.owner.proof(),
    };
    assert_eq!(
        post(&f.app, "/v1/auth/refresh", &request, None).await.0,
        StatusCode::OK
    );
    let reused = RefreshRequest {
        device_proof: f.owner.proof(),
        ..request
    };
    assert_eq!(
        post(&f.app, "/v1/auth/refresh", &reused, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    // A cached typed session must not bypass token-family revocation at the Rust authority.
    assert_eq!(
        f.control
            .identity
            .list_accounts(
                &f.owner.auth.session,
                &ListServerAccounts {
                    after: None,
                    limit: 10
                },
                correlation(),
                unix_millis_now()
            )
            .await,
        Err(eitmad_control_plane::ServerAccountError::Denied)
    );
    let restored = login(f, &f.owner, "administrator").await;
    assert_eq!(
        post(
            &f.app,
            "/v1/accounts/list",
            &ListServerAccounts {
                after: None,
                limit: 10
            },
            Some(&restored)
        )
        .await
        .0,
        StatusCode::OK
    );
}

async fn assert_activation_rotation(f: &Fixture) {
    let invited = invite(f, "rotation", DesktopAccountRole::Manager).await;
    let (request, key) = activation(invited.activation_token.unwrap(), 57);
    let client = activate(&f.app, &request, key).await;
    let refresh = RefreshRequest {
        refresh_token: client.auth.tokens.refresh_token.clone(),
        device_proof: client.proof(),
    };
    assert_eq!(
        post(&f.app, "/v1/auth/refresh", &refresh, None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        post(&f.app, "/v1/auth/activate", &request, None).await.0,
        StatusCode::UNAUTHORIZED,
        "activation retry must not return credentials consumed by a refresh rotation"
    );
}

async fn assert_denied_roles_and_scope(f: &Fixture, reception: &Client) {
    let manager = invite(f, "owner", DesktopAccountRole::Manager).await;
    let (request, key) = activation(manager.activation_token.unwrap(), 53);
    let manager = activate(&f.app, &request, key).await;
    for actor in [&manager, reception] {
        assert_eq!(
            post(
                &f.app,
                "/v1/accounts/list",
                &ListServerAccounts {
                    after: None,
                    limit: 10
                },
                Some(actor)
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
        let request = command(ServerAccountChange::Invite {
            organization_id: f.organization,
            username: "denied".into(),
            assignments: vec![],
        });
        assert_eq!(
            post(&f.app, "/v1/accounts/transition", &request, Some(actor))
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    let owner = list(f)
        .await
        .accounts
        .into_iter()
        .find(|a| a.account_id == f.owner.auth.session.account_id)
        .unwrap();
    assert!(owner.organizations[0].owner);
    assert!(owner.organizations[0].assignments.is_empty());
    let disable = command(ServerAccountChange::Deactivate {
        account_id: owner.account_id,
        expected_revision: owner.revision,
    });
    assert_eq!(
        post(&f.app, "/v1/accounts/transition", &disable, Some(&f.owner))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        post(
            &f.app,
            "/v1/accounts/list",
            &ListServerAccounts {
                after: None,
                limit: 10
            },
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_foreign_tenant(f).await;
}

async fn assert_revision_assignment_and_revocation(
    f: &Fixture,
    account: AccountId,
    reception: &Client,
    activation: &ActivateAccountRequest,
) {
    let stale = command(ServerAccountChange::Assign {
        account_id: account,
        expected_revision: 1,
        organization_id: f.organization,
        assignments: assignment(DesktopAccountRole::Manager, f.branches[1]),
    });
    assert_eq!(
        post(&f.app, "/v1/accounts/transition", &stale, Some(&f.owner))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let replace = command(ServerAccountChange::Assign {
        account_id: account,
        expected_revision: 2,
        organization_id: f.organization,
        assignments: assignment(DesktopAccountRole::Receptionist, f.branches[1]),
    });
    let result = change(f, &replace).await;
    assert_eq!(result.receipt.account.revision, 3);
    assert_eq!(
        result.receipt.account.organizations[0].assignments[0].branch_ids,
        vec![f.branches[1]]
    );
    assert_revoked(f, reception).await;
    assert_eq!(
        post(&f.app, "/v1/auth/activate", activation, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let logged = login(f, reception, "موظفة الاستقبال").await;
    let revoke = command(ServerAccountChange::RevokeSessions {
        account_id: account,
        expected_revision: 3,
    });
    let receipt = change(f, &revoke).await.receipt;
    assert_revoked(f, &logged).await;
    let relogged = login(f, &logged, "موظفة الاستقبال").await;
    assert_eq!(change(f, &revoke).await.receipt, receipt);
    assert!(
        f.control
            .authentication
            .revalidate_access(
                &relogged.auth.tokens.access_token,
                &relogged.auth.session,
                unix_millis_now()
            )
            .await
            .is_ok()
    );
    let disable = command(ServerAccountChange::Deactivate {
        account_id: account,
        expected_revision: 4,
    });
    let result = change(f, &disable).await;
    assert_eq!(result.receipt.account.revision, 5);
    assert!(
        result.receipt.account.organizations[0]
            .assignments
            .is_empty()
    );
    assert_revoked(f, &relogged).await;
    assert_eq!(change(f, &disable).await.receipt, result.receipt);
    assert_eq!(
        change(f, &replace).await.receipt.account.revision,
        3,
        "retries preserve their original receipt after later changes"
    );
}

async fn assert_pending_deactivation(f: &Fixture) {
    let invited = invite(f, "pending", DesktopAccountRole::Receptionist).await;
    let request = command(ServerAccountChange::Deactivate {
        account_id: invited.receipt.account.account_id,
        expected_revision: 1,
    });
    change(f, &request).await;
    let (activation, _) = activation(invited.activation_token.unwrap(), 54);
    assert_eq!(
        post(&f.app, "/v1/auth/activate", &activation, None).await.0,
        StatusCode::UNAUTHORIZED
    );
}

async fn assert_foreign_tenant(f: &Fixture) {
    // The bootstrap CLI deliberately creates only the first tenant. This fixture represents another tenant.
    let tenant = Uuid::new_v4();
    let org = OrganizationId::new(Uuid::new_v4());
    let branch = ScopeId::new(Uuid::new_v4());
    let account = AccountId::new(Uuid::new_v4());
    let user = Uuid::new_v4();
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO control.tenants(tenant_id,tenant_code,display_name,created_at) VALUES($1,'foreign-fixture','synthetic',1)").bind(tenant).execute(&mut *tx).await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.organizations(tenant_id,organization_id,display_name,created_at) VALUES($1,$2,'synthetic',1)").bind(tenant).bind(org.value()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO control.branches(tenant_id,organization_id,branch_id,created_at) VALUES($1,$2,$3,1)").bind(tenant).bind(org.value()).bind(branch.value()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO control.users(tenant_id,user_id,created_at) VALUES($1,$2,1)")
        .bind(tenant)
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.accounts(tenant_id,account_id,user_id,username,canonical_username,status,created_at) VALUES($1,$2,$3,'foreign','foreign','pending_activation',1)").bind(tenant).bind(account.value()).bind(user).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert!(
        !list(f)
            .await
            .accounts
            .iter()
            .any(|a| a.account_id == account)
    );
    let changes = [
        ServerAccountChange::Deactivate {
            account_id: account,
            expected_revision: 1,
        },
        ServerAccountChange::RevokeSessions {
            account_id: account,
            expected_revision: 1,
        },
        ServerAccountChange::Invite {
            organization_id: org,
            username: "foreign-denied".into(),
            assignments: vec![],
        },
        ServerAccountChange::Assign {
            account_id: f.owner.auth.session.account_id,
            expected_revision: 2,
            organization_id: f.organization,
            assignments: assignment(DesktopAccountRole::Receptionist, branch),
        },
    ];
    for change in changes {
        assert_eq!(
            post(
                &f.app,
                "/v1/accounts/transition",
                &command(change),
                Some(&f.owner)
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
}

async fn assert_audit_rollback(f: &Fixture) {
    let before = list(f).await.accounts.len();
    sqlx::raw_sql("CREATE FUNCTION audit.reject_account_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operation='eitmad.server.account.invite.v1' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER account_test_failure BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION audit.reject_account_test();").execute(&f.pool).await.unwrap();
    let request = command(ServerAccountChange::Invite {
        organization_id: f.organization,
        username: "rollback".into(),
        assignments: assignment(DesktopAccountRole::Manager, f.branches[0]),
    });
    assert_eq!(
        post(&f.app, "/v1/accounts/transition", &request, Some(&f.owner))
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::raw_sql("DROP TRIGGER account_test_failure ON audit.server_records; DROP FUNCTION audit.reject_account_test();").execute(&f.pool).await.unwrap();
    assert_eq!(list(f).await.accounts.len(), before);
    let accepted = change(f, &request).await;
    assert_eq!(accepted.receipt.account.revision, 1);
    assert_assignment_audit_rollback(f, accepted).await;
}

async fn assert_assignment_audit_rollback(f: &Fixture, invited: ServerAccountResult) {
    let account = invited.receipt.account.account_id;
    let (activation, key) = activation(invited.activation_token.unwrap(), 55);
    let manager = activate(&f.app, &activation, key).await;
    sqlx::raw_sql("CREATE FUNCTION audit.reject_assignment_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operation='eitmad.server.account.assign.v1' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER assignment_test_failure BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION audit.reject_assignment_test();").execute(&f.pool).await.unwrap();
    let request = command(ServerAccountChange::Assign {
        account_id: account,
        expected_revision: 2,
        organization_id: f.organization,
        assignments: vec![],
    });
    assert_eq!(
        post(&f.app, "/v1/accounts/transition", &request, Some(&f.owner))
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::raw_sql("DROP TRIGGER assignment_test_failure ON audit.server_records; DROP FUNCTION audit.reject_assignment_test();").execute(&f.pool).await.unwrap();
    let current = list(f)
        .await
        .accounts
        .into_iter()
        .find(|a| a.account_id == account)
        .unwrap();
    assert_eq!(current.revision, 2);
    assert_eq!(
        current.organizations[0].assignments[0].role,
        DesktopAccountRole::Manager
    );
    assert!(
        f.control
            .authentication
            .revalidate_access(
                &manager.auth.tokens.access_token,
                &manager.auth.session,
                unix_millis_now()
            )
            .await
            .is_ok()
    );
    assert!(
        change(f, &request).await.receipt.account.organizations[0]
            .assignments
            .is_empty()
    );
    assert_revoked(f, &manager).await;
}

async fn assert_role_replacement_and_conflict_retry(f: &Fixture) {
    let invited = invite(f, "roles", DesktopAccountRole::Manager).await;
    let account = invited.receipt.account.account_id;
    let (activation, key) = activation(invited.activation_token.unwrap(), 56);
    let client = activate(&f.app, &activation, key).await;
    // This intent is ahead of the current revision. Its rejection must remain stable later.
    let future = command(ServerAccountChange::Assign {
        account_id: account,
        expected_revision: 3,
        organization_id: f.organization,
        assignments: vec![],
    });
    assert_eq!(
        post(&f.app, "/v1/accounts/transition", &future, Some(&f.owner))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let mut both = assignment(DesktopAccountRole::Manager, f.branches[0]);
    both.extend(assignment(DesktopAccountRole::Receptionist, f.branches[1]));
    let first = command(ServerAccountChange::Assign {
        account_id: account,
        expected_revision: 2,
        organization_id: f.organization,
        assignments: both,
    });
    let second = ChangeServerAccount {
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        ..first.clone()
    };
    let (one, two) = tokio::join!(
        post(&f.app, "/v1/accounts/transition", &first, Some(&f.owner)),
        post(&f.app, "/v1/accounts/transition", &second, Some(&f.owner))
    );
    assert!(matches!(
        (one.0, two.0),
        (StatusCode::OK, StatusCode::CONFLICT) | (StatusCode::CONFLICT, StatusCode::OK)
    ));
    let current = list(f)
        .await
        .accounts
        .into_iter()
        .find(|a| a.account_id == account)
        .unwrap();
    assert_eq!(current.revision, 3);
    assert_eq!(current.organizations[0].assignments.len(), 2);
    assert_eq!(
        post(&f.app, "/v1/accounts/transition", &future, Some(&f.owner))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let replacement = command(ServerAccountChange::Assign {
        account_id: account,
        expected_revision: 3,
        organization_id: f.organization,
        assignments: assignment(DesktopAccountRole::Receptionist, f.branches[0]),
    });
    let result = change(f, &replacement).await;
    assert_eq!(
        result.receipt.account.organizations[0].assignments,
        assignment(DesktopAccountRole::Receptionist, f.branches[0])
    );
    assert_revoked(f, &client).await;
}

async fn assert_pagination_and_audit(f: &Fixture, key: IdempotencyKey) {
    let expected = list(f).await.accounts;
    let mut seen = Vec::new();
    let mut after = None;
    loop {
        let (status, value) = post(
            &f.app,
            "/v1/accounts/list",
            &ListServerAccounts { after, limit: 1 },
            Some(&f.owner),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let page: ServerAccountPage = serde_json::from_value(value).unwrap();
        seen.extend(page.accounts);
        after = page.next_after;
        if after.is_none() {
            break;
        }
    }
    assert_eq!(seen, expected);
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(f.owner.auth.session.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let rows=sqlx::query("SELECT outcome,count(*) AS count FROM audit.server_records WHERE tenant_id=$1 AND idempotency_key=$2 GROUP BY outcome").bind(f.owner.auth.session.tenant_id.value()).bind(key.value()).fetch_all(&mut *tx).await.unwrap();
    assert!(
        rows.iter()
            .any(|r| r.get::<String, _>("outcome") == "succeeded" && r.get::<i64, _>("count") == 1)
    );
    assert!(
        rows.iter()
            .any(|r| r.get::<String, _>("outcome") == "conflict")
    );
    let plaintext:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.account_results WHERE receipt::text LIKE '%activationToken%' OR receipt::text LIKE '%password%')").fetch_one(&mut *tx).await.unwrap();
    assert!(!plaintext);
    tx.commit().await.unwrap();
}
