use super::*;
use eitmad_contracts::{
    accounts::AccountPassword,
    setup::{InitializeWorkshop, PreparedWindowsWorkshop, SetupDesktopHandoff},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn first_desktop_retains_recovery_through_restart_and_revocation() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let (database, sync) =
        migrate_test_databases(&env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap()).await;
    let control = ControlPlane::new(database.pool(), TokenKey::new([9; 32]));
    let workshop = initialize(&control).await;
    let installation = Installation::new();
    let desktop = DesktopEnrollment::new(
        installation.directory.path().into(),
        installation.store.clone(),
        installation.secrets.clone(),
    );
    let login = desktop
        .first_desktop_request(
            workshop.tenant_code.clone(),
            "owner".into(),
            PASSWORD.into(),
        )
        .unwrap();
    control
        .pairing
        .enroll_first_desktop(
            &login,
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap();
    let address = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let handoff = SetupDesktopHandoff {
        version: 1,
        server_id: eitmad_contracts::discovery::ServerInstallationId::new(Uuid::new_v4()),
        public_ca_pem: std::fs::read_to_string(required_path(
            "EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE",
        ))
        .unwrap(),
        prepared: PreparedWindowsWorkshop {
            version: 1,
            endpoint: format!("https://localhost:{}/", address.port()),
            workshop,
        },
    };
    let identity = eitmad_contracts::pairing::PairingIdentity {
        server_id: handoff.server_id,
        endpoint: handoff.prepared.endpoint.clone(),
        public_ca_pem: handoff.public_ca_pem.clone(),
    };
    let state = ServerState::new(
        control.clone(),
        SyncCoordinator::new(&sync, direct_test_domains(&sync)),
    )
    .with_pairing(identity.clone());
    let mut handle = start_server(
        address,
        state.clone(),
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    let inputs = desktop.retain_first_desktop(&handoff, &login).unwrap();
    assert!(installation.sign_in(request(Some(inputs))).await.unwrap());
    assert_eq!(installation.settings().server_id, Some(handoff.server_id));
    assert_eq!(installation.settings().endpoint, identity.endpoint);
    let before = retained_files(&installation);
    // New enrollment objects and transport instances reload durable settings, as after app restart.
    assert!(!installation.sign_in(request(None)).await.unwrap());
    assert_server_access(&installation, &local_sign_in(&installation), true).await;
    handle.handle.shutdown();
    wait_stopped(address).await;
    handle = start_server(
        address,
        state,
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    assert_server_access(&installation, &local_sign_in(&installation), true).await;
    reject_replaced_server(
        &installation,
        address,
        &mut handle,
        &control,
        &sync,
        identity,
    )
    .await;
    revoke(&database, &handoff, login.device_id).await;
    assert_server_access(&installation, &local_sign_in(&installation), false).await;
    assert!(installation.sign_in(request(None)).await.is_err());
    assert!(
        retained_files(&installation) == before,
        "Recovery must preserve enrollment"
    );
    handle.graceful_shutdown(Some(Duration::from_secs(1)));
}

fn retained_files(installation: &Installation) -> Vec<Vec<u8>> {
    let mut files = vec![serde_json::to_vec(&installation.settings()).unwrap()];
    files.extend(
        [
            "desktop-first-server.json",
            "desktop-device.json",
            "desktop-trust.pem",
        ]
        .into_iter()
        .map(|name| std::fs::read(installation.directory.path().join(name)).unwrap()),
    );
    files
}

async fn initialize(control: &ControlPlane) -> eitmad_contracts::setup::InitializedWorkshop {
    control
        .identity
        .initialize_workshop(
            &InitializeWorkshop {
                version: 1,
                workshop_name: "Synthetic workshop".into(),
                owner_username: "owner".into(),
                owner_password: AccountPassword::new(PASSWORD),
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap()
}

async fn wait_stopped(address: SocketAddr) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while std::net::TcpStream::connect_timeout(&address, Duration::from_millis(25)).is_ok() {
        assert!(Instant::now() < deadline, "Synthetic server did not stop");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn reject_replaced_server(
    installation: &Installation,
    address: SocketAddr,
    handle: &mut LiveTestServer,
    control: &ControlPlane,
    sync: &SyncDatabase,
    identity: eitmad_contracts::pairing::PairingIdentity,
) {
    handle.handle.shutdown();
    wait_stopped(address).await;
    let mut substituted = identity.clone();
    substituted.server_id = eitmad_contracts::discovery::ServerInstallationId::new(Uuid::new_v4());
    let state = || {
        ServerState::new(
            control.clone(),
            SyncCoordinator::new(sync, direct_test_domains(sync)),
        )
    };
    *handle = start_server(
        address,
        state().with_pairing(substituted),
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    // The old address now serves a different installation with an otherwise valid certificate.
    assert_server_access(installation, &local_sign_in(installation), false).await;
    handle.handle.shutdown();
    wait_stopped(address).await;
    *handle = start_server(
        address,
        state().with_pairing(identity),
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
}

async fn revoke(database: &ControlDatabase, handoff: &SetupDesktopHandoff, device: DeviceId) {
    let mut tx = database.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(handoff.prepared.workshop.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE control.paired_devices SET revoked_at=$1 WHERE tenant_id=$2 AND device_id=$3",
    )
    .bind(eitmad_control_plane::unix_millis_now().0)
    .bind(handoff.prepared.workshop.tenant_id.value())
    .bind(device.value())
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
}
