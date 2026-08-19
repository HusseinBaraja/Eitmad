use std::collections::VecDeque;

use eitmad_contracts::{
    config::SecretReferenceId,
    identity::{AccountId, DeviceId},
    secrets::{SecretId, SecretKind},
    sync::{PullRequest, SyncMessage},
    sync_transport::{
        SyncCancellationReason, SyncFrameId, SyncStreamId, SyncTransportFrame, SyncTransportPayload,
    },
    transport::{CapabilityId, CorrelationId, IdempotencyKey, UnixMillis},
    updates::ReleaseVersion,
    versioning::{PeerHello, PeerKind, SupportedProtocol},
};
use eitmad_sync::{
    AuthenticationIdentity, ConnectionDriver, ConnectionTarget, EstablishedConnection,
    FailurePhase, HealthStatus, LanAdapter, LanDiscovery, LanDiscoveryReport, LanPeer,
    ReceiveOutcome, RetryAdvice, RetryPolicy, SessionSecurity, SimulatedTransport, SyncTransport,
    TransportAuthentication, TransportFailure, TransportFailureKind, WanAdapter, WanEndpoint,
};
use uuid::Uuid;

#[test]
fn disconnect_stops_delivery_and_reconnect_restores_it() {
    let mut transport = simulation(hello(1));
    transport.connect(UnixMillis(0)).unwrap();
    transport.disconnect(UnixMillis(1));

    let failure = transport.send(&frame(0, 1), UnixMillis(2)).unwrap_err();
    assert_eq!(failure.kind, TransportFailureKind::Disconnected);
    assert_eq!(transport.health().status, HealthStatus::Offline);

    transport.connect(UnixMillis(3)).unwrap();
    transport.send(&frame(0, 1), UnixMillis(4)).unwrap();
    assert_eq!(transport.take_outgoing(), vec![frame(0, 1)]);
    assert_eq!(transport.health().status, HealthStatus::Healthy);
}

#[test]
fn retry_backoff_gates_reconnect_until_due() {
    let mut transport = simulation(hello(1));
    transport.fail_next_connect(failure(
        TransportFailureKind::DriverUnavailable,
        FailurePhase::Connect,
    ));

    let failure = transport.connect(UnixMillis(1_000)).unwrap_err();
    assert_eq!(failure.retry, RetryAdvice::After { delay_ms: 250 });
    assert_eq!(transport.health().next_retry_at, Some(UnixMillis(1_250)));

    assert_eq!(
        transport.connect(UnixMillis(1_249)).unwrap_err().kind,
        TransportFailureKind::RetryNotReady
    );
    transport.connect(UnixMillis(1_250)).unwrap();
    assert_eq!(transport.health().consecutive_failures, 0);
}

#[test]
fn authentication_failure_is_terminal_and_redacted() {
    let mut transport = simulation(hello(1));
    transport.authenticate_next_connection_as(AuthenticationIdentity::Device(DeviceId::new(
        Uuid::from_u128(70),
    )));

    let failure = transport.connect(UnixMillis(0)).unwrap_err();
    assert_eq!(failure.kind, TransportFailureKind::AuthenticationFailed);
    assert_eq!(failure.phase, FailurePhase::Authentication);
    assert_eq!(failure.retry, RetryAdvice::Never);
    assert_eq!(failure.to_string(), "sync transport authentication failed");
}

#[test]
fn duplicate_delivery_is_ignored_but_identity_reuse_fails_closed() {
    let mut transport = simulation(hello(1));
    transport.connect(UnixMillis(0)).unwrap();
    let original = frame(0, 1);
    transport.inject_incoming(original.clone());
    transport.inject_incoming(original.clone());

    assert_eq!(
        transport.receive(UnixMillis(1)).unwrap(),
        ReceiveOutcome::Frame(Box::new(original.clone()))
    );
    assert_eq!(
        transport.receive(UnixMillis(2)).unwrap(),
        ReceiveOutcome::DuplicateIgnored {
            frame_id: original.frame_id
        }
    );

    let mut retried_with_new_frame_id = original.clone();
    retried_with_new_frame_id.frame_id = SyncFrameId::new(Uuid::from_u128(99));
    transport.inject_incoming(retried_with_new_frame_id);
    assert_eq!(
        transport.receive(UnixMillis(3)).unwrap(),
        ReceiveOutcome::DuplicateIgnored {
            frame_id: SyncFrameId::new(Uuid::from_u128(99))
        }
    );

    let mut conflicting = original;
    conflicting.end_of_stream = true;
    transport.inject_incoming(conflicting);
    assert_eq!(
        transport.receive(UnixMillis(4)).unwrap_err().kind,
        TransportFailureKind::DuplicateConflict
    );
}

