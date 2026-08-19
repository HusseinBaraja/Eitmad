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

impl ContractError {
    /// Returns the allowlisted error projection safe for IPC, logs, and crash reports.
    ///
    /// Free text and compatibility reasons are always removed. Identifier and
    /// integer parameters survive only when their names declare the matching
    /// metadata kind.
    #[must_use]
    pub fn redacted_for_external_boundary(&self) -> Self {
        let parameters = self
            .parameters
            .iter()
            .filter(|parameter| parameter_is_allowlisted(parameter))
            .cloned()
            .collect();
        let detail = match &self.detail {
            Some(ErrorDetail::Compatibility { .. }) => None,
            value => value.clone(),
        };
        Self {
            code: self.code.clone(),
            message_id: self.message_id.clone(),
            parameters,
            retry: self.retry,
            correlation_id: self.correlation_id,
            detail,
        }
    }
}

fn parameter_is_allowlisted(parameter: &ErrorParameter) -> bool {
    matches!(
        (parameter.name.as_str(), &parameter.value),
        (
            "actual-revision" | "expected-revision" | "maximum-payload-bytes" | "retry-after-ms",
            ErrorParameterValue::Integer(_)
        ) | (
            "configuration-key" | "relation" | "required-capability",
            ErrorParameterValue::Identifier(_)
        )
    )
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn external_error_projection_removes_free_text_and_mismatched_metadata() {
        let error = ContractError {
            code: ErrorCode::parse("eitmad.error.synthetic.v1").unwrap(),
            message_id: MessageId::parse("eitmad.message.synthetic.v1").unwrap(),
            parameters: vec![
                ErrorParameter {
                    name: ErrorParameterName::parse("configuration-key").unwrap(),
                    value: ErrorParameterValue::Identifier(
                        "eitmad.config.locale.primary.v1".to_owned(),
                    ),
                },
                ErrorParameter {
                    name: ErrorParameterName::parse("expected-revision").unwrap(),
                    value: ErrorParameterValue::Text("secret-sentinel".to_owned()),
                },
                ErrorParameter {
                    name: ErrorParameterName::parse("raw-provider-response").unwrap(),
                    value: ErrorParameterValue::Text("secret-sentinel".to_owned()),
                },
            ],
            retry: RetryDisposition::Never,
            correlation_id: CorrelationId::new(Uuid::from_u128(1)),
            detail: Some(ErrorDetail::Compatibility {
                reason: "secret-sentinel".to_owned(),
            }),
        };

        let safe = error.redacted_for_external_boundary();
        let encoded = serde_json::to_string(&safe).unwrap();
        assert!(!encoded.contains("secret-sentinel"));
        assert_eq!(safe.parameters.len(), 1);
        assert!(safe.detail.is_none());
        assert_eq!(safe.correlation_id, error.correlation_id);
    }
}
