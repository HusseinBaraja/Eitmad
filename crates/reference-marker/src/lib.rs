//! Product-neutral reference marker domain module.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_authorization::{AuthorizationError, AuthorizationService, MutationContext};
pub use eitmad_authorization::{
    REFERENCE_MARKER_READ_PERMISSION, REFERENCE_MARKER_WRITE_PERMISSION,
};
use eitmad_contracts::{
    commands::UpsertReferenceMarker,
    events::Event,
    identity::{AuthorizationContext, ScopeRef},
    reference_marker::{
        ListReferenceMarkers, ReferenceMarker, ReferenceMarkerChangeNotice, ReferenceMarkerPage,
        ReferenceMarkerSyncState,
    },
    sync::{ChangeId, ChangeOperation, ChangeRecord, EncodedDomainPayload, RecordId},
    transport::SchemaId,
};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, DurableIdempotency, DurablePublication, ReferenceMarkerCommit,
    ReferenceMarkerCommitOutcome,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

pub const REFERENCE_MARKER_SCHEMA_ID: &str = "eitmad.schema.reference-marker.v1";

const ORGANIZATION_SCOPE: &str = "organization";
const UPSERT_OPERATION: &str = "eitmad.reference-marker.upsert.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceMarkerError {
    Denied,
    UnsupportedScope,
    RevisionConflict {
        expected_revision: Option<u64>,
        actual_revision: Option<u64>,
    },
    IdempotencyMismatch,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceMarkerMutation {
    pub marker: ReferenceMarker,
    pub changed: bool,
    pub replayed: bool,
}

#[derive(Clone, Debug)]
pub struct ReferenceMarkerService {
    store: AuthorityStore,
    authorization: AuthorizationService,
}

