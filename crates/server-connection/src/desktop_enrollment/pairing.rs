use super::*;
use eitmad_contracts::{
    discovery::DiscoveredServer,
    pairing::{CompletePairing, PairingBinding, PairingChallenge, PairingOffer, PairingReceipt},
};
use eitmad_pairing::PairingClient;
use rustls::{ClientConfig, RootCertStore};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::sync::Arc;

struct Pending {
    request: CompletePairing,
    session_key: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
struct PendingSecret {
    session_key: Vec<u8>,
    request_hash: Vec<u8>,
}
impl Drop for PendingSecret {
    fn drop(&mut self) {
        self.session_key.zeroize();
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.session_key.zeroize();
    }
}

impl DesktopEnrollment {
    /// Pairs a discovered candidate with a code obtained from its Owner.
    /// Saves only authenticated public trust plus a secret-store-protected retry record.
    /// # Errors
    /// Rejects conflicts, interception, identity changes, expired codes, and failed completion.
    pub fn pair(
        &self,
        candidate: &DiscoveredServer,
        code: &str,
    ) -> Result<DesktopEnrollmentSettings, TransportFailure> {
        let _owner = self
            .admission
            .lock()
            .map_err(|_| authentication_failure())?;
        if candidate.identity_conflict || candidate.expires_at.0 <= unix_millis_now().0 {
            return Err(authentication_failure());
        }
        let retained = DesktopConnectionSettings::load(&self.directory)?;
        let (device, route) = self.device(retained.as_ref())?;
        let key = self.signing_key(device, &route)?;
        let pending_id = SecretId::new(
            SecretKind::parse("desktop-pairing-pending").map_err(|_| authentication_failure())?,
            SecretReferenceId::new(device.value()),
        );
        let pending = self.pending_pairing(candidate, code, device, &key, &pending_id)?;
        let binding = &pending.request.binding;
        if binding.offer.identity.server_id != candidate.server_id
            || binding.device_id != device
            || binding.device_public_key.base64
                != URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes())
        {
            return Err(authentication_failure());
        }
        self.check_local_workshop(&binding.offer)?;
        if let Some(settings) = retained.as_ref() {
            let offer = &binding.offer;
            if settings.endpoint != offer.identity.endpoint
                || settings.organization != offer.organization_id
                || settings.branch != offer.branch_id
                || settings
                    .tenant_code
                    .as_ref()
                    .is_some_and(|c| c != &offer.tenant_code)
                || read_trust(&settings.trusted_certificate)?
                    != offer.identity.public_ca_pem.as_bytes()
            {
                return Err(authentication_failure());
            }
        }
        let config = authenticated_config(&binding.offer)?;
        let receipt: PairingReceipt =
            exchange(&config, "/v1/pairing/complete", &pending.request, None)?;
        eitmad_pairing::verify_receipt(&pending.session_key, binding, &receipt)
            .map_err(|_| authentication_failure())?;
        let settings = self.pairing_settings(&receipt);
        let path = self.directory.join("desktop-pairing.json");
        if let Ok(bytes) = fs::read(&path) {
            let existing: PairingReceipt =
                serde_json::from_slice(&bytes).map_err(|_| authentication_failure())?;
            if existing.binding != receipt.binding {
                return Err(authentication_failure());
            }
        }
        atomic_write(
            &self.directory.join("desktop-pairing-ca.pem"),
            receipt.binding.offer.identity.public_ca_pem.as_bytes(),
        )?;
        atomic_write(
            &path,
            &serde_json::to_vec(&receipt).map_err(|_| authentication_failure())?,
        )?;
        self.secrets
            .delete(&pending_id)
            .map_err(|_| authentication_failure())?;
        remove_pending_file(&self.directory)?;
        Ok(settings)
    }

