//! Run only against a fresh disposable database owned by a non-superuser role.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use ed25519_dalek::{Signer as _, SigningKey};
use eitmad_contracts::{
    accounts::AccountPassword,
    discovery::ServerInstallationId,
    identity::{DeviceId, UserId},
    pairing::{
        CompletePairing, OpenPairing, OpenedPairing, PairingBinding, PairingIdentity,
        PairingReceipt,
    },
    server::{AuthenticatedServerSession, DeviceProof, DevicePublicKey, LoginRequest},
    setup::{InitializeWorkshop, InitializedWorkshop},
    transport::{CorrelationId, UnixMillis},
};
use eitmad_control_plane::{
    ControlDatabase, ControlPlane, PairingError, TokenKey, unix_millis_now,
};
use eitmad_pairing::PairingClient;
use eitmad_postgres_support as sqlx;
use eitmad_server_audit::AuditDatabase;
use uuid::Uuid;

const PASSWORD: &str = "synthetic-owner-password-123";
struct Fixture {
    control: ControlPlane,
    pool: sqlx::PgPool,
    owner: AuthenticatedServerSession,
    workshop: InitializedWorkshop,
    identity: PairingIdentity,
}
fn correlation() -> CorrelationId {
    CorrelationId::new(Uuid::new_v4())
}
fn login(workshop: &InitializedWorkshop, key: &SigningKey, device: DeviceId) -> LoginRequest {
    let now = unix_millis_now();
    let nonce = Uuid::new_v4().to_string();
    let proof = DeviceProof {
        device_id: device,
        issued_at: now,
        nonce: nonce.clone(),
        signature_base64: B64.encode(
            key.sign(format!("{}\n{}\n{nonce}", device.value(), now.0).as_bytes())
                .to_bytes(),
        ),
    };
    LoginRequest {
        tenant_code: workshop.tenant_code.clone(),
        username: "synthetic-owner".into(),
        password: PASSWORD.into(),
        device_id: device,
        device_label: "synthetic desktop".into(),
        device_public_key: DevicePublicKey {
            algorithm: "ed25519".into(),
            base64: B64.encode(key.verifying_key().as_bytes()),
        },
        device_proof: Some(proof),
    }
}
impl Fixture {
    async fn new() -> Self {
        let url = std::env::var("EITMAD_PAIRING_TEST_DATABASE_URL")
            .expect("fresh disposable database required");
        let db = ControlDatabase::connect(&url, 6).await.unwrap();
        let pool = db.pool();
        let fresh: bool = sqlx::query_scalar("SELECT to_regnamespace('control') IS NULL AND NOT (SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname=current_user)")
            .fetch_one(&pool).await.unwrap();
        assert!(fresh, "fresh database and enforced RLS required");
        db.migrate().await.unwrap();
        eitmad_sync_plane::SyncDatabase::connect(&url, 2)
            .await
            .unwrap()
            .migrate()
            .await
            .unwrap();
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
        let control = ControlPlane::new(pool.clone(), TokenKey::new([97; 32]));
        let workshop = control
            .identity
            .initialize_workshop(
                &InitializeWorkshop {
                    version: 1,
                    workshop_name: "Synthetic workshop".into(),
                    owner_username: "synthetic-owner".into(),
                    owner_password: AccountPassword::new(PASSWORD),
                },
                correlation(),
                unix_millis_now(),
            )
            .await
            .unwrap();
        let key = SigningKey::from_bytes(&[81; 32]);
        let device = DeviceId::new(Uuid::new_v4());
        assert!(
            control
                .authentication
                .login(
                    &login(&workshop, &key, device),
                    correlation(),
                    unix_millis_now()
                )
                .await
                .is_err(),
            "password cannot enroll"
        );
        control
            .pairing
            .enroll_first_desktop(
                &login(&workshop, &key, device),
                correlation(),
                unix_millis_now(),
            )
            .await
            .unwrap();
        assert_eq!(
            control
                .pairing
                .enroll_first_desktop(
                    &login(&workshop, &key, DeviceId::new(Uuid::new_v4())),
                    correlation(),
                    unix_millis_now()
                )
                .await,
            Err(PairingError::Denied)
        );
        let owner = control
            .authentication
            .login(
                &login(&workshop, &key, device),
                correlation(),
                unix_millis_now(),
            )
            .await
            .unwrap()
            .session;
        Self {
            control,
            pool,
            owner,
            workshop,
            identity: PairingIdentity {
                server_id: ServerInstallationId::new(Uuid::new_v4()),
                endpoint: "https://localhost:8443/".into(),
                public_ca_pem: "synthetic-public-ca".into(),
            },
        }
    }
    async fn open(&self) -> OpenedPairing {
        self.control
            .pairing
            .open(
                &self.owner,
                &self.identity,
                &self.open_request(),
                correlation(),
                unix_millis_now(),
            )
            .await
            .unwrap()
    }
    fn open_request(&self) -> OpenPairing {
        OpenPairing {
            version: 1,
            organization_id: self.workshop.organization_id,
            branch_id: self.workshop.branch_id,
        }
    }
    fn binding(opened: &OpenedPairing, key: &SigningKey) -> PairingBinding {
        PairingBinding {
            offer: opened.offer.clone(),
            attempt_id: Uuid::new_v4(),
            device_id: DeviceId::new(Uuid::new_v4()),
            device_public_key: DevicePublicKey {
                algorithm: "ed25519".into(),
                base64: B64.encode(key.verifying_key().as_bytes()),
            },
        }
    }
    async fn completion(
        &self,
        opened: &OpenedPairing,
        seed: u8,
    ) -> (CompletePairing, zeroize::Zeroizing<Vec<u8>>) {
        let key = SigningKey::from_bytes(&[seed; 32]);
        let (client, start) =
            PairingClient::start(&opened.code, Self::binding(opened, &key)).unwrap();
        let challenge = self
            .control
            .pairing
            .start(&self.identity, &start, correlation(), unix_millis_now())
            .await
            .unwrap();
        client.finish(&opened.code, &challenge, &key).unwrap()
    }
    async fn approved(&self, device: DeviceId) -> bool {
        let mut tx = self.pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
            .bind(self.owner.tenant_id.value().to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.paired_devices WHERE tenant_id=$1 AND device_id=$2)")
            .bind(self.owner.tenant_id.value()).bind(device.value()).fetch_one(&mut *tx).await.unwrap()
    }
    async fn complete(
        &self,
        request: &CompletePairing,
        now: UnixMillis,
    ) -> Result<PairingReceipt, PairingError> {
        self.control
            .pairing
            .complete(&self.identity, request, correlation(), now)
            .await
    }
    async fn expire_directory(&self) {
        sqlx::query("UPDATE control.pairing_directory SET expires_at=0")
            .execute(&self.pool)
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore = "requires a fresh disposable EITMAD_PAIRING_TEST_DATABASE_URL with enforced RLS"]
async fn pairing_enforces_security_and_recovers_interrupted_completion() {
    let f = Fixture::new().await;
    unauthorized(&f).await;
    attempts_and_expiry(&f).await;
    atomic_completion_and_retry(&f).await;
    changed_key_and_owner_revocation(&f).await;
    let hidden: i64 = sqlx::query_scalar("SELECT count(*) FROM control.paired_devices")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        hidden, 0,
        "forced RLS hides device approvals without tenant context"
    );
    f.pool.close().await;
}