impl ReferenceMarkerService {
    #[must_use]
    pub const fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            store,
            authorization,
        }
    }

    /// Applies one authorized, optimistic marker update.
    ///
    /// # Errors
    ///
    /// Returns a domain error for denial, scope mismatch, revision conflict,
    /// idempotency mismatch, or unavailable durable state.
    pub fn upsert(
        &self,
        context: &MutationContext,
        command: &UpsertReferenceMarker,
    ) -> Result<ReferenceMarkerMutation, ReferenceMarkerError> {
        if let Err(error) =
            self.authorize(&context.authorization, REFERENCE_MARKER_WRITE_PERMISSION)
        {
            if error == ReferenceMarkerError::Denied {
                self.audit_failure(
                    context,
                    AuditOutcome::Denied,
                    "eitmad.error.authorization-denied.v1",
                    command,
                )?;
            }
            return Err(error);
        }
        validate_scope(&context.authorization.scope)?;
        let revision = command
            .expected_revision
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(ReferenceMarkerError::Unavailable)?;
        let marker = ReferenceMarker {
            id: command.marker_id,
            scope: context.authorization.scope.clone(),
            label: command.label.clone(),
            revision,
            updated_at: context.occurred_at,
            sync_state: ReferenceMarkerSyncState::Pending,
        };
        let change_id = ChangeId::new(Uuid::new_v4());
        let change = sync_change(context, &marker, command.expected_revision, change_id)?;
        let response_json =
            serde_json::to_vec(&marker).map_err(|_| ReferenceMarkerError::Unavailable)?;
        let idempotency = DurableIdempotency {
            key: context.idempotency_key,
            request_hash: request_hash(command)?,
            response_json,
        };
        let publication = DurablePublication {
            event: Event::ReferenceMarkerChanged(ReferenceMarkerChangeNotice {
                marker_id: marker.id,
                scope: marker.scope.clone(),
                revision: marker.revision,
                changed_at: marker.updated_at,
                change_id,
            }),
            policy_changed: false,
        };
        let audit = audit_record(context, command);
        match self.store.commit_reference_marker(&ReferenceMarkerCommit {
            marker: &marker,
            expected_revision: command.expected_revision,
            operation: UPSERT_OPERATION,
            idempotency: &idempotency,
            audit: &audit,
            publication: &publication,
            change: &change,
        }) {
            Ok(ReferenceMarkerCommitOutcome::Committed { marker, changed }) => {
                Ok(ReferenceMarkerMutation {
                    marker,
                    changed,
                    replayed: false,
                })
            }
            Ok(ReferenceMarkerCommitOutcome::Replayed { response_json }) => {
                let marker = serde_json::from_slice(&response_json)
                    .map_err(|_| ReferenceMarkerError::Unavailable)?;
                Ok(ReferenceMarkerMutation {
                    marker,
                    changed: false,
                    replayed: true,
                })
            }
            Ok(ReferenceMarkerCommitOutcome::RevisionConflict { actual_revision }) => {
                Err(ReferenceMarkerError::RevisionConflict {
                    expected_revision: command.expected_revision,
                    actual_revision,
                })
            }
            Ok(ReferenceMarkerCommitOutcome::IdempotencyMismatch) => {
                Err(ReferenceMarkerError::IdempotencyMismatch)
            }
            Err(_) => Err(ReferenceMarkerError::Unavailable),
        }
    }

    /// Lists one authorized, bounded marker page.
    ///
    /// # Errors
    ///
    /// Returns a domain error for denial, scope mismatch, or unavailable storage.
    pub fn list(
        &self,
        context: &AuthorizationContext,
        query: &ListReferenceMarkers,
    ) -> Result<ReferenceMarkerPage, ReferenceMarkerError> {
        validate_scope(&context.scope)?;
        self.authorize(context, REFERENCE_MARKER_READ_PERMISSION)?;
        self.store
            .list_reference_markers(&context.scope, query.after, query.limit())
            .map_err(|_| ReferenceMarkerError::Unavailable)
    }

    /// Loads a bounded internal sync batch for the common sync protocol.
    ///
    /// # Errors
    ///
    /// Returns an unavailable error for invalid limits or malformed durable work.
    pub fn sync_batch(
        &self,
        scope: &ScopeRef,
        limit: u32,
    ) -> Result<Vec<ChangeRecord>, ReferenceMarkerError> {
        validate_scope(scope)?;
        self.store
            .reference_marker_sync_batch(scope, limit)
            .map_err(|_| ReferenceMarkerError::Unavailable)
    }

    fn authorize(
        &self,
        context: &AuthorizationContext,
        permission: &str,
    ) -> Result<(), ReferenceMarkerError> {
        self.authorization
            .authorize(context, permission)
            .map_err(|error| match error {
                AuthorizationError::Denied => ReferenceMarkerError::Denied,
                AuthorizationError::UnsupportedScope => ReferenceMarkerError::UnsupportedScope,
                _ => ReferenceMarkerError::Unavailable,
            })
    }

    fn audit_failure(
        &self,
        context: &MutationContext,
        outcome: AuditOutcome,
        error_code: &str,
        command: &UpsertReferenceMarker,
    ) -> Result<(), ReferenceMarkerError> {
        self.store
            .append_audit(
                &audit_record(context, command).with_outcome(outcome, Some(error_code.to_owned())),
            )
            .map_err(|_| ReferenceMarkerError::Unavailable)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MarkerSyncPayload<'a> {
    marker_id: eitmad_contracts::reference_marker::ReferenceMarkerId,
    label: &'a eitmad_contracts::reference_marker::ReferenceMarkerLabel,
    revision: u64,
}

fn sync_change(
    context: &MutationContext,
    marker: &ReferenceMarker,
    base_revision: Option<u64>,
    change_id: ChangeId,
) -> Result<ChangeRecord, ReferenceMarkerError> {
    let payload = serde_json::to_vec(&MarkerSyncPayload {
        marker_id: marker.id,
        label: &marker.label,
        revision: marker.revision,
    })
    .map_err(|_| ReferenceMarkerError::Unavailable)?;
    Ok(ChangeRecord {
        change_id,
        record_id: RecordId::new(marker.id.value()),
        scope: marker.scope.clone(),
        operation: ChangeOperation::Upsert,
        base_revision,
        revision: marker.revision,
        changed_at: marker.updated_at,
        idempotency_key: context.idempotency_key,
        payload: Some(EncodedDomainPayload {
            schema_id: SchemaId::parse(REFERENCE_MARKER_SCHEMA_ID)
                .expect("static marker schema ID is valid"),
            schema_version: 1,
            base64: STANDARD.encode(payload),
        }),
        merge: None,
    })
}

fn request_hash(command: &UpsertReferenceMarker) -> Result<[u8; 32], ReferenceMarkerError> {
    let encoded = serde_json::to_vec(&(UPSERT_OPERATION, command))
        .map_err(|_| ReferenceMarkerError::Unavailable)?;
    Ok(Sha256::digest(encoded).into())
}