#[test]
fn streaming_requires_order_and_closes_after_end_of_stream() {
    let mut transport = simulation(hello(1));
    transport.connect(UnixMillis(0)).unwrap();

    assert_eq!(
        transport
            .send(&frame(1, 10), UnixMillis(1))
            .unwrap_err()
            .kind,
        TransportFailureKind::StreamOutOfOrder
    );
    let mut final_frame = frame(0, 10);
    final_frame.end_of_stream = true;
    transport.send(&final_frame, UnixMillis(2)).unwrap();
    assert_eq!(
        transport
            .send(&frame(1, 12), UnixMillis(3))
            .unwrap_err()
            .kind,
        TransportFailureKind::StreamOutOfOrder
    );
}

#[test]
fn cancellation_emits_shared_wire_frame_and_stops_stream() {
    let mut transport = simulation(hello(1));
    transport.connect(UnixMillis(0)).unwrap();
    let stream_id = SyncStreamId::new(Uuid::from_u128(104));
    let correlation_id = CorrelationId::new(Uuid::from_u128(103));

    transport
        .cancel(
            stream_id,
            correlation_id,
            SyncCancellationReason::ClientRequested,
            UnixMillis(1),
        )
        .unwrap();

    let sent = transport.take_outgoing();
    assert!(matches!(
        sent[0].payload,
        SyncTransportPayload::Cancel(ref cancellation)
            if cancellation.stream_id == stream_id
                && cancellation.reason == SyncCancellationReason::ClientRequested
    ));
    assert_eq!(
        transport
            .send(&frame(0, 1), UnixMillis(2))
            .unwrap_err()
            .kind,
        TransportFailureKind::Cancelled
    );
}

#[test]
fn version_mismatch_stops_connection_before_traffic() {
    let mut transport = simulation(hello(2));

    let failure = transport.connect(UnixMillis(0)).unwrap_err();
    assert_eq!(failure.kind, TransportFailureKind::VersionMismatch);
    assert_eq!(failure.phase, FailurePhase::Negotiation);
    assert!(transport.negotiated_session().is_none());
}

#[test]
fn missing_sync_capability_stops_connection_before_traffic() {
    let mut remote = hello(1);
    remote.capabilities.clear();
    let mut transport = simulation(remote);

    let failure = transport.connect(UnixMillis(0)).unwrap_err();
    assert_eq!(failure.kind, TransportFailureKind::CapabilityMismatch);
    assert_eq!(failure.phase, FailurePhase::Negotiation);
    assert_eq!(failure.retry, RetryAdvice::Never);
    assert!(transport.negotiated_session().is_none());
}

#[test]
fn lan_partial_discovery_connects_to_reachable_peer_as_degraded() {
    let device_id = DeviceId::new(Uuid::from_u128(200));
    let driver = ScriptedDriver::new(AuthenticationIdentity::Device(device_id), hello(1));
    let discovery = StaticDiscovery(Ok(LanDiscoveryReport {
        peers: vec![LanPeer {
            peer_id: "workshop-engine".to_owned(),
            endpoint: "192.168.10.20:7443".to_owned(),
            priority: 10,
        }],
        partial_failures: 2,
    }));
    let mut adapter = LanAdapter::new(
        discovery,
        driver,
        hello(1),
        device_auth(device_id),
        RetryPolicy::default(),
    )
    .unwrap();

    adapter.connect(UnixMillis(0)).unwrap();
    assert_eq!(adapter.health().status, HealthStatus::Degraded);
    assert_eq!(
        adapter.health().last_failure.as_ref().unwrap().kind,
        TransportFailureKind::PartialNetwork
    );
}

