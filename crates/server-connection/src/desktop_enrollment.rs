//! Retryable enrollment. No password, token, or private key enters installation settings.
use super::{
    DesktopConnectionSettings, DirectServerConfig, URL_SAFE_NO_PAD, authentication_failure,
    encryption_failure, unix_millis_now,
};
use argon2::{Argon2, PasswordHasher as _, password_hash::SaltString};
use base64::Engine as _;
use ed25519_dalek::{Signer as _, SigningKey};
use eitmad_contracts::{
    config::SecretReferenceId,
    identity::{DeviceId, ScopeKind, ScopeRef},
    ipc::{DesktopEnrollmentSettings, DesktopSignInRequest},
    secrets::{SecretId, SecretKind},
    server::{
        AuthenticationResult, DesktopEnrollmentProfile, DeviceProof, DevicePublicKey, LoginRequest,
        RegisterBranchRequest,
    },
    transport::SchemaId,
};
use eitmad_secret_storage::{SecretMaterial, SecretStore};
use eitmad_storage::{AuthorityStore, DesktopAccount};
use eitmad_sync::TransportFailure;
use rand::RngCore as _;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs,
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Instant,
};
use uuid::Uuid;
use zeroize::{Zeroize as _, Zeroizing};
pub(super) mod pairing;

pub struct DesktopEnrollment {
    directory: PathBuf,
    store: AuthorityStore,
    secrets: SecretStore,
    admission: Mutex<()>,
}

impl DesktopEnrollment {
    #[must_use]
    pub fn new(directory: PathBuf, store: AuthorityStore, secrets: SecretStore) -> Self {
        Self {
            directory,
            store,
            secrets,
            admission: Mutex::new(()),
        }
    }

