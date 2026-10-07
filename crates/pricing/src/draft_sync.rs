//! One bounded draft outbox and incremental download cycle.

use std::time::{Duration, Instant};

use eitmad_authorization::{BoundaryAuditContext, now};
use eitmad_contracts::{
    authorization::AuthorizationRequest,
    identity::{AuthorizationContext, ScopeRef},
    quotation_draft::{QuotationDraftId, QuotationDraftSyncState},
    sync::{
        BatchAcknowledgement, ChangeBatch, ChangeId, ChangeRecord, LocalChangeDisposition,
        LocalChangeSubmission, PullRequest, ReconciliationDelivery, SyncMessage, SyncMode,
    },
    sync_transport::{SyncFrameId, SyncStreamId, SyncTransportFrame, SyncTransportPayload},
    transport::{CorrelationId, IdempotencyKey, SchemaId},
    versioning::NegotiatedSchema,
};
use eitmad_sync::{ReceiveOutcome, SyncEngine, SyncEngineError, SyncTransport, TransportFailure};
use uuid::Uuid;

use crate::{QUOTATION_DRAFT_SCHEMA, QuotationDraftError, QuotationDraftService};

#[derive(Debug)]
pub enum QuotationDraftSyncError {
    QuotationDraft(QuotationDraftError),
    Engine(SyncEngineError),
    Transport(TransportFailure),
    UnexpectedResponse,
    ResponseTimeout,
}

impl From<QuotationDraftError> for QuotationDraftSyncError {
    fn from(value: QuotationDraftError) -> Self {
        Self::QuotationDraft(value)
    }
}

impl From<SyncEngineError> for QuotationDraftSyncError {
    fn from(value: SyncEngineError) -> Self {
        Self::Engine(value)
    }
}

impl From<TransportFailure> for QuotationDraftSyncError {
    fn from(value: TransportFailure) -> Self {
        Self::Transport(value)
    }
}

/// Runs on a Rust worker thread. The draft outbox is the submission source;
/// the shared engine owns replay, conflicts, and the download checkpoint.
pub struct QuotationDraftSyncCycle<'a, T: SyncTransport> {
    pub drafts: &'a QuotationDraftService,
    pub engine: &'a mut SyncEngine,
    pub transport: &'a mut T,
    pub actor: &'a AuthorizationContext,
    /// Server branch selected by an authenticated Manager enrollment.
    pub server_catalog_scope: &'a ScopeRef,
    pub server_scope: &'a ScopeRef,
    pub request: &'a AuthorizationRequest,
    pub audit: &'a BoundaryAuditContext,
}

