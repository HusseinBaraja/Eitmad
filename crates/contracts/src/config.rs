use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::identity::ScopeRef;

open_id!(ConfigKey, "configuration key");
uuid_id!(SecretReferenceId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ConfigWriteValue {
    Boolean(bool),
    Integer(i64),
    Decimal(String),
    Text(String),
    TextList(Vec<String>),
    SecretReference(SecretReferenceId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ConfigReadValue {
    Boolean(bool),
    Integer(i64),
    Decimal(String),
    Text(String),
    TextList(Vec<String>),
    SecretReference(SecretReferenceId),
    Redacted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConfigSensitivity {
    Public,
    Sensitive,
    Secret,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RestartRequirement {
    None,
    Engine,
    Application,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigEntry {
    pub key: ConfigKey,
    pub value: ConfigReadValue,
    pub sensitivity: ConfigSensitivity,
    pub restart_requirement: RestartRequirement,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSnapshot {
    pub schema_version: u32,
    pub revision: u64,
    pub scope: ScopeRef,
    pub entries: Vec<ConfigEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChange {
    pub key: ConfigKey,
    pub value: ConfigWriteValue,
}