    /// Authenticates online and imports only the server-confirmed account and scope.
    /// Returns true when the supervisor must reopen the engine to attach the saved routes.
    /// # Errors
    /// Fails closed on trust, identity, credentials, storage, or interrupted network exchange.
    pub fn sign_in(&self, request: &DesktopSignInRequest) -> Result<bool, TransportFailure> {
        let _owner = self
            .admission
            .lock()
            .map_err(|_| authentication_failure())?;
        let retained = DesktopConnectionSettings::load(&self.directory)?;
        let inputs = if request.connection.is_none() && retained.is_none() {
            self.paired_connection()?
                .ok_or_else(authentication_failure)?
        } else {
            Self::inputs(request, retained.as_ref())?
        };
        self.check_paired_inputs(&inputs)?;
        let paired = self.paired_connection()?;
        let trust_path = Path::new(
            paired
                .as_ref()
                .map_or(&inputs.trusted_certificate, |paired| {
                    &paired.trusted_certificate
                }),
        );
        let trust = read_trust(trust_path)?;
        let config = DirectServerConfig::new(
            &inputs.endpoint,
            ScopeRef {
                kind: ScopeKind::parse("organization").map_err(|_| authentication_failure())?,
                id: eitmad_contracts::identity::ScopeId::new(inputs.organization.value()),
            },
            SchemaId::parse("eitmad.schema.desktop-enrollment.v1")
                .map_err(|_| authentication_failure())?,
            1,
            trust_path,
        )?;
        let (device, route) = self.device(retained.as_ref())?;
        let key = self.signing_key(device, &route)?;
        let mut login = LoginRequest {
            tenant_code: inputs.tenant_code.clone(),
            username: request.username.clone(),
            password: request.password.clone(),
            device_id: device,
            device_label: "جهاز الاعتماد".into(),
            device_public_key: DevicePublicKey {
                algorithm: "ed25519".into(),
                base64: URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()),
            },
            device_proof: Some(proof(device, &key)),
        };
        let result = exchange(&config, "/v1/auth/login", &login, None);
        login.password.zeroize();
        let authentication: AuthenticationResult = result?;
        // The request owns transient password input. No verifier is imported before server confirmation.
        if authentication.session.device_id != device {
            return Err(authentication_failure());
        }
        let profile: DesktopEnrollmentProfile = exchange(
            &config,
            "/v1/auth/desktop-profile",
            &RegisterBranchRequest {
                organization_id: inputs.organization,
                branch_id: inputs.branch,
            },
            Some((&authentication.tokens.access_token, proof(device, &key))),
        )?;
        self.validate_identity(&authentication, &profile, &inputs)?;
        self.import_account(&authentication, &profile, &inputs, &request.password)?;
        // Account-specific credentials can be replaced without losing the installation signing key.
        super::store_account_session(&self.secrets, &route, authentication, key.to_bytes())?;
        if retained.is_none() {
            atomic_write(&self.directory.join("desktop-trust.pem"), &trust)?;
        }
        let settings = DesktopConnectionSettings {
            version: 1,
            endpoint: config.endpoint.to_string(),
            trusted_certificate: retained.as_ref().map_or_else(
                || PathBuf::from("desktop-trust.pem"),
                |s| s.trusted_certificate.clone(),
            ),
            credential: route,
            organization: inputs.organization,
            branch: inputs.branch,
            tenant_code: Some(inputs.tenant_code),
            server_id: config
                .recovery
                .as_ref()
                .map(|retained| retained.server_id()),
        };
        atomic_write(
            &self.directory.join(DesktopConnectionSettings::FILE_NAME),
            &serde_json::to_vec(&settings).map_err(|_| authentication_failure())?,
        )?;
        Ok(retained.is_none())
    }

    fn import_account(
        &self,
        authentication: &AuthenticationResult,
        profile: &DesktopEnrollmentProfile,
        inputs: &DesktopEnrollmentSettings,
        password: &str,
    ) -> Result<(), TransportFailure> {
        let now = unix_millis_now();
        let installer = self
            .store
            .initialize_server_installation(
                authentication.session.tenant_id,
                inputs.organization,
                authentication.session.device_id,
                inputs.branch,
                now,
            )
            .map_err(|_| authentication_failure())?;
        let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes())
            .map_err(|_| authentication_failure())?;
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| authentication_failure())?
            .to_string();
        self.store
            .provision_desktop_account(
                &installer,
                &DesktopAccount {
                    account_id: authentication.session.account_id,
                    user_id: authentication.session.user_id,
                    tenant_id: authentication.session.tenant_id,
                    organization_id: inputs.organization,
                    password_hash: hash,
                    role: profile.role.into(),
                },
                &profile.username,
                now,
            )
            .map_err(|_| authentication_failure())?;
        Ok(())
    }

    fn inputs(
        request: &DesktopSignInRequest,
        retained: Option<&DesktopConnectionSettings>,
    ) -> Result<DesktopEnrollmentSettings, TransportFailure> {
        let inputs = match (&request.connection, retained) {
            (Some(inputs), _) => inputs.clone(),
            (None, Some(settings)) => DesktopEnrollmentSettings {
                endpoint: settings.endpoint.clone(),
                trusted_certificate: settings.trusted_certificate.to_string_lossy().into_owned(),
                tenant_code: settings
                    .tenant_code
                    .clone()
                    .ok_or_else(authentication_failure)?,
                organization: settings.organization,
                branch: settings.branch,
            },
            _ => return Err(authentication_failure()),
        };
        if let Some(settings) = retained {
            let expected = url::Url::parse(&settings.endpoint).map_err(|_| encryption_failure())?;
            let actual = url::Url::parse(&inputs.endpoint).map_err(|_| encryption_failure())?;
            if actual != expected
                || inputs.organization != settings.organization
                || inputs.branch != settings.branch
                || settings
                    .tenant_code
                    .as_ref()
                    .is_some_and(|code| code != &inputs.tenant_code)
                || read_trust(Path::new(&inputs.trusted_certificate))?
                    != read_trust(&settings.trusted_certificate)?
            {
                return Err(encryption_failure());
            }
        }
        Ok(inputs)
    }

    fn device(
        &self,
        retained: Option<&DesktopConnectionSettings>,
    ) -> Result<(DeviceId, SecretId), TransportFailure> {
        let path = self.directory.join("desktop-device.json");
        let existing = if self
            .store
            .has_local_installation()
            .map_err(|_| authentication_failure())?
        {
            self.store
                .existing_local_authorization_context()
                .map_err(|_| authentication_failure())?
                .identity
                .device_id
        } else {
            None
        };
        let device = match fs::read(&path) {
            Ok(bytes) => {
                serde_json::from_slice::<DeviceId>(&bytes).map_err(|_| authentication_failure())?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if retained.is_some() || self.paired_connection()?.is_some() {
                    return Err(authentication_failure());
                }
                let device = existing.unwrap_or_else(|| DeviceId::new(Uuid::new_v4()));
                atomic_write(
                    &path,
                    &serde_json::to_vec(&device).map_err(|_| authentication_failure())?,
                )?;
                device
            }
            Err(_) => return Err(authentication_failure()),
        };
        if existing.is_some_and(|id| id != device) || device.value().is_nil() {
            return Err(authentication_failure());
        }
        let route = retained.map_or_else(
            || {
                Ok(SecretId::new(
                    SecretKind::parse("desktop-route").map_err(|_| authentication_failure())?,
                    SecretReferenceId::new(device.value()),
                ))
            },
            |s| Ok(s.credential.clone()),
        )?;
        Ok((device, route))
    }

    fn signing_key(
        &self,
        device: DeviceId,
        route: &SecretId,
    ) -> Result<SigningKey, TransportFailure> {
        let id = SecretId::new(
            SecretKind::parse("desktop-device-key").map_err(|_| authentication_failure())?,
            SecretReferenceId::new(device.value()),
        );
        let seed = if let Some(material) = self
            .secrets
            .get(&id)
            .map_err(|_| authentication_failure())?
        {
            Zeroizing::new(
                material
                    .expose_secret()
                    .try_into()
                    .map_err(|_| authentication_failure())?,
            )
        } else {
            let seed = if let Some(seed) = self.imported_device_seed(device, route)? {
                seed
            } else {
                if DesktopConnectionSettings::load(&self.directory)?.is_some()
                    || self.paired_connection()?.is_some()
                    || self
                        .store
                        .has_local_installation()
                        .map_err(|_| authentication_failure())?
                {
                    return Err(authentication_failure());
                }
                let mut seed = Zeroizing::new([0_u8; 32]);
                rand::rng().fill_bytes(seed.as_mut());
                seed
            };
            self.secrets
                .set(
                    &id,
                    SecretMaterial::new(seed.to_vec()).map_err(|_| authentication_failure())?,
                )
                .map_err(|_| authentication_failure())?;
            seed
        };
        Ok(SigningKey::from_bytes(&seed))
    }

    // Retained installations already have this key inside their account credentials.
    // Extract it before token refresh can discard an expired token pair.
    fn imported_device_seed(
        &self,
        device: DeviceId,
        route: &SecretId,
    ) -> Result<Option<Zeroizing<[u8; 32]>>, TransportFailure> {
        if !self
            .store
            .has_local_installation()
            .map_err(|_| authentication_failure())?
        {
            return Ok(None);
        }
        let installer = self
            .store
            .existing_local_authorization_context()
            .map_err(|_| authentication_failure())?;
        for account in self
            .store
            .list_desktop_accounts(installer.tenant_id)
            .map_err(|_| authentication_failure())?
            .accounts
        {
            let id = super::account_credential_id(
                route,
                eitmad_contracts::identity::PrincipalId::new(account.user_id.value()),
            );
            if let Some(material) = self
                .secrets
                .get(&id)
                .map_err(|_| authentication_failure())?
            {
                let credential: super::StoredCredential =
                    serde_json::from_slice(material.expose_secret())
                        .map_err(|_| authentication_failure())?;
                if credential.device_id != device
                    || credential.tenant_id != installer.tenant_id
                    || credential.user_id != account.user_id
                    || credential.account_id != account.account_id
                {
                    return Err(authentication_failure());
                }
                return Ok(Some(Zeroizing::new(credential.signing_seed)));
            }
        }
        Ok(None)
    }

    fn validate_identity(
        &self,
        auth: &AuthenticationResult,
        profile: &DesktopEnrollmentProfile,
        inputs: &DesktopEnrollmentSettings,
    ) -> Result<(), TransportFailure> {
        if auth.session != profile.session
            || profile.organization != inputs.organization
            || profile.branch != inputs.branch
        {
            return Err(authentication_failure());
        }
        if self
            .store
            .has_local_installation()
            .map_err(|_| authentication_failure())?
        {
            let installer = self
                .store
                .existing_local_authorization_context()
                .map_err(|_| authentication_failure())?;
            if installer.tenant_id != auth.session.tenant_id
                || installer.identity.device_id != Some(auth.session.device_id)
                || self
                    .store
                    .local_organization_id()
                    .map_err(|_| authentication_failure())?
                    != inputs.organization
            {
                return Err(authentication_failure());
            }
            if let Some(account) = self
                .store
                .desktop_account(installer.tenant_id, &profile.username)
                .map_err(|_| authentication_failure())?
            {
                if account.account_id != auth.session.account_id
                    || account.user_id != auth.session.user_id
                {
                    return Err(authentication_failure());
                }
            }
        }
        Ok(())
    }
}

