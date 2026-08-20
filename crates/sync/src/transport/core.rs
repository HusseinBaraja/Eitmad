use std::collections::{BTreeMap, BTreeSet, VecDeque};

use eitmad_contracts::{
    sync_transport::{
        SyncCancellation, SyncCancellationReason, SyncFrameId, SyncStreamId, SyncTransportFrame,
        SyncTransportPayload,
    },
    transport::{CapabilityId, CorrelationId, IdempotencyKey, UnixMillis},
    versioning::{
        NegotiatedSession, NegotiationOutcome, NegotiationRejection, PeerHello, ProtocolVersion,
        negotiate,
    },
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    ConnectionDriver, ConnectionHealth, ConnectionTarget, EstablishedConnection, FailurePhase,
    HealthStatus, ReceiveOutcome, RetryAdvice, RetryPolicy, SessionSecurity,
    TransportAuthentication, TransportFailure, TransportFailureKind,
};

pub(crate) struct TransportCore<D> {
    driver: D,
    local_hello: PeerHello,
    authentication: TransportAuthentication,
    simulation_isolation_required: bool,
    retry_policy: RetryPolicy,
    health: ConnectionHealth,
    negotiated: Option<NegotiatedSession>,
    received_frames: BTreeMap<SyncFrameId, [u8; 32]>,
    received_keys: BTreeMap<IdempotencyKey, [u8; 32]>,
    received_order: VecDeque<(SyncFrameId, IdempotencyKey)>,
    incoming_sequences: BTreeMap<SyncStreamId, u64>,
    outgoing_sequences: BTreeMap<SyncStreamId, u64>,
    cancelled_streams: BTreeSet<SyncStreamId>,
    completed_incoming_streams: BTreeSet<SyncStreamId>,
    completed_outgoing_streams: BTreeSet<SyncStreamId>,
    terminal_stream_order: VecDeque<SyncStreamId>,
}

const MAX_RETAINED_FRAMES: usize = 4_096;
const MAX_RETAINED_STREAMS: usize = MAX_RETAINED_FRAMES * 2;

impl<D: ConnectionDriver> TransportCore<D> {
    pub(crate) fn new(
        driver: D,
        mut local_hello: PeerHello,
        authentication: TransportAuthentication,
        simulation_isolation_required: bool,
        retry_policy: RetryPolicy,
    ) -> Self {
        require_sync_capability(&mut local_hello);
        Self {
            driver,
            local_hello,
            authentication,
            simulation_isolation_required,
            retry_policy,
            health: ConnectionHealth {
                status: HealthStatus::Offline,
                target: None,
                last_success_at: None,
                last_failure: None,
                consecutive_failures: 0,
                next_retry_at: None,
                round_trip_ms: None,
            },
            negotiated: None,
            received_frames: BTreeMap::new(),
            received_keys: BTreeMap::new(),
            received_order: VecDeque::new(),
            incoming_sequences: BTreeMap::new(),
            outgoing_sequences: BTreeMap::new(),
            cancelled_streams: BTreeSet::new(),
            completed_incoming_streams: BTreeSet::new(),
            completed_outgoing_streams: BTreeSet::new(),
            terminal_stream_order: VecDeque::new(),
        }
    }