impl<T: SyncTransport> QuotationDraftSyncCycle<'_, T> {
    /// Delivers up to 50 local changes and downloads all available 50-record pages.
    ///
    /// # Errors
    ///
    /// Preserves local work and the last durable checkpoint on transport,
    /// storage, authorization, or projection failure.
    pub fn run(&mut self) -> Result<(), QuotationDraftSyncError> {
        if self.engine.domain_schema().map(SchemaId::as_str) != Some(QUOTATION_DRAFT_SCHEMA) {
            return Err(QuotationDraftSyncError::UnexpectedResponse);
        }
        let result = self.run_connected();
        if result.is_err() {
            self.transport.disconnect(now());
            let _ = self.engine.disconnect(self.actor, self.request, self.audit);
        }
        result
    }

    fn pending(&self) -> Result<Vec<ChangeRecord>, QuotationDraftSyncError> {
        match self.drafts.sync_batch(self.actor, 50) {
            Ok(pending) => Ok(pending),
            Err(QuotationDraftError::Denied) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    fn stage_pending(&mut self, pending: &[ChangeRecord]) -> Result<(), SyncEngineError> {
        for change in pending {
            self.engine.stage_committed_local_change(
                self.actor,
                self.request,
                self.audit,
                change.clone(),
            )?;
        }
        Ok(())
    }

    fn run_connected(&mut self) -> Result<(), QuotationDraftSyncError> {
        let session = self.transport.connect(now())?;
        let schema = NegotiatedSchema {
            schema_id: SchemaId::parse(QUOTATION_DRAFT_SCHEMA).expect("static draft schema"),
            version: 1,
        };
        self.engine.connect_negotiated(
            self.actor,
            self.request,
            self.audit,
            &session,
            SyncMode::LocalFirst,
            &schema,
        )?;
        let mut pending = self.pending()?;
        match self.stage_pending(&pending) {
            Ok(()) => {}
            Err(SyncEngineError::InvalidChange) => {
                // A projection can commit before engine reconciliation fails.
                // Replay that page before staging edits based on its revision.
                self.pull()?;
                pending = self.pending()?;
                self.stage_pending(&pending)?;
            }
            Err(error) => return Err(error.into()),
        }
        self.submit_pending(&pending)?;
        self.pull()
    }

    fn pull(&mut self) -> Result<(), QuotationDraftSyncError> {
        loop {
            let after = self.engine.metadata().checkpoint;
            let response = self.exchange(SyncMessage::Pull(PullRequest {
                after,
                maximum_records: 50,
            }))?;
            let SyncMessage::Changes(batch) = response else {
                return Err(QuotationDraftSyncError::UnexpectedResponse);
            };
            if batch.from_checkpoint != after
                || batch.records.len() > 50
                || (batch.has_more && batch.records.is_empty())
            {
                return Err(QuotationDraftSyncError::UnexpectedResponse);
            }
            self.apply_batch(&batch)?;
            let acknowledgement = BatchAcknowledgement {
                delivery_id: batch.delivery_id,
                checkpoint: batch.checkpoint,
                accepted_records: u32::try_from(batch.records.len())
                    .map_err(|_| QuotationDraftSyncError::UnexpectedResponse)?,
            };
            match self.exchange(SyncMessage::Acknowledge(acknowledgement.clone()))? {
                SyncMessage::Acknowledge(reply) if reply == acknowledgement => {}
                _ => return Err(QuotationDraftSyncError::UnexpectedResponse),
            }
            if !batch.has_more {
                break;
            }
        }
        Ok(())
    }

    fn submit_pending(
        &mut self,
        pending: &[eitmad_contracts::sync::ChangeRecord],
    ) -> Result<(), QuotationDraftSyncError> {
        let mut blocked = std::collections::HashSet::new();
        for change in pending {
            if blocked.contains(&change.record_id) {
                continue;
            }
            let mut server_change = change.clone();
            server_change.scope = self.server_scope.clone();
            let local_catalog = catalog_scope(self.actor);
            remap_payload(
                &mut server_change,
                &self.actor.scope,
                self.server_scope,
                &local_catalog,
                self.server_catalog_scope,
            )?;
            let response = self.exchange(SyncMessage::SubmitLocal(LocalChangeSubmission {
                change: server_change.clone(),
            }))?;
            let SyncMessage::LocalResult(result) = response else {
                return Err(QuotationDraftSyncError::UnexpectedResponse);
            };
            if result.submitted_change_id != change.change_id {
                return Err(QuotationDraftSyncError::UnexpectedResponse);
            }
            let exception = match result.disposition {
                LocalChangeDisposition::Applied {
                    authoritative_change,
                }
                | LocalChangeDisposition::Replayed {
                    authoritative_change,
                } => {
                    if authoritative_change.change_id != change.change_id
                        || authoritative_change.scope != *self.server_scope
                        || authoritative_change.record_id != server_change.record_id
                        || authoritative_change.idempotency_key != server_change.idempotency_key
                        || authoritative_change.revision != server_change.revision
                        || authoritative_change.base_revision != server_change.base_revision
                        || authoritative_change.operation != server_change.operation
                        || authoritative_change.payload != server_change.payload
                    {
                        return Err(QuotationDraftSyncError::UnexpectedResponse);
                    }
                    None
                }
                LocalChangeDisposition::Conflicted { conflict_id } => {
                    Some((QuotationDraftSyncState::Conflicted, Some(conflict_id)))
                }
                LocalChangeDisposition::Rejected { .. } => {
                    Some((QuotationDraftSyncState::Rejected, None))
                }
            };
            if let Some((state, conflict)) = exception {
                blocked.insert(change.record_id);
                // Dequeue all revisions before hiding the draft from sync_batch.
                // A failed status write retries without holding an absent change.
                for queued in pending.iter().filter(|c| c.record_id == change.record_id) {
                    self.hold_pending_change(queued.change_id)?;
                }
                self.drafts.mark_sync_exception(
                    self.actor,
                    QuotationDraftId::new(change.record_id.value()),
                    state,
                    conflict,
                    self.audit.correlation_id,
                    now(),
                )?;
            }
        }
        Ok(())
    }

    fn hold_pending_change(&mut self, change_id: ChangeId) -> Result<(), SyncEngineError> {
        if self
            .engine
            .pending_changes()
            .iter()
            .any(|c| c.change_id == change_id)
        {
            self.engine.hold_committed_local_change(
                self.actor,
                self.request,
                self.audit,
                change_id,
            )?;
        }
        Ok(())
    }
    fn apply_batch(&mut self, batch: &ChangeBatch) -> Result<(), QuotationDraftSyncError> {
        // Projection precedes checkpoint persistence. A crash can replay a page,
        // but it cannot skip a draft projection after the checkpoint advances.
        let mut local_records = Vec::with_capacity(batch.records.len());
        for change in &batch.records {
            if change.scope != *self.server_scope {
                return Err(QuotationDraftSyncError::UnexpectedResponse);
            }
            let mut local_change = change.clone();
            local_change.scope = self.actor.scope.clone();
            let local_catalog = catalog_scope(self.actor);
            remap_payload(
                &mut local_change,
                self.server_scope,
                &self.actor.scope,
                self.server_catalog_scope,
                &local_catalog,
            )?;
            self.drafts
                .project_confirmed(self.actor, &local_change, self.audit.correlation_id)?;
            local_records.push(local_change);
        }
        self.engine.reconcile(
            self.actor,
            self.request,
            self.audit,
            &ReconciliationDelivery {
                delivery_id: batch.delivery_id,
                idempotency_key: batch.idempotency_key,
                checkpoint: batch.checkpoint,
                received_at: now(),
                snapshot: None,
                changes: local_records,
                command_results: Vec::new(),
            },
        )?;
        Ok(())
    }

    fn exchange(&mut self, message: SyncMessage) -> Result<SyncMessage, QuotationDraftSyncError> {
        let protocol = self
            .transport
            .negotiated_session()
            .ok_or(QuotationDraftSyncError::UnexpectedResponse)?
            .protocol;
        let frame = SyncTransportFrame {
            frame_id: SyncFrameId::new(Uuid::new_v4()),
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            protocol_version: protocol,
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            stream_id: SyncStreamId::new(Uuid::new_v4()),
            sequence: 0,
            end_of_stream: true,
            payload: SyncTransportPayload::Message(Box::new(message)),
        };
        self.transport.send(&frame, now())?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if Instant::now() >= deadline {
                return Err(QuotationDraftSyncError::ResponseTimeout);
            }
            match self.transport.receive(now())? {
                ReceiveOutcome::NoFrame | ReceiveOutcome::DuplicateIgnored { .. } => {}
                ReceiveOutcome::Frame(reply) => {
                    if reply.stream_id != frame.stream_id {
                        return Err(QuotationDraftSyncError::UnexpectedResponse);
                    }
                    let SyncTransportPayload::Message(message) = reply.payload else {
                        return Err(QuotationDraftSyncError::UnexpectedResponse);
                    };
                    return Ok(*message);
                }
            }
        }
    }
}