async fn unauthorized(f: &Fixture) {
    let mut actor = f.owner.clone();
    actor.user_id = UserId::new(Uuid::new_v4());
    assert!(matches!(
        f.control
            .pairing
            .open(
                &actor,
                &f.identity,
                &f.open_request(),
                correlation(),
                unix_millis_now()
            )
            .await,
        Err(PairingError::Denied)
    ));
    let mut scope = f.open_request();
    scope.branch_id = eitmad_contracts::identity::ScopeId::new(Uuid::new_v4());
    assert!(matches!(
        f.control
            .pairing
            .open(
                &f.owner,
                &f.identity,
                &scope,
                correlation(),
                unix_millis_now()
            )
            .await,
        Err(PairingError::Denied)
    ));
}

async fn attempts_and_expiry(f: &Fixture) {
    let opened = f.open().await;
    let key = SigningKey::from_bytes(&[82; 32]);
    for index in 0..5 {
        let wrong_code = if opened.code == "00000000" {
            "11111111"
        } else {
            "00000000"
        };
        let (client, start) =
            PairingClient::start(wrong_code, Fixture::binding(&opened, &key)).unwrap();
        let challenge = f
            .control
            .pairing
            .start(&f.identity, &start, correlation(), unix_millis_now())
            .await
            .unwrap();
        assert!(client.finish(wrong_code, &challenge, &key).is_err());
        if index == 0 {
            assert!(
                f.control
                    .pairing
                    .start(&f.identity, &start, correlation(), unix_millis_now())
                    .await
                    .is_err(),
                "start replay"
            );
        }
    }
    let (_, start) = PairingClient::start(&opened.code, Fixture::binding(&opened, &key)).unwrap();
    assert!(
        f.control
            .pairing
            .start(&f.identity, &start, correlation(), unix_millis_now())
            .await
            .is_err(),
        "attempt limit"
    );
    f.expire_directory().await;
    let opened = f.open().await;
    let (request, _) = f.completion(&opened, 83).await;
    assert!(
        f.complete(&request, opened.offer.expires_at).await.is_err(),
        "expiry includes finalization"
    );
    assert!(!f.approved(request.binding.device_id).await);
    let (_, start) = PairingClient::start(&opened.code, Fixture::binding(&opened, &key)).unwrap();
    assert!(
        f.control
            .pairing
            .start(&f.identity, &start, correlation(), opened.offer.expires_at)
            .await
            .is_err()
    );
    f.expire_directory().await;
}

