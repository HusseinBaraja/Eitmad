//! Versioned contracts for the remote Eitmad server boundary.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    identity::{AccountId, DeviceId, OrganizationId, SessionId, TenantId, UserId},
    sync::{RecordChangeNotice, SyncMessage},
    sync_transport::SyncTransportFrame,
    transport::{CorrelationId, EventCursor, SchemaId, UnixMillis},
    versioning::PeerHello,
};

uuid_id!(InviteId);
uuid_id!(TokenFamilyId);
uuid_id!(LicenseId);
uuid_id!(EntitlementAssignmentId);
uuid_id!(ServerEventId);

open_id!(EntitlementId, "license entitlement identifier");
open_id!(UpdateChannelId, "update channel identifier");
open_id!(ServerErrorCode, "server error identifier");

pub const SERVER_API_VERSION: u16 = 1;
pub const DEFAULT_ACCESS_TOKEN_TTL_MS: i64 = 15 * 60 * 1_000;
pub const DEFAULT_REFRESH_TOKEN_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1_000;
pub const DEFAULT_SESSION_IDLE_TTL_MS: i64 = 14 * 24 * 60 * 60 * 1_000;
pub const DEFAULT_INVITE_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1_000;
pub const DEFAULT_PASSWORD_RESET_TTL_MS: i64 = 30 * 60 * 1_000;
pub const DEFAULT_LICENSE_GRACE_MS: i64 = 7 * 24 * 60 * 60 * 1_000;
pub const DEFAULT_SYNC_HISTORY_FLOOR_MS: i64 = 90 * 24 * 60 * 60 * 1_000;

/// Stable, operator-assigned tenant sign-in code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct TenantCode(String);

