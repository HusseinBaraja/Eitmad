//! Pairing v1. Only the Owner response carries a code; joining clients send OPAQUE messages.
use crate::{
    discovery::ServerInstallationId,
    identity::{DeviceId, OrganizationId, ScopeId, TenantId},
    server::{DevicePublicKey, TenantCode},
    transport::UnixMillis,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairingIdentity {
    pub server_id: ServerInstallationId,
    pub endpoint: String,
    pub public_ca_pem: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenPairing {
    pub version: u16,
    pub organization_id: OrganizationId,
    pub branch_id: ScopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairingOffer {
    pub version: u16,
    pub session_id: Uuid,
    pub identity: PairingIdentity,
    pub tenant_id: TenantId,
    pub tenant_code: TenantCode,
    pub organization_id: OrganizationId,
    pub branch_id: ScopeId,
    pub expires_at: UnixMillis,
}

/// Transient Owner-only secret. Do not log or persist the code.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenedPairing {
    pub offer: PairingOffer,
    pub code: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairingBinding {
    pub offer: PairingOffer,
    pub attempt_id: Uuid,
    pub device_id: DeviceId,
    pub device_public_key: DevicePublicKey,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartPairing {
    pub binding: PairingBinding,
    pub credential_request: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairingChallenge {
    pub binding: PairingBinding,
    pub credential_response: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletePairing {
    pub binding: PairingBinding,
    pub credential_finalization: String,
    pub device_signature: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairingReceipt {
    pub binding: PairingBinding,
    pub confirmation: String,
}
