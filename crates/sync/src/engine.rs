use std::{collections::BTreeMap, sync::Arc};

use eitmad_authorization::{BoundaryAuditContext, BoundaryError};
use eitmad_contracts::{
    authorization::AuthorizationRequest,
    identity::{AuthorizationContext, ScopeRef},
    sync::{
        CacheFreshness, ChangeId, ChangeOperation, ChangeRecord, CommandDisposition, ConflictId,
        ConflictRecord, ConflictStatus, ConnectionState, DeliveryId, EncodedDomainPayload,
        ErrorCodeRef, MergeMetadata, MergeStrategy, PendingCommand, PendingCommandId,
        ReconciliationDelivery, RecordAuthority, RecordId, RecordView, SnapshotId, SyncEvent,
        SyncMetadata, SyncMode, SyncSnapshot, SyncStatus,
    },
    transport::{IdempotencyKey, SchemaId, UnixMillis},
    versioning::{
        NegotiatedSession, NegotiationOutcome, NegotiationRejection, PeerHello, negotiate,
    },
};
use eitmad_observability_audit::{AuditExtensionPoint, MutationAuditRecord};
use eitmad_storage::{AuthorityStore, SyncStateCommitOutcome};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use crate::SyncAuthorization;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LocalChangeDraft {
    pub record_id: RecordId,
    pub operation: ChangeOperation,
    pub changed_at: UnixMillis,
    pub idempotency_key: IdempotencyKey,
    pub payload: Option<EncodedDomainPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandDraft {
    pub command_id: PendingCommandId,
    pub idempotency_key: IdempotencyKey,
    pub submitted_at: UnixMillis,
    pub command_schema: SchemaId,
    pub command_schema_version: u32,
    pub base64: String,
    pub optimistic_change: Option<ChangeRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictResolution {
    Defer,
    KeepLocal,
    KeepRemote,
    Merge(EncodedDomainPayload),
}

pub trait ConflictHook: Send + Sync {
    fn resolve(&self, conflict: &ConflictRecord) -> ConflictResolution;
}

#[derive(Clone, Debug, Default)]
struct DeferConflicts;

impl ConflictHook for DeferConflicts {
    fn resolve(&self, _conflict: &ConflictRecord) -> ConflictResolution {
        ConflictResolution::Defer
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalChangeOutcome {
    Queued(ChangeRecord),
    Replayed(ChangeRecord),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingCommandOutcome {
    Queued(PendingCommand),
    Replayed(PendingCommand),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryOutcome {
    Applied,
    DuplicateIgnored,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncEngineError {
    Authorization(BoundaryError),
    StorageUnavailable,
    StorageConflict,
    CorruptState,
    WrongMode,
    ScopeMismatch,
    InvalidChange,
    IdempotencyMismatch,
    IncompatibleMode,
    IncompatiblePeer(NegotiationRejection),
    StaleCache,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Replay<T> {
    key: IdempotencyKey,
    fingerprint: [u8; 32],
    value: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProcessedDelivery {
    delivery_id: DeliveryId,
    key: IdempotencyKey,
    fingerprint: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EngineState {
    metadata: SyncMetadata,
    records: BTreeMap<RecordId, ChangeRecord>,
    confirmed_records: BTreeMap<RecordId, ChangeRecord>,
    pending_changes: Vec<ChangeRecord>,
    pending_commands: Vec<PendingCommand>,
    conflicts: Vec<ConflictRecord>,
    local_replays: Vec<Replay<ChangeRecord>>,
    command_replays: Vec<Replay<PendingCommand>>,
    processed_deliveries: Vec<ProcessedDelivery>,
    last_snapshot: Option<SyncSnapshot>,
}

impl EngineState {
    fn new(mode: SyncMode) -> Self {
        Self {
            metadata: SyncMetadata {
                mode,
                connection: ConnectionState::Offline,
                checkpoint: None,
                last_successful_sync_at: None,
                server_generation: None,
                cache_valid_until: None,
            },
            records: BTreeMap::new(),
            confirmed_records: BTreeMap::new(),
            pending_changes: Vec::new(),
            pending_commands: Vec::new(),
            conflicts: Vec::new(),
            local_replays: Vec::new(),
            command_replays: Vec::new(),
            processed_deliveries: Vec::new(),
            last_snapshot: None,
        }
    }
}

pub struct SyncEngine {
    store: AuthorityStore,
    scope: ScopeRef,
    authorization: SyncAuthorization,
    conflict_hook: Arc<dyn ConflictHook>,
    storage_revision: u64,
    state: EngineState,
    events: Vec<SyncEvent>,
}

impl SyncEngine {
    /// Opens or creates one scoped durable sync engine.
    ///
    /// # Errors
    ///
    /// Fails closed when persisted state is unavailable, corrupt, or belongs to
    /// a different application mode.
    pub fn open(
        store: AuthorityStore,
        scope: ScopeRef,
        mode: SyncMode,
        authorization: SyncAuthorization,
    ) -> Result<Self, SyncEngineError> {
        Self::open_with_conflict_hook(store, scope, mode, authorization, Arc::new(DeferConflicts))
    }

    /// Opens a sync engine with a domain-owned conflict hook.
    ///
    /// # Errors
    ///
    /// Has the same failure contract as [`Self::open`].
    pub fn open_with_conflict_hook(
        store: AuthorityStore,
        scope: ScopeRef,
        mode: SyncMode,
        authorization: SyncAuthorization,
        conflict_hook: Arc<dyn ConflictHook>,
    ) -> Result<Self, SyncEngineError> {
        let stored = store
            .read_sync_state(&scope)
            .map_err(|_| SyncEngineError::StorageUnavailable)?;
        let (storage_revision, state) = if let Some(stored) = stored {
            if stored.application_mode != mode_name(mode) {
                return Err(SyncEngineError::IncompatibleMode);
            }
            let state = serde_json::from_slice::<EngineState>(&stored.state_json)
                .map_err(|_| SyncEngineError::CorruptState)?;
            if state.metadata.mode != mode {
                return Err(SyncEngineError::CorruptState);
            }
            (stored.revision, state)
        } else {
            let state = EngineState::new(mode);
            let encoded = serde_json::to_vec(&state).map_err(|_| SyncEngineError::CorruptState)?;
            let revision = match store
                .commit_sync_state(&scope, mode_name(mode), 0, &encoded, None)
                .map_err(|_| SyncEngineError::StorageUnavailable)?
            {
                SyncStateCommitOutcome::Committed { revision } => revision,
                SyncStateCommitOutcome::RevisionConflict { .. } => {
                    return Err(SyncEngineError::StorageConflict);
                }
            };
            (revision, state)
        };
        Ok(Self {
            store,
            scope,
            authorization,
            conflict_hook,
            storage_revision,
            state,
            events: Vec::new(),
        })
    }

    #[must_use]
    pub fn metadata(&self) -> &SyncMetadata {
        &self.state.metadata
    }

    #[must_use]
    pub fn pending_changes(&self) -> &[ChangeRecord] {
        &self.state.pending_changes
    }

    #[must_use]
    pub fn pending_commands(&self) -> &[PendingCommand] {
        &self.state.pending_commands
    }

    #[must_use]
    pub fn conflicts(&self) -> &[ConflictRecord] {
        &self.state.conflicts
    }

    #[must_use]
    /// Returns the current coalesced status.
    ///
    /// # Panics
    ///
    /// Panics only if the compile-time protocol error identifier is invalid.
    pub fn status(&self) -> SyncStatus {
        let open_conflicts = self
            .state
            .conflicts
            .iter()
            .filter(|conflict| conflict.status == ConflictStatus::Open)
            .count();
        if open_conflicts > 0 {
            return SyncStatus::Conflicted {
                records: open_conflicts as u64,
            };
        }
        let queued = self.state.pending_changes.len() + self.state.pending_commands.len();
        if queued > 0 {
            return SyncStatus::Queued {
                records: queued as u64,
            };
        }
        match (
            self.state.metadata.connection,
            self.state.metadata.checkpoint,
        ) {
            (ConnectionState::Offline, _) => SyncStatus::Offline,
            (ConnectionState::Connected, Some(checkpoint)) => SyncStatus::Current { checkpoint },
            (ConnectionState::Connected, None) => SyncStatus::Syncing {
                completed: 0,
                total: None,
            },
            (ConnectionState::Incompatible, _) => SyncStatus::Failed {
                reason: ErrorCodeRef::parse("eitmad.error.protocol-incompatible.v1")
                    .expect("static sync error is valid"),
            },
        }
    }

    /// Negotiates a compatible peer before any normal sync traffic.
    ///
    /// # Errors
    ///
    /// Returns a mode or protocol/schema/capability rejection and persists the
    /// incompatible state.
    pub fn connect(
        &mut self,
        local: &PeerHello,
        remote: &PeerHello,
        remote_mode: SyncMode,
    ) -> Result<NegotiatedSession, SyncEngineError> {
        let previous = self.state.clone();
        if remote_mode != self.state.metadata.mode {
            self.state.metadata.connection = ConnectionState::Incompatible;
            self.persist(None).inspect_err(|_| self.state = previous)?;
            return Err(SyncEngineError::IncompatibleMode);
        }
        match negotiate(local, remote) {
            NegotiationOutcome::Accepted(session) => {
                self.state.metadata.connection = ConnectionState::Connected;
                self.persist(None).inspect_err(|_| self.state = previous)?;
                self.events
                    .push(SyncEvent::ConnectionChanged(ConnectionState::Connected));
                Ok(session)
            }
            NegotiationOutcome::Rejected(rejection) => {
                self.state.metadata.connection = ConnectionState::Incompatible;
                self.persist(None).inspect_err(|_| self.state = previous)?;
                Err(SyncEngineError::IncompatiblePeer(rejection))
            }
        }
    }

    /// Marks transport loss while preserving durable local work and queues.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the offline marker cannot be persisted.
    pub fn disconnect(&mut self) -> Result<(), SyncEngineError> {
        let previous = self.state.clone();
        self.state.metadata.connection = ConnectionState::Offline;
        self.persist(None).inspect_err(|_| self.state = previous)?;
        self.events
            .push(SyncEvent::ConnectionChanged(ConnectionState::Offline));
        Ok(())
    }

    /// Applies and queues one authorized offline-capable local-first edit.
    ///
    /// # Errors
    ///
    /// Rejects wrong-mode, cross-scope, invalid, unauthorized, or mismatched
    /// idempotent work without changing local records.
    pub fn apply_local_change(
        &mut self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
        draft: LocalChangeDraft,
    ) -> Result<LocalChangeOutcome, SyncEngineError> {
        if self.state.metadata.mode != SyncMode::LocalFirst {
            return Err(SyncEngineError::WrongMode);
        }
        self.validate_actor(actor)?;
        validate_operation(draft.operation, draft.payload.as_ref())?;
        let fingerprint = fingerprint(&draft)?;
        if let Some(replay) = self
            .state
            .local_replays
            .iter()
            .find(|replay| replay.key == draft.idempotency_key)
        {
            return if replay.fingerprint == fingerprint {
                Ok(LocalChangeOutcome::Replayed(replay.value.clone()))
            } else {
                Err(SyncEngineError::IdempotencyMismatch)
            };
        }
        self.authorization
            .authorize(actor, request, audit)
            .map_err(SyncEngineError::Authorization)?;

        let previous = self.state.clone();
        let base_revision = self
            .state
            .records
            .get(&draft.record_id)
            .map(|record| record.revision);
        let revision = base_revision
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(SyncEngineError::InvalidChange)?;
        let record = ChangeRecord {
            change_id: ChangeId::new(Uuid::new_v4()),
            record_id: draft.record_id,
            scope: self.scope.clone(),
            operation: draft.operation,
            base_revision,
            revision,
            changed_at: draft.changed_at,
            idempotency_key: draft.idempotency_key,
            payload: draft.payload,
            merge: None,
        };
        self.state.records.insert(record.record_id, record.clone());
        self.state.pending_changes.push(record.clone());
        self.state.local_replays.push(Replay {
            key: draft.idempotency_key,
            fingerprint,
            value: record.clone(),
        });
        let mutation_audit = audit_record(actor, audit, Some(draft.idempotency_key));
        self.persist(Some(&mutation_audit))
            .inspect_err(|_| self.state = previous)?;
        self.events
            .push(SyncEvent::LocalChangeQueued(record.clone()));
        self.events.push(SyncEvent::StatusChanged(self.status()));
        Ok(LocalChangeOutcome::Queued(record))
    }

    /// Queues one authorized server command and optionally exposes optimistic state.
    ///
    /// # Errors
    ///
    /// Rejects wrong-mode, cross-scope, unauthorized, or mismatched idempotent
    /// commands without changing the cache.
    pub fn queue_command(
        &mut self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
        draft: CommandDraft,
    ) -> Result<PendingCommandOutcome, SyncEngineError> {
        if self.state.metadata.mode != SyncMode::ServerAuthoritative {
            return Err(SyncEngineError::WrongMode);
        }
        self.validate_actor(actor)?;
        if let Some(change) = &draft.optimistic_change {
            self.validate_record(change)?;
            validate_operation(change.operation, change.payload.as_ref())?;
        }
        let fingerprint = fingerprint(&draft)?;
        if let Some(replay) = self
            .state
            .command_replays
            .iter()
            .find(|replay| replay.key == draft.idempotency_key)
        {
            return if replay.fingerprint == fingerprint {
                Ok(PendingCommandOutcome::Replayed(replay.value.clone()))
            } else {
                Err(SyncEngineError::IdempotencyMismatch)
            };
        }
        self.authorization
            .authorize(actor, request, audit)
            .map_err(SyncEngineError::Authorization)?;

        let previous = self.state.clone();
        let pending = PendingCommand {
            command_id: draft.command_id,
            scope: self.scope.clone(),
            actor: actor.clone(),
            idempotency_key: draft.idempotency_key,
            submitted_at: draft.submitted_at,
            command_schema: draft.command_schema,
            command_schema_version: draft.command_schema_version,
            base_revision: draft
                .optimistic_change
                .as_ref()
                .and_then(|change| change.base_revision),
            base64: draft.base64,
            optimistic_change: draft.optimistic_change,
        };
        self.state.pending_commands.push(pending.clone());
        self.state.command_replays.push(Replay {
            key: draft.idempotency_key,
            fingerprint,
            value: pending.clone(),
        });
        self.rebuild_server_view();
        let mutation_audit = audit_record(actor, audit, Some(draft.idempotency_key));
        self.persist(Some(&mutation_audit))
            .inspect_err(|_| self.state = previous)?;
        self.events.push(SyncEvent::CommandQueued(pending.clone()));
        self.events.push(SyncEvent::StatusChanged(self.status()));
        Ok(PendingCommandOutcome::Queued(pending))
    }

    /// Applies one authorized, idempotent delivery after compatibility negotiation.
    ///
    /// # Errors
    ///
    /// Rejects offline/incompatible sessions, cross-scope content, unauthorized
    /// content, or reused delivery identities with different bytes.
    pub fn reconcile(
        &mut self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
        delivery: &ReconciliationDelivery,
    ) -> Result<DeliveryOutcome, SyncEngineError> {
        if self.state.metadata.connection != ConnectionState::Connected {
            return Err(SyncEngineError::IncompatiblePeer(
                NegotiationRejection::NoCommonProtocol,
            ));
        }
        self.validate_actor(actor)?;
        for change in &delivery.changes {
            self.validate_record(change)?;
            validate_operation(change.operation, change.payload.as_ref())?;
        }
        if let Some(snapshot) = &delivery.snapshot {
            self.validate_snapshot(snapshot)?;
        }
        let fingerprint = fingerprint(delivery)?;
        if let Some(processed) = self.state.processed_deliveries.iter().find(|processed| {
            processed.delivery_id == delivery.delivery_id
                || processed.key == delivery.idempotency_key
        }) {
            return if processed.fingerprint == fingerprint {
                self.events
                    .push(SyncEvent::DuplicateDeliveryIgnored(delivery.delivery_id));
                Ok(DeliveryOutcome::DuplicateIgnored)
            } else {
                Err(SyncEngineError::IdempotencyMismatch)
            };
        }
        self.authorization
            .authorize(actor, request, audit)
            .map_err(SyncEngineError::Authorization)?;

        let previous = self.state.clone();
        match self.state.metadata.mode {
            SyncMode::LocalFirst => self.reconcile_local_first(delivery)?,
            SyncMode::ServerAuthoritative => self.reconcile_server_authoritative(delivery)?,
        }
        self.state.metadata.checkpoint = Some(delivery.checkpoint);
        self.state.metadata.last_successful_sync_at = Some(delivery.received_at);
        self.state.processed_deliveries.push(ProcessedDelivery {
            delivery_id: delivery.delivery_id,
            key: delivery.idempotency_key,
            fingerprint,
        });
        let mutation_audit = audit_record(actor, audit, Some(delivery.idempotency_key));
        self.persist(Some(&mutation_audit))
            .inspect_err(|_| self.state = previous)?;
        self.events.push(SyncEvent::StatusChanged(self.status()));
        Ok(DeliveryOutcome::Applied)
    }

    /// Reads local durable or explicitly optimistic state.
    ///
    /// # Errors
    ///
    /// Server-confirmed cache entries fail closed after their validity deadline;
    /// optimistic entries remain visible but are labeled stale and non-authoritative.
    pub fn read_record(
        &self,
        record_id: RecordId,
        now: UnixMillis,
    ) -> Result<Option<RecordView>, SyncEngineError> {
        let Some(record) = self.state.records.get(&record_id).cloned() else {
            return Ok(None);
        };
        if self.state.metadata.mode == SyncMode::LocalFirst {
            return Ok(Some(RecordView {
                record,
                authority: RecordAuthority::LocalDurable,
                freshness: CacheFreshness::Fresh,
            }));
        }
        let optimistic = self.state.pending_commands.iter().rev().any(|command| {
            command
                .optimistic_change
                .as_ref()
                .is_some_and(|change| change.record_id == record_id)
        });
        let fresh = self
            .state
            .metadata
            .cache_valid_until
            .is_some_and(|valid_until| now <= valid_until);
        if !optimistic && !fresh {
            return Err(SyncEngineError::StaleCache);
        }
        Ok(Some(RecordView {
            record,
            authority: if optimistic {
                RecordAuthority::Optimistic
            } else {
                RecordAuthority::ServerConfirmed
            },
            freshness: if fresh {
                CacheFreshness::Fresh
            } else {
                CacheFreshness::Stale
            },
        }))
    }

    #[must_use]
    pub fn drain_events(&mut self) -> Vec<SyncEvent> {
        std::mem::take(&mut self.events)
    }

    fn reconcile_local_first(
        &mut self,
        delivery: &ReconciliationDelivery,
    ) -> Result<(), SyncEngineError> {
        if let Some(snapshot) = &delivery.snapshot {
            for remote in &snapshot.records {
                self.merge_local_first(remote, delivery.received_at)?;
            }
            self.state.last_snapshot = Some(snapshot.clone());
            self.events
                .push(SyncEvent::SnapshotApplied(snapshot.clone()));
        }
        for remote in &delivery.changes {
            self.merge_local_first(remote, delivery.received_at)?;
        }
        Ok(())
    }

    fn merge_local_first(
        &mut self,
        remote: &ChangeRecord,
        detected_at: UnixMillis,
    ) -> Result<(), SyncEngineError> {
        let pending_index = self
            .state
            .pending_changes
            .iter()
            .position(|local| local.record_id == remote.record_id);
        let Some(pending_index) = pending_index else {
            let should_apply = self
                .state
                .records
                .get(&remote.record_id)
                .is_none_or(|current| remote.revision > current.revision);
            if should_apply {
                self.state.records.insert(remote.record_id, remote.clone());
            }
            return Ok(());
        };
        let local = self.state.pending_changes[pending_index].clone();
        if remote.change_id == local.change_id {
            self.state.pending_changes.remove(pending_index);
            self.state.records.insert(remote.record_id, remote.clone());
            return Ok(());
        }
        if remote.revision <= local.base_revision.unwrap_or(0) {
            return Ok(());
        }
        let mut conflict = ConflictRecord {
            conflict_id: ConflictId::new(Uuid::new_v4()),
            scope: self.scope.clone(),
            record_id: remote.record_id,
            local: local.clone(),
            remote: remote.clone(),
            detected_at,
            status: ConflictStatus::Open,
            resolution: None,
        };
        self.events
            .push(SyncEvent::ConflictDetected(conflict.clone()));
        match self.conflict_hook.resolve(&conflict) {
            ConflictResolution::Defer => self.state.conflicts.push(conflict),
            ConflictResolution::KeepRemote => {
                let merge = merge_metadata(MergeStrategy::KeepRemote, &local, remote, detected_at);
                conflict.status = ConflictStatus::Resolved;
                conflict.resolution = Some(merge);
                self.state.pending_changes.remove(pending_index);
                self.state.records.insert(remote.record_id, remote.clone());
                self.state.conflicts.push(conflict.clone());
                self.events.push(SyncEvent::ConflictResolved(conflict));
            }
            ConflictResolution::KeepLocal => {
                let merge = merge_metadata(MergeStrategy::KeepLocal, &local, remote, detected_at);
                let mut rebased = local.clone();
                rebased.change_id = ChangeId::new(Uuid::new_v4());
                rebased.base_revision = Some(remote.revision);
                rebased.revision = remote
                    .revision
                    .checked_add(1)
                    .ok_or(SyncEngineError::InvalidChange)?;
                rebased.merge = Some(merge.clone());
                self.state.pending_changes[pending_index] = rebased.clone();
                self.state.records.insert(rebased.record_id, rebased);
                conflict.status = ConflictStatus::Resolved;
                conflict.resolution = Some(merge);
                self.state.conflicts.push(conflict.clone());
                self.events.push(SyncEvent::ConflictResolved(conflict));
            }
            ConflictResolution::Merge(payload) => {
                let merge = merge_metadata(MergeStrategy::DomainMerge, &local, remote, detected_at);
                let merged = ChangeRecord {
                    change_id: ChangeId::new(Uuid::new_v4()),
                    record_id: remote.record_id,
                    scope: self.scope.clone(),
                    operation: ChangeOperation::Upsert,
                    base_revision: Some(remote.revision),
                    revision: remote
                        .revision
                        .checked_add(1)
                        .ok_or(SyncEngineError::InvalidChange)?,
                    changed_at: detected_at,
                    idempotency_key: local.idempotency_key,
                    payload: Some(payload),
                    merge: Some(merge.clone()),
                };
                self.state.pending_changes[pending_index] = merged.clone();
                self.state.records.insert(merged.record_id, merged);
                conflict.status = ConflictStatus::Resolved;
                conflict.resolution = Some(merge);
                self.state.conflicts.push(conflict.clone());
                self.events.push(SyncEvent::ConflictResolved(conflict));
            }
        }
        Ok(())
    }

    fn reconcile_server_authoritative(
        &mut self,
        delivery: &ReconciliationDelivery,
    ) -> Result<(), SyncEngineError> {
        if let Some(snapshot) = &delivery.snapshot {
            self.state.confirmed_records = snapshot
                .records
                .iter()
                .cloned()
                .map(|record| (record.record_id, record))
                .collect();
            self.state.metadata.server_generation = Some(snapshot.server_generation);
            self.state.metadata.cache_valid_until = Some(snapshot.valid_until);
            self.state.last_snapshot = Some(snapshot.clone());
            self.events
                .push(SyncEvent::SnapshotApplied(snapshot.clone()));
        }
        for change in &delivery.changes {
            let should_apply = self
                .state
                .confirmed_records
                .get(&change.record_id)
                .is_none_or(|current| change.revision > current.revision);
            if should_apply {
                self.state
                    .confirmed_records
                    .insert(change.record_id, change.clone());
            }
        }
        for result in &delivery.command_results {
            let Some(index) = self
                .state
                .pending_commands
                .iter()
                .position(|command| command.command_id == result.command_id)
            else {
                continue;
            };
            let pending = self.state.pending_commands.remove(index);
            match &result.disposition {
                CommandDisposition::Accepted {
                    authoritative_change,
                } => {
                    if let Some(change) = authoritative_change {
                        self.validate_record(change)?;
                        self.state
                            .confirmed_records
                            .insert(change.record_id, change.as_ref().clone());
                    }
                }
                CommandDisposition::Denied { reason } => {
                    self.events.push(SyncEvent::CommandDenied {
                        command_id: pending.command_id,
                        reason: reason.clone(),
                    });
                }
            }
        }
        self.rebuild_server_view();
        Ok(())
    }

    fn rebuild_server_view(&mut self) {
        self.state.records.clone_from(&self.state.confirmed_records);
        for command in &self.state.pending_commands {
            if let Some(change) = &command.optimistic_change {
                self.state.records.insert(change.record_id, change.clone());
            }
        }
    }

    fn validate_actor(&self, actor: &AuthorizationContext) -> Result<(), SyncEngineError> {
        (actor.scope == self.scope)
            .then_some(())
            .ok_or(SyncEngineError::ScopeMismatch)
    }

    fn validate_record(&self, record: &ChangeRecord) -> Result<(), SyncEngineError> {
        (record.scope == self.scope)
            .then_some(())
            .ok_or(SyncEngineError::ScopeMismatch)
    }

    fn validate_snapshot(&self, snapshot: &SyncSnapshot) -> Result<(), SyncEngineError> {
        if snapshot.scope != self.scope || snapshot.valid_until < snapshot.created_at {
            return Err(SyncEngineError::InvalidChange);
        }
        snapshot
            .records
            .iter()
            .try_for_each(|record| self.validate_record(record))
    }

    fn persist(&mut self, audit: Option<&MutationAuditRecord>) -> Result<(), SyncEngineError> {
        let encoded = serde_json::to_vec(&self.state).map_err(|_| SyncEngineError::CorruptState)?;
        match self
            .store
            .commit_sync_state(
                &self.scope,
                mode_name(self.state.metadata.mode),
                self.storage_revision,
                &encoded,
                audit,
            )
            .map_err(|_| SyncEngineError::StorageUnavailable)?
        {
            SyncStateCommitOutcome::Committed { revision } => {
                self.storage_revision = revision;
                Ok(())
            }
            SyncStateCommitOutcome::RevisionConflict { .. } => {
                Err(SyncEngineError::StorageConflict)
            }
        }
    }
}

fn validate_operation(
    operation: ChangeOperation,
    payload: Option<&EncodedDomainPayload>,
) -> Result<(), SyncEngineError> {
    match (operation, payload) {
        (ChangeOperation::Upsert, Some(_)) | (ChangeOperation::Tombstone, None) => Ok(()),
        _ => Err(SyncEngineError::InvalidChange),
    }
}

fn mode_name(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::LocalFirst => "local-first",
        SyncMode::ServerAuthoritative => "server-authoritative",
    }
}

fn fingerprint(value: &impl Serialize) -> Result<[u8; 32], SyncEngineError> {
    let encoded = serde_json::to_vec(value).map_err(|_| SyncEngineError::CorruptState)?;
    Ok(Sha256::digest(encoded).into())
}

fn audit_record(
    actor: &AuthorizationContext,
    audit: &BoundaryAuditContext,
    idempotency_key: Option<IdempotencyKey>,
) -> MutationAuditRecord {
    let mut record = MutationAuditRecord::from_authorization(
        actor,
        audit.occurred_at,
        audit.correlation_id,
        audit.operation.clone(),
        audit.target.clone(),
    );
    record.causation_id = audit.causation_id;
    record.idempotency_key = idempotency_key.or(audit.idempotency_key);
    record.extension_points.clone_from(&audit.extension_points);
    if !record
        .extension_points
        .contains(&AuditExtensionPoint::SyncBoundary)
    {
        record
            .extension_points
            .push(AuditExtensionPoint::SyncBoundary);
    }
    record
}

fn merge_metadata(
    strategy: MergeStrategy,
    local: &ChangeRecord,
    remote: &ChangeRecord,
    merged_at: UnixMillis,
) -> MergeMetadata {
    MergeMetadata {
        strategy,
        common_ancestor_revision: local.base_revision,
        source_changes: vec![local.change_id, remote.change_id],
        merged_at,
    }
}

#[allow(dead_code)]
fn _contract_identity_guards(
    _snapshot: SnapshotId,
    _delivery: DeliveryId,
    _command: PendingCommandId,
) {
}