fn proof(device: DeviceId, key: &SigningKey) -> DeviceProof {
    let issued_at = unix_millis_now();
    let nonce = Uuid::new_v4().to_string();
    let message = format!("{}\n{}\n{nonce}", device.value(), issued_at.0);
    DeviceProof {
        device_id: device,
        issued_at,
        nonce,
        signature_base64: URL_SAFE_NO_PAD.encode(key.sign(message.as_bytes()).to_bytes()),
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), TransportFailure> {
    let pending = path.with_extension(format!("{}.next", Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&pending, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(pending);
    }
    result.map_err(|_| authentication_failure())
}

pub(crate) fn read_trust(path: &Path) -> Result<Vec<u8>, TransportFailure> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(16_385).read_to_end(&mut bytes))
        .map_err(|_| encryption_failure())?;
    if bytes.len() > 16_384 {
        return Err(encryption_failure());
    }
    Ok(bytes)
}

fn exchange<T: DeserializeOwned>(
    config: &DirectServerConfig,
    route: &str,
    input: &impl Serialize,
    auth: Option<(&str, DeviceProof)>,
) -> Result<T, TransportFailure> {
    let deadline = Instant::now() + super::CONNECT_TIMEOUT + super::IO_TIMEOUT;
    let mut url = config.endpoint.clone();
    url.set_path(route);
    let mut socket = config.connect(&url, deadline)?;
    let body = Zeroizing::new(serde_json::to_vec(input).map_err(|_| authentication_failure())?);
    let mut headers = Zeroizing::new(format!(
        "{} {route} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n",
        if route == "/v1/pairing/offer" {
            "GET"
        } else {
            "POST"
        },
        super::host_header(&url)?,
        body.len()
    ));
    if let Some((token, proof)) = auth {
        headers.push_str(&format!(
            "Authorization: Bearer {token}\r\nx-eitmad-device-proof: {}\r\n",
            URL_SAFE_NO_PAD
                .encode(serde_json::to_vec(&proof).map_err(|_| authentication_failure())?)
        ));
    }
    headers.push_str("\r\n");
    socket
        .write_all(headers.as_bytes())
        .and_then(|()| socket.write_all(&body))
        .map_err(|_| super::unavailable(eitmad_sync::FailurePhase::Send))?;
    let mut response = Zeroizing::new(Vec::new());
    socket
        .take(32_769)
        .read_to_end(&mut response)
        .map_err(|_| super::unavailable(eitmad_sync::FailurePhase::Receive))?;
    if response.len() > 32_768 {
        return Err(authentication_failure());
    }
    let (status, body) = super::parse_http_response(&response)?;
    let body = Zeroizing::new(body);
    if status != 200 {
        return Err(authentication_failure());
    }
    serde_json::from_slice(&body).map_err(|_| authentication_failure())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_contracts::{
        accounts::DesktopAccountRole,
        identity::{AccountId, OrganizationId, ScopeId, SessionId, TenantId, UserId},
        server::{AuthenticatedServerSession, IssuedTokens, TenantCode},
        transport::UnixMillis,
    };

    #[test]
    fn enrollment_rejects_server_and_retained_identity_mismatches_without_replacing_accounts() {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let secrets = SecretStore::open(
            directory.path(),
            Some(eitmad_secret_storage::FallbackEncryptionKey::new([97; 32])),
        )
        .unwrap();
        let service = DesktopEnrollment::new(directory.path().into(), store.clone(), secrets);
        let inputs = DesktopEnrollmentSettings {
            endpoint: "https://localhost/".into(),
            trusted_certificate: "unused.pem".into(),
            tenant_code: TenantCode::parse("synthetic").unwrap(),
            organization: OrganizationId::new(Uuid::new_v4()),
            branch: ScopeId::new(Uuid::new_v4()),
        };
        let auth = AuthenticationResult {
            session: AuthenticatedServerSession {
                session_id: SessionId::new(Uuid::new_v4()),
                account_id: AccountId::new(Uuid::new_v4()),
                user_id: UserId::new(Uuid::new_v4()),
                device_id: DeviceId::new(Uuid::new_v4()),
                tenant_id: TenantId::new(Uuid::new_v4()),
                issued_at: UnixMillis(1),
                expires_at: UnixMillis(i64::MAX),
            },
            tokens: IssuedTokens {
                access_token: "synthetic".into(),
                refresh_token: "synthetic".into(),
                access_expires_at: UnixMillis(i64::MAX - 1),
                refresh_expires_at: UnixMillis(i64::MAX),
            },
        };
        let profile = DesktopEnrollmentProfile {
            session: auth.session.clone(),
            organization: inputs.organization,
            branch: inputs.branch,
            username: "مدير".into(),
            role: DesktopAccountRole::Manager,
        };
        let mut foreign = profile.clone();
        foreign.session.device_id = DeviceId::new(Uuid::new_v4());
        assert!(service.validate_identity(&auth, &foreign, &inputs).is_err());
        assert!(!store.has_local_installation().unwrap());
        service
            .import_account(&auth, &profile, &inputs, "synthetic-password-123")
            .unwrap();
        let original = store
            .desktop_account(auth.session.tenant_id, "مدير")
            .unwrap()
            .unwrap();
        for field in ["tenant", "user", "account", "device"] {
            let mut foreign = auth.clone();
            match field {
                "tenant" => foreign.session.tenant_id = TenantId::new(Uuid::new_v4()),
                "user" => foreign.session.user_id = UserId::new(Uuid::new_v4()),
                "account" => foreign.session.account_id = AccountId::new(Uuid::new_v4()),
                _ => foreign.session.device_id = DeviceId::new(Uuid::new_v4()),
            }
            let profile = DesktopEnrollmentProfile {
                session: foreign.session.clone(),
                ..profile.clone()
            };
            assert!(
                service
                    .validate_identity(&foreign, &profile, &inputs)
                    .is_err(),
                "{field}"
            );
        }
        let retained = store
            .desktop_account(auth.session.tenant_id, "مدير")
            .unwrap()
            .unwrap();
        assert_eq!(original.account_id, retained.account_id);
        assert_eq!(original.password_hash, retained.password_hash);
    }
}
