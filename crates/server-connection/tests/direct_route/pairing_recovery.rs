use super::*;
use eitmad_contracts::pairing::{CompletePairing, OpenPairing, PairingBinding};
use eitmad_pairing::PairingClient;
use eitmad_secret_storage::SecretMaterial;
use sha2::{Digest as _, Sha256};

fn enrollment(client: &Installation) -> DesktopEnrollment {
    DesktopEnrollment::new(
        client.directory.path().into(),
        client.store.clone(),
        client.secrets.clone(),
    )
}

fn secret_id(client: &Installation, kind: &str) -> SecretId {
    SecretId::new(
        SecretKind::parse(kind).unwrap(),
        SecretReferenceId::new(client.device().value()),
    )
}

// Stop immediately before HTTP completion using the real PAKE challenge and persisted client format.
async fn interrupt_completion(
    client: &Installation,
    server: &ProvisionedServer,
) -> CompletePairing {
    let service = enrollment(client);
    let login = service
        .first_desktop_request(
            TenantCode::parse("direct-test").unwrap(),
            String::new(),
            String::new(),
        )
        .unwrap();
    let material = client
        .secrets
        .get(&secret_id(client, "desktop-device-key"))
        .unwrap()
        .unwrap();
    let key = SigningKey::from_bytes(material.expose_secret().try_into().unwrap());
    let now = eitmad_control_plane::unix_millis_now();
    let opened = server
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
            now,
        )
        .await
        .unwrap();
    let (pake, start) = PairingClient::start(
        &opened.code,
        PairingBinding {
            offer: opened.offer,
            attempt_id: Uuid::new_v4(),
            device_id: login.device_id,
            device_public_key: login.device_public_key,
        },
    )
    .unwrap();
    let challenge = server
        .control
        .pairing
        .start(
            &server.pairing_identity,
            &start,
            CorrelationId::new(Uuid::new_v4()),
            now,
        )
        .await
        .unwrap();
    let (request, session_key) = pake.finish(&opened.code, &challenge, &key).unwrap();
    let bytes = serde_json::to_vec(&request).unwrap();
    std::fs::write(
        client.directory.path().join("desktop-pairing-pending.json"),
        &bytes,
    )
    .unwrap();
    let secret = serde_json::to_vec(&serde_json::json!({
        "session_key": session_key.to_vec(), "request_hash": Sha256::digest(&bytes).to_vec(),
    }))
    .unwrap();
    client
        .secrets
        .set(
            &secret_id(client, "desktop-pairing-pending"),
            SecretMaterial::new(secret).unwrap(),
        )
        .unwrap();
    request
}

