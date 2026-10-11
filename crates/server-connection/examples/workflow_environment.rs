//! Persistent synthetic environment provisioning through Rust authority and the real TLS route.
use argon2::{Argon2, PasswordHasher as _, password_hash::SaltString};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer as _, SigningKey};
use eitmad_authorization::{MutationContext, now};
use eitmad_contracts::{
    accounts::DesktopAccountRole,
    config::SecretReferenceId,
    identity::{
        AuthorizationContext, DeviceId, OrganizationId, PrincipalId, PrincipalKind, ScopeId,
        ScopeKind, ScopeRef, TenantId,
    },
    secrets::{SecretId, SecretKind},
    server::{
        ActivateAccountRequest, AuthenticationResult, DeviceProof, DevicePublicKey, LoginRequest,
        RegisterBranchRequest, TenantCode,
    },
    transport::{CorrelationId, IdempotencyKey, SchemaId, UnixMillis},
};
use eitmad_control_plane::{BootstrapInput, ControlDatabase, ControlPlane, TokenKey, WorkflowRole};
use eitmad_engine_runtime::DesktopAuthenticator;
use eitmad_secret_storage::SecretStore;
use eitmad_server_connection::{
    DirectCatalogSyncClient, DirectDiscountApprovalClient, DirectPriceClient, DirectServerConfig,
    store_session,
};
use eitmad_storage::{AuthorityStore, DesktopAccount, DesktopRole};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;
#[path = "workflow_environment/customers.rs"]
mod customers;
#[path = "workflow_environment/seed.rs"]
mod seed;

type Result<T> = std::result::Result<T, &'static str>;
fn checked<T, E>(value: std::result::Result<T, E>, stage: &'static str) -> Result<T> {
    value.map_err(|_| stage)
}
fn correlation() -> CorrelationId {
    CorrelationId::new(Uuid::new_v4())
}
fn scope(kind: &str, id: Uuid) -> Result<ScopeRef> {
    Ok(ScopeRef {
        kind: checked(ScopeKind::parse(kind), "scope")?,
        id: ScopeId::new(id),
    })
}
fn mutation(actor: &AuthorizationContext, step: u128) -> MutationContext {
    MutationContext {
        authorization: actor.clone(),
        correlation_id: correlation(),
        causation_id: None,
        idempotency_key: IdempotencyKey::new(Uuid::from_u128(step)),
        occurred_at: now(),
    }
}
fn deadline() -> UnixMillis {
    UnixMillis(now().0 + 30_000)
}

