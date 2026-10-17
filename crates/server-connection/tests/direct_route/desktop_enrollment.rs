use super::*;
use eitmad_contracts::ipc::{DesktopEnrollmentSettings, DesktopSignInRequest};
use eitmad_server_connection::{DesktopConnectionSettings, DesktopEnrollment};

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
    let server = provision_server(
        &database,
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    let inputs = DesktopEnrollmentSettings {
        endpoint: format!("https://localhost:{}/", server.address.port()),
        trusted_certificate: required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE")
            .to_string_lossy()
            .into_owned(),
        tenant_code: TenantCode::parse("direct-test").unwrap(),
        organization: OrganizationId::new(server.scope.id.value()),
        branch: server.branch_scope.id,
    };
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
    let device = first.device();
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
    let second = Installation::new();
    assert!(second.sign_in(request(Some(inputs))).await.unwrap());
    assert_ne!(first.device(), second.device());
    assert_eq!(
        first.store.local_organization_id().unwrap(),
        second.store.local_organization_id().unwrap()
    );
    verify_windows_shell(&inputs_for_shell(&server)).await;
    server
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
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

async fn verify_windows_shell(inputs: &DesktopEnrollmentSettings) {
    let Some(engine) = env::var_os("EITMAD_DESKTOP_ENROLLMENT_ENGINE") else {
        return;
    };
    let desktop = Installation::new();
    let directory = desktop.directory.path().to_owned();
    let input = serde_json::to_string(inputs).unwrap();
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new("dotnet")
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .args([
                "test",
                "shells/windows/tests/Eitmad.WindowsShell.Tests.csproj",
                "--configuration",
                "Release",
                "--no-build",
                "--nologo",
                "--filter",
                "FullyQualifiedName~FreshDesktopEnrollsReopensAndReauthenticates",
            ])
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
    wrong_password.password = "incorrect-test-password".into();
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
