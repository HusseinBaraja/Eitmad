use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

uuid_id!(PrincipalId);
uuid_id!(DeviceId);
uuid_id!(UserId);
uuid_id!(AccountId);
uuid_id!(ServiceId);
uuid_id!(SessionId);
uuid_id!(WorkspaceId);
uuid_id!(OrganizationId);
uuid_id!(ScopeId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct TenantId(uuid::Uuid);

impl TenantId {
    #[must_use]
    pub const fn new(value: uuid::Uuid) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> uuid::Uuid {
        self.0
    }
}

impl From<uuid::Uuid> for TenantId {
    fn from(value: uuid::Uuid) -> Self {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for TenantId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = uuid::Uuid::deserialize(deserializer)?;
        (!value.is_nil())
            .then(|| Self::new(value))
            .ok_or_else(|| serde::de::Error::custom("tenant ID must be assigned"))
    }
}

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
    pub tenant_id: TenantId,
    pub workspace_id: Option<WorkspaceId>,
    pub scope: ScopeRef,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(uuid::Uuid::from_u128(1)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(uuid::Uuid::from_u128(2)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(uuid::Uuid::from_u128(3)),
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(uuid::Uuid::from_u128(4)),
            },
        }
    }

    #[test]
    fn tenant_id_deserialization_requires_an_assigned_value() {
        let valid = serde_json::to_value(context()).unwrap();
        assert!(serde_json::from_value::<AuthorizationContext>(valid.clone()).is_ok());

        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove("tenantId");
        assert!(serde_json::from_value::<AuthorizationContext>(missing).is_err());

        let mut unspecified = valid;
        unspecified["tenantId"] = serde_json::json!(uuid::Uuid::nil());
        assert!(serde_json::from_value::<AuthorizationContext>(unspecified).is_err());
    }
}
