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
    server_accounts::{
        ChangeServerAccount, ListServerAccounts, ServerAccountChange, ServerRoleAssignment,
    },
    transport::{CorrelationId, IdempotencyKey, SchemaId, UnixMillis},
};
use eitmad_control_plane::{BootstrapInput, ControlDatabase, ControlPlane, TokenKey};
use eitmad_engine_runtime::DesktopAuthenticator;
use eitmad_secret_storage::SecretStore;
use eitmad_server_connection::{
    DirectCatalogSyncClient, DirectCustomerSyncClient, DirectDiscountApprovalClient,
    DirectPriceClient, DirectServerConfig, store_account_session,
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
#[cfg(test)]
#[path = "workflow_environment/tests.rs"]
mod tests;

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
    if journal.seeded {
        return prepare_shared_desktop(&root, &settings, &journal, &control, false).await;
    }
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
    let manager = provision_manager(&control, &journal, &settings, manager).await?;
    let reception = receptionist(&root, &settings, &control, &mut journal, &manager).await?;
    journal.reception_invite = None;
    save(&root, &journal)?;
    // A Receptionist must never be able to provision another account.
    if control
        .identity
        .change_account(
            &reception.session,
            &ChangeServerAccount {
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                change: ServerAccountChange::RevokeSessions {
                    account_id: reception.session.account_id,
                    expected_revision: 2,
                },
            },
            correlation(),
            now(),
        )
        .await
        .err()
        != Some(eitmad_control_plane::ServerAccountError::Denied)
    {
        return Err("receptionist provisioning denial");
    }
    let manager_client = local_client(&root, &settings, &journal, manager, true, false)?;
    let reception_client = local_client(&root, &settings, &journal, reception, false, false)?;
    checked(
        tokio::task::spawn_blocking(move || {
            seed::run(&manager_client, &reception_client)?;
            seed::verify(&manager_client, &reception_client)
        })
        .await,
        "seed worker",
    )??;
    journal.seeded = true;
    save(&root, &journal)?;
    prepare_shared_desktop(&root, &settings, &journal, &control, true).await
}

async fn prepare_shared_desktop(
    root: &Path,
    settings: &Settings,
    journal: &Journal,
    control: &ControlPlane,
    verify: bool,
) -> Result<()> {
    // Retain the existing Receptionist database, including local drafts and audit history.
    // Register both accounts on its device through the normal authenticated server operation.
    let manager = authenticate(
        control,
        "manager",
        &settings.manager_password,
        settings.receptionist_device,
        &settings.receptionist_seed,
        None,
    )
    .await?;
    let reception = authenticate(
        control,
        "receptionist",
        &settings.receptionist_password,
        settings.receptionist_device,
        &settings.receptionist_seed,
        None,
    )
    .await?;
    if !verify {
        let directory = root.join("receptionist");
        for (authentication, name, password, role) in [
            (
                manager,
                "manager",
                &settings.manager_password,
                DesktopRole::Manager,
            ),
            (
                reception,
                "receptionist",
                &settings.receptionist_password,
                DesktopRole::Receptionist,
            ),
        ] {
            import_account(&directory, journal, &authentication, name, password, role)?;
            let (_, credential) =
                persist_route(&directory, authentication, settings.receptionist_seed)?;
            persist_connection(root, &directory, settings, journal, credential)?;
        }
        return Ok(());
    }
    let manager = local_client(root, settings, journal, manager, true, true)?;
    let reception = local_client(root, settings, journal, reception, false, true)?;
    checked(
        tokio::task::spawn_blocking(move || seed::verify(&manager, &reception)).await,
        "shared desktop verification worker",
    )?
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

async fn provision_manager(
    control: &ControlPlane,
    journal: &Journal,
    settings: &Settings,
    manager: AuthenticationResult,
) -> Result<AuthenticationResult> {
    let page = checked(
        control
            .identity
            .list_accounts(
                &manager.session,
                &ListServerAccounts {
                    after: None,
                    limit: 100,
                },
                correlation(),
                now(),
            )
            .await,
        "list accounts",
    )?;
    let current = page
        .accounts
        .into_iter()
        .find(|a| a.account_id == manager.session.account_id)
        .ok_or("owner account")?;
    if current.organizations.iter().any(|o| {
        o.organization_id == journal.organization
            && o.assignments
                .iter()
                .any(|a| a.role == DesktopAccountRole::Manager)
    }) {
        return Ok(manager);
    }
    checked(
        control
            .identity
            .change_account(
                &manager.session,
                &ChangeServerAccount {
                    idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                    change: ServerAccountChange::Assign {
                        account_id: current.account_id,
                        expected_revision: current.revision,
                        organization_id: journal.organization,
                        assignments: vec![ServerRoleAssignment {
                            role: DesktopAccountRole::Manager,
                            branch_ids: vec![journal.branch],
                        }],
                    },
                },
                correlation(),
                now(),
            )
            .await,
        "assign manager role",
    )?;
    // Role assignment revokes the old session. Sign in again before provisioning or sync.
    authenticate(
        control,
        "manager",
        &settings.manager_password,
        settings.manager_device,
        &settings.manager_seed,
        None,
    )
    .await
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
                    .change_account(
                        &manager.session,
                        &ChangeServerAccount {
                            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                            change: ServerAccountChange::Invite {
                                organization_id: journal.organization,
                                username: "receptionist".into(),
                                assignments: vec![ServerRoleAssignment {
                                    role: DesktopAccountRole::Receptionist,
                                    branch_ids: vec![journal.branch],
                                }],
                            },
                        },
                        correlation(),
                        now(),
                    )
                    .await,
                "invite receptionist",
            )?;
            journal.reception_invite = invite.activation_token;
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
    customers: DirectCustomerSyncClient,
}
fn local_client(
    root: &Path,
    settings: &Settings,
    journal: &Journal,
    authentication: AuthenticationResult,
    manager: bool,
    shared: bool,
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
    let seed = if shared {
        settings.receptionist_seed
    } else {
        seed
    };
    let directory = root.join(if shared { "receptionist" } else { name });
    let (store, installer) =
        import_account(&directory, journal, &authentication, name, password, role)?;
    let (actor, branch) = sign_in(&store, installer, name, password)?;
    let (secrets, credential) = persist_route(&directory, authentication, seed)?;
    persist_connection(root, &directory, settings, journal, credential.clone())?;
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
            store.clone(),
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
        customers: DirectCustomerSyncClient::from_config(
            checked(
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
            store,
        ),
    })
}

