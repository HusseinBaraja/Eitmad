//! One bounded customer outbox and incremental download cycle.

use std::time::{Duration, Instant};

use eitmad_authorization::{BoundaryAuditContext, now};
use eitmad_contracts::{
    authorization::AuthorizationRequest,
    customer::{CustomerId, CustomerSyncState},
    identity::AuthorizationContext,
    sync::{
        BatchAcknowledgement, ChangeBatch, LocalChangeDisposition, LocalChangeSubmission,
        PullRequest, ReconciliationDelivery, SyncMessage, SyncMode,
    },
    sync_transport::{SyncFrameId, SyncStreamId, SyncTransportFrame, SyncTransportPayload},
    transport::{CorrelationId, IdempotencyKey, SchemaId},
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
    pub request: &'a AuthorizationRequest,
    pub audit: &'a BoundaryAuditContext,
}

impl<T: SyncTransport> CustomerSyncCycle<'_, T> {
    /// Delivers up to 50 local changes and downloads all available 50-record pages.
    ///
    /// # Errors
    ///
    /// Preserves local work and the last durable checkpoint on transport,
    /// storage, authorization, or projection failure.
    pub fn run(&mut self) -> Result<(), CustomerSyncError> {
        let pending = self.customers.sync_batch(&self.actor.scope, 50)?;
        for change in &pending {
            self.engine.stage_committed_local_change(
                self.actor,
                self.request,
                self.audit,
                change.clone(),
            )?;
        }
        let result = self.run_connected(&pending);
        if result.is_err() {
            self.transport.disconnect(now());
            let _ = self.engine.disconnect(self.actor, self.request, self.audit);
        }
        result
    }

    fn run_connected(
        &mut self,
        pending: &[eitmad_contracts::sync::ChangeRecord],
    ) -> Result<(), CustomerSyncError> {
        let session = self.transport.connect(now())?;
        let schema = SchemaId::parse(CUSTOMER_SCHEMA_ID).expect("static customer schema");
        self.engine.connect_negotiated(
            self.actor,
            self.request,
            self.audit,
            &session,
            SyncMode::LocalFirst,
            &schema,
            1,
        )?;
        for change in pending {
            let response = self.exchange(SyncMessage::SubmitLocal(LocalChangeSubmission {
                change: change.clone(),
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
                    if authoritative_change.change_id != change.change_id {
                        return Err(CustomerSyncError::UnexpectedResponse);
                    }
                    None
                }
                LocalChangeDisposition::Conflicted { .. } => Some(CustomerSyncState::Conflicted),
                LocalChangeDisposition::Rejected { .. } => Some(CustomerSyncState::Rejected),
            };
            if let Some(state) = exception {
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
                accepted_records: batch.records.len() as u32,
            };
            match self.exchange(SyncMessage::Acknowledge(acknowledgement.clone()))? {
                SyncMessage::Acknowledge(reply) if reply == acknowledgement => {}
                _ => return Err(CustomerSyncError::UnexpectedResponse),
            }
            if !batch.has_more {
                break;
            }
        }
        Ok(())
    }

    fn apply_batch(&mut self, batch: &ChangeBatch) -> Result<(), CustomerSyncError> {
        // Projection precedes checkpoint persistence. A crash can replay a page,
        // but it cannot skip a customer projection after the checkpoint advances.
        for change in &batch.records {
            self.customers
                .project_confirmed(self.actor, change, self.audit.correlation_id)?;
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
                changes: batch.records.clone(),
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
            payload: SyncTransportPayload::Message(message),
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
                    return Ok(message);
                }
            }
        }
    }
}
