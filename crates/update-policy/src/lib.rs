//! Update eligibility, compatibility, rollout, and signature policy.

use std::collections::{BTreeMap, BTreeSet};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
use eitmad_contracts::updates::{
    SignedUpdateManifest, UpdateCheckOutcome, UpdateClientProfile, UpdateIneligibilityReason,
    UpdateManifest, UpdatePackageMetadata, UpdateSigningKeyId,
};
use sha2::{Digest as _, Sha256};

#[derive(Clone, Debug, Default)]
pub struct TrustedUpdateKeys {
    keys: BTreeMap<UpdateSigningKeyId, VerifyingKey>,
}

impl TrustedUpdateKeys {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key_id: UpdateSigningKeyId, key: VerifyingKey) {
        self.keys.insert(key_id, key);
    }

    #[must_use]
    pub fn contains(&self, key_id: &UpdateSigningKeyId) -> bool {
        self.keys.contains_key(key_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignatureVerificationError;

impl std::fmt::Display for SignatureVerificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("update manifest signature is invalid")
    }
}

impl std::error::Error for SignatureVerificationError {}

/// Produces the one canonical byte representation covered by update signatures.
///
/// # Errors
///
/// Returns an error only when the Rust-owned manifest cannot serialize.
pub fn manifest_signing_bytes(manifest: &UpdateManifest) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(manifest)
}

/// Verifies an Ed25519 signature against the configured key ring.
///
/// # Errors
///
/// Rejects unknown keys, algorithms, malformed signatures, and changed content.
pub fn verify_manifest(
    signed: &SignedUpdateManifest,
    trusted: &TrustedUpdateKeys,
) -> Result<(), SignatureVerificationError> {
    if signed.signature.algorithm != "ed25519" {
        return Err(SignatureVerificationError);
    }
    let key = trusted
        .keys
        .get(&signed.signature.key_id)
        .ok_or(SignatureVerificationError)?;
    let bytes = manifest_signing_bytes(&signed.manifest).map_err(|_| SignatureVerificationError)?;
    let encoded = STANDARD
        .decode(&signed.signature.signature_base64)
        .map_err(|_| SignatureVerificationError)?;
    let signature = Signature::from_slice(&encoded).map_err(|_| SignatureVerificationError)?;
    key.verify(&bytes, &signature)
        .map_err(|_| SignatureVerificationError)
}

#[must_use]
pub fn evaluate_update(
    signed: &SignedUpdateManifest,
    trusted: &TrustedUpdateKeys,
    client: &UpdateClientProfile,
    now: eitmad_contracts::transport::UnixMillis,
) -> UpdateCheckOutcome {
    if verify_manifest(signed, trusted).is_err() {
        return ineligible(UpdateIneligibilityReason::SignatureInvalid);
    }
    let manifest = &signed.manifest;
    if manifest.channel != client.channel {
        return ineligible(UpdateIneligibilityReason::ChannelMismatch);
    }
    if manifest.revoked {
        return ineligible(UpdateIneligibilityReason::ManifestRevoked);
    }
    if manifest.rollout.paused {
        return ineligible(UpdateIneligibilityReason::RolloutPaused);
    }
    if now.0 < manifest.rollout.starts_at.0 {
        return ineligible(UpdateIneligibilityReason::RolloutNotStarted);
    }
    if client.current_version.value() >= manifest.version.value() {
        return UpdateCheckOutcome::UpToDate;
    }
    let rules = &manifest.compatibility;
    if rules
        .minimum_current_version
        .as_ref()
        .is_some_and(|minimum| client.current_version.value() < minimum.value())
    {
        return incompatible(UpdateIneligibilityReason::ClientTooOld);
    }
    if rules
        .maximum_current_version
        .as_ref()
        .is_some_and(|maximum| client.current_version.value() > maximum.value())
    {
        return incompatible(UpdateIneligibilityReason::ClientTooNew);
    }
    if rules
        .blocked_current_versions
        .iter()
        .any(|version| version == &client.current_version)
    {
        return incompatible(UpdateIneligibilityReason::ClientBlocked);
    }
    if (client.protocol_major, client.protocol_minor)
        < (rules.minimum_protocol_major, rules.minimum_protocol_minor)
    {
        return incompatible(UpdateIneligibilityReason::ProtocolIncompatible);
    }
    let client_capabilities = client.capabilities.iter().collect::<BTreeSet<_>>();
    if rules
        .required_capabilities
        .iter()
        .any(|capability| !client_capabilities.contains(capability))
    {
        return incompatible(UpdateIneligibilityReason::CapabilityMissing);
    }
    if !included_in_rollout(manifest, client) {
        return ineligible(UpdateIneligibilityReason::OutsideRollout);
    }
    let Some(package) = select_package(&manifest.packages, client) else {
        return incompatible(UpdateIneligibilityReason::PackageUnavailable);
    };
    UpdateCheckOutcome::Available {
        manifest: Box::new(signed.clone()),
        package: package.clone(),
    }
}

