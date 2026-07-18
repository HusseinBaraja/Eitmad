use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    authorization::{RelationshipId, RelationshipPage},
    config::ConfigSnapshot,
    permissions::EffectivePermissions,
    sync::SyncStatus,
    updates::UpdateState,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetConfiguration {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetEffectivePermissions {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetUpdateState {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetSyncStatus {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListScopeRelationships {
    pub after: Option<RelationshipId>,
    #[schemars(range(min = 1, max = 500))]
    limit: u32,
}

impl ListScopeRelationships {
    /// Creates a bounded relationship page query.
    ///
    /// # Errors
    ///
    /// Returns [`crate::transport::PageSizeError`] for a zero or oversized page.
    pub fn new(
        after: Option<RelationshipId>,
        limit: u32,
    ) -> Result<Self, crate::transport::PageSizeError> {
        crate::transport::PageRequest::new(None, limit)?;
        Ok(Self { after, limit })
    }

    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }
}

impl<'de> Deserialize<'de> for ListScopeRelationships {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawQuery {
            after: Option<RelationshipId>,
            limit: u32,
        }

        let query = RawQuery::deserialize(deserializer)?;
        Self::new(query.after, query.limit).map_err(serde::de::Error::custom)
    }
}

tagged_contract! {
    /// Authorized read-only requests.
    pub enum Query {
        Configuration(GetConfiguration) => "eitmad.config.get.v1",
        EffectivePermissions(GetEffectivePermissions) => "eitmad.permissions.get-effective.v1",
        ScopeRelationships(ListScopeRelationships) => "eitmad.authorization.relationships.list.v1",
        UpdateState(GetUpdateState) => "eitmad.update.get-state.v1",
        SyncStatus(GetSyncStatus) => "eitmad.sync.get-status.v1"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum QueryResult {
    Configuration(ConfigSnapshot),
    EffectivePermissions(EffectivePermissions),
    ScopeRelationships(RelationshipPage),
    UpdateState(UpdateState),
    SyncStatus(SyncStatus),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_pages_are_bounded_during_deserialization() {
        assert!(
            serde_json::from_str::<ListScopeRelationships>(r#"{"after":null,"limit":1}"#).is_ok()
        );
        assert!(
            serde_json::from_str::<ListScopeRelationships>(r#"{"after":null,"limit":0}"#).is_err()
        );
        assert!(
            serde_json::from_str::<ListScopeRelationships>(r#"{"after":null,"limit":501}"#)
                .is_err()
        );
    }
}