#[derive(Deserialize)]
struct Settings {
    database_url: String,
    token_key: String,
    endpoint: String,
    manager_password: String,
    receptionist_password: String,
    manager_seed: [u8; 32],
    receptionist_seed: [u8; 32],
    manager_device: DeviceId,
    receptionist_device: DeviceId,
}
// Activation secrets remain only in the ACL-protected disposable state directory.
#[derive(Serialize, Deserialize)]
struct Journal {
    tenant: TenantId,
    organization: OrganizationId,
    branch: ScopeId,
    owner_invite: Option<String>,
    reception_invite: Option<String>,
    seeded: bool,
}
fn save(root: &Path, journal: &Journal) -> Result<()> {
    let bytes = checked(serde_json::to_vec(journal), "encode provisioning journal")?;
    checked(
        fs::write(root.join("provision.next"), bytes),
        "write provisioning journal",
    )?;
    checked(
        fs::rename(root.join("provision.next"), root.join("provision.json")),
        "commit provisioning journal",
    )
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let _ = rustls::crypto::ring::default_provider().install_default();
    match prepare().await {
        Ok(()) => {
            println!("Authenticated workflow environment is ready; accepted work is retained.");
            std::process::ExitCode::SUCCESS
        }
        Err(stage) => {
            eprintln!("Workflow environment failed at: {stage}. State was not reset.");
            std::process::ExitCode::FAILURE
        }
    }
}
async fn prepare() -> Result<()> {
    let (root, settings) = load_settings()?;
    let database = checked(
        ControlDatabase::connect(&settings.database_url, 4).await,
        "control database",
    )?;
    let control = ControlPlane::new(
        database.pool(),
        checked(TokenKey::from_base64(&settings.token_key), "token key")?,
    );
    let mut journal = load_or_bootstrap(&root, &control).await?;
    let manager = authenticate(
        &control,
        "manager",
        &settings.manager_password,
        settings.manager_device,
        &settings.manager_seed,
        journal.owner_invite.as_deref(),
    )
    .await?;
    journal.owner_invite = None;
    save(&root, &journal)?;
    checked(
        control
            .branches
            .register(
                &manager.session,
                &RegisterBranchRequest {
                    organization_id: journal.organization,
                    branch_id: journal.branch,
                },
                correlation(),
                now(),
            )
            .await,
        "register branch",
    )?;
    checked(
        control
            .identity
            .grant_workflow_role(
                &manager.session,
                &WorkflowRole {
                    organization: journal.organization,
                    branch: journal.branch,
                    role: DesktopAccountRole::Manager,
                },
                manager.session.user_id,
                correlation(),
                now(),
            )
            .await,
        "assign manager role",
    )?;
    let reception = receptionist(&root, &settings, &control, &mut journal, &manager).await?;
    journal.reception_invite = None;
    save(&root, &journal)?;
    // A Receptionist must never be able to provision another account.
    if control
        .identity
        .grant_workflow_role(
            &reception.session,
            &WorkflowRole {
                organization: journal.organization,
                branch: journal.branch,
                role: DesktopAccountRole::Manager,
            },
            reception.session.user_id,
            correlation(),
            now(),
        )
        .await
        != Err(eitmad_control_plane::IdentityError::Denied)
    {
        return Err("receptionist provisioning denial");
    }
    let manager_client = local_client(&root, &settings, &journal, manager, true)?;
    let reception_client = local_client(&root, &settings, &journal, reception, false)?;
    let seeded = journal.seeded;
    checked(
        tokio::task::spawn_blocking(move || {
            if !seeded {
                seed::run(&manager_client, &reception_client)?;
            }
            seed::verify(&manager_client, &reception_client)
        })
        .await,
        "seed worker",
    )??;
    journal.seeded = true;
    save(&root, &journal)
}

async fn load_or_bootstrap(root: &Path, control: &ControlPlane) -> Result<Journal> {
    let journal_path = root.join("provision.json");
    let journal: Journal = if journal_path.exists() {
        checked(
            serde_json::from_slice(&checked(fs::read(&journal_path), "read journal")?),
            "journal",
        )?
    } else {
        let bootstrap = checked(
            control
                .identity
                .bootstrap(
                    &BootstrapInput {
                        tenant_code: checked(TenantCode::parse("workflow-e2e"), "tenant code")?,
                        tenant_display_name: "اختبار الاعتماد".into(),
                        organization_display_name: "مصنع الأثاث التجريبي".into(),
                        owner_username: "manager".into(),
                    },
                    correlation(),
                    now(),
                )
                .await,
            "bootstrap empty test database",
        )?;
        let j = Journal {
            tenant: bootstrap.tenant_id,
            organization: bootstrap.organization_id,
            branch: ScopeId::new(Uuid::new_v4()),
            owner_invite: Some(bootstrap.invite_token),
            reception_invite: None,
            seeded: false,
        };
        save(root, &j)?;
        j
    };
    Ok(journal)
}