fn select_package<'a>(
    packages: &'a [UpdatePackageMetadata],
    client: &UpdateClientProfile,
) -> Option<&'a UpdatePackageMetadata> {
    packages.iter().find(|package| {
        package.platform == client.platform && package.architecture == client.architecture
    })
}

fn included_in_rollout(manifest: &UpdateManifest, client: &UpdateClientProfile) -> bool {
    if manifest.rollout.percentage_bps >= 10_000 {
        return true;
    }
    let mut hasher = Sha256::new();
    hasher.update(manifest.rollout.cohort_seed.as_bytes());
    hasher.update(client.device_id.value().as_bytes());
    hasher.update(manifest.manifest_id.value().as_bytes());
    let digest = hasher.finalize();
    let bucket = u16::from_be_bytes([digest[0], digest[1]]) % 10_000;
    bucket < manifest.rollout.percentage_bps
}

const fn ineligible(reason: UpdateIneligibilityReason) -> UpdateCheckOutcome {
    UpdateCheckOutcome::Ineligible { reason }
}

const fn incompatible(reason: UpdateIneligibilityReason) -> UpdateCheckOutcome {
    UpdateCheckOutcome::Incompatible { reason }
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signer as _, SigningKey};
    use eitmad_contracts::{
        identity::DeviceId,
        server::UpdateChannelId,
        transport::{CapabilityId, UnixMillis},
        updates::{
            ReleaseVersion, StagedRollout, UpdateArchitectureId, UpdateCompatibilityRules,
            UpdateManifestId, UpdateManifestSignature, UpdatePackageKind, UpdatePlatformId,
        },
    };
    use uuid::Uuid;

    use super::*;

    fn signed_manifest() -> (SignedUpdateManifest, TrustedUpdateKeys) {
        let key = SigningKey::from_bytes(&[7; 32]);
        let manifest = UpdateManifest {
            schema_version: 1,
            manifest_id: UpdateManifestId::new(Uuid::from_u128(1)),
            channel: UpdateChannelId::parse("stable").unwrap(),
            version: ReleaseVersion::new(semver::Version::new(2, 0, 0)),
            published_at: UnixMillis(10),
            rollout: StagedRollout {
                percentage_bps: 10_000,
                cohort_seed: "release-2".to_owned(),
                starts_at: UnixMillis(10),
                paused: false,
            },
            compatibility: UpdateCompatibilityRules {
                minimum_current_version: Some(ReleaseVersion::new(semver::Version::new(1, 0, 0))),
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
                download_url: "https://updates.example/eitmad.msix".to_owned(),
                size_bytes: 100,
                sha256_hex: "0".repeat(64),
            }],
            revoked: false,
        };
        let signature = key.sign(&manifest_signing_bytes(&manifest).unwrap());
        let key_id = UpdateSigningKeyId::parse("release-2026").unwrap();
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

    fn client() -> UpdateClientProfile {
        UpdateClientProfile {
            device_id: DeviceId::new(Uuid::from_u128(2)),
            channel: UpdateChannelId::parse("stable").unwrap(),
            current_version: ReleaseVersion::new(semver::Version::new(1, 5, 0)),
            protocol_major: 1,
            protocol_minor: 5,
            capabilities: vec![CapabilityId::parse("eitmad.capability.update.v1").unwrap()],
            platform: UpdatePlatformId::parse("windows").unwrap(),
            architecture: UpdateArchitectureId::parse("x86-64").unwrap(),
        }
    }

    #[test]
    fn manifest_signatures_reject_changed_content() {
        let (mut signed, trusted) = signed_manifest();
        assert!(verify_manifest(&signed, &trusted).is_ok());
        signed.manifest.rollout.paused = true;
        assert_eq!(
            verify_manifest(&signed, &trusted),
            Err(SignatureVerificationError)
        );
    }

    #[test]
    fn incompatible_clients_are_rejected_before_rollout() {
        let (signed, trusted) = signed_manifest();
        let mut client = client();
        client.protocol_minor = 4;
        assert_eq!(
            evaluate_update(&signed, &trusted, &client, UnixMillis(20)),
            UpdateCheckOutcome::Incompatible {
                reason: UpdateIneligibilityReason::ProtocolIncompatible
            }
        );
        client.protocol_minor = 5;
        client.current_version = ReleaseVersion::new(semver::Version::new(0, 9, 0));
        assert_eq!(
            evaluate_update(&signed, &trusted, &client, UnixMillis(20)),
            UpdateCheckOutcome::Incompatible {
                reason: UpdateIneligibilityReason::ClientTooOld
            }
        );
    }

    #[test]
    fn channel_rules_reject_cross_channel_manifests() {
        let (signed, trusted) = signed_manifest();
        let mut client = client();
        client.channel = UpdateChannelId::parse("beta").unwrap();
        assert_eq!(
            evaluate_update(&signed, &trusted, &client, UnixMillis(20)),
            UpdateCheckOutcome::Ineligible {
                reason: UpdateIneligibilityReason::ChannelMismatch
            }
        );
    }
}
