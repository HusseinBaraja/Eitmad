use super::*;
use eitmad_contracts::ipc::{DesktopEnrollmentSettings, DesktopSignInRequest};
use eitmad_server_connection::{DesktopConnectionSettings, DesktopEnrollment};

#[path = "idle_customer_sync.rs"]
mod idle_customer_sync;

const PASSWORD: &str = "synthetic-test-password-123";

struct Installation {
    directory: tempfile::TempDir,
    store: AuthorityStore,
    secrets: SecretStore,
}
impl Installation {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let secrets =
            SecretStore::open(directory.path(), Some(FallbackEncryptionKey::new([73; 32])))
                .unwrap();
        Self {
            directory,
            store,
            secrets,
        }
    }
    async fn sign_in(
        &self,
        request: DesktopSignInRequest,
    ) -> Result<bool, eitmad_sync::TransportFailure> {
        let enrollment = DesktopEnrollment::new(
            self.directory.path().into(),
            self.store.clone(),
            self.secrets.clone(),
        );
        tokio::task::spawn_blocking(move || enrollment.sign_in(&request))
            .await
            .unwrap()
    }
    async fn pair(&self, server: &ProvisionedServer) {
        self.try_pair(server).await.unwrap();
    }
    async fn try_pair(
        &self,
        server: &ProvisionedServer,
    ) -> Result<(), eitmad_sync::TransportFailure> {
        use eitmad_contracts::{discovery::DiscoveredServer, pairing::OpenPairing};
        let code = if self
            .directory
            .path()
            .join("desktop-pairing-pending.json")
            .exists()
        {
            String::new()
        } else {
            server
                .control
                .pairing
                .open(
                    &server.authentication.session,
                    &server.pairing_identity,
                    &OpenPairing {
                        version: 1,
                        organization_id: OrganizationId::new(server.scope.id.value()),
                        branch_id: server.branch_scope.id,
                    },
                    CorrelationId::new(Uuid::new_v4()),
                    eitmad_control_plane::unix_millis_now(),
                )
                .await
                .unwrap()
                .code
        };
        let candidate = DiscoveredServer {
            server_id: server.pairing_identity.server_id,
            address: server.address.ip().to_string(),
            port: server.address.port(),
            expires_at: UnixMillis(eitmad_control_plane::unix_millis_now().0 + 60_000),
            identity_conflict: false,
        };
        let enrollment = DesktopEnrollment::new(
            self.directory.path().into(),
            self.store.clone(),
            self.secrets.clone(),
        );
        tokio::task::spawn_blocking(move || enrollment.pair(&candidate, &code).map(|_| ()))
            .await
            .unwrap()
    }
    fn settings(&self) -> DesktopConnectionSettings {
        DesktopConnectionSettings::load(self.directory.path())
            .unwrap()
            .unwrap()
    }
    fn device(&self) -> DeviceId {
        serde_json::from_slice(
            &std::fs::read(self.directory.path().join("desktop-device.json")).unwrap(),
        )
        .unwrap()
    }
    fn credential(&self, user: eitmad_contracts::identity::UserId) -> SecretId {
        eitmad_server_connection::account_credential_id(
            &self.settings().credential,
            PrincipalId::new(user.value()),
        )
    }
}
impl Drop for Installation {
    fn drop(&mut self) {
        let Ok(bytes) = std::fs::read(self.directory.path().join("desktop-device.json")) else {
            return;
        };
        let device: DeviceId = serde_json::from_slice(&bytes).unwrap();
        let id = SecretId::new(
            SecretKind::parse("desktop-device-key").unwrap(),
            SecretReferenceId::new(device.value()),
        );
        self.secrets.delete(&id).unwrap();
        if let Ok(Some(settings)) = DesktopConnectionSettings::load(self.directory.path()) {
            let installer = self.store.existing_local_authorization_context().unwrap();
            for account in self
                .store
                .list_desktop_accounts(installer.tenant_id)
                .unwrap()
                .accounts
            {
                self.secrets
                    .delete(&eitmad_server_connection::account_credential_id(
                        &settings.credential,
                        PrincipalId::new(account.user_id.value()),
                    ))
                    .unwrap();
            }
        }
    }
}