fn audit_record(context: &MutationContext, command: &UpsertReferenceMarker) -> MutationAuditRecord {
    let mut record = MutationAuditRecord::from_authorization(
        &context.authorization,
        context.occurred_at,
        context.correlation_id,
        UPSERT_OPERATION,
        AuditTarget {
            kind: "reference-marker".to_owned(),
            identifiers: vec![command.marker_id.value().to_string()],
        },
    );
    record.causation_id = context.causation_id;
    record.idempotency_key = Some(context.idempotency_key);
    record.changed_identifiers = vec!["label".to_owned()];
    record
}

fn validate_scope(scope: &ScopeRef) -> Result<(), ReferenceMarkerError> {
    (scope.kind.as_str() == ORGANIZATION_SCOPE)
        .then_some(())
        .ok_or(ReferenceMarkerError::UnsupportedScope)
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::{
        identity::{
            AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId,
            TenantId,
        },
        reference_marker::{ListReferenceMarkers, ReferenceMarkerId, ReferenceMarkerLabel},
        transport::{CorrelationId, IdempotencyKey, UnixMillis},
    };
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;

    fn authorization(value: u128) -> AuthorizationContext {
        let tenant = Uuid::from_u128(1);
        AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(value + 10)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(value)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(tenant),
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(tenant),
            },
        }
    }

    fn mutation(authorization: AuthorizationContext, value: u128) -> MutationContext {
        MutationContext {
            authorization,
            correlation_id: CorrelationId::new(Uuid::from_u128(value + 20)),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(value + 30)),
            occurred_at: UnixMillis(i64::try_from(value).unwrap()),
        }
    }

    #[test]
    fn complete_local_first_flow_preserves_arabic_and_confirms_sync() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let authorization_service =
            AuthorizationService::new(store.clone()).with_development_ephemeral_owner(true);
        let service = ReferenceMarkerService::new(store.clone(), authorization_service);
        let authorization = authorization(1);
        let marker_id = ReferenceMarkerId::new(Uuid::from_u128(40));
        let result = service
            .upsert(
                &mutation(authorization.clone(), 1),
                &UpsertReferenceMarker {
                    marker_id,
                    expected_revision: None,
                    label: ReferenceMarkerLabel::parse("مرجع REF-١٢").unwrap(),
                },
            )
            .unwrap();

        assert!(result.changed);
        assert_eq!(result.marker.revision, 1);
        assert_eq!(result.marker.label.as_str(), "مرجع REF-١٢");
        let page = service
            .list(
                &authorization,
                &ListReferenceMarkers::new(None, 10).unwrap(),
            )
            .unwrap();
        assert_eq!(page.items, vec![result.marker.clone()]);
        let batch = service.sync_batch(&authorization.scope, 10).unwrap();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].record_id.value(), marker_id.value());
        assert_eq!(batch[0].base_revision, None);
        assert!(batch[0].payload.as_ref().unwrap().base64.len() < 512);

        store
            .confirm_reference_marker_sync(&authorization.scope, batch[0].change_id)
            .unwrap();
        let confirmed = service
            .list(
                &authorization,
                &ListReferenceMarkers::new(None, 10).unwrap(),
            )
            .unwrap();
        assert_eq!(
            confirmed.items[0].sync_state,
            ReferenceMarkerSyncState::Confirmed
        );
        assert!(
            service
                .sync_batch(&authorization.scope, 10)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn denied_mutation_is_audited_without_storing_the_label() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let service =
            ReferenceMarkerService::new(store.clone(), AuthorizationService::new(store.clone()));
        let context = mutation(authorization(2), 2);
        let error = service
            .upsert(
                &context,
                &UpsertReferenceMarker {
                    marker_id: ReferenceMarkerId::new(Uuid::from_u128(50)),
                    expected_revision: None,
                    label: ReferenceMarkerLabel::parse("سري").unwrap(),
                },
            )
            .unwrap_err();
        assert_eq!(error, ReferenceMarkerError::Denied);

        let connection = Connection::open(store.path()).unwrap();
        let (outcome, changed): (String, String) = connection
            .query_row(
                "SELECT outcome, changed_identifiers FROM mutation_audit
                 WHERE operation = ?1 ORDER BY rowid DESC LIMIT 1",
                [UPSERT_OPERATION],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(outcome, "\"denied\"");
        assert!(!changed.contains("سري"));
    }
}
