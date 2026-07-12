use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{errors::ErrorCode, transport::UpdateHandoffId};

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
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum InstallerOutcome {
    Succeeded { installed_version: ReleaseVersion },
    Failed { error_code: ErrorCode },
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum UpdateState {
    Idle,
    Checking,
    Available {
        version: ReleaseVersion,
    },
    Downloading {
        version: ReleaseVersion,
        #[schemars(range(max = 10000))]
        progress_bps: u16,
    },
    Paused {
        version: ReleaseVersion,
        #[schemars(range(max = 10000))]
        progress_bps: u16,
    },
    Preflight {
        version: ReleaseVersion,
    },
    Ready {
        version: ReleaseVersion,
    },
    InstallationHandoff {
        handoff_id: UpdateHandoffId,
        version: ReleaseVersion,
    },
    Installing {
        handoff_id: UpdateHandoffId,
        version: ReleaseVersion,
    },
    Verifying {
        version: ReleaseVersion,
    },
    Succeeded {
        version: ReleaseVersion,
    },
    Failed {
        version: Option<ReleaseVersion>,
        error_code: ErrorCode,
    },
    Revoked {
        version: ReleaseVersion,
    },
    RecoveryRequired {
        error_code: ErrorCode,
    },
}
