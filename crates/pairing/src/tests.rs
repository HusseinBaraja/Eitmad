use super::*;
use eitmad_contracts::{
    discovery::ServerInstallationId,
    identity::{DeviceId, OrganizationId, ScopeId, TenantId},
    pairing::{PairingIdentity, PairingOffer},
    server::{DevicePublicKey, TenantCode},
    transport::UnixMillis,
};
use uuid::Uuid;

fn binding(key: &SigningKey) -> PairingBinding {
    PairingBinding {
        offer: PairingOffer {
            version: 1,
            session_id: Uuid::new_v4(),
            identity: PairingIdentity {
                server_id: ServerInstallationId::new(Uuid::new_v4()),
                endpoint: "https://synthetic.test:8443/".into(),
                public_ca_pem: "synthetic-public-ca".into(),
            },
            tenant_id: TenantId::new(Uuid::new_v4()),
            tenant_code: TenantCode::parse("synthetic").unwrap(),
            organization_id: OrganizationId::new(Uuid::new_v4()),
            branch_id: ScopeId::new(Uuid::new_v4()),
            expires_at: UnixMillis(180_000),
        },
        attempt_id: Uuid::new_v4(),
        device_id: DeviceId::new(Uuid::new_v4()),
        device_public_key: DevicePublicKey {
            algorithm: "ed25519".into(),
            base64: B64.encode(key.verifying_key().as_bytes()),
        },
    }
}

#[test]
fn opaque_confirms_both_peers_and_the_committed_receipt() {
    let key = SigningKey::from_bytes(&[71; 32]);
    let binding = binding(&key);
    let registration = register("12345678", binding.offer.session_id.as_bytes()).unwrap();
    let (client, start) = PairingClient::start("12345678", binding.clone()).unwrap();
    let (response, state) = challenge(&registration, &start).unwrap();
    let (completion, secret) = client.finish("12345678", &response, &key).unwrap();
    let receipt = complete(&state, &completion).unwrap();
    verify_receipt(&secret, &binding, &receipt).unwrap();
    assert!(verify_receipt(&[0; 64], &binding, &receipt).is_err());
}

#[test]
fn incorrect_code_and_intercepted_messages_do_not_authenticate() {
    let key = SigningKey::from_bytes(&[72; 32]);
    let binding = binding(&key);
    let registration = register("12345678", binding.offer.session_id.as_bytes()).unwrap();
    let (client, start) = PairingClient::start("87654321", binding.clone()).unwrap();
    let (response, _) = challenge(&registration, &start).unwrap();
    assert!(client.finish("87654321", &response, &key).is_err());
    let (client, start) = PairingClient::start("12345678", binding.clone()).unwrap();
    let (_, state) = challenge(&registration, &start).unwrap();
    let (_, attacker) = PairingClient::start("12345678", binding).unwrap();
    let (intercepted, _) = challenge(&registration, &attacker).unwrap();
    assert!(client.finish("12345678", &intercepted, &key).is_err());
    assert!(
        complete(
            &state,
            &CompletePairing {
                binding: attacker.binding,
                credential_finalization: B64.encode([0; 64]),
                device_signature: String::new()
            }
        )
        .is_err()
    );
}

#[test]
fn every_trust_and_device_field_is_authenticated_by_opaque() {
    let key = SigningKey::from_bytes(&[73; 32]);
    let original = binding(&key);
    let registration = register("12345678", original.offer.session_id.as_bytes()).unwrap();
    for field in 0..10 {
        let mut changed = original.clone();
        match field {
            0 => changed.offer.identity.server_id = ServerInstallationId::new(Uuid::new_v4()),
            1 => changed.offer.identity.public_ca_pem.push('x'),
            2 => changed.offer.identity.endpoint = "https://intercepted.test/".into(),
            3 => changed.offer.tenant_id = TenantId::new(Uuid::new_v4()),
            4 => changed.offer.organization_id = OrganizationId::new(Uuid::new_v4()),
            5 => changed.offer.branch_id = ScopeId::new(Uuid::new_v4()),
            6 => changed.offer.session_id = Uuid::new_v4(),
            7 => changed.attempt_id = Uuid::new_v4(),
            8 => changed.device_id = DeviceId::new(Uuid::new_v4()),
            _ => changed.offer.expires_at.0 += 1,
        }
        let (client, mut start) = PairingClient::start("12345678", changed.clone()).unwrap();
        // A relay substitutes the real offer on the server side and restores its lie on return.
        start.binding = original.clone();
        let (mut response, _) = challenge(&registration, &start).unwrap();
        response.binding = changed;
        assert!(
            client.finish("12345678", &response, &key).is_err(),
            "binding field {field}"
        );
    }
}

#[test]
fn completion_cannot_replace_the_device_or_reuse_another_exchange() {
    let key = SigningKey::from_bytes(&[74; 32]);
    let registration_binding = binding(&key);
    let registration =
        register("12345678", registration_binding.offer.session_id.as_bytes()).unwrap();
    let (client, start) = PairingClient::start("12345678", registration_binding).unwrap();
    let (response, state) = challenge(&registration, &start).unwrap();
    let (mut completion, _) = client.finish("12345678", &response, &key).unwrap();
    let changed_key = SigningKey::from_bytes(&[75; 32]);
    completion.binding.device_public_key.base64 =
        B64.encode(changed_key.verifying_key().as_bytes());
    completion.device_signature = B64.encode(
        changed_key
            .sign(&completion_bytes(&completion).unwrap())
            .to_bytes(),
    );
    assert!(complete(&state, &completion).is_err());
}