async fn atomic_completion_and_retry(f: &Fixture) {
    let opened = f.open().await;
    let (request, key) = f.completion(&opened, 84).await;
    let (other, _) = f.completion(&opened, 85).await;
    sqlx::raw_sql("CREATE FUNCTION audit.reject_pairing_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operation='eitmad.server.pairing.complete.v1' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_pairing_test BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION audit.reject_pairing_test()")
        .execute(&f.pool).await.unwrap();
    assert!(matches!(
        f.complete(&request, unix_millis_now()).await,
        Err(PairingError::Unavailable)
    ));
    assert!(
        !f.approved(request.binding.device_id).await,
        "approval must roll back with audit"
    );
    sqlx::raw_sql("DROP TRIGGER reject_pairing_test ON audit.server_records; DROP FUNCTION audit.reject_pairing_test()")
        .execute(&f.pool).await.unwrap();
    let receipt = f.complete(&request, unix_millis_now()).await.unwrap();
    eitmad_pairing::verify_receipt(&key, &request.binding, &receipt).unwrap();
    // Simulate response loss and a server process restart; the receipt must come from PostgreSQL.
    let restarted = eitmad_control_plane::PairingService::new(f.pool.clone());
    let retry = restarted
        .complete(
            &f.identity,
            &request,
            correlation(),
            opened.offer.expires_at,
        )
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&receipt).unwrap(),
        serde_json::to_value(&retry).unwrap()
    );
    assert!(
        f.complete(&other, unix_millis_now()).await.is_err(),
        "a code approves only one device"
    );
    assert!(!f.approved(other.binding.device_id).await);
    let device_key = SigningKey::from_bytes(&[84; 32]);
    let authenticated = f
        .control
        .authentication
        .login(
            &login(&f.workshop, &device_key, request.binding.device_id),
            correlation(),
            unix_millis_now(),
        )
        .await
        .unwrap();
    assert_eq!(authenticated.session.device_id, request.binding.device_id);
    let mut changed = login(&f.workshop, &device_key, request.binding.device_id);
    changed.device_public_key = other.binding.device_public_key;
    assert!(
        f.control
            .authentication
            .login(&changed, correlation(), unix_millis_now())
            .await
            .is_err()
    );
    revoked_approval_blocks_authentication(f, &authenticated, &device_key).await;
}

async fn changed_key_and_owner_revocation(f: &Fixture) {
    let opened = f.open().await;
    let (mut request, _) = f.completion(&opened, 86).await;
    request.binding.device_public_key.base64 =
        B64.encode(SigningKey::from_bytes(&[87; 32]).verifying_key().as_bytes());
    assert!(f.complete(&request, unix_millis_now()).await.is_err());
    assert!(!f.approved(request.binding.device_id).await);
    let (request, _) = f.completion(&opened, 88).await;
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(f.owner.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM control.relationship_tuples WHERE tenant_id=$1 AND relation='eitmad.relation.organization.owner.v1'")
        .bind(f.owner.tenant_id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert!(
        f.complete(&request, unix_millis_now()).await.is_err(),
        "Owner must still be authorized"
    );
    assert!(!f.approved(request.binding.device_id).await);
}

async fn revoked_approval_blocks_authentication(
    f: &Fixture,
    authenticated: &eitmad_contracts::server::AuthenticationResult,
    key: &SigningKey,
) {
    let device = authenticated.session.device_id;
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(f.owner.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE control.paired_devices SET revoked_at=$3 WHERE tenant_id=$1 AND device_id=$2",
    )
    .bind(f.owner.tenant_id.value())
    .bind(device.value())
    .bind(unix_millis_now().0)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(
        f.control
            .authentication
            .login(
                &login(&f.workshop, key, device),
                correlation(),
                unix_millis_now()
            )
            .await
            .is_err()
    );
    let proof = login(&f.workshop, key, device).device_proof.unwrap();
    assert!(
        f.control
            .authentication
            .authenticate_access(
                &authenticated.tokens.access_token,
                &proof,
                unix_millis_now()
            )
            .await
            .is_err()
    );
    assert!(
        f.control
            .authentication
            .refresh(
                &eitmad_contracts::server::RefreshRequest {
                    refresh_token: authenticated.tokens.refresh_token.clone(),
                    device_proof: login(&f.workshop, key, device).device_proof.unwrap(),
                },
                correlation(),
                unix_millis_now()
            )
            .await
            .is_err()
    );
}
