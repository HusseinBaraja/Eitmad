use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

uuid_id!(PrincipalId);
uuid_id!(DeviceId);
uuid_id!(ServiceId);
uuid_id!(SessionId);
uuid_id!(ScopeId);

open_id!(ScopeKind, "scope kind");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PrincipalKind {
    User,
    Device,
    Service,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatedIdentity {
    pub principal_id: PrincipalId,
    pub principal_kind: PrincipalKind,
    pub device_id: Option<DeviceId>,
    pub service_id: Option<ServiceId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScopeRef {
    pub kind: ScopeKind,
    pub id: ScopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationContext {
    pub session_id: SessionId,
    pub identity: AuthenticatedIdentity,
    pub scope: ScopeRef,
}