    pub(crate) fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }

    pub(crate) fn connect(
        &mut self,
        target: &ConnectionTarget,
        now: UnixMillis,
    ) -> Result<NegotiatedSession, TransportFailure> {
        if self
            .health
            .next_retry_at
            .is_some_and(|retry_at| now < retry_at)
        {
            return Err(TransportFailure::new(
                TransportFailureKind::RetryNotReady,
                FailurePhase::Connect,
                self.current_retry_advice(now),
            ));
        }
        self.health.status = HealthStatus::Connecting;
        self.health.target = Some(target.clone());
        let established = match self.driver.establish(target, &self.authentication) {
            Ok(established) => established,
            Err(failure) => return Err(self.record_failure(failure, now)),
        };
        if let Err(failure) = self.validate_established(&established) {
            self.driver.close();
            return Err(self.record_failure(failure, now));
        }
        let negotiated = match negotiate(&self.local_hello, &established.remote_hello) {
            NegotiationOutcome::Accepted(session) => session,
            NegotiationOutcome::Rejected(rejection) => {
                self.driver.close();
                let failure = negotiation_failure(&rejection);
                return Err(self.record_failure(failure, now));
            }
        };
        self.negotiated = Some(negotiated.clone());
        self.health.status = HealthStatus::Healthy;
        self.health.last_success_at = Some(now);
        self.health.last_failure = None;
        self.health.consecutive_failures = 0;
        self.health.next_retry_at = None;
        self.health.round_trip_ms = established.round_trip_ms;
        Ok(negotiated)
    }

    pub(crate) fn disconnect(&mut self) {
        self.driver.close();
        self.negotiated = None;
        self.health.status = HealthStatus::Offline;
        self.health.next_retry_at = None;
    }

    pub(crate) fn send(
        &mut self,
        frame: &SyncTransportFrame,
        now: UnixMillis,
    ) -> Result<(), TransportFailure> {
        let negotiated = self.require_connection(FailurePhase::Send)?;
        validate_protocol(
            frame.protocol_version,
            negotiated.protocol,
            frame.correlation_id,
        )?;
        if self.cancelled_streams.contains(&frame.stream_id) {
            return Err(TransportFailure::new(
                TransportFailureKind::Cancelled,
                FailurePhase::Send,
                RetryAdvice::Never,
            )
            .with_correlation(frame.correlation_id));
        }
        if self.completed_outgoing_streams.contains(&frame.stream_id) {
            return Err(TransportFailure::new(
                TransportFailureKind::StreamOutOfOrder,
                FailurePhase::Send,
                RetryAdvice::Never,
            )
            .with_correlation(frame.correlation_id));
        }
        let expected = self
            .outgoing_sequences
            .get(&frame.stream_id)
            .map_or(0, |sequence| sequence.saturating_add(1));
        if frame.sequence != expected {
            return Err(TransportFailure::new(
                TransportFailureKind::StreamOutOfOrder,
                FailurePhase::Send,
                RetryAdvice::Never,
            )
            .with_correlation(frame.correlation_id));
        }
        if let Err(failure) = self.driver.send(frame) {
            return Err(self.record_failure(failure, now));
        }
        self.outgoing_sequences
            .insert(frame.stream_id, frame.sequence);
        if frame.end_of_stream {
            self.completed_outgoing_streams.insert(frame.stream_id);
            self.retain_terminal_stream(frame.stream_id);
        }
        self.health.last_success_at = Some(now);
        Ok(())
    }

    pub(crate) fn receive(&mut self, now: UnixMillis) -> Result<ReceiveOutcome, TransportFailure> {
        let negotiated = self.require_connection(FailurePhase::Receive)?.clone();
        let frame = match self.driver.receive() {
            Ok(Some(frame)) => frame,
            Ok(None) => return Ok(ReceiveOutcome::NoFrame),
            Err(failure) => return Err(self.record_failure(failure, now)),
        };
        validate_protocol(
            frame.protocol_version,
            negotiated.protocol,
            frame.correlation_id,
        )?;
        let fingerprint = frame_fingerprint(&frame)?;
        let retained = self
            .received_frames
            .get(&frame.frame_id)
            .or_else(|| self.received_keys.get(&frame.idempotency_key));
        if let Some(retained) = retained {
            if retained == &fingerprint {
                return Ok(ReceiveOutcome::DuplicateIgnored {
                    frame_id: frame.frame_id,
                });
            }
            return Err(TransportFailure::new(
                TransportFailureKind::DuplicateConflict,
                FailurePhase::Receive,
                RetryAdvice::Never,
            )
            .with_correlation(frame.correlation_id));
        }
        if self.completed_incoming_streams.contains(&frame.stream_id) {
            return Err(TransportFailure::new(
                TransportFailureKind::StreamOutOfOrder,
                FailurePhase::Receive,
                RetryAdvice::Never,
            )
            .with_correlation(frame.correlation_id));
        }
        let expected = self
            .incoming_sequences
            .get(&frame.stream_id)
            .map_or(0, |sequence| sequence.saturating_add(1));
        if frame.sequence != expected {
            return Err(TransportFailure::new(
                TransportFailureKind::StreamOutOfOrder,
                FailurePhase::Receive,
                RetryAdvice::Never,
            )
            .with_correlation(frame.correlation_id));
        }
        let cancellation = match &frame.payload {
            SyncTransportPayload::Cancel(cancellation) => {
                if cancellation.stream_id != frame.stream_id {
                    return Err(TransportFailure::new(
                        TransportFailureKind::StreamOutOfOrder,
                        FailurePhase::Cancellation,
                        RetryAdvice::Never,
                    )
                    .with_correlation(frame.correlation_id));
                }
                Some(cancellation.stream_id)
            }
            _ => None,
        };
        self.received_frames.insert(frame.frame_id, fingerprint);
        self.received_keys
            .insert(frame.idempotency_key, fingerprint);
        self.received_order
            .push_back((frame.frame_id, frame.idempotency_key));
        while self.received_order.len() > MAX_RETAINED_FRAMES {
            if let Some((frame_id, idempotency_key)) = self.received_order.pop_front() {
                self.received_frames.remove(&frame_id);
                self.received_keys.remove(&idempotency_key);
            }
        }
        self.incoming_sequences
            .insert(frame.stream_id, frame.sequence);
        if let Some(stream_id) = cancellation {
            self.cancelled_streams.insert(stream_id);
        }
        if frame.end_of_stream {
            self.completed_incoming_streams.insert(frame.stream_id);
        }
        if cancellation.is_some() || frame.end_of_stream {
            self.retain_terminal_stream(frame.stream_id);
        }
        self.health.last_success_at = Some(now);
        Ok(ReceiveOutcome::Frame(Box::new(frame)))
    }

    pub(crate) fn cancel(
        &mut self,
        stream_id: SyncStreamId,
        correlation_id: CorrelationId,
        reason: SyncCancellationReason,
        now: UnixMillis,
    ) -> Result<(), TransportFailure> {
        let protocol_version = self
            .require_connection(FailurePhase::Cancellation)?
            .protocol;
        if self.cancelled_streams.contains(&stream_id)
            || self.completed_outgoing_streams.contains(&stream_id)
        {
            return Ok(());
        }
        let next_sequence = self
            .outgoing_sequences
            .get(&stream_id)
            .map_or(0, |sequence| sequence.saturating_add(1));
        let frame = SyncTransportFrame {
            frame_id: SyncFrameId::new(Uuid::new_v4()),
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            protocol_version,
            correlation_id,
            stream_id,
            sequence: next_sequence,
            end_of_stream: true,
            payload: SyncTransportPayload::Cancel(SyncCancellation {
                stream_id,
                last_accepted_sequence: self.incoming_sequences.get(&stream_id).copied(),
                reason,
            }),
        };
        if let Err(failure) = self.driver.send(&frame) {
            return Err(self.record_failure(failure, now));
        }
        self.outgoing_sequences.insert(stream_id, next_sequence);
        self.cancelled_streams.insert(stream_id);
        self.completed_outgoing_streams.insert(stream_id);
        self.retain_terminal_stream(stream_id);
        self.health.last_success_at = Some(now);
        Ok(())
    }

    fn retain_terminal_stream(&mut self, stream_id: SyncStreamId) {
        if self.terminal_stream_order.contains(&stream_id) {
            return;
        }
        self.terminal_stream_order.push_back(stream_id);
        while self.terminal_stream_order.len() > MAX_RETAINED_STREAMS {
            if let Some(evicted) = self.terminal_stream_order.pop_front() {
                self.incoming_sequences.remove(&evicted);
                self.outgoing_sequences.remove(&evicted);
                self.cancelled_streams.remove(&evicted);
                self.completed_incoming_streams.remove(&evicted);
                self.completed_outgoing_streams.remove(&evicted);
            }
        }
    }

    pub(crate) const fn health(&self) -> &ConnectionHealth {
        &self.health
    }

    pub(crate) fn negotiated_session(&self) -> Option<&NegotiatedSession> {
        self.negotiated.as_ref()
    }

    pub(crate) fn mark_degraded(&mut self, failure: TransportFailure) {
        self.health.status = HealthStatus::Degraded;
        self.health.last_failure = Some(failure);
    }

    pub(crate) fn record_external_failure(
        &mut self,
        failure: TransportFailure,
        now: UnixMillis,
    ) -> TransportFailure {
        self.record_failure(failure, now)
    }

    pub(crate) fn allow_immediate_retry(&mut self) {
        self.health.next_retry_at = None;
    }

    pub(crate) fn replace_last_failure(&mut self, failure: TransportFailure) {
        self.health.last_failure = Some(failure);
    }

    fn validate_established(
        &self,
        established: &EstablishedConnection,
    ) -> Result<(), TransportFailure> {
        if established.authenticated_as != self.authentication.identity() {
            return Err(TransportFailure::new(
                TransportFailureKind::AuthenticationFailed,
                FailurePhase::Authentication,
                RetryAdvice::Never,
            ));
        }
        if !self.simulation_isolation_required && !secure_enough(&established.security) {
            return Err(TransportFailure::new(
                TransportFailureKind::EncryptionRequired,
                FailurePhase::Encryption,
                RetryAdvice::Never,
            ));
        }
        if self.simulation_isolation_required
            && established.security != SessionSecurity::IsolatedSimulation
        {
            return Err(TransportFailure::new(
                TransportFailureKind::DriverUnavailable,
                FailurePhase::Encryption,
                RetryAdvice::Never,
            ));
        }
        Ok(())
    }

    fn require_connection(
        &self,
        phase: FailurePhase,
    ) -> Result<&NegotiatedSession, TransportFailure> {
        self.negotiated.as_ref().ok_or_else(|| {
            TransportFailure::new(
                TransportFailureKind::Disconnected,
                phase,
                RetryAdvice::Immediate,
            )
        })
    }

    fn record_failure(
        &mut self,
        mut failure: TransportFailure,
        now: UnixMillis,
    ) -> TransportFailure {
        self.driver.close();
        self.negotiated = None;
        self.health.status = HealthStatus::Offline;
        self.health.consecutive_failures = self.health.consecutive_failures.saturating_add(1);
        let advice = if failure.retry == RetryAdvice::Never {
            RetryAdvice::Never
        } else {
            self.retry_advice()
        };
        failure.retry = advice;
        self.health.next_retry_at = match advice {
            RetryAdvice::After { delay_ms } => add_millis(now, delay_ms),
            RetryAdvice::Immediate => Some(now),
            RetryAdvice::Never => None,
        };
        self.health.last_failure = Some(failure.clone());
        failure
    }

    fn retry_advice(&self) -> RetryAdvice {
        if self.health.consecutive_failures >= self.retry_policy.maximum_attempts {
            return RetryAdvice::Never;
        }
        let exponent = self.health.consecutive_failures.saturating_sub(1).min(63);
        let delay = self
            .retry_policy
            .initial_delay_ms
            .saturating_mul(1_u64 << exponent)
            .min(self.retry_policy.maximum_delay_ms);
        RetryAdvice::After { delay_ms: delay }
    }

    fn current_retry_advice(&self, now: UnixMillis) -> RetryAdvice {
        let Some(retry_at) = self.health.next_retry_at else {
            return RetryAdvice::Immediate;
        };
        let remaining = retry_at.0.saturating_sub(now.0);
        RetryAdvice::After {
            delay_ms: u64::try_from(remaining).unwrap_or(0),
        }
    }
}