impl TenantCode {
    /// Parses a lower-case ASCII tenant code.
    ///
    /// # Errors
    ///
    /// Returns [`ServerIdentifierError`] when the value is not 3-32 characters
    /// or contains characters other than lower-case ASCII letters, digits, and
    /// internal hyphens.
    pub fn parse(value: impl Into<String>) -> Result<Self, ServerIdentifierError> {
        let value = value.into();
        let valid = (3..=32).contains(&value.len())
            && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            && value
                .as_bytes()
                .last()
                .is_some_and(|value| value.is_ascii_lowercase() || value.is_ascii_digit())
            && value
                .bytes()
                .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'-')
            && !value.contains("--");
        valid
            .then_some(Self(value.clone()))
            .ok_or(ServerIdentifierError { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for TenantCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerIdentifierError {
    value: String,
}

impl std::fmt::Display for ServerIdentifierError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid server identifier: {}", self.value)
    }
}

impl std::error::Error for ServerIdentifierError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AccountStatus {
    PendingActivation,
    Active,
    Disabled,
    Locked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub account_id: AccountId,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub username: String,
    pub status: AccountStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationSummary {
    pub organization_id: OrganizationId,
    pub tenant_id: TenantId,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DevicePublicKey {
    pub algorithm: String,
    pub base64: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProof {
    pub device_id: DeviceId,
    pub nonce: String,
    pub issued_at: UnixMillis,
    pub signature_base64: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActivateAccountRequest {
    pub invite_token: String,
    pub password: String,
    pub device_id: DeviceId,
    pub device_label: String,
    pub device_public_key: DevicePublicKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub tenant_code: TenantCode,
    pub username: String,
    pub password: String,
    pub device_id: DeviceId,
    pub device_label: String,
    pub device_public_key: DevicePublicKey,
    pub device_proof: Option<DeviceProof>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub refresh_token: String,
    pub device_proof: DeviceProof,
}

/// Secret-bearing token response. It intentionally has no `Debug` implementation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: UnixMillis,
    pub refresh_expires_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionPolicy {
    pub access_token_ttl_ms: i64,
    pub refresh_token_ttl_ms: i64,
    pub idle_ttl_ms: i64,
}

impl Default for SessionPolicy {
    fn default() -> Self {
        Self {
            access_token_ttl_ms: DEFAULT_ACCESS_TOKEN_TTL_MS,
            refresh_token_ttl_ms: DEFAULT_REFRESH_TOKEN_TTL_MS,
            idle_ttl_ms: DEFAULT_SESSION_IDLE_TTL_MS,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatedServerSession {
    pub session_id: SessionId,
    pub account_id: AccountId,
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub tenant_id: TenantId,
    pub issued_at: UnixMillis,
    pub expires_at: UnixMillis,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationResult {
    pub session: AuthenticatedServerSession,
    pub tokens: IssuedTokens,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateInviteRequest {
    pub username: String,
    pub organization_ids: Vec<OrganizationId>,
    pub delivery_destination: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InviteCreated {
    pub invite_id: InviteId,
    pub account_id: AccountId,
    pub expires_at: UnixMillis,
    pub delivery_id: ServerEventId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LicenseStatus {
    Active,
    Grace,
    Expired,
    Suspended,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseState {
    pub license_id: LicenseId,
    pub tenant_id: TenantId,
    pub provider_revision: String,
    pub status: LicenseStatus,
    pub valid_until: Option<UnixMillis>,
    pub grace_until: Option<UnixMillis>,
    pub entitlements: Vec<EntitlementId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveUpdateAssignment {
    pub channel: UpdateChannelId,
    pub source: UpdateAssignmentSource,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UpdateAssignmentSource {
    GlobalDefault,
    TenantDefault,
    DeviceOverride,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerFailure {
    pub code: ServerErrorCode,
    pub correlation_id: CorrelationId,
    pub retry_after_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerConnectionHello {
    pub api_version: u16,
    pub peer: PeerHello,
    pub resume_after: Option<EventCursor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerSubscriptionRequest {
    pub schema_id: SchemaId,
    pub resume_after: Option<EventCursor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerSubscriptionEvent {
    pub event_id: ServerEventId,
    pub cursor: EventCursor,
    pub occurred_at: UnixMillis,
    pub change: RecordChangeNotice,
}

tagged_contract! {
    pub enum ServerClientMessage {
        Hello(ServerConnectionHello) => "eitmad.server.hello.v1",
        Sync(SyncTransportFrame) => "eitmad.server.sync.v1",
        Subscribe(ServerSubscriptionRequest) => "eitmad.server.subscribe.v1",
        Acknowledge(ServerSubscriptionAcknowledgement) => "eitmad.server.acknowledge.v1"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerSubscriptionAcknowledgement {
    pub cursor: EventCursor,
}

tagged_contract! {
    pub enum ServerMessage {
        Hello(PeerHello) => "eitmad.server.hello-accepted.v1",
        Sync(SyncMessage) => "eitmad.server.sync-message.v1",
        Event(ServerSubscriptionEvent) => "eitmad.server.event.v1",
        Failure(ServerFailure) => "eitmad.server.failure.v1"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_codes_are_stable_and_bounded() {
        assert_eq!(
            TenantCode::parse("al-eitmad").unwrap().as_str(),
            "al-eitmad"
        );
        for invalid in ["AL-EITMAD", "aa", "-eitmad", "eitmad-", "eitmad--shop"] {
            assert!(TenantCode::parse(invalid).is_err());
        }
    }

    #[test]
    fn session_defaults_match_the_server_policy() {
        let policy = SessionPolicy::default();
        assert_eq!(policy.access_token_ttl_ms, 900_000);
        assert_eq!(policy.refresh_token_ttl_ms, 2_592_000_000);
        assert_eq!(policy.idle_ttl_ms, 1_209_600_000);
    }

    #[test]
    fn token_results_do_not_derive_debug() {
        let schema = schemars::schema_for!(AuthenticationResult);
        assert!(
            serde_json::to_string(&schema)
                .unwrap()
                .contains("accessToken")
        );
    }
}