#[test]
fn lan_rejects_unencrypted_authenticated_connection() {
    let device_id = DeviceId::new(Uuid::from_u128(201));
    let mut driver = ScriptedDriver::new(AuthenticationIdentity::Device(device_id), hello(1));
    driver.security = SessionSecurity::isolated_simulation();
    let discovery = StaticDiscovery(Ok(LanDiscoveryReport {
        peers: vec![LanPeer {
            peer_id: "reception-engine".to_owned(),
            endpoint: "192.168.10.21:7443".to_owned(),
            priority: 1,
        }],
        partial_failures: 0,
    }));
    let mut adapter = LanAdapter::new(
        discovery,
        driver,
        hello(1),
        device_auth(device_id),
        RetryPolicy::default(),
    )
    .unwrap();

    assert_eq!(
        adapter.connect(UnixMillis(0)).unwrap_err().kind,
        TransportFailureKind::EncryptionRequired
    );
}

#[test]
fn wan_uses_relay_as_degraded_route_when_server_is_unavailable() {
    let account_id = AccountId::new(Uuid::from_u128(300));
    let device_id = DeviceId::new(Uuid::from_u128(301));
    let identity = AuthenticationIdentity::AccountDevice {
        account_id,
        device_id,
    };
    let mut driver = ScriptedDriver::new(identity, hello(1));
    driver.server_failures.push_back(failure(
        TransportFailureKind::DriverUnavailable,
        FailurePhase::Connect,
    ));
    let mut adapter = WanAdapter::new(
        wan_endpoint(),
        driver,
        hello(1),
        account_device_auth(account_id, device_id),
        RetryPolicy::default(),
    )
    .unwrap();

    adapter.connect(UnixMillis(0)).unwrap();
    assert_eq!(adapter.health().status, HealthStatus::Degraded);
    assert!(matches!(
        adapter.health().target,
        Some(ConnectionTarget::WanRelay { .. })
    ));
    assert_eq!(
        adapter.health().last_failure.as_ref().unwrap().kind,
        TransportFailureKind::ServerUnavailable
    );
}

#[test]
fn wan_reports_relay_unavailable_after_direct_and_relay_failures() {
    let account_id = AccountId::new(Uuid::from_u128(310));
    let device_id = DeviceId::new(Uuid::from_u128(311));
    let identity = AuthenticationIdentity::AccountDevice {
        account_id,
        device_id,
    };
    let mut driver = ScriptedDriver::new(identity, hello(1));
    driver.server_failures.push_back(failure(
        TransportFailureKind::DriverUnavailable,
        FailurePhase::Connect,
    ));
    driver.relay_failures.push_back(failure(
        TransportFailureKind::DriverUnavailable,
        FailurePhase::Connect,
    ));
    let mut adapter = WanAdapter::new(
        wan_endpoint(),
        driver,
        hello(1),
        account_device_auth(account_id, device_id),
        RetryPolicy::default(),
    )
    .unwrap();

    let failure = adapter.connect(UnixMillis(0)).unwrap_err();
    assert_eq!(failure.kind, TransportFailureKind::RelayUnavailable);
    assert_eq!(adapter.health().status, HealthStatus::Offline);
}

#[test]
fn wan_does_not_route_around_authentication_failure() {
    let account_id = AccountId::new(Uuid::from_u128(320));
    let device_id = DeviceId::new(Uuid::from_u128(321));
    let wrong_device = DeviceId::new(Uuid::from_u128(322));
    let identity = AuthenticationIdentity::AccountDevice {
        account_id,
        device_id: wrong_device,
    };
    let driver = ScriptedDriver::new(identity, hello(1));
    let mut adapter = WanAdapter::new(
        wan_endpoint(),
        driver,
        hello(1),
        account_device_auth(account_id, device_id),
        RetryPolicy::default(),
    )
    .unwrap();

    let failure = adapter.connect(UnixMillis(0)).unwrap_err();
    assert_eq!(failure.kind, TransportFailureKind::AuthenticationFailed);
    assert_eq!(failure.retry, RetryAdvice::Never);
    assert!(matches!(
        adapter.health().target,
        Some(ConnectionTarget::WanServer { .. })
    ));
}

fn simulation(remote: PeerHello) -> SimulatedTransport {
    SimulatedTransport::new(hello(1), remote, RetryPolicy::default())
}