async fn receptionist(
    root: &Path,
    settings: &Settings,
    control: &ControlPlane,
    journal: &mut Journal,
    manager: &AuthenticationResult,
) -> Result<AuthenticationResult> {
    // A login attempt distinguishes an activated account from a pending invitation on retry.
    match authenticate(
        control,
        "receptionist",
        &settings.receptionist_password,
        settings.receptionist_device,
        &settings.receptionist_seed,
        journal.reception_invite.as_deref(),
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(_) if journal.reception_invite.is_none() && !journal.seeded => {
            let invite = checked(
                control
                    .identity
                    .invite_workflow_account(
                        &manager.session,
                        &WorkflowRole {
                            organization: journal.organization,
                            branch: journal.branch,
                            role: DesktopAccountRole::Receptionist,
                        },
                        "receptionist",
                        correlation(),
                        now(),
                    )
                    .await,
                "invite receptionist",
            )?;
            journal.reception_invite = Some(invite.activation_token);
            save(root, journal)?;
            authenticate(
                control,
                "receptionist",
                &settings.receptionist_password,
                settings.receptionist_device,
                &settings.receptionist_seed,
                journal.reception_invite.as_deref(),
            )
            .await
        }
        Err(stage) => Err(stage),
    }
}

async fn authenticate(
    control: &ControlPlane,
    username: &str,
    password: &str,
    device: DeviceId,
    seed: &[u8; 32],
    invite: Option<&str>,
) -> Result<AuthenticationResult> {
    let signing = SigningKey::from_bytes(seed);
    let public = DevicePublicKey {
        algorithm: "ed25519".into(),
        base64: URL_SAFE_NO_PAD.encode(signing.verifying_key().as_bytes()),
    };
    // Login first permits recovery after activation committed but before the journal update.
    let issued_at = now();
    let nonce = Uuid::new_v4().to_string();
    let message = format!("{}\n{}\n{nonce}", device.value(), issued_at.0);
    let login = LoginRequest {
        tenant_code: checked(TenantCode::parse("workflow-e2e"), "tenant code")?,
        username: username.into(),
        password: password.into(),
        device_id: device,
        device_label: "جهاز اختبار".into(),
        device_public_key: public.clone(),
        device_proof: Some(DeviceProof {
            device_id: device,
            nonce,
            issued_at,
            signature_base64: URL_SAFE_NO_PAD.encode(signing.sign(message.as_bytes()).to_bytes()),
        }),
    };
    if let Ok(result) = control
        .authentication
        .login(&login, correlation(), now())
        .await
    {
        return Ok(result);
    }
    let token = invite.ok_or("server account login")?;
    checked(
        control
            .authentication
            .activate(
                &ActivateAccountRequest {
                    invite_token: token.into(),
                    password: password.into(),
                    device_id: device,
                    device_label: "جهاز اختبار".into(),
                    device_public_key: public,
                },
                correlation(),
                now(),
            )
            .await,
        "activate server account",
    )
}

