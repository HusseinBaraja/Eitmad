use std::time::{Duration, Instant};

use eitmad_authorization::{BoundaryAuditContext, now};
use eitmad_contracts::{
    authorization::AuthorizationRequest,
    customer::{CustomerId, CustomerSyncState},
    identity::{AuthorizationContext, ScopeRef},
    sync::{
        BatchAcknowledgement, ChangeBatch, LocalChangeDisposition, LocalChangeSubmission,
        PullRequest, ReconciliationDelivery, SyncMessage, SyncMode,
    },
    sync_transport::{SyncFrameId, SyncStreamId, SyncTransportFrame, SyncTransportPayload},
    transport::{CorrelationId, IdempotencyKey, SchemaId},
    versioning::NegotiatedSchema,
};
use eitmad_sync::{ReceiveOutcome, SyncEngine, SyncEngineError, SyncTransport, TransportFailure};
use uuid::Uuid;

use crate::{CUSTOMER_SCHEMA_ID, CustomerError, CustomerService};

#[derive(Debug)]
pub enum CustomerSyncError {
    Customer(CustomerError),
    Engine(SyncEngineError),
    Transport(TransportFailure),
    UnexpectedResponse,
    ResponseTimeout,
}

impl From<CustomerError> for CustomerSyncError {
    fn from(value: CustomerError) -> Self {
        Self::Customer(value)
    }
}

impl From<SyncEngineError> for CustomerSyncError {
    fn from(value: SyncEngineError) -> Self {
        Self::Engine(value)
    }
}

impl From<TransportFailure> for CustomerSyncError {
    fn from(value: TransportFailure) -> Self {
        Self::Transport(value)
    }
}

/// Runs on a Rust worker thread. The customer outbox is the submission source;
/// the shared engine owns replay, conflicts, and the download checkpoint.
pub struct CustomerSyncCycle<'a, T: SyncTransport> {
    pub customers: &'a CustomerService,
    pub engine: &'a mut SyncEngine,
    pub transport: &'a mut T,
    pub actor: &'a AuthorizationContext,
    pub server_scope: &'a ScopeRef,
    pub request: &'a AuthorizationRequest,
    pub audit: &'a BoundaryAuditContext,
}

