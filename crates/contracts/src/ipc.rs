use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    accounts::DesktopAccountRole,
    errors::ContractError,
    events::Event,
    identity::AuthorizationContext,
    transport::{
        CommandEnvelope, CommandOutcome, CommandResponseEnvelope, CorrelationId, EventEnvelope,
        QueryEnvelope, QueryOutcome, QueryResponseEnvelope, RequestId, SubscriptionClosedEnvelope,
        SubscriptionEnvelope, SubscriptionOutcome, SubscriptionResponseEnvelope, UnixMillis,
        UnsubscribeRequest, UnsubscribeResponse,
    },
    versioning::{NegotiatedSession, NegotiationRejection, PeerHello},
};

pub const MAX_IPC_FRAME_BYTES: u32 = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub peer: PeerHello,
    pub bootstrap_token: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeAccepted {
    pub engine: PeerHello,
    pub negotiated: NegotiatedSession,
    pub authorization: AuthorizationContext,
}

/// Password input is transient and must never be logged or persisted by a shell.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSignInRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for DesktopSignInRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DesktopSignInRequest")
            .field("request_id", &self.request_id)
            .field("correlation_id", &self.correlation_id)
            .field("username", &"[redacted]")
            .field("password", &"[redacted]")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSessionState {
    pub authorization: Option<AuthorizationContext>,
    pub customer_authorization: Option<AuthorizationContext>,
    pub account_role: DesktopAccountRole,
    pub expires_at: Option<UnixMillis>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSessionRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DesktopSessionStatus {
    Active,
    SignedOut,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSessionResponse {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub status: DesktopSessionStatus,
    pub state: Option<DesktopSessionState>,
    pub error: Option<ContractError>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum HandshakeRejection {
    AuthenticationRequired,
    AuthenticationFailed,
    Negotiation(NegotiationRejection),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", content = "payload", rename_all = "camelCase")]
pub enum HandshakeOutcome {
    Accepted(Box<HandshakeAccepted>),
    Rejected(HandshakeRejection),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeResponse {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub outcome: HandshakeOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownResponse {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub accepted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IpcFailureResponse {
    pub request_id: Option<RequestId>,
    pub error: ContractError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload")]
pub enum IpcClientMessage {
    #[serde(rename = "eitmad.ipc.handshake.v1")]
    Handshake(HandshakeRequest),
    #[serde(rename = "eitmad.ipc.desktop-sign-in.v1")]
    DesktopSignIn(DesktopSignInRequest),
    #[serde(rename = "eitmad.ipc.desktop-session-state.v1")]
    DesktopSessionState(DesktopSessionRequest),
    #[serde(rename = "eitmad.ipc.desktop-sign-out.v1")]
    DesktopSignOut(DesktopSessionRequest),
    #[serde(rename = "eitmad.ipc.command.v1")]
    Command(CommandEnvelope),
    #[serde(rename = "eitmad.ipc.query.v1")]
    Query(QueryEnvelope),
    #[serde(rename = "eitmad.ipc.subscribe.v1")]
    Subscribe(SubscriptionEnvelope),
    #[serde(rename = "eitmad.ipc.unsubscribe.v1")]
    Unsubscribe(UnsubscribeRequest),
    #[serde(rename = "eitmad.ipc.shutdown.v1")]
    Shutdown(ShutdownRequest),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload")]
pub enum IpcServerMessage {
    #[serde(rename = "eitmad.ipc.handshake-response.v1")]
    Handshake(HandshakeResponse),
    #[serde(rename = "eitmad.ipc.desktop-session-response.v1")]
    DesktopSession(DesktopSessionResponse),
    #[serde(rename = "eitmad.ipc.command-response.v1")]
    Command(CommandResponseEnvelope),
    #[serde(rename = "eitmad.ipc.query-response.v1")]
    Query(QueryResponseEnvelope),
    #[serde(rename = "eitmad.ipc.subscribe-response.v1")]
    Subscribe(SubscriptionResponseEnvelope),
    #[serde(rename = "eitmad.ipc.unsubscribe-response.v1")]
    Unsubscribe(UnsubscribeResponse),
    #[serde(rename = "eitmad.ipc.event.v1")]
    Event(EventEnvelope),
    #[serde(rename = "eitmad.ipc.subscription-closed.v1")]
    SubscriptionClosed(SubscriptionClosedEnvelope),
    #[serde(rename = "eitmad.ipc.shutdown-response.v1")]
    Shutdown(ShutdownResponse),
    #[serde(rename = "eitmad.ipc.failure.v1")]
    Failure(IpcFailureResponse),
}

impl IpcServerMessage {
    /// Returns a clone with every nested error sanitized for the IPC boundary.
    #[must_use]
    pub fn redacted_for_external_boundary(&self) -> Self {
        let mut message = self.clone();
        match &mut message {
            Self::DesktopSession(response) => {
                if let Some(error) = &mut response.error {
                    *error = error.redacted_for_external_boundary();
                }
            }
            Self::Command(response) => {
                if let CommandOutcome::Failed(error) = &mut response.outcome {
                    *error = error.redacted_for_external_boundary();
                }
            }
            Self::Query(response) => {
                if let QueryOutcome::Failed(error) = &mut response.outcome {
                    *error = error.redacted_for_external_boundary();
                }
            }
            Self::Subscribe(response) => {
                if let SubscriptionOutcome::Failed(error) = &mut response.outcome {
                    *error = error.redacted_for_external_boundary();
                }
            }
            Self::Event(envelope) => {
                if let Event::ErrorRaised(scoped) = &mut envelope.event {
                    scoped.error = scoped.error.redacted_for_external_boundary();
                }
            }
            Self::Failure(response) => {
                response.error = response.error.redacted_for_external_boundary();
            }
            Self::Handshake(_)
            | Self::Unsubscribe(_)
            | Self::SubscriptionClosed(_)
            | Self::Shutdown(_) => {}
        }
        message
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::{
        errors::{
            ErrorCode, ErrorParameter, ErrorParameterName, ErrorParameterValue, MessageId,
            RetryDisposition,
        },
        events::ScopedError,
        identity::{ScopeId, ScopeKind, ScopeRef},
        transport::{EventCursor, SubscriptionId, UnixMillis},
    };

    fn unsafe_error(correlation_id: CorrelationId) -> ContractError {
        ContractError {
            code: ErrorCode::parse("eitmad.error.synthetic.v1").unwrap(),
            message_id: MessageId::parse("eitmad.message.synthetic.v1").unwrap(),
            parameters: vec![ErrorParameter {
                name: ErrorParameterName::parse("unsafe-message").unwrap(),
                value: ErrorParameterValue::Text("secret-sentinel".to_owned()),
            }],
            retry: RetryDisposition::Never,
            correlation_id,
            detail: None,
        }
    }

    #[test]
    fn ipc_projection_redacts_every_nested_error_path() {
        let correlation_id = CorrelationId::new(Uuid::from_u128(1));
        let request_id = RequestId::new(Uuid::from_u128(2));
        let messages = [
            (
                "command",
                IpcServerMessage::Command(CommandResponseEnvelope {
                    request_id,
                    correlation_id,
                    outcome: CommandOutcome::Failed(unsafe_error(correlation_id)),
                }),
            ),
            (
                "query",
                IpcServerMessage::Query(QueryResponseEnvelope {
                    request_id,
                    correlation_id,
                    outcome: QueryOutcome::Failed(unsafe_error(correlation_id)),
                }),
            ),
            (
                "subscription",
                IpcServerMessage::Subscribe(SubscriptionResponseEnvelope {
                    request_id,
                    correlation_id,
                    outcome: SubscriptionOutcome::Failed(unsafe_error(correlation_id)),
                }),
            ),
            (
                "event",
                IpcServerMessage::Event(EventEnvelope {
                    subscription_id: SubscriptionId::new(Uuid::from_u128(3)),
                    correlation_id,
                    sequence: 1,
                    cursor: EventCursor::new(Uuid::from_u128(4)),
                    occurred_at: UnixMillis(5),
                    event: Event::ErrorRaised(ScopedError {
                        scope: ScopeRef {
                            kind: ScopeKind::parse("organization").unwrap(),
                            id: ScopeId::new(Uuid::from_u128(6)),
                        },
                        error: unsafe_error(correlation_id),
                    }),
                }),
            ),
            (
                "failure",
                IpcServerMessage::Failure(IpcFailureResponse {
                    request_id: Some(request_id),
                    error: unsafe_error(correlation_id),
                }),
            ),
        ];

        for (name, message) in messages {
            let encoded = serde_json::to_string(&message.redacted_for_external_boundary()).unwrap();
            assert!(!encoded.contains("secret-sentinel"), "{name} leaked data");
            assert!(
                encoded.contains(&correlation_id.value().to_string()),
                "{name} lost correlation"
            );
        }
    }
}