fn persist_connection(
    root: &Path,
    directory: &Path,
    settings: &Settings,
    journal: &Journal,
    credential: SecretId,
) -> Result<()> {
    let connection = eitmad_server_connection::DesktopConnectionSettings {
        server_id: None,
        version: 1,
        endpoint: settings.endpoint.clone(),
        trusted_certificate: checked(
            root.join("certificates/trusted-ca.pem").canonicalize(),
            "connection trust anchor",
        )?,
        credential,
        organization: journal.organization,
        branch: journal.branch,
        tenant_code: Some(checked(TenantCode::parse("workflow-e2e"), "tenant code")?),
    };
    let bytes = checked(
        serde_json::to_vec(&connection),
        "encode connection settings",
    )?;
    let pending = directory.join("connection.next");
    checked(fs::write(&pending, bytes), "write connection settings")?;
    checked(
        fs::rename(
            pending,
            directory.join(eitmad_server_connection::DesktopConnectionSettings::FILE_NAME),
        ),
        "commit connection settings",
    )
}

fn persist_route(
    directory: &Path,
    authentication: AuthenticationResult,
    seed: [u8; 32],
) -> Result<(SecretStore, SecretId)> {
    let secrets = checked(
        SecretStore::open(directory, None),
        "Windows credential store",
    )?;
    let credential = SecretId::new(
        checked(SecretKind::parse("workflow-e2e"), "credential kind")?,
        SecretReferenceId::new(authentication.session.device_id.value()),
    );
    checked(
        store_account_session(&secrets, &credential, authentication, seed),
        "persist authenticated route",
    )?;
    Ok((secrets, credential))
}

fn import_account(
    directory: &Path,
    journal: &Journal,
    authentication: &AuthenticationResult,
    name: &str,
    password: &str,
    role: DesktopRole,
) -> Result<(AuthorityStore, AuthorizationContext)> {
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
    import_missing_account(&store, &installer, authentication, name, password, role)?;
    Ok((store, installer))
}

fn import_missing_account(
    store: &AuthorityStore,
    installer: &AuthorizationContext,
    authentication: &AuthenticationResult,
    name: &str,
    password: &str,
    role: DesktopRole,
) -> Result<()> {
    let accounts = checked(
        store.list_desktop_accounts(installer.tenant_id),
        "read imported accounts",
    )?;
    if let Some(existing) = accounts
        .accounts
        .iter()
        .find(|a| a.account_id == authentication.session.account_id || a.username == name)
    {
        if existing.account_id != authentication.session.account_id
            || existing.user_id != authentication.session.user_id
            || existing.username != name
        {
            return Err("imported account identity mismatch");
        }
        return Ok(());
    }
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
            installer,
            &DesktopAccount {
                account_id: authentication.session.account_id,
                user_id: authentication.session.user_id,
                tenant_id: installer.tenant_id,
                organization_id: checked(
                    store.local_organization_id(),
                    "installation organization",
                )?,
                password_hash: hash,
                role,
            },
            name,
            now(),
        ),
        "import verified account",
    )
}

fn sign_in(
    store: &AuthorityStore,
    installer: AuthorizationContext,
    name: &str,
    password: &str,
) -> Result<(AuthorizationContext, AuthorizationContext)> {
    let mut process = installer;
    process.identity.principal_kind = PrincipalKind::Device;
    process.identity.principal_id = PrincipalId::new(
        process
            .identity
            .device_id
            .ok_or("installation device")?
            .value(),
    );
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
    Ok((actor, branch))
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