fn request(connection: Option<DesktopEnrollmentSettings>) -> DesktopSignInRequest {
    DesktopSignInRequest {
        request_id: eitmad_contracts::transport::RequestId::new(Uuid::new_v4()),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        username: "owner".into(),
        password: PASSWORD.into(),
        server_login: true,
        connection,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn desktop_enrollment_recovers_interruption_expiry_and_credential_loss() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let mut server = provision_server(
        &database,
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    let inputs = inputs_for_shell(&server);
    let first = Installation::new();
    let mut wrong_trust = inputs.clone();
    wrong_trust.trusted_certificate = required_path("EITMAD_DIRECT_TEST_WRONG_CERTIFICATE")
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        first
            .sign_in(request(Some(wrong_trust)))
            .await
            .unwrap_err()
            .kind,
        TransportFailureKind::EncryptionRequired
    );
    assert!(!first.store.has_local_installation().unwrap());
    assert!(
        DesktopConnectionSettings::load(first.directory.path())
            .unwrap()
            .is_none()
    );
    // Interrupt local persistence after the server has durably approved this device.
    let pairing_blocker = first.directory.path().join("desktop-pairing-ca.pem");
    std::fs::create_dir(&pairing_blocker).unwrap();
    assert!(first.try_pair(&server).await.is_err());
    let device = first.device();
    assert!(!first.store.has_local_installation().unwrap());
    std::fs::remove_dir(&pairing_blocker).unwrap();
    first.pair(&server).await;
    assert_eq!(first.device(), device);
    // Fail after authenticated import and credential storage, before the settings commit.
    let blocker = first.directory.path().join("desktop-trust.pem");
    std::fs::create_dir(&blocker).unwrap();
    assert!(first.sign_in(request(Some(inputs.clone()))).await.is_err());
    assert!(first.store.has_local_installation().unwrap());
    assert!(
        DesktopConnectionSettings::load(first.directory.path())
            .unwrap()
            .is_none()
    );
    std::fs::remove_dir(&blocker).unwrap();
    assert!(first.sign_in(request(Some(inputs.clone()))).await.unwrap());
    assert_eq!(
        first.device(),
        device,
        "Interrupted setup must retain device identity"
    );
    assert_retained_recovery(&first, &server, &inputs).await;
    assert_server_restart(&first, &mut server).await;
    let second = Installation::new();
    second.pair(&server).await;
    // Native onboarding retains the authenticated pairing; sign-in needs no copied settings.
    assert!(second.sign_in(request(None)).await.unwrap());
    assert_ne!(first.device(), second.device());
    assert_eq!(
        first.store.local_organization_id().unwrap(),
        second.store.local_organization_id().unwrap()
    );
    verify_windows_shell(&server).await;
    server
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
}

async fn assert_server_restart(installation: &Installation, server: &mut ProvisionedServer) {
    server.handle.handle.shutdown();
    let deadline = Instant::now() + Duration::from_secs(3);
    while std::net::TcpStream::connect_timeout(&server.address, Duration::from_millis(25)).is_ok() {
        assert!(Instant::now() < deadline, "Synthetic server did not stop");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    server.handle = start_server(
        server.address,
        server.state.clone(),
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    assert_server_access(installation, &local_sign_in(installation), true).await;
}

fn inputs_for_shell(server: &ProvisionedServer) -> DesktopEnrollmentSettings {
    DesktopEnrollmentSettings {
        endpoint: format!("https://localhost:{}/", server.address.port()),
        trusted_certificate: required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE")
            .to_string_lossy()
            .into_owned(),
        tenant_code: TenantCode::parse("direct-test").unwrap(),
        organization: OrganizationId::new(server.scope.id.value()),
        branch: server.branch_scope.id,
    }
}

fn windows_shell_test(name: &str) -> std::process::Command {
    let mut command = std::process::Command::new("dotnet");
    command
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args([
            "test",
            "shells/windows/tests/Eitmad.WindowsShell.Tests.csproj",
            "--configuration",
            "Release",
            "--no-build",
            "--nologo",
            "--filter",
        ])
        .arg(format!("FullyQualifiedName~{name}"));
    command
}

async fn verify_windows_shell(server: &ProvisionedServer) {
    let inputs = inputs_for_shell(server);
    let Some(engine) = env::var_os("EITMAD_DESKTOP_ENROLLMENT_ENGINE") else {
        return;
    };
    let desktop = Installation::new();
    desktop.pair(server).await;
    let directory = desktop.directory.path().to_owned();
    let input = serde_json::to_string(&inputs).unwrap();
    let result = tokio::task::spawn_blocking(move || {
        windows_shell_test("FreshDesktopEnrollsReopensAndReauthenticates")
            .env("EITMAD_DESKTOP_ENROLLMENT_INPUT", input)
            .env("EITMAD_DESKTOP_ENROLLMENT_ENGINE", engine)
            .env("EITMAD_DESKTOP_ENROLLMENT_DIRECTORY", directory)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(
        result.status.success(),
        "Windows enrollment failed: {}",
        String::from_utf8_lossy(&result.stdout)
    );
}

async fn assert_retained_recovery(
    installation: &Installation,
    server: &ProvisionedServer,
    inputs: &DesktopEnrollmentSettings,
) {
    let settings_bytes = std::fs::read(
        installation
            .directory
            .path()
            .join(DesktopConnectionSettings::FILE_NAME),
    )
    .unwrap();
    let account = installation
        .store
        .desktop_account(server.authentication.session.tenant_id, "owner")
        .unwrap()
        .unwrap();
    let actor = local_sign_in(installation);
    let auth = AuthorizationService::new(installation.store.clone());
    eitmad_material::MaterialService::new(installation.store.clone(), auth)
        .save_category(
            &MutationContext {
                authorization: actor.clone(),
                correlation_id: CorrelationId::new(Uuid::new_v4()),
                causation_id: None,
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                occurred_at: eitmad_authorization::now(),
            },
            &eitmad_contracts::material::SaveMaterialCategory {
                id: None,
                expected_revision: None,
                name: "أخشاب محفوظة".into(),
                archived: false,
            },
        )
        .unwrap();
    let before = saved_work(&installation.store);
    assert!(before.1 > 0, "The scenario must contain pending work");
    let mut wrong_password = request(None);
    wrong_password.password = "synthetic-incorrect-password".into();
    assert!(installation.sign_in(wrong_password).await.is_err());
    let mut foreign = inputs.clone();
    foreign.organization = OrganizationId::new(Uuid::new_v4());
    assert!(installation.sign_in(request(Some(foreign))).await.is_err());
    assert_eq!(saved_work(&installation.store), before);
    assert_eq!(
        std::fs::read(
            installation
                .directory
                .path()
                .join(DesktopConnectionSettings::FILE_NAME)
        )
        .unwrap(),
        settings_bytes
    );
    let credential = installation.credential(account.user_id);
    let material = installation.secrets.get(&credential).unwrap().unwrap();
    let mut expired: serde_json::Value = serde_json::from_slice(material.expose_secret()).unwrap();
    expired["accessExpiresAt"] = serde_json::json!(0);
    expired["refreshExpiresAt"] = serde_json::json!(1);
    installation
        .secrets
        .set(
            &credential,
            eitmad_secret_storage::SecretMaterial::new(serde_json::to_vec(&expired).unwrap())
                .unwrap(),
        )
        .unwrap();
    assert_server_access(installation, &actor, false).await;
    assert!(!installation.sign_in(request(None)).await.unwrap());
    assert_eq!(saved_work(&installation.store), before);
    assert_server_access(installation, &local_sign_in(installation), true).await;
    installation.secrets.delete(&credential).unwrap();
    assert_server_access(installation, &local_sign_in(installation), false).await;
    assert!(!installation.sign_in(request(None)).await.unwrap());
    assert_eq!(saved_work(&installation.store), before);
    assert_server_access(installation, &local_sign_in(installation), true).await;
    assert_eq!(
        installation
            .store
            .desktop_account(actor.tenant_id, "owner")
            .unwrap()
            .unwrap()
            .account_id,
        account.account_id
    );
}

fn local_sign_in(installation: &Installation) -> AuthorizationContext {
    let mut process = installation
        .store
        .existing_local_authorization_context()
        .unwrap();
    let device = process.identity.device_id.unwrap();
    process.identity.principal_kind = PrincipalKind::Device;
    process.identity.principal_id = PrincipalId::new(device.value());
    eitmad_engine_runtime::DesktopAuthenticator::new(installation.store.clone())
        .sign_in(
            &process,
            "owner",
            PASSWORD,
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .unwrap()
        .authorization
        .unwrap()
}

fn saved_work(store: &AuthorityStore) -> (i64, i64) {
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    (
        connection
            .query_row("SELECT COUNT(*) FROM material_categories", [], |r| r.get(0))
            .unwrap(),
        connection
            .query_row("SELECT COUNT(*) FROM publication_outbox", [], |r| r.get(0))
            .unwrap(),
    )
}

async fn assert_server_access(
    installation: &Installation,
    actor: &AuthorizationContext,
    expected: bool,
) {
    let settings = installation.settings();
    let config = DirectServerConfig::new(
        &settings.endpoint,
        ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(settings.organization.value()),
        },
        schema_id(),
        1,
        &settings.trusted_certificate,
    )
    .unwrap();
    let driver = DirectServerDriver::new(config, installation.secrets.clone(), hello());
    let endpoint = format!(
        "https://localhost:{}/",
        url::Url::parse(&settings.endpoint).unwrap().port().unwrap()
    );
    let mut transport = WanAdapter::new(
        eitmad_sync::WanEndpoint {
            server: endpoint,
            relay: None,
        },
        driver,
        hello(),
        TransportAuthentication::AccountDevice {
            account_id: installation
                .store
                .desktop_account(actor.tenant_id, "owner")
                .unwrap()
                .unwrap()
                .account_id,
            device_id: installation.device(),
            credential: installation.credential(eitmad_contracts::identity::UserId::new(
                actor.identity.principal_id.value(),
            )),
        },
        RetryPolicy::default(),
    )
    .unwrap();
    let connected =
        tokio::task::spawn_blocking(move || transport.connect(eitmad_authorization::now()).is_ok())
            .await
            .unwrap();
    assert_eq!(connected, expected);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires disposable PostgreSQL, TLS certificates, and the current Windows engine and shell tests"]
async fn shared_accounts_revoke_two_live_engines_and_preserve_offline_authorship() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let certificate = required_path("EITMAD_DIRECT_TEST_CERTIFICATE");
    let private_key = required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY");
    let server = provision_server(&database, &certificate, &private_key).await;
    let inputs = serde_json::to_string(&inputs_for_shell(&server)).unwrap();
    let first = Installation::new();
    first.pair(&server).await;
    let second = Installation::new();
    second.pair(&server).await;
    let signals = tempfile::tempdir().unwrap();
    let root = signals.path().to_owned();
    let first_path = first.directory.path().to_owned();
    let second_path = second.directory.path().to_owned();
    let signal_path = root.clone();
    let engine =
        env::var_os("EITMAD_DESKTOP_ENROLLMENT_ENGINE").expect("set the current engine executable");
    let mut shell = tokio::task::spawn_blocking(move || {
        windows_shell_test("TwoEnginesShareAccountsAndRevokeCachedAccessWithoutRestart")
            .env("EITMAD_SHARED_ACCOUNT_INPUT", inputs)
            .env("EITMAD_SHARED_ACCOUNT_DIRECTORY", signal_path)
            .env("EITMAD_SHARED_ACCOUNT_FIRST", first_path)
            .env("EITMAD_SHARED_ACCOUNT_SECOND", second_path)
            .env("EITMAD_DESKTOP_ENROLLMENT_ENGINE", engine)
            .output()
            .unwrap()
    });
    wait_shared_phase(&root, "ready-offline", &mut shell).await;
    let retained_categories = saved_work(&second.store).0;
    server.handle.handle.shutdown();
    tokio::time::sleep(Duration::from_millis(250)).await;
    std::fs::write(root.join("server-offline"), b"ready").unwrap();
    wait_shared_phase(&root, "pending-created", &mut shell).await;
    let author = downgrade_offline_author(&database, &server, &second.store).await;
    let restarted = start_server(
        server.address,
        server.state.clone(),
        &certificate,
        &private_key,
    )
    .await;
    std::fs::write(root.join("server-online"), b"ready").unwrap();
    let result = shell.await.unwrap();
    assert!(
        result.status.success(),
        "Shared account clients failed: {}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert_ne!(first.device(), second.device());
    assert!(retained_categories >= 2);
    assert_eq!(
        saved_work(&second.store).0,
        retained_categories,
        "Retained records survive and denied offline writes add no records"
    );
    assert!(
        second
            .store
            .pending_catalog_actors()
            .unwrap()
            .contains(&author),
        "Pending work must retain the exact original authorization context"
    );
    assert!(
        !eitmad_engine_runtime::DesktopAuthenticator::new(second.store.clone())
            .active(&author, eitmad_authorization::now())
            .unwrap()
    );
    restarted.handle.shutdown();
}

async fn downgrade_offline_author(
    database: &str,
    server: &ProvisionedServer,
    store: &AuthorityStore,
) -> AuthorizationContext {
    let control = ControlPlane::new(
        ControlDatabase::connect(database, 4).await.unwrap().pool(),
        TokenKey::new([9; 32]),
    );
    let page = control
        .identity
        .list_accounts(
            &server.authentication.session,
            &eitmad_contracts::server_accounts::ListServerAccounts {
                after: None,
                limit: 100,
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
    let account = page
        .accounts
        .into_iter()
        .find(|a| a.username == "shared.operator")
        .unwrap();
    let pending_before = store.pending_catalog_actors().unwrap();
    let author = pending_before
        .iter()
        .find(|a| a.identity.principal_id.value() == account.user_id.value())
        .unwrap()
        .clone();
    control
        .identity
        .change_account(
            &server.authentication.session,
            &eitmad_contracts::server_accounts::ChangeServerAccount {
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                change: eitmad_contracts::server_accounts::ServerAccountChange::Assign {
                    account_id: account.account_id,
                    expected_revision: account.revision,
                    organization_id: OrganizationId::new(server.scope.id.value()),
                    assignments: vec![eitmad_contracts::server_accounts::ServerRoleAssignment {
                        role: eitmad_contracts::accounts::DesktopAccountRole::Receptionist,
                        branch_ids: vec![server.branch_scope.id],
                    }],
                },
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
    author
}

async fn wait_shared_phase(
    root: &Path,
    name: &str,
    shell: &mut tokio::task::JoinHandle<std::process::Output>,
) {
    let deadline = Instant::now() + Duration::from_secs(90);
    while !root.join(name).exists() {
        if shell.is_finished() {
            let result = shell.await.unwrap();
            panic!(
                "Windows client stopped before phase {name}: {} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        assert!(
            Instant::now() < deadline,
            "Windows client did not reach phase {name}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
