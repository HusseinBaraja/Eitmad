use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{errors::ContractError, identity::ScopeRef};

uuid_id!(BackgroundJobId);
open_id!(BackgroundJobKind, "background job kind");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BackgroundJobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundJobStatus {
    pub job_id: BackgroundJobId,
    pub scope: ScopeRef,
    pub job_kind: BackgroundJobKind,
    pub state: BackgroundJobState,
    pub completed_units: u64,
    pub total_units: Option<u64>,
    pub error: Option<ContractError>,
}
