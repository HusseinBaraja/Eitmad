use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    errors::ContractError,
    identity::{AuthenticatedIdentity, AuthorizationContext, ScopeRef},
    transport::{
        CommandEnvelope, CommandResponseEnvelope, CorrelationId, QueryEnvelope,
        QueryResponseEnvelope, RequestId,
    },
    versioning::{NegotiatedSession, NegotiationRejection, PeerHello},
};

pub const MAX_IPC_FRAME_BYTES: u32 = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentIdentityAssertion {
    pub identity: AuthenticatedIdentity,
    pub scope: ScopeRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub peer: PeerHello,
    pub development_bearer_token: String,
    pub asserted_authorization: DevelopmentIdentityAssertion,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeAccepted {
    pub engine: PeerHello,
    pub negotiated: NegotiatedSession,
    pub authorization: AuthorizationContext,
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
    #[serde(rename = "eitmad.ipc.command.v1")]
    Command(CommandEnvelope),
    #[serde(rename = "eitmad.ipc.query.v1")]
    Query(QueryEnvelope),
    #[serde(rename = "eitmad.ipc.shutdown.v1")]
    Shutdown(ShutdownRequest),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload")]
pub enum IpcServerMessage {
    #[serde(rename = "eitmad.ipc.handshake-response.v1")]
    Handshake(HandshakeResponse),
    #[serde(rename = "eitmad.ipc.command-response.v1")]
    Command(CommandResponseEnvelope),
    #[serde(rename = "eitmad.ipc.query-response.v1")]
    Query(QueryResponseEnvelope),
    #[serde(rename = "eitmad.ipc.shutdown-response.v1")]
    Shutdown(ShutdownResponse),
    #[serde(rename = "eitmad.ipc.failure.v1")]
    Failure(IpcFailureResponse),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_limit_is_eight_mebibytes() {
        assert_eq!(MAX_IPC_FRAME_BYTES, 8_388_608);
    }
}
