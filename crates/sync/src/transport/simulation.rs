use std::collections::VecDeque;

use eitmad_contracts::{
    sync_transport::{SyncCancellationReason, SyncStreamId, SyncTransportFrame},
    transport::{CorrelationId, UnixMillis},
    versioning::{NegotiatedSession, PeerHello},
};

use super::{
    AuthenticationIdentity, ConnectionDriver, ConnectionHealth, ConnectionTarget,
    EstablishedConnection, FailurePhase, ReceiveOutcome, RetryAdvice, RetryPolicy, SessionSecurity,
    SyncTransport, TransportAuthentication, TransportCore, TransportFailure, TransportFailureKind,
    TransportKind,
};

pub struct SimulatedTransport {
    core: TransportCore<SimulatedDriver>,
}

impl SimulatedTransport {
    #[must_use]
    pub fn new(local_hello: PeerHello, remote_hello: PeerHello, retry_policy: RetryPolicy) -> Self {
        Self {
            core: TransportCore::new(
                SimulatedDriver::new(remote_hello),
                local_hello,
                TransportAuthentication::Simulation,
                false,
                retry_policy,
            ),
        }
    }

    pub fn inject_incoming(&mut self, frame: SyncTransportFrame) {
        self.core.driver_mut().incoming.push_back(frame);
    }

    #[must_use]
    pub fn take_outgoing(&mut self) -> Vec<SyncTransportFrame> {
        self.core.driver_mut().outgoing.drain(..).collect()
    }

    pub fn fail_next_connect(&mut self, failure: TransportFailure) {
        self.core.driver_mut().connect_failures.push_back(failure);
    }

    pub fn fail_next_send(&mut self, failure: TransportFailure) {
        self.core.driver_mut().send_failures.push_back(failure);
    }

    pub fn fail_next_receive(&mut self, failure: TransportFailure) {
        self.core.driver_mut().receive_failures.push_back(failure);
    }

    pub fn authenticate_next_connection_as(&mut self, identity: AuthenticationIdentity) {
        self.core.driver_mut().authenticated_as = identity;
    }

    pub fn set_remote_hello(&mut self, hello: PeerHello) {
        self.core.driver_mut().remote_hello = hello;
    }
}

impl SyncTransport for SimulatedTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Simulation
    }

    fn connect(&mut self, now: UnixMillis) -> Result<NegotiatedSession, TransportFailure> {
        self.core.connect(&ConnectionTarget::Simulation, now)
    }

    fn disconnect(&mut self, now: UnixMillis) {
        self.core.disconnect(now);
    }

    fn send(
        &mut self,
        frame: &SyncTransportFrame,
        now: UnixMillis,
    ) -> Result<(), TransportFailure> {
        self.core.send(frame, now)
    }

    fn receive(&mut self, now: UnixMillis) -> Result<ReceiveOutcome, TransportFailure> {
        self.core.receive(now)
    }

    fn cancel(
        &mut self,
        stream_id: SyncStreamId,
        correlation_id: CorrelationId,
        reason: SyncCancellationReason,
        now: UnixMillis,
    ) -> Result<(), TransportFailure> {
        self.core.cancel(stream_id, correlation_id, reason, now)
    }

    fn health(&self) -> &ConnectionHealth {
        self.core.health()
    }

    fn negotiated_session(&self) -> Option<&NegotiatedSession> {
        self.core.negotiated_session()
    }
}

struct SimulatedDriver {
    remote_hello: PeerHello,
    authenticated_as: AuthenticationIdentity,
    connected: bool,
    incoming: VecDeque<SyncTransportFrame>,
    outgoing: VecDeque<SyncTransportFrame>,
    connect_failures: VecDeque<TransportFailure>,
    send_failures: VecDeque<TransportFailure>,
    receive_failures: VecDeque<TransportFailure>,
}

impl SimulatedDriver {
    fn new(remote_hello: PeerHello) -> Self {
        Self {
            remote_hello,
            authenticated_as: AuthenticationIdentity::Simulation,
            connected: false,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            connect_failures: VecDeque::new(),
            send_failures: VecDeque::new(),
            receive_failures: VecDeque::new(),
        }
    }

    fn disconnected(phase: FailurePhase) -> TransportFailure {
        TransportFailure::new(
            TransportFailureKind::Disconnected,
            phase,
            RetryAdvice::Immediate,
        )
    }
}

impl ConnectionDriver for SimulatedDriver {
    fn establish(
        &mut self,
        target: &ConnectionTarget,
        authentication: &TransportAuthentication,
    ) -> Result<EstablishedConnection, TransportFailure> {
        if target != &ConnectionTarget::Simulation
            || authentication != &TransportAuthentication::Simulation
        {
            return Err(TransportFailure::new(
                TransportFailureKind::AuthenticationFailed,
                FailurePhase::Authentication,
                RetryAdvice::Never,
            ));
        }
        if let Some(failure) = self.connect_failures.pop_front() {
            return Err(failure);
        }
        self.connected = true;
        Ok(EstablishedConnection {
            remote_hello: self.remote_hello.clone(),
            authenticated_as: self.authenticated_as.clone(),
            security: SessionSecurity::isolated_simulation(),
            round_trip_ms: Some(0),
        })
    }

    fn send(&mut self, frame: &SyncTransportFrame) -> Result<(), TransportFailure> {
        if !self.connected {
            return Err(Self::disconnected(FailurePhase::Send));
        }
        if let Some(failure) = self.send_failures.pop_front() {
            return Err(failure);
        }
        self.outgoing.push_back(frame.clone());
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<SyncTransportFrame>, TransportFailure> {
        if !self.connected {
            return Err(Self::disconnected(FailurePhase::Receive));
        }
        if let Some(failure) = self.receive_failures.pop_front() {
            return Err(failure);
        }
        Ok(self.incoming.pop_front())
    }

    fn close(&mut self) {
        self.connected = false;
    }
}
