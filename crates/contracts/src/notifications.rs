use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    errors::{ErrorParameter, MessageId},
    identity::ScopeRef,
    transport::CorrelationId,
};

uuid_id!(NotificationId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NotificationSeverity {
    Information,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub notification_id: NotificationId,
    pub scope: ScopeRef,
    pub severity: NotificationSeverity,
    pub message_id: MessageId,
    pub parameters: Vec<ErrorParameter>,
    pub correlation_id: Option<CorrelationId>,
}
