use std::{collections::BTreeMap, time::Duration};

use eitmad_contracts::{
    errors::ErrorCode,
    identity::AuthorizationContext,
    observability::{
        ComponentId, DataClassification, ObservationEventId, ObservationFieldName,
        ObservationSeverity, ObservationValueKind,
    },
    transport::{CorrelationId, UnixMillis},
};
use serde::{Deserialize, Serialize};

use crate::{AuditExtensionPoint, AuditTarget, MutationAuditRecord};

pub const MAX_SENSITIVE_DEBUG_DURATION: Duration = Duration::from_secs(30 * 60);
pub const SENSITIVE_DEBUG_WARNING: &str = "Sensitive diagnostic fields are temporarily enabled. Secrets remain redacted. Output requires restricted handling.";

const DEBUG_ENABLE_OPERATION: &str = "eitmad.observability.sensitive-debug.enable.v1";
const DEBUG_EXPIRE_OPERATION: &str = "eitmad.observability.sensitive-debug.expire.v1";

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
        context: RedactionContext,
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
                DataClassification::Sensitive if context.sensitive_allowed => {
                    StructuredValue::Value(value)
                }
                DataClassification::Sensitive | DataClassification::Secret => {
                    StructuredValue::Redacted
                }
            };
            output.insert(name, value);
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
    pub occurred_at: UnixMillis,
    pub event_id: ObservationEventId,
    pub component: ComponentId,
    pub severity: ObservationSeverity,
    pub correlation_id: CorrelationId,
    pub fields: BTreeMap<ObservationFieldName, StructuredValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StructuredErrorClass {
    Validation,
    Authorization,
    Dependency,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredError {
    pub code: ErrorCode,
    pub class: StructuredErrorClass,
    pub correlation_id: CorrelationId,
    pub metadata: BTreeMap<ObservationFieldName, StructuredValue>,
}

impl StructuredError {
    #[must_use]
    pub fn new(
        code: ErrorCode,
        class: StructuredErrorClass,
        correlation_id: CorrelationId,
    ) -> Self {
        Self {
            code,
            class,
            correlation_id,
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub occurred_at: UnixMillis,
    pub component: ComponentId,
    pub correlation_id: CorrelationId,
    pub error: StructuredError,
    pub recent_metadata: Vec<StructuredLog>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RedactionContext {
    sensitive_allowed: bool,
}

impl RedactionContext {
    #[must_use]
    pub const fn metadata_only() -> Self {
        Self {
            sensitive_allowed: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensitiveDebugError {
    InvalidDuration,
    AlreadyActive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SensitiveDebugStatus {
    Disabled,
    Active {
        expires_at: UnixMillis,
        warning: &'static str,
    },
    Expired,
}

#[derive(Clone, Debug)]
struct SensitiveDebugActivation {
    authorization: AuthorizationContext,
    correlation_id: CorrelationId,
    expires_at: UnixMillis,
}

#[derive(Clone, Debug, Default)]
pub struct SensitiveDebugController {
    activation: Option<SensitiveDebugActivation>,
}

#[derive(Clone, Debug)]
pub struct SensitiveDebugEvaluation {
    pub status: SensitiveDebugStatus,
    pub redaction: RedactionContext,
    pub expiry_audit: Option<MutationAuditRecord>,
}

impl SensitiveDebugController {
    /// Enables bounded sensitive diagnostics and returns the mandatory audit record.
    /// Secret-classified fields remain redacted.
    ///
    /// # Errors
    ///
    /// Rejects zero, over-limit, overflowing, or overlapping activations.
    pub fn enable(
        &mut self,
        authorization: &AuthorizationContext,
        correlation_id: CorrelationId,
        now: UnixMillis,
        duration: Duration,
    ) -> Result<MutationAuditRecord, SensitiveDebugError> {
        if self.activation.is_some() {
            return Err(SensitiveDebugError::AlreadyActive);
        }
        let duration_ms = i64::try_from(duration.as_millis())
            .ok()
            .filter(|value| *value > 0)
            .filter(|_| duration <= MAX_SENSITIVE_DEBUG_DURATION)
            .ok_or(SensitiveDebugError::InvalidDuration)?;
        let expires_at = UnixMillis(
            now.0
                .checked_add(duration_ms)
                .ok_or(SensitiveDebugError::InvalidDuration)?,
        );
        self.activation = Some(SensitiveDebugActivation {
            authorization: authorization.clone(),
            correlation_id,
            expires_at,
        });
        Ok(debug_audit(
            authorization,
            correlation_id,
            now,
            DEBUG_ENABLE_OPERATION,
            expires_at,
        ))
    }

    #[must_use]
    pub fn evaluate(&mut self, now: UnixMillis) -> SensitiveDebugEvaluation {
        let Some(activation) = self.activation.as_ref() else {
            return SensitiveDebugEvaluation {
                status: SensitiveDebugStatus::Disabled,
                redaction: RedactionContext::metadata_only(),
                expiry_audit: None,
            };
        };
        if now.0 < activation.expires_at.0 {
            return SensitiveDebugEvaluation {
                status: SensitiveDebugStatus::Active {
                    expires_at: activation.expires_at,
                    warning: SENSITIVE_DEBUG_WARNING,
                },
                redaction: RedactionContext {
                    sensitive_allowed: true,
                },
                expiry_audit: None,
            };
        }
        let activation = self.activation.take().expect("activation was present");
        SensitiveDebugEvaluation {
            status: SensitiveDebugStatus::Expired,
            redaction: RedactionContext::metadata_only(),
            expiry_audit: Some(debug_audit(
                &activation.authorization,
                activation.correlation_id,
                now,
                DEBUG_EXPIRE_OPERATION,
                activation.expires_at,
            )),
        }
    }
}

fn debug_audit(
    authorization: &AuthorizationContext,
    correlation_id: CorrelationId,
    occurred_at: UnixMillis,
    operation: &str,
    expires_at: UnixMillis,
) -> MutationAuditRecord {
    let mut record = MutationAuditRecord::from_authorization(
        authorization,
        occurred_at,
        correlation_id,
        operation,
        AuditTarget {
            kind: "sensitive-debug-mode".to_owned(),
            identifiers: vec![format!("expires-at-ms:{}", expires_at.0)],
        },
    );
    record.extension_points = vec![
        AuditExtensionPoint::SecurityEvent,
        AuditExtensionPoint::SensitiveDebugMode,
    ];
    record
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use eitmad_contracts::{
        identity::{
            AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, ScopeRef,
            SessionId, TenantId,
        },
        observability::{DataClassification, ObservationValueKind},
    };
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

    fn authorization() -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(1)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(2)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(Uuid::from_u128(3)),
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(Uuid::from_u128(4)),
            },
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
                RedactionContext::metadata_only(),
            )
            .unwrap();
        let encoded = serde_json::to_string(&log).unwrap();

        assert!(encoded.contains("configuration-read"));
        assert!(!encoded.contains("عميل تجريبي"));
        assert!(!encoded.contains("never-log-this-token"));
        assert_eq!(
            log.fields[&ObservationFieldName::parse("access-token").unwrap()],
            StructuredValue::Redacted
        );
    }

    #[test]
    fn contract_rejects_unknown_fields_and_wrong_kinds() {
        let unknown = contract().redact(
            UnixMillis(1),
            ComponentId::parse("engine-runtime").unwrap(),
            ObservationSeverity::Info,
            CorrelationId::new(Uuid::from_u128(5)),
            [(
                ObservationFieldName::parse("undeclared").unwrap(),
                ObservationValue::Text("value".to_owned()),
            )],
            RedactionContext::metadata_only(),
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
            RedactionContext::metadata_only(),
        );
        assert_eq!(wrong_kind, Err(ObservationContractError::WrongValueKind));
    }

    #[test]
    fn sensitive_debug_expires_and_never_reveals_secrets() {
        let mut controller = SensitiveDebugController::default();
        let correlation = CorrelationId::new(Uuid::from_u128(5));
        let enabled = controller
            .enable(
                &authorization(),
                correlation,
                UnixMillis(1_000),
                Duration::from_secs(60),
            )
            .unwrap();
        assert!(enabled.validate_complete().is_ok());
        assert!(
            serde_json::to_string(&enabled)
                .unwrap()
                .contains("expires-at-ms:61000")
        );

        let active = controller.evaluate(UnixMillis(60_999));
        assert!(matches!(active.status, SensitiveDebugStatus::Active { .. }));
        let active_log = contract()
            .redact(
                UnixMillis(60_999),
                ComponentId::parse("engine-runtime").unwrap(),
                ObservationSeverity::Warning,
                correlation,
                [
                    (
                        ObservationFieldName::parse("customer-label").unwrap(),
                        ObservationValue::Text("عميل تجريبي".to_owned()),
                    ),
                    (
                        ObservationFieldName::parse("access-token").unwrap(),
                        ObservationValue::Text("never-log-this-token".to_owned()),
                    ),
                ],
                active.redaction,
            )
            .unwrap();
        let active_json = serde_json::to_string(&active_log).unwrap();
        assert!(active_json.contains("عميل تجريبي"));
        assert!(!active_json.contains("never-log-this-token"));

        let expired = controller.evaluate(UnixMillis(61_000));
        assert_eq!(expired.status, SensitiveDebugStatus::Expired);
        assert!(expired.expiry_audit.unwrap().validate_complete().is_ok());
        assert_eq!(
            controller.evaluate(UnixMillis(61_001)).status,
            SensitiveDebugStatus::Disabled
        );
    }

    #[test]
    fn invalid_sensitive_debug_durations_are_rejected() {
        let mut controller = SensitiveDebugController::default();
        let context = authorization();
        let correlation = CorrelationId::new(Uuid::from_u128(5));
        assert_eq!(
            controller.enable(&context, correlation, UnixMillis(0), Duration::ZERO),
            Err(SensitiveDebugError::InvalidDuration)
        );
        assert_eq!(
            controller.enable(
                &context,
                correlation,
                UnixMillis(0),
                MAX_SENSITIVE_DEBUG_DURATION + Duration::from_millis(1),
            ),
            Err(SensitiveDebugError::InvalidDuration)
        );
    }
}