    fn pending_pairing(
        &self,
        candidate: &DiscoveredServer,
        code: &str,
        device: DeviceId,
        key: &SigningKey,
        pending_id: &SecretId,
    ) -> Result<Pending, TransportFailure> {
        if let Some(secret) = self
            .secrets
            .get(pending_id)
            .map_err(|_| authentication_failure())?
        {
            let stored: PendingSecret = serde_json::from_slice(secret.expose_secret())
                .map_err(|_| authentication_failure())?;
            let bytes = read_pairing_file(&self.directory.join("desktop-pairing-pending.json"))
                .map_err(|_| authentication_failure())?;
            if Sha256::digest(&bytes).as_slice() != stored.request_hash {
                return Err(authentication_failure());
            }
            let request = serde_json::from_slice(&bytes).map_err(|_| authentication_failure())?;
            Ok(Pending {
                request,
                session_key: stored.session_key.clone(),
            })
        } else {
            let config = bootstrap(candidate)?;
            let offer: PairingOffer = exchange(&config, "/v1/pairing/offer", &(), None)?;
            if offer.identity.server_id != candidate.server_id
                || offer.expires_at.0 <= unix_millis_now().0
            {
                return Err(authentication_failure());
            }
            let binding = PairingBinding {
                offer,
                attempt_id: Uuid::new_v4(),
                device_id: device,
                device_public_key: DevicePublicKey {
                    algorithm: "ed25519".into(),
                    base64: URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()),
                },
            };
            let (client, request) =
                PairingClient::start(code, binding).map_err(|_| authentication_failure())?;
            let challenge: PairingChallenge =
                exchange(&config, "/v1/pairing/start", &request, None)?;
            let (request, session_key) = client
                .finish(code, &challenge, key)
                .map_err(|_| authentication_failure())?;
            let pending = Pending {
                request,
                session_key: session_key.to_vec(),
            };
            // The public transcript can include a CA bundle larger than a credential-store entry.
            // Keep its integrity hash and only the session key in the existing bounded secret store.
            let request_bytes =
                serde_json::to_vec(&pending.request).map_err(|_| authentication_failure())?;
            atomic_write(
                &self.directory.join("desktop-pairing-pending.json"),
                &request_bytes,
            )?;
            let stored = PendingSecret {
                session_key: pending.session_key.clone(),
                request_hash: Sha256::digest(&request_bytes).to_vec(),
            };
            let bytes =
                Zeroizing::new(serde_json::to_vec(&stored).map_err(|_| authentication_failure())?);
            self.secrets
                .set(
                    pending_id,
                    SecretMaterial::new(bytes.to_vec()).map_err(|_| authentication_failure())?,
                )
                .map_err(|_| authentication_failure())?;
            Ok(pending)
        }
    }

    /// Discards an abandoned exchange while retaining the device key, saved trust, and data.
    /// # Errors
    /// Fails closed if identity or secret storage is unavailable.
    pub fn discard_pending_pairing(&self) -> Result<(), TransportFailure> {
        let _owner = self
            .admission
            .lock()
            .map_err(|_| authentication_failure())?;
        let retained = DesktopConnectionSettings::load(&self.directory)?;
        let (device, _) = self.device(retained.as_ref())?;
        let id = SecretId::new(
            SecretKind::parse("desktop-pairing-pending").map_err(|_| authentication_failure())?,
            SecretReferenceId::new(device.value()),
        );
        self.secrets
            .delete(&id)
            .map_err(|_| authentication_failure())?;
        remove_pending_file(&self.directory)
    }

    /// Creates the private local first-desktop enrollment request. The seed remains in this client.
    /// # Errors
    /// Fails closed if the retained device identity or secret store is unavailable.
    pub fn first_desktop_request(
        &self,
        tenant_code: eitmad_contracts::server::TenantCode,
        username: String,
        password: String,
    ) -> Result<LoginRequest, TransportFailure> {
        let _owner = self
            .admission
            .lock()
            .map_err(|_| authentication_failure())?;
        let retained = DesktopConnectionSettings::load(&self.directory)?;
        let (device, route) = self.device(retained.as_ref())?;
        let key = self.signing_key(device, &route)?;
        Ok(LoginRequest {
            tenant_code,
            username,
            password,
            device_id: device,
            device_label: "first desktop".into(),
            device_public_key: DevicePublicKey {
                algorithm: "ed25519".into(),
                base64: URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()),
            },
            device_proof: Some(proof(device, &key)),
        })
    }

    /// Returns authenticated pairing settings without requiring another use of the code.
    /// # Errors
    /// Rejects damaged saved public pairing state.
    pub fn paired_connection(&self) -> Result<Option<DesktopEnrollmentSettings>, TransportFailure> {
        match read_pairing_file(&self.directory.join("desktop-pairing.json")) {
            Ok(bytes) => {
                let receipt =
                    serde_json::from_slice(&bytes).map_err(|_| authentication_failure())?;
                Ok(Some(self.pairing_settings(&receipt)))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(authentication_failure()),
        }
    }

    pub(super) fn check_paired_inputs(
        &self,
        inputs: &DesktopEnrollmentSettings,
    ) -> Result<(), TransportFailure> {
        if let Some(paired) = self.paired_connection()? {
            if inputs.endpoint != paired.endpoint
                || inputs.tenant_code != paired.tenant_code
                || inputs.organization != paired.organization
                || inputs.branch != paired.branch
                || read_trust(Path::new(&inputs.trusted_certificate))?
                    != read_trust(Path::new(&paired.trusted_certificate))?
            {
                return Err(authentication_failure());
            }
        }
        Ok(())
    }
    fn pairing_settings(&self, receipt: &PairingReceipt) -> DesktopEnrollmentSettings {
        let offer = &receipt.binding.offer;
        DesktopEnrollmentSettings {
            endpoint: offer.identity.endpoint.clone(),
            trusted_certificate: self
                .directory
                .join("desktop-pairing-ca.pem")
                .to_string_lossy()
                .into_owned(),
            tenant_code: offer.tenant_code.clone(),
            organization: offer.organization_id,
            branch: offer.branch_id,
        }
    }
    fn check_local_workshop(&self, offer: &PairingOffer) -> Result<(), TransportFailure> {
        if self
            .store
            .has_local_installation()
            .map_err(|_| authentication_failure())?
        {
            let local = self
                .store
                .existing_local_authorization_context()
                .map_err(|_| authentication_failure())?;
            if local.tenant_id != offer.tenant_id
                || self
                    .store
                    .local_organization_id()
                    .map_err(|_| authentication_failure())?
                    != offer.organization_id
            {
                return Err(authentication_failure());
            }
        }
        Ok(())
    }
}

