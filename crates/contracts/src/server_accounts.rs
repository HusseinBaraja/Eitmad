//! Version-one server account administration. Owner is a relationship, never an assignable role.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    accounts::DesktopAccountRole,
    identity::{AccountId, OrganizationId, ScopeId, TenantId, UserId},
    server::InviteId,
    transport::{IdempotencyKey, UnixMillis},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerRoleAssignment {
    pub role: DesktopAccountRole,
    pub branch_ids: Vec<ScopeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ServerAccountStatus {
    PendingActivation,
    Active,
    Disabled,
    Locked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerAccountOrganization {
    pub organization_id: OrganizationId,
    pub owner: bool,
    pub assignments: Vec<ServerRoleAssignment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerAccount {
    pub tenant_id: TenantId,
    pub account_id: AccountId,
    pub user_id: UserId,
    pub username: String,
    pub status: ServerAccountStatus,
    pub revision: u64,
    pub organizations: Vec<ServerAccountOrganization>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListServerAccounts {
    pub after: Option<AccountId>,
    pub limit: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerAccountPage {
    pub accounts: Vec<ServerAccount>,
    pub next_after: Option<AccountId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum ServerAccountChange {
    #[serde(rename_all = "camelCase")]
    Invite {
        organization_id: OrganizationId,
        username: String,
        assignments: Vec<ServerRoleAssignment>,
    },
    #[serde(rename_all = "camelCase")]
    Assign {
        account_id: AccountId,
        expected_revision: u64,
        organization_id: OrganizationId,
        assignments: Vec<ServerRoleAssignment>,
    },
    #[serde(rename_all = "camelCase")]
    Deactivate {
        account_id: AccountId,
        expected_revision: u64,
    },
    #[serde(rename_all = "camelCase")]
    RevokeSessions {
        account_id: AccountId,
        expected_revision: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeServerAccount {
    pub idempotency_key: IdempotencyKey,
    pub change: ServerAccountChange,
}

/// Receipt metadata is safe to persist. The activation credential is returned separately.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerAccountReceipt {
    pub account: ServerAccount,
    pub invitation_id: Option<InviteId>,
    pub invitation_expires_at: Option<UnixMillis>,
}

/// Secret-bearing response. Do not log or include it in diagnostics.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerAccountResult {
    pub receipt: ServerAccountReceipt,
    pub activation_token: Option<String>,
}
