use eitmad_contracts::{
    sync_transport::{SyncCancellationReason, SyncStreamId, SyncTransportFrame},
    transport::{CorrelationId, UnixMillis},
    versioning::{NegotiatedSession, PeerHello},
};

use super::{
    ConnectionDriver, ConnectionHealth, ConnectionTarget, FailurePhase, ReceiveOutcome,
    RetryAdvice, RetryPolicy, SyncTransport, TransportAuthentication, TransportCore,
    TransportFailure, TransportFailureKind, TransportKind,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanPeer {
    pub peer_id: String,
    pub endpoint: String,
    /// Lower values are preferred. Peer IDs break equal-priority ties.
    pub priority: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanDiscoveryReport {
    pub peers: Vec<LanPeer>,
    pub partial_failures: u32,
}

pub trait LanDiscovery {
    /// Discovers reachable LAN peers and reports partial interface failures.
    ///
    /// # Errors
    ///
    /// Returns a structured discovery failure when no discovery source can run.
    fn discover(&mut self) -> Result<LanDiscoveryReport, TransportFailure>;
}

pub struct LanAdapter<D, N> {
    discovery: D,
    core: TransportCore<N>,
}

impl<D: LanDiscovery, N: ConnectionDriver> LanAdapter<D, N> {
    /// Creates a LAN adapter that requires device authentication and encryption.
    ///
    /// # Errors
    ///
    /// Returns an authentication failure when the caller does not supply device credentials.
    pub fn new(
        discovery: D,
        driver: N,
        local_hello: PeerHello,
        authentication: TransportAuthentication,
        retry_policy: RetryPolicy,
    ) -> Result<Self, TransportFailure> {
        if !matches!(authentication, TransportAuthentication::Device { .. }) {
            return Err(TransportFailure::new(
                TransportFailureKind::AuthenticationFailed,
                FailurePhase::Authentication,
                RetryAdvice::Never,
            ));
        }
        Ok(Self {
            discovery,
            core: TransportCore::new(driver, local_hello, authentication, false, retry_policy),
        })
    }

    pub fn driver_mut(&mut self) -> &mut N {
        self.core.driver_mut()
    }
}

impl<D: LanDiscovery, N: ConnectionDriver> SyncTransport for LanAdapter<D, N> {
    fn kind(&self) -> TransportKind {
        TransportKind::Lan
    }

    fn connect(&mut self, now: UnixMillis) -> Result<NegotiatedSession, TransportFailure> {
        let report = match self.discovery.discover() {
            Ok(report) => report,
            Err(failure) => return Err(self.core.record_external_failure(failure, now)),
        };
        let Some(peer) = report.peers.iter().min_by(|left, right| {
            (left.priority, &left.peer_id).cmp(&(right.priority, &right.peer_id))
        }) else {
            let kind = if report.partial_failures == 0 {
                TransportFailureKind::NoLanPeer
            } else {
                TransportFailureKind::PartialNetwork
            };
            let failure = TransportFailure::new(
                kind,
                FailurePhase::Discovery,
                RetryAdvice::After { delay_ms: 1_000 },
            );
            return Err(self.core.record_external_failure(failure, now));
        };
        let target = ConnectionTarget::Lan {
            peer_id: peer.peer_id.clone(),
            endpoint: peer.endpoint.clone(),
        };
        let session = self.core.connect(&target, now)?;
        if report.partial_failures > 0 {
            self.core.mark_degraded(TransportFailure::new(
                TransportFailureKind::PartialNetwork,
                FailurePhase::Discovery,
                RetryAdvice::After { delay_ms: 1_000 },
            ));
        }
        Ok(session)
    }

    fn disconnect(&mut self, _now: UnixMillis) {
        self.core.disconnect();
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
