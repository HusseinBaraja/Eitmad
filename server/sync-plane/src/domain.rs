use std::{collections::BTreeMap, sync::Arc};

use eitmad_contracts::{
    identity::ScopeRef,
    server::AuthenticatedServerSession,
    sync::{ChangeOperation, ConflictRecord, EncodedDomainPayload, RecordId, SyncMode},
    transport::{IdempotencyKey, SchemaId},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainDescriptor {
    pub schema_id: SchemaId,
    pub minimum_schema_version: u32,
    pub maximum_schema_version: u32,
    pub mode: SyncMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncIntent {
    Read,
    Write,
    ResolveConflict,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalOperationDraft {
    pub scope: ScopeRef,
    pub schema_id: SchemaId,
    pub schema_version: u32,
    pub record_id: RecordId,
    pub operation: ChangeOperation,
    pub base_revision: Option<u64>,
    pub idempotency_key: IdempotencyKey,
    pub payload: Option<EncodedDomainPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSubmission {
    pub scope: ScopeRef,
    pub schema_id: SchemaId,
    pub schema_version: u32,
    pub record_id: RecordId,
    pub base_revision: Option<u64>,
    pub idempotency_key: IdempotencyKey,
    pub command_base64: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoritativeChangeDraft {
    pub record_id: RecordId,
    pub operation: ChangeOperation,
    pub payload: Option<EncodedDomainPayload>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DomainValidationError {
    #[error("domain operation is denied")]
    Denied,
    #[error("domain payload is invalid")]
    Invalid,
    #[error("domain revision conflicts with authority")]
    Conflict,
}

pub trait DomainSyncHandler: Send + Sync {
    fn descriptor(&self) -> DomainDescriptor;
    fn authorize(
        &self,
        session: &AuthenticatedServerSession,
        scope: &ScopeRef,
        intent: SyncIntent,
    ) -> bool;
    /// Validates one local-first operation against domain invariants.
    ///
    /// # Errors
    ///
    /// Returns a denial, invalid-payload, or conflict result.
    fn validate_local(&self, draft: &LocalOperationDraft) -> Result<(), DomainValidationError>;
    /// Executes one server-authoritative domain command.
    ///
    /// # Errors
    ///
    /// Returns a denial, invalid-payload, or conflict result.
    fn execute_command(
        &self,
        command: &CommandSubmission,
    ) -> Result<AuthoritativeChangeDraft, DomainValidationError>;
    /// Produces an optional authoritative conflict resolution.
    ///
    /// # Errors
    ///
    /// Returns a denial or validation failure from the domain handler.
    fn resolve_conflict(
        &self,
        _conflict: &ConflictRecord,
    ) -> Result<Option<AuthoritativeChangeDraft>, DomainValidationError> {
        Ok(None)
    }
}

#[derive(Clone, Default)]
pub struct DomainRegistry {
    handlers: Arc<BTreeMap<String, Arc<dyn DomainSyncHandler>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DomainRegistryError {
    #[error("domain schema is already registered")]
    Duplicate,
    #[error("domain schema is not registered")]
    Unknown,
    #[error("domain schema version is incompatible")]
    IncompatibleVersion,
}

impl DomainRegistry {
    /// Builds an immutable registry and rejects duplicate schema ownership.
    ///
    /// # Errors
    ///
    /// Returns [`DomainRegistryError::Duplicate`] for duplicate schema IDs.
    pub fn new(
        handlers: impl IntoIterator<Item = Arc<dyn DomainSyncHandler>>,
    ) -> Result<Self, DomainRegistryError> {
        let mut values = BTreeMap::new();
        for handler in handlers {
            let schema = handler.descriptor().schema_id.as_str().to_owned();
            if values.insert(schema, handler).is_some() {
                return Err(DomainRegistryError::Duplicate);
            }
        }
        Ok(Self {
            handlers: Arc::new(values),
        })
    }

    pub(crate) fn get(
        &self,
        schema_id: &SchemaId,
        schema_version: u32,
    ) -> Result<Arc<dyn DomainSyncHandler>, DomainRegistryError> {
        let handler = self
            .handlers
            .get(schema_id.as_str())
            .cloned()
            .ok_or(DomainRegistryError::Unknown)?;
        let descriptor = handler.descriptor();
        if !(descriptor.minimum_schema_version..=descriptor.maximum_schema_version)
            .contains(&schema_version)
        {
            return Err(DomainRegistryError::IncompatibleVersion);
        }
        Ok(handler)
    }

    #[must_use]
    pub fn descriptors(&self) -> Vec<DomainDescriptor> {
        self.handlers
            .values()
            .map(|handler| handler.descriptor())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::sync::ConflictRecord;

    use super::*;

    struct Handler;

    impl DomainSyncHandler for Handler {
        fn descriptor(&self) -> DomainDescriptor {
            DomainDescriptor {
                schema_id: SchemaId::parse("eitmad.schema.synthetic.v1").unwrap(),
                minimum_schema_version: 1,
                maximum_schema_version: 2,
                mode: SyncMode::LocalFirst,
            }
        }

        fn authorize(
            &self,
            _session: &AuthenticatedServerSession,
            _scope: &ScopeRef,
            _intent: SyncIntent,
        ) -> bool {
            true
        }

        fn validate_local(
            &self,
            _draft: &LocalOperationDraft,
        ) -> Result<(), DomainValidationError> {
            Ok(())
        }

        fn execute_command(
            &self,
            _command: &CommandSubmission,
        ) -> Result<AuthoritativeChangeDraft, DomainValidationError> {
            Err(DomainValidationError::Denied)
        }

        fn resolve_conflict(
            &self,
            _conflict: &ConflictRecord,
        ) -> Result<Option<AuthoritativeChangeDraft>, DomainValidationError> {
            Ok(None)
        }
    }

    #[test]
    fn registry_rejects_duplicates_and_unknown_versions() {
        let one: Arc<dyn DomainSyncHandler> = Arc::new(Handler);
        assert!(DomainRegistry::new([one.clone(), one.clone()]).is_err());
        let registry = DomainRegistry::new([one]).unwrap();
        let schema = SchemaId::parse("eitmad.schema.synthetic.v1").unwrap();
        assert!(registry.get(&schema, 1).is_ok());
        assert!(matches!(
            registry.get(&schema, 3),
            Err(DomainRegistryError::IncompatibleVersion)
        ));
    }
}
