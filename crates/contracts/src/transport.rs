use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    commands::{Command, CommandResult},
    errors::ContractError,
    events::{Event, Subscription},
    identity::AuthorizationContext,
    queries::{Query, QueryResult},
    versioning::ProtocolVersion,
};

pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion { major: 1, minor: 2 };
pub const MAX_PAGE_SIZE: u32 = 500;

uuid_id!(RequestId);
uuid_id!(CorrelationId);
uuid_id!(CausationId);
uuid_id!(IdempotencyKey);
uuid_id!(SubscriptionId);
uuid_id!(OperationId);
uuid_id!(UpdateHandoffId);

uuid_id!(EventCursor);
open_id!(CapabilityId, "capability identifier");
open_id!(SchemaId, "schema identifier");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentifierError {
    kind: &'static str,
    value: String,
}

impl IdentifierError {
    #[must_use]
    pub fn kind(&self) -> &'static str {
        self.kind
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Display for IdentifierError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid {}: {}", self.kind, self.value)
    }
}

impl std::error::Error for IdentifierError {}

pub(crate) fn validate_open_identifier(
    value: &str,
    kind: &'static str,
) -> Result<(), IdentifierError> {
    let valid_length = (3..=128).contains(&value.len());
    let valid_edges = value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .as_bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    let valid_characters = value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
    });
    let valid_segments = !value.contains("..") && !value.contains("__") && !value.contains("--");

    if valid_length && valid_edges && valid_characters && valid_segments {
        Ok(())
    } else {
        Err(IdentifierError {
            kind,
            value: value.to_owned(),
        })
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct UnixMillis(pub i64);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageRequest {
    pub cursor: Option<EventCursor>,
    #[schemars(range(min = 1, max = 500))]
    limit: u32,
}

impl<'de> Deserialize<'de> for PageRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawPageRequest {
            cursor: Option<EventCursor>,
            limit: u32,
        }

        let raw = RawPageRequest::deserialize(deserializer)?;
        Self::new(raw.cursor, raw.limit).map_err(serde::de::Error::custom)
    }
}

impl PageRequest {
    /// Creates a bounded page request.
    ///
    /// # Errors
    ///
    /// Returns [`PageSizeError`] when `limit` is zero or exceeds
    /// [`MAX_PAGE_SIZE`].
    pub fn new(cursor: Option<EventCursor>, limit: u32) -> Result<Self, PageSizeError> {
        if (1..=MAX_PAGE_SIZE).contains(&limit) {
            Ok(Self { cursor, limit })
        } else {
            Err(PageSizeError { limit })
        }
    }

    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageSizeError {
    pub limit: u32,
}

impl std::fmt::Display for PageSizeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "page size must be between 1 and {MAX_PAGE_SIZE}")
    }
}

impl std::error::Error for PageSizeError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandEnvelope {
    pub protocol_version: ProtocolVersion,
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub authorization: AuthorizationContext,
    pub deadline: UnixMillis,
    pub idempotency_key: IdempotencyKey,
    pub command: Command,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueryEnvelope {
    pub protocol_version: ProtocolVersion,
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub authorization: AuthorizationContext,
    pub deadline: UnixMillis,
    pub query: Query,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionEnvelope {
    pub protocol_version: ProtocolVersion,
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub authorization: AuthorizationContext,
    pub subscription: Subscription,
    pub resume_after: Option<EventCursor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", content = "payload", rename_all = "camelCase")]
pub enum CommandOutcome {
    Succeeded(CommandResult),
    Failed(ContractError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", content = "payload", rename_all = "camelCase")]
pub enum QueryOutcome {
    Succeeded(QueryResult),
    Failed(ContractError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponseEnvelope {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub outcome: CommandOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponseEnvelope {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub outcome: QueryOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub subscription_id: SubscriptionId,
    pub correlation_id: CorrelationId,
    pub sequence: u64,
    pub cursor: EventCursor,
    pub occurred_at: UnixMillis,
    pub event: Event,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionAccepted {
    pub subscription_id: SubscriptionId,
    pub stream_cursor: EventCursor,
    pub resumed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", content = "payload", rename_all = "camelCase")]
pub enum SubscriptionOutcome {
    Succeeded(SubscriptionAccepted),
    Failed(ContractError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionResponseEnvelope {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub outcome: SubscriptionOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub subscription_id: SubscriptionId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeResponse {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub subscription_id: SubscriptionId,
    pub accepted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SubscriptionCloseReason {
    ClientRequested,
    Backpressure,
    EngineStopping,
    AuthorizationRevoked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionClosedEnvelope {
    pub subscription_id: SubscriptionId,
    pub correlation_id: CorrelationId,
    pub last_delivered_cursor: Option<EventCursor>,
    pub reason: SubscriptionCloseReason,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ConfigReadValue,
        queries::{GetConfiguration, Query},
    };

    #[test]
    fn open_identifier_rejects_protocol_typos() {
        assert!(CapabilityId::parse("eitmad.sync.v1").is_ok());
        assert!(CapabilityId::parse("Eitmad Sync").is_err());
        assert!(CapabilityId::parse("eitmad..sync").is_err());
    }

    #[test]
    fn page_size_is_bounded() {
        assert_eq!(PageRequest::new(None, 1).unwrap().limit(), 1);
        assert_eq!(
            PageRequest::new(None, MAX_PAGE_SIZE).unwrap().limit(),
            MAX_PAGE_SIZE
        );
        assert!(PageRequest::new(None, 0).is_err());
        assert!(PageRequest::new(None, MAX_PAGE_SIZE + 1).is_err());
    }

    #[test]
    fn page_size_is_bounded_during_deserialization() {
        let valid = serde_json::from_str::<PageRequest>(r#"{"cursor":null,"limit":500}"#)
            .expect("maximum page size should deserialize");
        assert_eq!(valid.limit(), MAX_PAGE_SIZE);

        assert!(serde_json::from_str::<PageRequest>(r#"{"cursor":null,"limit":0}"#).is_err());
        assert!(serde_json::from_str::<PageRequest>(r#"{"cursor":null,"limit":501}"#).is_err());
    }

    #[test]
    fn serde_rejects_invalid_open_identifiers() {
        let result = serde_json::from_str::<CapabilityId>(r#""Eitmad Sync""#);
        assert!(result.is_err());
    }

    #[test]
    fn additive_fields_are_ignored_but_unknown_operations_fail() {
        let known = r#"{
            "kind":"eitmad.config.get.v1",
            "payload":{},
            "futureField":"safe to ignore"
        }"#;
        assert_eq!(
            serde_json::from_str::<Query>(known).unwrap(),
            Query::Configuration(GetConfiguration {})
        );

        let unknown = r#"{"kind":"eitmad.unknown.query.v1","payload":{}}"#;
        assert!(serde_json::from_str::<Query>(unknown).is_err());
    }

    #[test]
    fn arabic_and_mixed_direction_text_round_trips_without_bidi_controls() {
        let value = ConfigReadValue::Text("خزانة Wardrobe 120 cm - فرع صنعاء".to_owned());
        let encoded = serde_json::to_string(&value).unwrap();
        let decoded = serde_json::from_str::<ConfigReadValue>(&encoded).unwrap();

        assert_eq!(decoded, value);
        assert!(!encoded.contains(['\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}']));
    }
}