fn secure_enough(security: &SessionSecurity) -> bool {
    matches!(
        security,
        SessionSecurity::AuthenticatedEncryption {
            peer_authenticated: true,
            forward_secrecy: true
        }
    )
}

fn require_sync_capability(hello: &mut PeerHello) {
    let capability = CapabilityId::parse("eitmad.capability.sync.v1")
        .expect("the static sync capability identifier is valid");
    if !hello.capabilities.contains(&capability) {
        hello.capabilities.push(capability.clone());
    }
    if !hello.required_capabilities.contains(&capability) {
        hello.required_capabilities.push(capability);
    }
}

fn validate_protocol(
    actual: ProtocolVersion,
    negotiated: ProtocolVersion,
    correlation_id: CorrelationId,
) -> Result<(), TransportFailure> {
    (actual == negotiated).then_some(()).ok_or_else(|| {
        TransportFailure::new(
            TransportFailureKind::VersionMismatch,
            FailurePhase::Negotiation,
            RetryAdvice::Never,
        )
        .with_correlation(correlation_id)
    })
}

fn negotiation_failure(rejection: &NegotiationRejection) -> TransportFailure {
    let kind = match rejection {
        NegotiationRejection::NoCommonProtocol => TransportFailureKind::VersionMismatch,
        NegotiationRejection::MissingCapability { .. } => TransportFailureKind::CapabilityMismatch,
        NegotiationRejection::IncompatibleSchema { .. } => TransportFailureKind::SchemaMismatch,
    };
    TransportFailure::new(kind, FailurePhase::Negotiation, RetryAdvice::Never)
}