fn remap_payload(
    change: &mut eitmad_contracts::sync::ChangeRecord,
    from: &ScopeRef,
    to: &ScopeRef,
    catalog_from: &ScopeRef,
    catalog_to: &ScopeRef,
) -> Result<(), QuotationDraftSyncError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let payload = change
        .payload
        .as_mut()
        .ok_or(QuotationDraftSyncError::UnexpectedResponse)?;
    if payload.schema_id.as_str() != QUOTATION_DRAFT_SCHEMA || payload.schema_version != 1 {
        return Err(QuotationDraftSyncError::UnexpectedResponse);
    }
    let mut snapshot: eitmad_contracts::quotation_draft::QuotationDraftSnapshot =
        serde_json::from_slice(
            &STANDARD
                .decode(&payload.base64)
                .map_err(|_| QuotationDraftSyncError::UnexpectedResponse)?,
        )
        .map_err(|_| QuotationDraftSyncError::UnexpectedResponse)?;
    if snapshot.evaluation.scope != *from {
        return Err(QuotationDraftSyncError::UnexpectedResponse);
    }
    snapshot.evaluation.scope = to.clone();
    for line in &mut snapshot.intent.lines {
        remap_target(
            &mut line.configuration.selection.target,
            catalog_from,
            catalog_to,
        )?;
    }
    for line in &mut snapshot.evaluation.lines {
        remap_target(&mut line.price.snapshot.target, catalog_from, catalog_to)?;
    }
    payload.base64 = STANDARD.encode(
        serde_json::to_vec(&snapshot).map_err(|_| QuotationDraftSyncError::UnexpectedResponse)?,
    );
    Ok(())
}

fn catalog_scope(actor: &AuthorizationContext) -> ScopeRef {
    ScopeRef {
        kind: eitmad_contracts::identity::ScopeKind::parse("organization").expect("static kind"),
        id: eitmad_contracts::identity::ScopeId::new(actor.tenant_id.value()),
    }
}
fn remap_target(
    target: &mut eitmad_contracts::pricing::PriceTarget,
    from: &ScopeRef,
    to: &ScopeRef,
) -> Result<(), QuotationDraftSyncError> {
    use eitmad_contracts::pricing::PriceTarget;
    if target.scope() != from {
        return Err(QuotationDraftSyncError::UnexpectedResponse);
    }
    match target {
        PriceTarget::Product(v) => v.scope = to.clone(),
        PriceTarget::Furniture(v) => v.scope = to.clone(),
    }
    Ok(())
}