struct Client {
    store: AuthorityStore,
    actor: AuthorizationContext,
    branch: AuthorizationContext,
    catalog: Arc<DirectCatalogSyncClient>,
    prices: Arc<DirectPriceClient>,
    workflow: DirectDiscountApprovalClient,
    secrets: SecretStore,
    credential: SecretId,
    customer_config: DirectServerConfig,
    account: eitmad_contracts::identity::AccountId,
    device: DeviceId,
}
fn local_client(
    root: &Path,
    settings: &Settings,
    journal: &Journal,
    authentication: AuthenticationResult,
    manager: bool,
) -> Result<Client> {
    let (name, password, seed, role) = if manager {
        (
            "manager",
            &settings.manager_password,
            settings.manager_seed,
            DesktopRole::Manager,
        )
    } else {
        (
            "receptionist",
            &settings.receptionist_password,
            settings.receptionist_seed,
            DesktopRole::Receptionist,
        )
    };
    let directory = root.join(name);
    let (store, actor, branch) =
        import_account(&directory, journal, &authentication, name, password, role)?;
    let secrets = checked(
        SecretStore::open(&directory, None),
        "Windows credential store",
    )?;
    let credential = SecretId::new(
        checked(SecretKind::parse("workflow-e2e"), "credential kind")?,
        SecretReferenceId::new(authentication.session.device_id.value()),
    );
    let account = authentication.session.account_id;
    let device = authentication.session.device_id;
    checked(
        store_session(&secrets, &credential, authentication, seed),
        "persist authenticated route",
    )?;
    checked(
        fs::write(
            directory.join("route.json"),
            checked(serde_json::to_vec(&credential), "credential reference")?,
        ),
        "write non-secret route reference",
    )?;
    let config = checked(
        DirectServerConfig::new(
            &settings.endpoint,
            scope("organization", journal.organization.value())?,
            checked(
                SchemaId::parse("eitmad.schema.pricing.v1"),
                "pricing schema",
            )?,
            1,
            &root.join("certificates/trusted-ca.pem"),
        ),
        "TLS route",
    )?;
    Ok(Client {
        store: store.clone(),
        actor,
        branch,
        catalog: Arc::new(DirectCatalogSyncClient::from_config(
            config.clone(),
            secrets.clone(),
            credential.clone(),
            store,
        )),
        prices: Arc::new(DirectPriceClient::from_config(
            config.clone(),
            secrets.clone(),
            credential.clone(),
        )),
        workflow: DirectDiscountApprovalClient::from_config(
            config.clone(),
            secrets.clone(),
            credential.clone(),
            scope("branch", journal.branch.value())?,
        ),
        customer_config: checked(
            DirectServerConfig::new(
                &settings.endpoint,
                scope("branch", journal.branch.value())?,
                checked(
                    SchemaId::parse("eitmad.schema.customer.v1"),
                    "customer schema",
                )?,
                1,
                &root.join("certificates/trusted-ca.pem"),
            ),
            "customer TLS route",
        )?,
        secrets,
        credential,
        account,
        device,
    })
}

fn import_account(
    directory: &Path,
    journal: &Journal,
    authentication: &AuthenticationResult,
    name: &str,
    password: &str,
    role: DesktopRole,
) -> Result<(AuthorityStore, AuthorizationContext, AuthorizationContext)> {
    let store = checked(AuthorityStore::open(directory), "open client authority")?;
    let installer = checked(
        store.initialize_server_installation(
            journal.tenant,
            journal.organization,
            authentication.session.device_id,
            journal.branch,
            now(),
        ),
        "import server installation",
    )?;
    let salt = checked(
        SaltString::encode_b64(Uuid::new_v4().as_bytes()),
        "password salt",
    )?;
    let hash = checked(
        Argon2::default().hash_password(password.as_bytes(), &salt),
        "desktop credential",
    )?
    .to_string();
    checked(
        store.provision_desktop_account(
            &installer,
            &DesktopAccount {
                account_id: authentication.session.account_id,
                user_id: authentication.session.user_id,
                tenant_id: journal.tenant,
                organization_id: journal.organization,
                password_hash: hash,
                role,
            },
            name,
            now(),
        ),
        "import verified account",
    )?;
    let mut process = installer;
    process.identity.principal_kind = PrincipalKind::Device;
    process.identity.principal_id = PrincipalId::new(authentication.session.device_id.value());
    let signed_in = checked(
        DesktopAuthenticator::new(store.clone()).sign_in(
            &process,
            name,
            password,
            correlation(),
            now(),
        ),
        "desktop sign in",
    )?;
    let actor = signed_in.authorization.ok_or("desktop authorization")?;
    let branch = signed_in
        .customer_authorization
        .ok_or("desktop branch authorization")?;
    Ok((store, actor, branch))
}

fn load_settings() -> Result<(PathBuf, Settings)> {
    let root = PathBuf::from(checked(
        env::var_os("EITMAD_E2E_ROOT").ok_or(()),
        "state directory",
    )?);
    if checked(
        fs::read_to_string(root.join("disposable-eitmad-e2e")),
        "disposable state marker",
    )? != "eitmad-workflow-e2e-v1"
    {
        return Err("disposable state marker");
    }
    let settings: Settings = checked(
        serde_json::from_slice(&checked(
            fs::read(root.join("secrets.json")),
            "read private settings",
        )?),
        "private settings",
    )?;
    Ok((root, settings))
}