fn frame_fingerprint(frame: &SyncTransportFrame) -> Result<[u8; 32], TransportFailure> {
    let encoded = serde_json::to_vec(&(
        frame.protocol_version,
        frame.correlation_id,
        frame.stream_id,
        frame.sequence,
        frame.end_of_stream,
        &frame.payload,
    ))
    .map_err(|_| {
        TransportFailure::new(
            TransportFailureKind::DriverUnavailable,
            FailurePhase::Receive,
            RetryAdvice::Never,
        )
    })?;
    Ok(Sha256::digest(encoded).into())
}

fn add_millis(now: UnixMillis, delay_ms: u64) -> Option<UnixMillis> {
    let delay = i64::try_from(delay_ms).ok()?;
    now.0.checked_add(delay).map(UnixMillis)
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::{
        updates::ReleaseVersion,
        versioning::{PeerKind, SupportedProtocol},
    };

    use crate::transport::AuthenticationIdentity;

    use super::*;

    #[test]
    fn terminal_stream_retention_evicts_oldest_state() {
        let mut core = TransportCore::new(
            (),
            hello(),
            TransportAuthentication::Simulation,
            true,
            RetryPolicy::default(),
        );

        for value in 1..=u128::try_from(MAX_RETAINED_STREAMS + 1).unwrap() {
            let stream_id = SyncStreamId::new(Uuid::from_u128(value));
            core.incoming_sequences.insert(stream_id, 0);
            core.outgoing_sequences.insert(stream_id, 0);
            core.cancelled_streams.insert(stream_id);
            core.completed_incoming_streams.insert(stream_id);
            core.completed_outgoing_streams.insert(stream_id);
            core.retain_terminal_stream(stream_id);
        }

        assert_eq!(core.terminal_stream_order.len(), MAX_RETAINED_STREAMS);
        assert_eq!(core.incoming_sequences.len(), MAX_RETAINED_STREAMS);
        assert_eq!(core.outgoing_sequences.len(), MAX_RETAINED_STREAMS);
        assert_eq!(core.cancelled_streams.len(), MAX_RETAINED_STREAMS);
        assert_eq!(core.completed_incoming_streams.len(), MAX_RETAINED_STREAMS);
        assert_eq!(core.completed_outgoing_streams.len(), MAX_RETAINED_STREAMS);
        let evicted = SyncStreamId::new(Uuid::from_u128(1));
        assert!(!core.incoming_sequences.contains_key(&evicted));
        assert!(!core.cancelled_streams.contains(&evicted));
    }

    #[test]
    fn simulation_isolation_reports_driver_mismatch() {
        let core = TransportCore::new(
            (),
            hello(),
            TransportAuthentication::Simulation,
            true,
            RetryPolicy::default(),
        );
        let established = EstablishedConnection {
            remote_hello: hello(),
            authenticated_as: AuthenticationIdentity::Simulation,
            security: SessionSecurity::encrypted(),
            round_trip_ms: None,
        };

        let failure = core.validate_established(&established).unwrap_err();
        assert_eq!(failure.kind, TransportFailureKind::DriverUnavailable);
        assert_eq!(failure.phase, FailurePhase::Encryption);
    }

    fn hello() -> PeerHello {
        PeerHello {
            peer_kind: PeerKind::Engine,
            product_version: ReleaseVersion::new(semver::Version::new(1, 0, 0)),
            protocols: vec![SupportedProtocol {
                major: 1,
                minimum_minor: 0,
                maximum_minor: 3,
            }],
            capabilities: Vec::new(),
            required_capabilities: Vec::new(),
            schemas: Vec::new(),
        }
    }

    impl ConnectionDriver for () {
        fn establish(
            &mut self,
            _target: &ConnectionTarget,
            _authentication: &TransportAuthentication,
        ) -> Result<EstablishedConnection, TransportFailure> {
            unreachable!("stream-retention tests do not connect")
        }

        fn send(&mut self, _frame: &SyncTransportFrame) -> Result<(), TransportFailure> {
            Ok(())
        }

        fn receive(&mut self) -> Result<Option<SyncTransportFrame>, TransportFailure> {
            Ok(None)
        }

        fn close(&mut self) {}
    }
}
