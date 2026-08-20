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
pub struct WanEndpoint {
    pub server: String,
    pub relay: Option<String>,
}

pub struct WanAdapter<N> {
    endpoint: WanEndpoint,
    core: TransportCore<N>,
}

impl<N: ConnectionDriver> WanAdapter<N> {
    /// Creates a WAN adapter that requires account/device authentication and encryption.
    ///
    /// # Errors
    ///
    /// Returns an authentication failure when account/device credentials are absent.
    pub fn new(
        endpoint: WanEndpoint,
        driver: N,
        local_hello: PeerHello,
        authentication: TransportAuthentication,
        retry_policy: RetryPolicy,
    ) -> Result<Self, TransportFailure> {
        if !matches!(
            authentication,
            TransportAuthentication::AccountDevice { .. }
        ) {
            return Err(TransportFailure::new(
                TransportFailureKind::AuthenticationFailed,
                FailurePhase::Authentication,
                RetryAdvice::Never,
            ));
        }
        Ok(Self {
            endpoint,
            core: TransportCore::new(driver, local_hello, authentication, false, retry_policy),
        })
    }

    pub fn driver_mut(&mut self) -> &mut N {
        self.core.driver_mut()
    }
}

impl<N: ConnectionDriver> SyncTransport for WanAdapter<N> {
    fn kind(&self) -> TransportKind {
        TransportKind::Wan
    }

    fn connect(&mut self, now: UnixMillis) -> Result<NegotiatedSession, TransportFailure> {
        let server_target = ConnectionTarget::WanServer {
            endpoint: self.endpoint.server.clone(),
        };
        let direct = self.core.connect(&server_target, now);
        match direct {
            Ok(session) => Ok(session),
            Err(mut server_failure) => {
                if !is_connectivity_failure(&server_failure.kind) {
                    return Err(server_failure);
                }
                server_failure.kind = TransportFailureKind::ServerUnavailable;
                let Some(relay) = self.endpoint.relay.clone() else {
                    self.core.replace_last_failure(server_failure.clone());
                    return Err(server_failure);
                };
                self.core.allow_immediate_retry();
                let relay_target = ConnectionTarget::WanRelay { endpoint: relay };
                match self.core.connect(&relay_target, now) {
                    Ok(session) => {
                        self.core.mark_degraded(server_failure);
                        Ok(session)
                    }
                    Err(mut relay_failure) => {
                        relay_failure.kind = TransportFailureKind::RelayUnavailable;
                        relay_failure.phase = FailurePhase::Connect;
                        self.core.replace_last_failure(relay_failure.clone());
                        Err(relay_failure)
                    }
                }
            }
        }
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

fn is_connectivity_failure(kind: &TransportFailureKind) -> bool {
    matches!(
        kind,
        TransportFailureKind::Disconnected
            | TransportFailureKind::PartialNetwork
            | TransportFailureKind::ServerUnavailable
            | TransportFailureKind::DriverUnavailable
    )
}
