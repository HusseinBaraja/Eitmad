mod core;
mod lan;
mod simulation;
mod wan;

use eitmad_contracts::{
    identity::{AccountId, DeviceId},
    secrets::SecretId,
    sync_transport::{SyncCancellationReason, SyncStreamId, SyncTransportFrame},
    transport::{CorrelationId, UnixMillis},
    versioning::{NegotiatedSession, PeerHello},
};

pub use lan::{LanAdapter, LanDiscovery, LanDiscoveryReport, LanPeer};
pub use simulation::SimulatedTransport;
pub use wan::{WanAdapter, WanEndpoint};

pub(crate) use core::TransportCore;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportKind {
    Simulation,
    Lan,
    Wan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportAuthentication {
    Simulation,
    Device {
        device_id: DeviceId,
        credential: SecretId,
    },
    AccountDevice {
        account_id: AccountId,
        device_id: DeviceId,
        credential: SecretId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthenticationIdentity {
    Simulation,
    Device(DeviceId),
    AccountDevice {
        account_id: AccountId,
        device_id: DeviceId,
    },
}

impl TransportAuthentication {
    fn identity(&self) -> AuthenticationIdentity {
        match self {
            Self::Simulation => AuthenticationIdentity::Simulation,
            Self::Device { device_id, .. } => AuthenticationIdentity::Device(*device_id),
            Self::AccountDevice {
                account_id,
                device_id,
                ..
            } => AuthenticationIdentity::AccountDevice {
                account_id: *account_id,
                device_id: *device_id,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectionTarget {
    Simulation,
    Lan { peer_id: String, endpoint: String },
    WanServer { endpoint: String },
    WanRelay { endpoint: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionSecurity {
    IsolatedSimulation,
    AuthenticatedEncryption {
        peer_authenticated: bool,
        forward_secrecy: bool,
    },
}

impl SessionSecurity {
    #[must_use]
    pub const fn encrypted() -> Self {
        Self::AuthenticatedEncryption {
            peer_authenticated: true,
            forward_secrecy: true,
        }
    }

    #[must_use]
    pub const fn isolated_simulation() -> Self {
        Self::IsolatedSimulation
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EstablishedConnection {
    pub remote_hello: PeerHello,
    pub authenticated_as: AuthenticationIdentity,
    pub security: SessionSecurity,
    pub round_trip_ms: Option<u64>,
}

pub trait ConnectionDriver {
    /// Establishes and authenticates one route.
    ///
    /// # Errors
    ///
    /// Returns a structured connection, authentication, or encryption failure.
    fn establish(
        &mut self,
        target: &ConnectionTarget,
        authentication: &TransportAuthentication,
    ) -> Result<EstablishedConnection, TransportFailure>;

    /// Sends one frame over the established route.
    ///
    /// # Errors
    ///
    /// Returns a structured delivery failure without exposing credential material.
    fn send(&mut self, frame: &SyncTransportFrame) -> Result<(), TransportFailure>;

    /// Receives the next available frame.
    ///
    /// # Errors
    ///
    /// Returns a structured receive failure when the route cannot deliver safely.
    fn receive(&mut self) -> Result<Option<SyncTransportFrame>, TransportFailure>;

    fn close(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailurePhase {
    Discovery,
    Authentication,
    Encryption,
    Negotiation,
    Connect,
    Send,
    Receive,
    Cancellation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportFailureKind {
    Disconnected,
    AuthenticationFailed,
    EncryptionRequired,
    VersionMismatch,
    CapabilityMismatch,
    SchemaMismatch,
    NoLanPeer,
    PartialNetwork,
    ServerUnavailable,
    RelayUnavailable,
    RetryNotReady,
    Cancelled,
    DuplicateConflict,
    StreamOutOfOrder,
    DriverUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryAdvice {
    Never,
    Immediate,
    After { delay_ms: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportFailure {
    pub kind: TransportFailureKind,
    pub phase: FailurePhase,
    pub retry: RetryAdvice,
    pub correlation_id: Option<CorrelationId>,
}

impl TransportFailure {
    #[must_use]
    pub const fn new(kind: TransportFailureKind, phase: FailurePhase, retry: RetryAdvice) -> Self {
        Self {
            kind,
            phase,
            retry,
            correlation_id: None,
        }
    }

    #[must_use]
    pub const fn with_correlation(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

impl std::fmt::Display for TransportFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self.kind {
            TransportFailureKind::Disconnected => "sync transport is disconnected",
            TransportFailureKind::AuthenticationFailed => "sync transport authentication failed",
            TransportFailureKind::EncryptionRequired => "sync transport encryption is insufficient",
            TransportFailureKind::VersionMismatch => {
                "sync transport protocol version is incompatible"
            }
            TransportFailureKind::CapabilityMismatch => {
                "sync transport capability negotiation failed"
            }
            TransportFailureKind::SchemaMismatch => "sync transport schema negotiation failed",
            TransportFailureKind::NoLanPeer => "sync LAN peer is unavailable",
            TransportFailureKind::PartialNetwork => "sync network is partially available",
            TransportFailureKind::ServerUnavailable => "sync server is unavailable",
            TransportFailureKind::RelayUnavailable => "sync relay is unavailable",
            TransportFailureKind::RetryNotReady => "sync transport retry is not ready",
            TransportFailureKind::Cancelled => "sync stream is cancelled",
            TransportFailureKind::DuplicateConflict => "sync frame identity was reused",
            TransportFailureKind::StreamOutOfOrder => "sync stream frame is out of order",
            TransportFailureKind::DriverUnavailable => "sync transport driver is unavailable",
        })
    }
}

impl std::error::Error for TransportFailure {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub initial_delay_ms: u64,
    pub maximum_delay_ms: u64,
    pub maximum_attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_delay_ms: 250,
            maximum_delay_ms: 30_000,
            maximum_attempts: 5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthStatus {
    Offline,
    Connecting,
    Healthy,
    Degraded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionHealth {
    pub status: HealthStatus,
    pub target: Option<ConnectionTarget>,
    pub last_success_at: Option<UnixMillis>,
    pub last_failure: Option<TransportFailure>,
    pub consecutive_failures: u32,
    pub next_retry_at: Option<UnixMillis>,
    pub round_trip_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiveOutcome {
    NoFrame,
    Frame(Box<SyncTransportFrame>),
    DuplicateIgnored {
        frame_id: eitmad_contracts::sync_transport::SyncFrameId,
    },
}

pub trait SyncTransport {
    fn kind(&self) -> TransportKind;

    /// Establishes authentication, encryption, and version/capability negotiation.
    ///
    /// # Errors
    ///
    /// Returns a structured failure when the route cannot become safe and compatible.
    fn connect(&mut self, now: UnixMillis) -> Result<NegotiatedSession, TransportFailure>;

    fn disconnect(&mut self, now: UnixMillis);

    /// Sends one ordered stream frame.
    ///
    /// # Errors
    ///
    /// Returns a structured failure for disconnects, cancellation, or invalid ordering.
    fn send(&mut self, frame: &SyncTransportFrame, now: UnixMillis)
    -> Result<(), TransportFailure>;

    /// Receives and validates one stream frame or duplicate outcome.
    ///
    /// # Errors
    ///
    /// Returns a structured failure for invalid identity, order, version, or connectivity.
    fn receive(&mut self, now: UnixMillis) -> Result<ReceiveOutcome, TransportFailure>;

    /// Cancels one stream and sends the cancellation on the shared wire protocol.
    ///
    /// # Errors
    ///
    /// Returns a structured failure when cancellation cannot be delivered safely.
    fn cancel(
        &mut self,
        stream_id: SyncStreamId,
        correlation_id: CorrelationId,
        reason: SyncCancellationReason,
        now: UnixMillis,
    ) -> Result<(), TransportFailure>;

    fn health(&self) -> &ConnectionHealth;

    fn negotiated_session(&self) -> Option<&NegotiatedSession>;
}
