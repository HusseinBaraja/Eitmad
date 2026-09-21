//! Typed desktop-account administration contracts.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::identity::{AccountId, UserId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DesktopAccountRole {
    Manager,
    Receptionist,
}

/// A secret-bearing password value. Debug output is always redacted.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AccountPassword(String);

impl AccountPassword {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for AccountPassword {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AccountPassword([REDACTED])")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAccountSummary {
    pub account_id: AccountId,
    pub user_id: UserId,
    pub display_name: String,
    pub username: String,
    pub role: DesktopAccountRole,
    pub active: bool,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAccountPage {
    pub accounts: Vec<DesktopAccountSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesktopAccount {
    pub display_name: String,
    pub username: String,
    pub password: AccountPassword,
    pub role: DesktopAccountRole,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDesktopAccount {
    pub account_id: AccountId,
    pub expected_revision: u64,
    pub display_name: String,
    pub role: DesktopAccountRole,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeactivateDesktopAccount {
    pub account_id: AccountId,
    pub expected_revision: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListDesktopAccounts {}