async fn expire(server: &ProvisionedServer, request: &CompletePairing) {
    let db = ControlDatabase::connect(&env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap(), 2)
        .await
        .unwrap();
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(request.binding.offer.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE control.pairing_sessions SET expires_at=0 WHERE session_id=$1")
        .bind(request.binding.offer.session_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE control.pairing_directory SET expires_at=0 WHERE server_id=$1")
        .bind(server.pairing_identity.server_id.value())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn pairing_recovery_preserves_identity_through_expiry_abandonment_and_lost_receipts() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let server = provision_server(
        &env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap(),
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    let client = Installation::new();
    let completion = interrupt_completion(&client, &server).await;
    let device = client.device();
    let key_id = secret_id(&client, "desktop-device-key");
    let key = client.secrets.get(&key_id).unwrap().unwrap();
    let pending = client.directory.path().join("desktop-pairing-pending.json");
    let transcript = std::fs::read(&pending).unwrap();
    expire(&server, &completion).await;
    assert_eq!(
        enrollment(&client).pending_pairing_server().unwrap(),
        Some(server.pairing_identity.server_id)
    );
    assert!(client.try_pair(&server).await.is_err());
    assert_eq!(std::fs::read(&pending).unwrap(), transcript);
    assert!(enrollment(&client).paired_connection().unwrap().is_none());
    assert_missing_key_cannot_be_replaced(&client, &server, &key_id, key.expose_secret()).await;
    assert_missing_device_cannot_be_replaced(&client, &server).await;
    verify_native_abandonment(&client, &server).await;

    // Explicit abandonment retains the private key, including when the exchange is uncertain.
    enrollment(&client).discard_pending_pairing().unwrap();
    assert!(
        enrollment(&client)
            .pending_pairing_server()
            .unwrap()
            .is_none()
    );
    assert!(
        client
            .secrets
            .get(&secret_id(&client, "desktop-pairing-pending"))
            .unwrap()
            .is_none()
    );
    assert_eq!(client.device(), device);
    assert!(
        client
            .secrets
            .get(&key_id)
            .unwrap()
            .unwrap()
            .expose_secret()
            == key.expose_secret(),
        "Abandonment must preserve the device key"
    );
    // A fresh Owner code now completes with the original device identity.
    client.pair(&server).await;
    assert_eq!(client.device(), device);
    assert!(client.sign_in(request(None)).await.unwrap());
    assert_committed_state_retained(&client, &server);
    verify_lost_receipt(&server).await;
    verify_abandoned_committed_exchange(&server).await;
    server
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
}

async fn verify_native_abandonment(client: &Installation, server: &ProvisionedServer) {
    let Some(engine) = env::var_os("EITMAD_DESKTOP_ENROLLMENT_ENGINE") else {
        return;
    };
    let directory = client.directory.path().to_owned();
    let candidate = serde_json::to_string(&eitmad_contracts::discovery::DiscoveredServer {
        server_id: server.pairing_identity.server_id,
        address: server.address.ip().to_string(),
        port: server.address.port(),
        expires_at: UnixMillis(eitmad_control_plane::unix_millis_now().0 + 60_000),
        identity_conflict: false,
    })
    .unwrap();
    let result = tokio::task::spawn_blocking(move || {
        windows_shell_test("PendingPairingCanBeRetriedAndExplicitlyAbandoned")
            .env("EITMAD_PAIRING_RECOVERY_CANDIDATE", candidate)
            .env("EITMAD_DESKTOP_ENROLLMENT_ENGINE", engine)
            .env("EITMAD_DESKTOP_ENROLLMENT_DIRECTORY", directory)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(
        result.status.success(),
        "Windows pairing recovery failed: {}",
        String::from_utf8_lossy(&result.stdout)
    );
}

async fn assert_missing_key_cannot_be_replaced(
    client: &Installation,
    server: &ProvisionedServer,
    id: &SecretId,
    seed: &[u8],
) {
    client.secrets.delete(id).unwrap();
    assert!(client.try_pair(server).await.is_err());
    assert!(enrollment(client).discard_pending_pairing().is_err());
    assert!(client.secrets.get(id).unwrap().is_none());
    client
        .secrets
        .set(id, SecretMaterial::new(seed.to_vec()).unwrap())
        .unwrap();
}

async fn assert_missing_device_cannot_be_replaced(
    client: &Installation,
    server: &ProvisionedServer,
) {
    let path = client.directory.path().join("desktop-device.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(client.try_pair(server).await.is_err());
    assert!(enrollment(client).discard_pending_pairing().is_err());
    assert!(!path.exists());
    std::fs::write(&path, bytes).unwrap();
}

fn assert_committed_state_retained(client: &Installation, server: &ProvisionedServer) {
    let receipt = std::fs::read(client.directory.path().join("desktop-pairing.json")).unwrap();
    let trust = std::fs::read(client.directory.path().join("desktop-pairing-ca.pem")).unwrap();
    let settings = std::fs::read(
        client
            .directory
            .path()
            .join(DesktopConnectionSettings::FILE_NAME),
    )
    .unwrap();
    let account = client
        .store
        .desktop_account(server.authentication.session.tenant_id, "owner")
        .unwrap()
        .unwrap();
    enrollment(client).discard_pending_pairing().unwrap();
    assert_eq!(
        std::fs::read(client.directory.path().join("desktop-pairing.json")).unwrap(),
        receipt
    );
    assert_eq!(
        std::fs::read(client.directory.path().join("desktop-pairing-ca.pem")).unwrap(),
        trust
    );
    assert_eq!(
        std::fs::read(
            client
                .directory
                .path()
                .join(DesktopConnectionSettings::FILE_NAME)
        )
        .unwrap(),
        settings
    );
    assert!(
        client
            .store
            .desktop_account(server.authentication.session.tenant_id, "owner")
            .unwrap()
            .unwrap()
            .password_hash
            == account.password_hash,
        "Abandonment must preserve the account verifier"
    );
}

async fn verify_lost_receipt(server: &ProvisionedServer) {
    let client = Installation::new();
    let request = interrupt_completion(&client, server).await;
    // The server commits, but the caller loses its receipt. Expiry must not hide that result.
    server
        .control
        .pairing
        .complete(
            &server.pairing_identity,
            &request,
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap();
    expire(server, &request).await;
    client.pair(server).await;
    let bytes = std::fs::read(client.directory.path().join("desktop-pairing.json")).unwrap();
    let receipt: eitmad_contracts::pairing::PairingReceipt =
        serde_json::from_slice(&bytes).unwrap();
    assert_eq!(receipt.binding, request.binding);
    assert!(
        enrollment(&client)
            .pending_pairing_server()
            .unwrap()
            .is_none()
    );
    assert_eq!(client.device(), request.binding.device_id);
    // Even a fresh valid code cannot replace committed local trust.
    assert!(client.try_pair(server).await.is_err());
    assert_eq!(
        std::fs::read(client.directory.path().join("desktop-pairing.json")).unwrap(),
        bytes
    );
    // The replacement-refusal check opened an unused Owner code; release this fixture's offer.
    expire(server, &request).await;
}

async fn verify_abandoned_committed_exchange(server: &ProvisionedServer) {
    let client = Installation::new();
    let request = interrupt_completion(&client, server).await;
    server
        .control
        .pairing
        .complete(
            &server.pairing_identity,
            &request,
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap();
    expire(server, &request).await;
    enrollment(&client).discard_pending_pairing().unwrap();
    // Abandonment cannot undo a remote approval. A new exchange must fail without a new identity.
    assert!(client.try_pair(server).await.is_err());
    assert_eq!(client.device(), request.binding.device_id);
    let material = client
        .secrets
        .get(&secret_id(&client, "desktop-device-key"))
        .unwrap()
        .unwrap();
    let key = SigningKey::from_bytes(material.expose_secret().try_into().unwrap());
    assert_eq!(
        URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()),
        request.binding.device_public_key.base64
    );
    assert!(enrollment(&client).paired_connection().unwrap().is_none());
    enrollment(&client).discard_pending_pairing().unwrap();
}
