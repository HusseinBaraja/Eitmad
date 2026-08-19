use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

open_id!(ObservationEventId, "observation event identifier");
open_id!(ObservationFieldName, "observation field name");
open_id!(ComponentId, "component identifier");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ObservationSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DataClassification {
    Metadata,
    Sensitive,
    Secret,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ObservationValueKind {
    Boolean,
    Integer,
    Identifier,
    Text,
}
