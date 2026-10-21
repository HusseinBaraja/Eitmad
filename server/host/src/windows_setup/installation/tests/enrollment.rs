use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer as _, SigningKey};
use eitmad_contracts::{
    config::SecretReferenceId,
    secrets::{SecretId, SecretKind},
    server::{DeviceProof, LoginRequest},
    setup::PreparedWindowsWorkshop,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_control_plane::{
    ControlDatabase, ControlPlane, PairingError, TokenKey, unix_millis_now,
};
use eitmad_postgres_support as sqlx;
use eitmad_secret_storage::{FallbackEncryptionKey, SecretStore};
use eitmad_server_connection::DesktopEnrollment;
use eitmad_storage::AuthorityStore;
use uuid::Uuid;

fn correlation() -> CorrelationId {
    CorrelationId::new(Uuid::new_v4())
}

fn desktop(root: &TestRoot) -> (DesktopEnrollment, SecretStore) {
    let directory = root.0.join("original-user");
    fs::create_dir(&directory).unwrap();
    let store = AuthorityStore::open(&directory).unwrap();
    let secrets =
        SecretStore::open(&directory, Some(FallbackEncryptionKey::new([91; 32]))).unwrap();
    (
        DesktopEnrollment::new(directory, store, secrets.clone()),
        secrets,
    )
}

fn expired_proof(login: &LoginRequest, secrets: &SecretStore) -> DeviceProof {
    let id = SecretId::new(
        SecretKind::parse("desktop-device-key").unwrap(),
        SecretReferenceId::new(login.device_id.value()),
    );
    let material = secrets.get(&id).unwrap().unwrap();
    let key = SigningKey::from_bytes(material.expose_secret().try_into().unwrap());
    let mut proof = login.device_proof.clone().unwrap();
    // Simulate six minutes of UAC/installation without sleeping or changing the OS clock.
    proof.issued_at = UnixMillis(proof.issued_at.0 - 360_000);
    proof.signature_base64 = URL_SAFE_NO_PAD.encode(
        key.sign(
            format!(
                "{}\n{}\n{}",
                proof.device_id.value(),
                proof.issued_at.0,
                proof.nonce
            )
            .as_bytes(),
        )
        .to_bytes(),
    );
    proof
}

struct ProofAfterReady<'a> {
    desktop: &'a DesktopEnrollment,
    original: &'a LoginRequest,
    ready: std::rc::Rc<std::cell::Cell<bool>>,
    bytes: Option<std::io::Cursor<Vec<u8>>>,
}

impl std::io::Read for ProofAfterReady<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let bytes = self.bytes.get_or_insert_with(|| {
            assert!(
                self.ready.get(),
                "proof must be requested after host readiness"
            );
            let proof =
                super::super::super::guided::fresh_first_desktop_proof(self.original, self.desktop)
                    .unwrap();
            assert_eq!(proof.device_id, self.original.device_id);
            assert!(
                proof.issued_at.0 - self.original.device_proof.as_ref().unwrap().issued_at.0
                    >= 360_000
            );
            std::io::Cursor::new(serde_json::to_vec(&proof).unwrap())
        });
        bytes.read(output)
    }
}

async fn prepare_database(
    request: &InstallWindowsServer,
) -> (ControlPlane, PreparedWindowsWorkshop, sqlx::PgPool) {
    let url = Zeroizing::new(
        std::env::var("EITMAD_FIRST_DESKTOP_TEST_DATABASE_URL")
            .expect("fresh disposable database required"),
    );
    let database = ControlDatabase::connect(&url, 2).await.unwrap();
    let pool = database.pool();
    let fresh: bool = sqlx::query_scalar("SELECT to_regnamespace('control') IS NULL AND NOT (SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname=current_user)")
        .fetch_one(&pool).await.unwrap();
    assert!(fresh, "fresh database with enforced RLS required");
    database.migrate().await.unwrap();
    eitmad_sync_plane::SyncDatabase::connect(&url, 1)
        .await
        .unwrap()
        .migrate()
        .await
        .unwrap();
    eitmad_admin_plane::AdminDatabase::connect(&url, 1)
        .await
        .unwrap()
        .migrate()
        .await
        .unwrap();
    eitmad_server_audit::AuditDatabase::from_pool(pool.clone())
        .migrate()
        .await
        .unwrap();
    let control = ControlPlane::new(pool.clone(), TokenKey::new([92; 32]));
    let workshop = control
        .identity
        .initialize_workshop(
            &request.preparation.workshop,
            correlation(),
            unix_millis_now(),
        )
        .await
        .unwrap();
    let prepared = PreparedWindowsWorkshop {
        version: 1,
        endpoint: "https://synthetic.invalid:58443/".into(),
        workshop,
    };
    fs::create_dir_all(Path::new(&request.service_state_directory).join("private")).unwrap();
    fs::write(
        Path::new(&request.service_state_directory).join("private/database-url"),
        url.as_bytes(),
    )
    .unwrap();
    fs::write(
        Path::new(&request.service_state_directory).join("private/trusted-ca.pem"),
        b"synthetic public CA",
    )
    .unwrap();
    fs::create_dir(&request.preparation.state_directory).unwrap();
    fs::write(
        Path::new(&request.preparation.state_directory).join("prepared-workshop.json"),
        serde_json::to_vec(&prepared).unwrap(),
    )
    .unwrap();
    (control, prepared, pool)
}

