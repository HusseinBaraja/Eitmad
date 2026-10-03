use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    identity::DeviceId,
    server::UpdateChannelId,
    transport::{CapabilityId, UnixMillis},
};

uuid_id!(UpdateManifestId);
open_id!(UpdateSigningKeyId, "update signing key identifier");
open_id!(UpdatePlatformId, "update platform identifier");
open_id!(UpdateArchitectureId, "update architecture identifier");
open_id!(UpdatePackageKind, "update package kind");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ReleaseVersion(semver::Version);

impl ReleaseVersion {
    #[must_use]
    pub const fn new(value: semver::Version) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> &semver::Version {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCompatibilityRules {
    pub minimum_current_version: Option<ReleaseVersion>,
    pub maximum_current_version: Option<ReleaseVersion>,
    pub blocked_current_versions: Vec<ReleaseVersion>,
    pub minimum_protocol_major: u16,
    pub minimum_protocol_minor: u16,
    pub required_capabilities: Vec<CapabilityId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StagedRollout {
    #[schemars(range(max = 10000))]
    pub percentage_bps: u16,
    pub cohort_seed: String,
    pub starts_at: UnixMillis,
    pub paused: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePackageMetadata {
    pub platform: UpdatePlatformId,
    pub architecture: UpdateArchitectureId,
    pub package_kind: UpdatePackageKind,
    pub download_url: String,
    pub size_bytes: u64,
    pub sha256_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub schema_version: u16,
    pub manifest_id: UpdateManifestId,
    pub channel: UpdateChannelId,
    pub version: ReleaseVersion,
    pub published_at: UnixMillis,
    pub rollout: StagedRollout,
    pub compatibility: UpdateCompatibilityRules,
    pub packages: Vec<UpdatePackageMetadata>,
    pub revoked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifestSignature {
    pub algorithm: String,
    pub key_id: UpdateSigningKeyId,
    pub signature_base64: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignedUpdateManifest {
    pub manifest: UpdateManifest,
    pub signature: UpdateManifestSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClientProfile {
    pub device_id: DeviceId,
    pub channel: UpdateChannelId,
    pub current_version: ReleaseVersion,
    pub protocol_major: u16,
    pub protocol_minor: u16,
    pub capabilities: Vec<CapabilityId>,
    pub platform: UpdatePlatformId,
    pub architecture: UpdateArchitectureId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UpdateIneligibilityReason {
    ChannelMismatch,
    ManifestRevoked,
    RolloutPaused,
    RolloutNotStarted,
    OutsideRollout,
    ClientTooOld,
    ClientTooNew,
    ClientBlocked,
    ProtocolIncompatible,
    CapabilityMissing,
    PackageUnavailable,
    SignatureInvalid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UpdateCheckOutcome {
    UpToDate,
    Available {
        manifest: Box<SignedUpdateManifest>,
        package: UpdatePackageMetadata,
    },
    Ineligible {
        reason: UpdateIneligibilityReason,
    },
    Incompatible {
        reason: UpdateIneligibilityReason,
    },
}