fn hello(major: u16) -> PeerHello {
    PeerHello {
        peer_kind: PeerKind::Engine,
        product_version: ReleaseVersion::new(semver::Version::new(1, 0, 0)),
        protocols: vec![SupportedProtocol {
            major,
            minimum_minor: 0,
            maximum_minor: 3,
        }],
        capabilities: vec![CapabilityId::parse("eitmad.capability.sync.v1").unwrap()],
        required_capabilities: Vec::new(),
        schemas: Vec::new(),
    }
}

fn frame(sequence: u64, value: u128) -> SyncTransportFrame {
    SyncTransportFrame {
        frame_id: SyncFrameId::new(Uuid::from_u128(value)),
        idempotency_key: IdempotencyKey::new(Uuid::from_u128(value + 1)),
        protocol_version: eitmad_contracts::PROTOCOL_VERSION,
        correlation_id: CorrelationId::new(Uuid::from_u128(103)),
        stream_id: SyncStreamId::new(Uuid::from_u128(104)),
        sequence,
        end_of_stream: false,
        payload: SyncTransportPayload::Message(SyncMessage::Pull(PullRequest {
            after: None,
            maximum_records: 100,
        })),
    }
}

fn failure(kind: TransportFailureKind, phase: FailurePhase) -> TransportFailure {
    TransportFailure::new(kind, phase, RetryAdvice::Immediate)
}

fn secret() -> SecretId {
    SecretId::new(
        SecretKind::parse("sync-transport-credential").unwrap(),
        SecretReferenceId::new(Uuid::from_u128(900)),
    )
}

fn device_auth(device_id: DeviceId) -> TransportAuthentication {
    TransportAuthentication::Device {
        device_id,
        credential: secret(),
    }
}

fn account_device_auth(account_id: AccountId, device_id: DeviceId) -> TransportAuthentication {
    TransportAuthentication::AccountDevice {
        account_id,
        device_id,
        credential: secret(),
    }
}

fn wan_endpoint() -> WanEndpoint {
    WanEndpoint {
        server: "https://sync.eitmad.example".to_owned(),
        relay: Some("https://relay.eitmad.example".to_owned()),
    }
}

struct StaticDiscovery(Result<LanDiscoveryReport, TransportFailure>);

impl LanDiscovery for StaticDiscovery {
    fn discover(&mut self) -> Result<LanDiscoveryReport, TransportFailure> {
        self.0.clone()
    }
}

struct ScriptedDriver {
    identity: AuthenticationIdentity,
    hello: PeerHello,
    security: SessionSecurity,
    connected: bool,
    server_failures: VecDeque<TransportFailure>,
    relay_failures: VecDeque<TransportFailure>,
}

impl ScriptedDriver {
    fn new(identity: AuthenticationIdentity, hello: PeerHello) -> Self {
        Self {
            identity,
            hello,
            security: SessionSecurity::encrypted(),
            connected: false,
            server_failures: VecDeque::new(),
            relay_failures: VecDeque::new(),
        }
    }
}

impl ConnectionDriver for ScriptedDriver {
    fn establish(
        &mut self,
        target: &ConnectionTarget,
        _authentication: &TransportAuthentication,
    ) -> Result<EstablishedConnection, TransportFailure> {
        let failure = match target {
            ConnectionTarget::WanServer { .. } => self.server_failures.pop_front(),
            ConnectionTarget::WanRelay { .. } => self.relay_failures.pop_front(),
            ConnectionTarget::Lan { .. } => None,
            ConnectionTarget::Simulation => Some(failure(
                TransportFailureKind::DriverUnavailable,
                FailurePhase::Connect,
            )),
        };
        if let Some(failure) = failure {
            return Err(failure);
        }
        self.connected = true;
        Ok(EstablishedConnection {
            remote_hello: self.hello.clone(),
            authenticated_as: self.identity.clone(),
            security: self.security.clone(),
            round_trip_ms: Some(12),
        })
    }

    fn send(&mut self, _frame: &SyncTransportFrame) -> Result<(), TransportFailure> {
        self.connected
            .then_some(())
            .ok_or_else(|| failure(TransportFailureKind::Disconnected, FailurePhase::Send))
    }

    fn receive(&mut self) -> Result<Option<SyncTransportFrame>, TransportFailure> {
        self.connected
            .then_some(None)
            .ok_or_else(|| failure(TransportFailureKind::Disconnected, FailurePhase::Receive))
    }

    fn close(&mut self) {
        self.connected = false;
    }
}