impl<T: SyncTransport> CustomerSyncCycle<'_, T> {
    /// Delivers up to 50 local changes and downloads all available 50-record pages.
    /// Returns the number of submitted changes confirmed as applied or replayed.
    /// Rejected and conflicted changes remain in the outbox and do not count as progress.
    ///
    /// # Errors
    ///
    /// Preserves local work and the last durable checkpoint on transport,
    /// storage, authorization, or projection failure.
    pub fn run(&mut self) -> Result<usize, CustomerSyncError> {
        let pending = self.customers.sync_batch_for_author(self.actor)?;
        for change in &pending {
            self.engine.stage_committed_local_change(
                self.actor,
                self.request,
                self.audit,
                change.clone(),
            )?;
        }
        self.finish_cycle(&pending)
    }

    /// Downloads all available pages without loading, staging, or submitting local edits.
    /// Projection checks current customer read authority for every record.
    ///
    /// # Errors
    /// Preserves pending edits, conflicts, and the last durable checkpoint on failure.
    pub fn download(&mut self) -> Result<(), CustomerSyncError> {
        self.customers.authorize_read(self.actor)?;
        self.finish_cycle(&[]).map(|_| ())
    }

    fn finish_cycle(
        &mut self,
        pending: &[eitmad_contracts::sync::ChangeRecord],
    ) -> Result<usize, CustomerSyncError> {
        let result = self.run_connected(pending);
        if result.is_err() {
            self.transport.disconnect(now());
            let _ = self.engine.disconnect(self.actor, self.request, self.audit);
        }
        result
    }

    fn run_connected(
        &mut self,
        pending: &[eitmad_contracts::sync::ChangeRecord],
    ) -> Result<usize, CustomerSyncError> {
        let session = self.transport.connect(now())?;
        let schema = NegotiatedSchema {
            schema_id: SchemaId::parse(CUSTOMER_SCHEMA_ID).expect("static customer schema"),
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
        let mut confirmed = 0;
        for change in pending {
            let mut server_change = change.clone();
            server_change.scope = self.server_scope.clone();
            let response = self.exchange(SyncMessage::SubmitLocal(LocalChangeSubmission {
                change: server_change,
            }))?;
            let SyncMessage::LocalResult(result) = response else {
                return Err(CustomerSyncError::UnexpectedResponse);
            };
            if result.submitted_change_id != change.change_id {
                return Err(CustomerSyncError::UnexpectedResponse);
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
                    {
                        return Err(CustomerSyncError::UnexpectedResponse);
                    }
                    confirmed += 1;
                    None
                }
                LocalChangeDisposition::Conflicted { .. } => Some(CustomerSyncState::Conflicted),
                LocalChangeDisposition::Rejected { .. } => Some(CustomerSyncState::Rejected),
            };
            if let Some(state) = exception {
                // Dequeue first: if the customer status write fails, the durable
                // outbox retries this submission on the next cycle.
                self.engine.hold_committed_local_change(
                    self.actor,
                    self.request,
                    self.audit,
                    change.change_id,
                )?;
                self.customers.mark_sync_exception(
                    self.actor,
                    CustomerId::new(change.record_id.value()),
                    state,
                    self.audit.correlation_id,
                    now(),
                )?;
            }
        }
        loop {
            let after = self.engine.metadata().checkpoint;
            let response = self.exchange(SyncMessage::Pull(PullRequest {
                after,
                maximum_records: 50,
            }))?;
            let SyncMessage::Changes(batch) = response else {
                return Err(CustomerSyncError::UnexpectedResponse);
            };
            if batch.from_checkpoint != after
                || batch.records.len() > 50
                || (batch.has_more && batch.records.is_empty())
            {
                return Err(CustomerSyncError::UnexpectedResponse);
            }
            self.apply_batch(&batch)?;
            let acknowledgement = BatchAcknowledgement {
                delivery_id: batch.delivery_id,
                checkpoint: batch.checkpoint,
                accepted_records: u32::try_from(batch.records.len())
                    .map_err(|_| CustomerSyncError::UnexpectedResponse)?,
            };
            match self.exchange(SyncMessage::Acknowledge(acknowledgement.clone()))? {
                SyncMessage::Acknowledge(reply) if reply == acknowledgement => {}
                _ => return Err(CustomerSyncError::UnexpectedResponse),
            }
            if !batch.has_more {
                break;
            }
        }
        Ok(confirmed)
    }

    fn apply_batch(&mut self, batch: &ChangeBatch) -> Result<(), CustomerSyncError> {
        self.customers.authorize_read(self.actor)?;
        // Projection precedes checkpoint persistence. A crash can replay a page,
        // but it cannot skip a customer projection after the checkpoint advances.
        let mut local_records = Vec::with_capacity(batch.records.len());
        for change in &batch.records {
            if change.scope != *self.server_scope {
                return Err(CustomerSyncError::UnexpectedResponse);
            }
            let mut local_change = change.clone();
            local_change.scope = self.actor.scope.clone();
            self.customers.project_confirmed(
                self.actor,
                &local_change,
                self.audit.correlation_id,
            )?;
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

    fn exchange(&mut self, message: SyncMessage) -> Result<SyncMessage, CustomerSyncError> {
        let protocol = self
            .transport
            .negotiated_session()
            .ok_or(CustomerSyncError::UnexpectedResponse)?
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
                return Err(CustomerSyncError::ResponseTimeout);
            }
            match self.transport.receive(now())? {
                ReceiveOutcome::NoFrame | ReceiveOutcome::DuplicateIgnored { .. } => {}
                ReceiveOutcome::Frame(reply) => {
                    if reply.stream_id != frame.stream_id {
                        return Err(CustomerSyncError::UnexpectedResponse);
                    }
                    let SyncTransportPayload::Message(message) = reply.payload else {
                        return Err(CustomerSyncError::UnexpectedResponse);
                    };
                    return Ok(*message);
                }
            }
        }
    }
}
