use std::collections::BTreeMap;

use eitmad_contracts::{
    observability::{
        ComponentId, DataClassification, ObservationEventId, ObservationFieldName,
        ObservationSeverity, ObservationValueKind,
    },
    transport::{CorrelationId, UnixMillis},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationFieldContract {
    pub name: ObservationFieldName,
    pub classification: DataClassification,
    pub value_kind: ObservationValueKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationContract {
    event_id: ObservationEventId,
    fields: BTreeMap<ObservationFieldName, ObservationFieldContract>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationContractError {
    DuplicateField,
    UnknownField,
    WrongValueKind,
}

impl ObservationContract {
    /// Creates an allowlisted diagnostic event contract.
    ///
    /// # Errors
    ///
    /// Returns an error when a field name is declared more than once.
    pub fn new(
        event_id: ObservationEventId,
        fields: impl IntoIterator<Item = ObservationFieldContract>,
    ) -> Result<Self, ObservationContractError> {
        let mut by_name = BTreeMap::new();
        for field in fields {
            if by_name.insert(field.name.clone(), field).is_some() {
                return Err(ObservationContractError::DuplicateField);
            }
        }
        Ok(Self {
            event_id,
            fields: by_name,
        })
    }

    /// Applies the contract before an event reaches a log or crash-report sink.
    ///
    /// # Errors
    ///
    /// Returns an error for undeclared fields or values of the wrong kind.
    pub fn redact(
        &self,
        occurred_at: UnixMillis,
        component: ComponentId,
        severity: ObservationSeverity,
        correlation_id: CorrelationId,
        values: impl IntoIterator<Item = (ObservationFieldName, ObservationValue)>,
    ) -> Result<StructuredLog, ObservationContractError> {
        let mut output = BTreeMap::new();
        for (name, value) in values {
            let Some(contract) = self.fields.get(&name) else {
                return Err(ObservationContractError::UnknownField);
            };
            if contract.value_kind != value.kind() {
                return Err(ObservationContractError::WrongValueKind);
            }
            let value = match contract.classification {
                DataClassification::Metadata => StructuredValue::Value(value),
                DataClassification::Sensitive | DataClassification::Secret => {
                    StructuredValue::Redacted
                }
            };
            if output.insert(name, value).is_some() {
                return Err(ObservationContractError::DuplicateField);
            }
        }
        Ok(StructuredLog {
            occurred_at,
            event_id: self.event_id.clone(),
            component,
            severity,
            correlation_id,
            fields: output,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ObservationValue {
    Boolean(bool),
    Integer(i64),
    Identifier(String),
    Text(String),
}

impl ObservationValue {
    const fn kind(&self) -> ObservationValueKind {
        match self {
            Self::Boolean(_) => ObservationValueKind::Boolean,
            Self::Integer(_) => ObservationValueKind::Integer,
            Self::Identifier(_) => ObservationValueKind::Identifier,
            Self::Text(_) => ObservationValueKind::Text,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "disposition", content = "value", rename_all = "camelCase")]
pub enum StructuredValue {
    Value(ObservationValue),
    Redacted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredLog {
    occurred_at: UnixMillis,
    event_id: ObservationEventId,
    component: ComponentId,
    severity: ObservationSeverity,
    correlation_id: CorrelationId,
    fields: BTreeMap<ObservationFieldName, StructuredValue>,
}

impl StructuredLog {
    #[must_use]
    pub const fn correlation_id(&self) -> CorrelationId {
        self.correlation_id
    }

    #[must_use]
    pub const fn fields(&self) -> &BTreeMap<ObservationFieldName, StructuredValue> {
        &self.fields
    }
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::observability::{DataClassification, ObservationValueKind};
    use uuid::Uuid;

    use super::*;

    fn field(
        name: &str,
        classification: DataClassification,
        value_kind: ObservationValueKind,
    ) -> ObservationFieldContract {
        ObservationFieldContract {
            name: ObservationFieldName::parse(name).unwrap(),
            classification,
            value_kind,
        }
    }

    fn contract() -> ObservationContract {
        ObservationContract::new(
            ObservationEventId::parse("eitmad.observation.synthetic.v1").unwrap(),
            [
                field(
                    "operation",
                    DataClassification::Metadata,
                    ObservationValueKind::Identifier,
                ),
                field(
                    "customer-label",
                    DataClassification::Sensitive,
                    ObservationValueKind::Text,
                ),
                field(
                    "access-token",
                    DataClassification::Secret,
                    ObservationValueKind::Text,
                ),
            ],
        )
        .unwrap()
    }

    #[test]
    fn metadata_only_logging_redacts_sensitive_and_secret_fields() {
        let log = contract()
            .redact(
                UnixMillis(1),
                ComponentId::parse("engine-runtime").unwrap(),
                ObservationSeverity::Info,
                CorrelationId::new(Uuid::from_u128(5)),
                [
                    (
                        ObservationFieldName::parse("operation").unwrap(),
                        ObservationValue::Identifier("configuration-read".to_owned()),
                    ),
                    (
                        ObservationFieldName::parse("customer-label").unwrap(),
                        ObservationValue::Text("عميل تجريبي".to_owned()),
                    ),
                    (
                        ObservationFieldName::parse("access-token").unwrap(),
                        ObservationValue::Text("never-log-this-token".to_owned()),
                    ),
                ],
            )
            .unwrap();
        let encoded = serde_json::to_string(&log).unwrap();

        assert!(encoded.contains("configuration-read"));
        assert!(!encoded.contains("عميل تجريبي"));
        assert!(!encoded.contains("never-log-this-token"));
        assert_eq!(
            log.fields()[&ObservationFieldName::parse("access-token").unwrap()],
            StructuredValue::Redacted
        );
    }

    #[test]
    fn contract_rejects_duplicate_unknown_and_wrong_kind_fields() {
        let duplicate_declaration = ObservationContract::new(
            ObservationEventId::parse("eitmad.observation.duplicate.v1").unwrap(),
            [
                field(
                    "operation",
                    DataClassification::Metadata,
                    ObservationValueKind::Identifier,
                ),
                field(
                    "operation",
                    DataClassification::Metadata,
                    ObservationValueKind::Identifier,
                ),
            ],
        );
        assert_eq!(
            duplicate_declaration,
            Err(ObservationContractError::DuplicateField)
        );

        let duplicate_input = contract().redact(
            UnixMillis(1),
            ComponentId::parse("engine-runtime").unwrap(),
            ObservationSeverity::Info,
            CorrelationId::new(Uuid::from_u128(5)),
            [
                (
                    ObservationFieldName::parse("operation").unwrap(),
                    ObservationValue::Identifier("configuration-read".to_owned()),
                ),
                (
                    ObservationFieldName::parse("operation").unwrap(),
                    ObservationValue::Identifier("configuration-write".to_owned()),
                ),
            ],
        );
        assert_eq!(
            duplicate_input,
            Err(ObservationContractError::DuplicateField)
        );

        let unknown = contract().redact(
            UnixMillis(1),
            ComponentId::parse("engine-runtime").unwrap(),
            ObservationSeverity::Info,
            CorrelationId::new(Uuid::from_u128(5)),
            [(
                ObservationFieldName::parse("undeclared").unwrap(),
                ObservationValue::Text("value".to_owned()),
            )],
        );
        assert_eq!(unknown, Err(ObservationContractError::UnknownField));

        let wrong_kind = contract().redact(
            UnixMillis(1),
            ComponentId::parse("engine-runtime").unwrap(),
            ObservationSeverity::Info,
            CorrelationId::new(Uuid::from_u128(5)),
            [(
                ObservationFieldName::parse("operation").unwrap(),
                ObservationValue::Text("value".to_owned()),
            )],
        );
        assert_eq!(wrong_kind, Err(ObservationContractError::WrongValueKind));
    }
}