fn authenticated_config(offer: &PairingOffer) -> Result<DirectServerConfig, TransportFailure> {
    DirectServerConfig::from_ca_pem(
        &offer.identity.endpoint,
        ScopeRef {
            kind: ScopeKind::parse("organization").map_err(|_| authentication_failure())?,
            id: eitmad_contracts::identity::ScopeId::new(offer.organization_id.value()),
        },
        SchemaId::parse("eitmad.schema.desktop-enrollment.v1")
            .map_err(|_| authentication_failure())?,
        1,
        offer.identity.public_ca_pem.as_bytes(),
    )
}

// The untrusted verifier exists only inside the PAKE bootstrap, with a closed set of routes.
// Passwords, bearer tokens, completion, and product operations always use normal CA/name checks.
fn bootstrap(candidate: &DiscoveredServer) -> Result<DirectServerConfig, TransportFailure> {
    let address: std::net::IpAddr = candidate
        .address
        .parse()
        .map_err(|_| authentication_failure())?;
    let endpoint = url::Url::parse(&format!(
        "https://{}/",
        std::net::SocketAddr::new(address, candidate.port)
    ))
    .map_err(|_| encryption_failure())?;
    let mut tls =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| encryption_failure())?
            .with_root_certificates(RootCertStore::empty())
            .with_no_client_auth();
    tls.dangerous()
        .set_certificate_verifier(Arc::new(PakeBootstrapVerifier));
    Ok(DirectServerConfig {
        endpoint,
        scope: ScopeRef {
            kind: ScopeKind::parse("organization").map_err(|_| authentication_failure())?,
            id: eitmad_contracts::identity::ScopeId::new(Uuid::nil()),
        },
        schema_id: SchemaId::parse("eitmad.schema.desktop-enrollment.v1")
            .map_err(|_| authentication_failure())?,
        schema_version: 1,
        tls: Arc::new(tls),
    })
}

#[derive(Debug)]
struct PakeBootstrapVerifier;
impl rustls::client::danger::ServerCertVerifier for PakeBootstrapVerifier {
    fn verify_server_cert(
        &self,
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn read_pairing_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(32_769).read_to_end(&mut bytes)?;
    if bytes.len() > 32_768 {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    Ok(bytes)
}
fn remove_pending_file(directory: &Path) -> Result<(), TransportFailure> {
    match fs::remove_file(directory.join("desktop-pairing-pending.json")) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(authentication_failure()),
    }
}
