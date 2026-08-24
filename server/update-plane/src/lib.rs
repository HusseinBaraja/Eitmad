//! Signed update manifest publication and channel distribution.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use eitmad_contracts::{
    server::{AuthenticatedServerSession, UpdateChannelId},
    transport::{CorrelationId, UnixMillis},
    updates::{SignedUpdateManifest, UpdateCheckOutcome, UpdateClientProfile, UpdateManifestId},
};
use eitmad_release_policy::{TrustedUpdateKeys, evaluate_update, verify_manifest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdatePlaneAction {
    PublishManifest,
    RevokeManifest,
}

impl UpdatePlaneAction {
    #[must_use]
    pub const fn operation(self) -> &'static str {
        match self {
            Self::PublishManifest => "eitmad.update.manifest.publish.v1",
            Self::RevokeManifest => "eitmad.update.manifest.revoke.v1",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateAuditOutcome {
    Succeeded,
    Denied,
    Invalid,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UpdatePlaneError {
    #[error("update publication is denied")]
    Denied,
    #[error("update manifest is invalid")]
    Invalid,
    #[error("update manifest already exists")]
    Conflict,
    #[error("update manifest was not found")]
    NotFound,
    #[error("update distribution is unavailable")]
    Unavailable,
}

#[async_trait]
pub trait UpdatePublicationSecurity: Send + Sync {
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        action: UpdatePlaneAction,
        channel: &UpdateChannelId,
    ) -> Result<(), UpdatePlaneError>;

    async fn audit(
        &self,
        actor: &AuthenticatedServerSession,
        action: UpdatePlaneAction,
        outcome: UpdateAuditOutcome,
        correlation_id: CorrelationId,
        manifest_id: UpdateManifestId,
        now: UnixMillis,
    ) -> Result<(), UpdatePlaneError>;
}

pub trait ManifestRepository: Send + Sync {
    /// Lists all manifests in one exact channel.
    ///
    /// # Errors
    ///
    /// Returns a stable repository availability or data error.
    fn list(
        &self,
        channel: &UpdateChannelId,
    ) -> Result<Vec<SignedUpdateManifest>, UpdatePlaneError>;
    /// Inserts one immutable manifest.
    ///
    /// # Errors
    ///
    /// Returns a conflict or repository availability error.
    fn insert(&self, manifest: &SignedUpdateManifest) -> Result<(), UpdatePlaneError>;
    /// Removes a just-inserted manifest when audit commit fails.
    ///
    /// # Errors
    ///
    /// Returns a repository availability error.
    fn remove(&self, manifest: &SignedUpdateManifest) -> Result<(), UpdatePlaneError>;
}

#[derive(Default)]
pub struct MemoryManifestRepository {
    manifests: RwLock<Vec<SignedUpdateManifest>>,
}

impl ManifestRepository for MemoryManifestRepository {
    fn list(
        &self,
        channel: &UpdateChannelId,
    ) -> Result<Vec<SignedUpdateManifest>, UpdatePlaneError> {
        Ok(self
            .manifests
            .read()
            .map_err(|_| UpdatePlaneError::Unavailable)?
            .iter()
            .filter(|signed| signed.manifest.channel == *channel)
            .cloned()
            .collect())
    }

    fn insert(&self, manifest: &SignedUpdateManifest) -> Result<(), UpdatePlaneError> {
        let mut manifests = self
            .manifests
            .write()
            .map_err(|_| UpdatePlaneError::Unavailable)?;
        if manifests
            .iter()
            .any(|existing| existing.manifest.manifest_id == manifest.manifest.manifest_id)
        {
            return Err(UpdatePlaneError::Conflict);
        }
        manifests.push(manifest.clone());
        Ok(())
    }

    fn remove(&self, manifest: &SignedUpdateManifest) -> Result<(), UpdatePlaneError> {
        let mut manifests = self
            .manifests
            .write()
            .map_err(|_| UpdatePlaneError::Unavailable)?;
        manifests.retain(|existing| existing.manifest.manifest_id != manifest.manifest.manifest_id);
        Ok(())
    }
}

/// Durable manifest repository with one immutable JSON file per manifest.
pub struct FileManifestRepository {
    root: PathBuf,
}

impl FileManifestRepository {
    /// Opens a dedicated update-manifest directory.
    ///
    /// # Errors
    ///
    /// Returns an availability error when the directory cannot be created.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, UpdatePlaneError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|_| UpdatePlaneError::Unavailable)?;
        Ok(Self { root })
    }

    fn channel_directory(&self, channel: &UpdateChannelId) -> PathBuf {
        self.root.join(channel.as_str())
    }

    fn path_for(&self, manifest: &SignedUpdateManifest) -> PathBuf {
        self.channel_directory(&manifest.manifest.channel)
            .join(format!("{}.json", manifest.manifest.manifest_id.value()))
    }
}

impl ManifestRepository for FileManifestRepository {
    fn list(
        &self,
        channel: &UpdateChannelId,
    ) -> Result<Vec<SignedUpdateManifest>, UpdatePlaneError> {
        let directory = self.channel_directory(channel);
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut manifests = Vec::new();
        for entry in fs::read_dir(directory).map_err(|_| UpdatePlaneError::Unavailable)? {
            let path = entry.map_err(|_| UpdatePlaneError::Unavailable)?.path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let bytes = fs::read(path).map_err(|_| UpdatePlaneError::Unavailable)?;
            manifests.push(serde_json::from_slice(&bytes).map_err(|_| UpdatePlaneError::Invalid)?);
        }
        Ok(manifests)
    }

    fn insert(&self, manifest: &SignedUpdateManifest) -> Result<(), UpdatePlaneError> {
        let directory = self.channel_directory(&manifest.manifest.channel);
        fs::create_dir_all(&directory).map_err(|_| UpdatePlaneError::Unavailable)?;
        let path = self.path_for(manifest);
        if path.exists() {
            return Err(UpdatePlaneError::Conflict);
        }
        let temporary = path.with_extension("json.pending");
        let bytes = serde_json::to_vec_pretty(manifest).map_err(|_| UpdatePlaneError::Invalid)?;
        fs::write(&temporary, bytes).map_err(|_| UpdatePlaneError::Unavailable)?;
        fs::rename(&temporary, &path).map_err(|_| UpdatePlaneError::Unavailable)
    }

    fn remove(&self, manifest: &SignedUpdateManifest) -> Result<(), UpdatePlaneError> {
        let path = self.path_for(manifest);
        if path.exists() {
            fs::remove_file(path).map_err(|_| UpdatePlaneError::Unavailable)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct UpdateCatalog {
    security: Arc<dyn UpdatePublicationSecurity>,
    repository: Arc<dyn ManifestRepository>,
    trusted_keys: TrustedUpdateKeys,
    channels: BTreeSet<UpdateChannelId>,
}

impl UpdateCatalog {
    #[must_use]
    pub fn new(
        security: Arc<dyn UpdatePublicationSecurity>,
        repository: Arc<dyn ManifestRepository>,
        trusted_keys: TrustedUpdateKeys,
    ) -> Self {
        Self {
            security,
            repository,
            trusted_keys,
            channels: ["stable", "beta", "canary"]
                .into_iter()
                .filter_map(|channel| UpdateChannelId::parse(channel).ok())
                .collect(),
        }
    }

    /// Publishes one authorized, signed, immutable channel manifest.
    ///
    /// # Errors
    ///
    /// Returns a denial, validation, conflict, repository, or audit error.
    pub async fn publish(
        &self,
        actor: &AuthenticatedServerSession,
        signed: &SignedUpdateManifest,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), UpdatePlaneError> {
        let manifest = &signed.manifest;
        if let Err(error) = self
            .security
            .authorize(actor, UpdatePlaneAction::PublishManifest, &manifest.channel)
            .await
        {
            self.security
                .audit(
                    actor,
                    UpdatePlaneAction::PublishManifest,
                    UpdateAuditOutcome::Denied,
                    correlation_id,
                    manifest.manifest_id,
                    now,
                )
                .await?;
            return Err(error);
        }
        if !self.valid_manifest(signed)? {
            self.security
                .audit(
                    actor,
                    UpdatePlaneAction::PublishManifest,
                    UpdateAuditOutcome::Invalid,
                    correlation_id,
                    manifest.manifest_id,
                    now,
                )
                .await?;
            return Err(UpdatePlaneError::Invalid);
        }
        self.repository.insert(signed)?;
        if let Err(error) = self
            .security
            .audit(
                actor,
                UpdatePlaneAction::PublishManifest,
                UpdateAuditOutcome::Succeeded,
                correlation_id,
                manifest.manifest_id,
                now,
            )
            .await
        {
            self.repository.remove(signed)?;
            return Err(error);
        }
        Ok(())
    }

    /// Selects the newest signed manifest for one exact channel and client.
    ///
    /// # Errors
    ///
    /// Returns a repository availability or stored-data error.
    pub fn check(
        &self,
        client: &UpdateClientProfile,
        now: UnixMillis,
    ) -> Result<UpdateCheckOutcome, UpdatePlaneError> {
        let mut manifests = self.repository.list(&client.channel)?;
        manifests.sort_by(|left, right| {
            right
                .manifest
                .version
                .value()
                .cmp(left.manifest.version.value())
        });
        Ok(manifests
            .first()
            .map_or(UpdateCheckOutcome::UpToDate, |manifest| {
                evaluate_update(manifest, &self.trusted_keys, client, now)
            }))
    }

    fn valid_manifest(&self, signed: &SignedUpdateManifest) -> Result<bool, UpdatePlaneError> {
        let manifest = &signed.manifest;
        if verify_manifest(signed, &self.trusted_keys).is_err()
            || manifest.schema_version != 1
            || !self.channels.contains(&manifest.channel)
            || manifest.rollout.percentage_bps > 10_000
            || manifest.rollout.cohort_seed.is_empty()
            || manifest.packages.is_empty()
            || manifest
                .compatibility
                .minimum_current_version
                .as_ref()
                .is_some_and(|minimum| {
                    manifest
                        .compatibility
                        .maximum_current_version
                        .as_ref()
                        .is_some_and(|maximum| minimum.value() > maximum.value())
                })
            || (manifest.channel.as_str() == "stable" && !manifest.version.value().pre.is_empty())
            || manifest.packages.iter().any(|package| {
                !package.download_url.starts_with("https://")
                    || package.size_bytes == 0
                    || package.sha256_hex.len() != 64
                    || !package
                        .sha256_hex
                        .bytes()
                        .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
            })
        {
            return Ok(false);
        }
        let packages = manifest
            .packages
            .iter()
            .map(|package| {
                (
                    &package.platform,
                    &package.architecture,
                    &package.package_kind,
                )
            })
            .collect::<BTreeSet<_>>();
        if packages.len() != manifest.packages.len() {
            return Ok(false);
        }
        let existing = self.repository.list(&manifest.channel)?;
        Ok(!existing.iter().any(|current| {
            current.manifest.manifest_id == manifest.manifest_id
                || current.manifest.version == manifest.version
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signer as _, SigningKey};
    use eitmad_contracts::{
        identity::{AccountId, DeviceId, SessionId, TenantId, UserId},
        transport::CapabilityId,
        updates::{
            ReleaseVersion, StagedRollout, UpdateArchitectureId, UpdateCompatibilityRules,
            UpdateManifest, UpdateManifestSignature, UpdatePackageKind, UpdatePackageMetadata,
            UpdatePlatformId, UpdateSigningKeyId,
        },
    };
    use eitmad_release_policy::manifest_signing_bytes;
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct TestSecurity {
        allowed: bool,
        audits: Mutex<Vec<UpdateAuditOutcome>>,
    }

    #[async_trait]
    impl UpdatePublicationSecurity for TestSecurity {
        async fn authorize(
            &self,
            _: &AuthenticatedServerSession,
            _: UpdatePlaneAction,
            _: &UpdateChannelId,
        ) -> Result<(), UpdatePlaneError> {
            self.allowed.then_some(()).ok_or(UpdatePlaneError::Denied)
        }

        async fn audit(
            &self,
            _: &AuthenticatedServerSession,
            _: UpdatePlaneAction,
            outcome: UpdateAuditOutcome,
            _: CorrelationId,
            _: UpdateManifestId,
            _: UnixMillis,
        ) -> Result<(), UpdatePlaneError> {
            self.audits.lock().unwrap().push(outcome);
            Ok(())
        }
    }

    fn actor() -> AuthenticatedServerSession {
        AuthenticatedServerSession {
            session_id: SessionId::new(Uuid::from_u128(1)),
            account_id: AccountId::new(Uuid::from_u128(2)),
            user_id: UserId::new(Uuid::from_u128(3)),
            device_id: DeviceId::new(Uuid::from_u128(4)),
            tenant_id: TenantId::new(Uuid::from_u128(5)),
            issued_at: UnixMillis(0),
            expires_at: UnixMillis(i64::MAX),
        }
    }

    fn signed(
        channel: &str,
        version: semver::Version,
    ) -> (SignedUpdateManifest, TrustedUpdateKeys) {
        let key = SigningKey::from_bytes(&[9; 32]);
        let key_id = UpdateSigningKeyId::parse("release-2026").unwrap();
        let manifest = UpdateManifest {
            schema_version: 1,
            manifest_id: UpdateManifestId::new(Uuid::new_v4()),
            channel: UpdateChannelId::parse(channel).unwrap(),
            version: ReleaseVersion::new(version),
            published_at: UnixMillis(10),
            rollout: StagedRollout {
                percentage_bps: 10_000,
                cohort_seed: "seed".to_owned(),
                starts_at: UnixMillis(10),
                paused: false,
            },
            compatibility: UpdateCompatibilityRules {
                minimum_current_version: None,
                maximum_current_version: None,
                blocked_current_versions: Vec::new(),
                minimum_protocol_major: 1,
                minimum_protocol_minor: 5,
                required_capabilities: vec![
                    CapabilityId::parse("eitmad.capability.update.v1").unwrap(),
                ],
            },
            packages: vec![UpdatePackageMetadata {
                platform: UpdatePlatformId::parse("windows").unwrap(),
                architecture: UpdateArchitectureId::parse("x86-64").unwrap(),
                package_kind: UpdatePackageKind::parse("msix").unwrap(),
                download_url: "https://updates.example/app.msix".to_owned(),
                size_bytes: 10,
                sha256_hex: "a".repeat(64),
            }],
            revoked: false,
        };
        let signature = key.sign(&manifest_signing_bytes(&manifest).unwrap());
        let signed = SignedUpdateManifest {
            manifest,
            signature: UpdateManifestSignature {
                algorithm: "ed25519".to_owned(),
                key_id: key_id.clone(),
                signature_base64: STANDARD.encode(signature.to_bytes()),
            },
        };
        let mut trusted = TrustedUpdateKeys::new();
        trusted.insert(key_id, key.verifying_key());
        (signed, trusted)
    }

    #[tokio::test]
    async fn publication_requires_authorization_and_rolls_back_on_denial() {
        let (manifest, keys) = signed("stable", semver::Version::new(2, 0, 0));
        let security = Arc::new(TestSecurity::default());
        let repository = Arc::new(MemoryManifestRepository::default());
        let catalog = UpdateCatalog::new(security.clone(), repository.clone(), keys);
        assert_eq!(
            catalog
                .publish(
                    &actor(),
                    &manifest,
                    CorrelationId::new(Uuid::new_v4()),
                    UnixMillis(20)
                )
                .await,
            Err(UpdatePlaneError::Denied)
        );
        assert!(
            repository
                .list(&manifest.manifest.channel)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            security.audits.lock().unwrap().as_slice(),
            &[UpdateAuditOutcome::Denied]
        );
    }

    #[tokio::test]
    async fn channel_rules_reject_prerelease_on_stable_and_duplicate_versions() {
        let (prerelease, keys) = signed("stable", semver::Version::parse("2.0.0-beta.1").unwrap());
        let security = Arc::new(TestSecurity {
            allowed: true,
            ..TestSecurity::default()
        });
        let repository = Arc::new(MemoryManifestRepository::default());
        let catalog = UpdateCatalog::new(security, repository.clone(), keys);
        assert_eq!(
            catalog
                .publish(
                    &actor(),
                    &prerelease,
                    CorrelationId::new(Uuid::new_v4()),
                    UnixMillis(20)
                )
                .await,
            Err(UpdatePlaneError::Invalid)
        );

        let (first, keys) = signed("stable", semver::Version::new(2, 0, 0));
        let catalog = UpdateCatalog::new(
            Arc::new(TestSecurity {
                allowed: true,
                ..TestSecurity::default()
            }),
            repository,
            keys,
        );
        catalog
            .publish(
                &actor(),
                &first,
                CorrelationId::new(Uuid::new_v4()),
                UnixMillis(20),
            )
            .await
            .unwrap();
        let (duplicate, _) = signed("stable", semver::Version::new(2, 0, 0));
        assert_eq!(
            catalog
                .publish(
                    &actor(),
                    &duplicate,
                    CorrelationId::new(Uuid::new_v4()),
                    UnixMillis(21)
                )
                .await,
            Err(UpdatePlaneError::Invalid)
        );
    }

    #[test]
    fn file_repository_reloads_signed_manifests() {
        let directory = TempDir::new().unwrap();
        let repository = FileManifestRepository::open(directory.path()).unwrap();
        let (manifest, _) = signed("beta", semver::Version::parse("2.0.0-beta.1").unwrap());
        repository.insert(&manifest).unwrap();
        assert_eq!(
            repository.list(&manifest.manifest.channel).unwrap(),
            vec![manifest]
        );
    }
}