#[tokio::test]
#[ignore = "requires a fresh disposable EITMAD_FIRST_DESKTOP_TEST_DATABASE_URL with enforced RLS"]
async fn delayed_server_installation_refreshes_proof_and_enrolls_the_same_device() {
    let root = TestRoot::new();
    let mut request = root.request();
    let (control, prepared, pool) = prepare_database(&request).await;
    let (desktop, secrets) = desktop(&root);
    let owner = &request.preparation.workshop;
    let mut original = desktop
        .first_desktop_request(
            prepared.workshop.tenant_code.clone(),
            owner.owner_username.clone(),
            owner.owner_password.expose_secret().into(),
        )
        .unwrap();
    original.device_proof = Some(expired_proof(&original, &secrets));
    assert_eq!(
        control
            .pairing
            .enroll_first_desktop(&original, correlation(), unix_millis_now())
            .await,
        Err(PairingError::Denied)
    );
    request.first_desktop = Some(original.clone());
    let mut platform = FakePlatform::default();
    let mut input = ProofAfterReady {
        desktop: &desktop,
        original: &original,
        ready: platform.ready.clone(),
        bytes: None,
    };
    install_server(&request, &mut platform, &mut input)
        .await
        .unwrap();
    assert_eq!(
        read_state(Path::new(&request.server_progress_directory))
            .unwrap()
            .progress
            .status,
        Status::Succeeded
    );
    let mut login = original.clone();
    login.device_proof =
        Some(super::super::super::guided::fresh_first_desktop_proof(&original, &desktop).unwrap());
    let session = control
        .authentication
        .login(&login, correlation(), unix_millis_now())
        .await
        .unwrap()
        .session;
    assert_eq!(session.device_id, original.device_id);
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(prepared.workshop.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let outcomes: Vec<String> = sqlx::query_scalar("SELECT outcome FROM audit.server_records WHERE operation='eitmad.server.pairing.first-desktop.v1' ORDER BY occurred_at")
        .fetch_all(&mut *tx).await.unwrap();
    assert_eq!(outcomes, ["denied", "succeeded"]);
}

#[test]
fn proof_refresh_refuses_a_replacement_device_identity() {
    let root = TestRoot::new();
    let (desktop, _) = desktop(&root);
    let mut original = desktop
        .first_desktop_request(
            eitmad_contracts::server::TenantCode::parse("synthetic").unwrap(),
            "synthetic-owner".into(),
            "Synthetic-password-123!".into(),
        )
        .unwrap();
    original.device_id = eitmad_contracts::identity::DeviceId::new(Uuid::new_v4());
    assert!(super::super::super::guided::fresh_first_desktop_proof(&original, &desktop).is_err());
}

#[test]
fn private_installation_input_reads_one_bounded_value_before_the_proof() {
    let root = TestRoot::new();
    let mut request = root.request();
    let proof = DeviceProof {
        device_id: eitmad_contracts::identity::DeviceId::new(Uuid::new_v4()),
        nonce: "synthetic-proof-nonce".into(),
        issued_at: UnixMillis(1_000_000),
        signature_base64: "synthetic-signature".into(),
    };
    let mut input = std::io::Cursor::new(format!(
        "{}\n{}",
        serde_json::to_string_pretty(&request).unwrap(),
        serde_json::to_string(&proof).unwrap()
    ));
    let _: InstallWindowsServer = read_private(&mut input).unwrap();
    assert_eq!(read_private::<DeviceProof>(&mut input).unwrap(), proof);
    request.service_name = "x".repeat(65536);
    assert!(
        read_private::<InstallWindowsServer>(&mut std::io::Cursor::new(
            serde_json::to_vec(&request).unwrap()
        ))
        .is_err()
    );
}
