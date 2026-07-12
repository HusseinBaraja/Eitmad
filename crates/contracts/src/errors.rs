use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::runtime::LifecycleStage;
use crate::transport::CorrelationId;

open_id!(ErrorCode, "error code");
open_id!(MessageId, "message identifier");
open_id!(ErrorParameterName, "error parameter name");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ErrorParameterValue {
    Text(String),
    Integer(i64),
    Identifier(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorParameter {
    pub name: ErrorParameterName,
    pub value: ErrorParameterValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "retryAfterMs", rename_all = "camelCase")]
pub enum RetryDisposition {
    Never,
    SafeImmediately,
    SafeAfterDelay(u64),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum ErrorDetail {
    Validation {
        fields: Vec<ErrorParameterName>,
    },
    RevisionConflict {
        expected: u64,
        actual: u64,
    },
    Compatibility {
        reason: String,
    },
    Lifecycle {
        stage: LifecycleStage,
    },
    Deadline {
        deadline: crate::transport::UnixMillis,
    },
    PayloadLimit {
        maximum_bytes: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContractError {
    pub code: ErrorCode,
    pub message_id: MessageId,
    pub parameters: Vec<ErrorParameter>,
    pub retry: RetryDisposition,
    pub correlation_id: CorrelationId,
    pub detail: Option<ErrorDetail>,
}
