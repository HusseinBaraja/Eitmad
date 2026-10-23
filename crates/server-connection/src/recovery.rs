//! Address discovery cannot replace the retained TLS name, public trust, scope, or device.
use super::{
    DesktopConnectionSettings, DiagnosticPhase, DirectServerConfig, FailurePhase, RequestGate,
    TlsStream, TransportFailure, TransportFailureKind, connect_tls_at, desktop_enrollment,
    encryption_failure, host_header, parse_http_response, unavailable, unix_millis_now,
};
use eitmad_contracts::identity::DeviceId;
use eitmad_contracts::{
    discovery::{DiscoverServers, DiscoveredServer, DiscoveryStatus, ServerDiscoveryResult},
    pairing::PairingIdentity,
    transport::{CorrelationId, RequestId},
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    io::{Read as _, Write as _},
    net::{SocketAddr, TcpStream},
    path::Path,
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};
use tokio::sync::watch;
use url::Url;
use uuid::Uuid;
#[cfg(test)]
mod tests;

struct Control {
    generation: AtomicU64,
    changed: watch::Sender<u64>,
    sockets: Mutex<Vec<Weak<TcpStream>>>,
}

/// Cloneable cancellation handle for one retained server route.
#[derive(Clone)]
pub struct ConnectionRecovery(Arc<Control>);

impl ConnectionRecovery {
    fn new() -> Self {
        let (changed, _) = watch::channel(0);
        Self(Arc::new(Control {
            generation: AtomicU64::new(0),
            changed,
            sockets: Mutex::new(Vec::new()),
        }))
    }

    /// Cancels discovery, closes active sockets, and invalidates results already in flight.
    /// The next explicit operation can start a new bounded attempt.
    pub fn cancel(&self) {
        let generation = self.0.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.0.changed.send_replace(generation);
        if let Ok(mut sockets) = self.0.sockets.lock() {
            for socket in sockets.drain(..).filter_map(|socket| socket.upgrade()) {
                let _ = socket.shutdown(std::net::Shutdown::Both);
            }
        }
    }

    pub(super) fn cancelled(&self, generation: u64) -> bool {
        self.0.generation.load(Ordering::SeqCst) != generation
    }

    pub(super) fn register(
        &self,
        socket: &TcpStream,
        generation: u64,
    ) -> Result<Arc<TcpStream>, TransportFailure> {
        let socket = Arc::new(
            socket
                .try_clone()
                .map_err(|_| unavailable(FailurePhase::Connect))?,
        );
        let mut sockets = self
            .0
            .sockets
            .lock()
            .map_err(|_| unavailable(FailurePhase::Connect))?;
        self.check(generation)?;
        sockets.retain(|socket| socket.strong_count() != 0);
        sockets.push(Arc::downgrade(&socket));
        Ok(socket)
    }

    fn check(&self, generation: u64) -> Result<(), TransportFailure> {
        if self.cancelled(generation) {
            Err(unavailable(FailurePhase::Cancellation))
        } else {
            Ok(())
        }
    }
}

pub(super) struct RetainedServer {
    identity: PairingIdentity,
    pub(super) control: ConnectionRecovery,
    gate: RequestGate,
    route: Mutex<Route>,
}

#[derive(Default)]
struct Route {
    address: Option<(u64, SocketAddr)>,
    retry_at: Option<Instant>,
}

impl RetainedServer {
    pub(super) fn server_id(&self) -> eitmad_contracts::discovery::ServerInstallationId {
        self.identity.server_id
    }
    pub(super) fn load(
        path: &Path,
        config: &DirectServerConfig,
        pem: &[u8],
    ) -> Result<Option<Arc<Self>>, TransportFailure> {
        let directory = path.parent().ok_or_else(encryption_failure)?;
        // A retained paired installation must fail closed if its receipt or trust is lost.
        DesktopConnectionSettings::load(directory)?;
        let Some(saved) = desktop_enrollment::retained::RetainedEnrollment::load(directory)? else {
            return Ok(None);
        };
        if saved.identity.endpoint != config.endpoint.as_str()
            || saved.identity.public_ca_pem.as_bytes() != pem
            || !((config.scope.kind.as_str() == "organization"
                && (config.scope.id.value() == saved.organization.value()
                    || config.scope.id.value() == saved.tenant_id.value()))
                || (config.scope.kind.as_str() == "branch" && config.scope.id == saved.branch))
        {
            return Err(encryption_failure());
        }
        let device: DeviceId = serde_json::from_slice(
            &desktop_enrollment::pairing::read_pairing_file(&directory.join("desktop-device.json"))
                .map_err(|_| encryption_failure())?,
        )
        .map_err(|_| encryption_failure())?;
        if saved.device != device || device.value().is_nil() {
            return Err(encryption_failure());
        }
        Ok(Some(Arc::new(Self {
            identity: saved.identity,
            control: ConnectionRecovery::new(),
            gate: RequestGate::new(DiagnosticPhase::Connect, FailurePhase::Connect),
            route: Mutex::new(Route::default()),
        })))
    }

    pub(super) fn connect(
        &self,
        config: &DirectServerConfig,
        url: &Url,
        deadline: Instant,
    ) -> Result<TlsStream, TransportFailure> {
        self.connect_using(config, url, deadline, discover)
    }

    fn connect_using(
        &self,
        config: &DirectServerConfig,
        url: &Url,
        deadline: Instant,
        scan: impl FnOnce(
            &ConnectionRecovery,
            u64,
            Instant,
        ) -> Result<Vec<DiscoveredServer>, TransportFailure>,
    ) -> Result<TlsStream, TransportFailure> {
        let generation = self.control.0.generation.load(Ordering::SeqCst);
        let _owner = self.gate.acquire(deadline)?;
        self.control.check(generation)?;
        let result = self.try_connect(config, url, deadline, generation, scan);
        // An old attempt must not publish success or a cached address after cancellation.
        if self.control.cancelled(generation) {
            return Err(unavailable(FailurePhase::Cancellation));
        }
        if result.as_ref().is_err_and(|failure| {
            failure.kind == TransportFailureKind::ServerUnavailable
                && failure.phase != FailurePhase::Cancellation
        }) {
            self.route
                .lock()
                .map_err(|_| unavailable(FailurePhase::Connect))?
                .retry_at = Some(Instant::now() + Duration::from_secs(2));
        }
        result
    }

    fn try_connect(
        &self,
        config: &DirectServerConfig,
        url: &Url,
        deadline: Instant,
        generation: u64,
        scan: impl FnOnce(
            &ConnectionRecovery,
            u64,
            Instant,
        ) -> Result<Vec<DiscoveredServer>, TransportFailure>,
    ) -> Result<TlsStream, TransportFailure> {
        let cached = {
            let route = self
                .route
                .lock()
                .map_err(|_| unavailable(FailurePhase::Connect))?;
            if route.retry_at.is_some_and(|at| at > Instant::now()) {
                return Err(unavailable(FailurePhase::Connect));
            }
            route
                .address
                .filter(|(owner, _)| *owner == generation)
                .map(|(_, address)| address)
        };
        let initial = self.open(
            config,
            url,
            cached,
            deadline.min(Instant::now() + Duration::from_secs(1)),
            generation,
        );
        let initial_failure = match initial {
            Ok(mut stream) => {
                stream.sock.deadline = Some(deadline);
                return Ok(stream);
            }
            // A DHCP lease can assign the old address to another host. No credentials
            // have been sent, so rediscover the retained identity with unchanged trust.
            Err(failure)
                if matches!(
                    failure.kind,
                    TransportFailureKind::ServerUnavailable
                        | TransportFailureKind::EncryptionRequired
                ) =>
            {
                failure
            }
            Err(failure) => return Err(failure),
        };
        self.control.check(generation)?;
        // Also pace failed adapter reads and busy discovery results.
        self.route
            .lock()
            .map_err(|_| unavailable(FailurePhase::Connect))?
            .retry_at = Some(Instant::now() + Duration::from_secs(2));
        let candidates = scan(&self.control, generation, deadline).map_err(|failure| {
            if failure.phase != FailurePhase::Cancellation
                && initial_failure.kind == TransportFailureKind::EncryptionRequired
            {
                initial_failure.clone()
            } else {
                failure
            }
        })?;
        if candidates.iter().any(|candidate| {
            candidate.server_id == self.identity.server_id && candidate.identity_conflict
        }) {
            return Err(encryption_failure());
        }
        for candidate in candidates
            .into_iter()
            .filter(|candidate| {
                candidate.server_id == self.identity.server_id
                    && candidate.expires_at.0 > unix_millis_now().0
            })
            .take(3)
        {
            self.control.check(generation)?;
            let address = SocketAddr::new(
                candidate
                    .address
                    .parse::<std::net::Ipv4Addr>()
                    .map_err(|_| encryption_failure())?
                    .into(),
                candidate.port,
            );
            if address.port() == 0 {
                return Err(encryption_failure());
            }
            match self.open(config, url, Some(address), deadline, generation) {
                Ok(stream) => {
                    self.control.check(generation)?;
                    self.route
                        .lock()
                        .map_err(|_| unavailable(FailurePhase::Connect))?
                        .address = Some((generation, address));
                    self.route
                        .lock()
                        .map_err(|_| unavailable(FailurePhase::Connect))?
                        .retry_at = None;
                    return Ok(stream);
                }
                Err(failure) if failure.kind != TransportFailureKind::ServerUnavailable => {
                    return Err(failure);
                }
                Err(_) => (),
            }
        }
        self.route
            .lock()
            .map_err(|_| unavailable(FailurePhase::Connect))?
            .retry_at = Some(Instant::now() + Duration::from_secs(2));
        Err(initial_failure)
    }

    fn open(
        &self,
        config: &DirectServerConfig,
        url: &Url,
        address: Option<SocketAddr>,
        deadline: Instant,
        generation: u64,
    ) -> Result<TlsStream, TransportFailure> {
        let mut stream = connect_tls_at(
            url,
            &config.tls,
            deadline,
            address,
            Some(&(self.control.clone(), generation)),
        )?;
        verify_identity(&mut stream, url, &self.identity)?;
        Ok(stream)
    }
}

fn discover(
    control: &ConnectionRecovery,
    generation: u64,
    deadline: Instant,
) -> Result<Vec<DiscoveredServer>, TransportFailure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| unavailable(FailurePhase::Discovery))?;
    runtime.block_on(discover_using(
        control,
        generation,
        deadline,
        |request, cancellation| async move {
            eitmad_server_discovery::discover(&request, cancellation).await
        },
    ))
}

async fn discover_using<F: std::future::Future<Output = ServerDiscoveryResult>>(
    control: &ConnectionRecovery,
    generation: u64,
    deadline: Instant,
    scan: impl FnOnce(DiscoverServers, watch::Receiver<bool>) -> F,
) -> Result<Vec<DiscoveredServer>, TransportFailure> {
    let mut changed = control.0.changed.subscribe();
    control.check(generation)?;
    let request = DiscoverServers {
        selected_server_id: None,
        request_id: RequestId::new(Uuid::new_v4()),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
    };
    let (_stop, cancellation) = watch::channel(false);
    tokio::select! {
        biased;
        _ = changed.changed() => Err(unavailable(FailurePhase::Cancellation)),
        result = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), scan(request, cancellation)) => {
            let result = result.map_err(|_| unavailable(FailurePhase::Discovery))?;
            if result.status == DiscoveryStatus::Complete { Ok(result.servers) } else { Err(unavailable(FailurePhase::Discovery)) }
        }
    }
}

// Authenticate installation identity on this exact TLS socket before sending any credential.
// A bounded Content-Length response leaves the connection ready for HTTP or WebSocket upgrade.
fn verify_identity(
    stream: &mut TlsStream,
    url: &Url,
    expected: &PairingIdentity,
) -> Result<(), TransportFailure> {
    write!(
        stream,
        "GET /v1/server-identity HTTP/1.1\r\nHost: {}\r\nConnection: keep-alive\r\n\r\n",
        host_header(url)?
    )
    .and_then(|()| stream.flush())
    .map_err(|_| unavailable(FailurePhase::Connect))?;
    let mut bytes = Vec::new();
    while bytes.len() <= 4096 && !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream
            .read_exact(&mut byte)
            .map_err(|_| unavailable(FailurePhase::Connect))?;
        bytes.push(byte[0]);
    }
    let header = std::str::from_utf8(&bytes).map_err(|_| encryption_failure())?;
    if !header.ends_with("\r\n\r\n") {
        return Err(encryption_failure());
    }
    let lengths: Vec<_> = header
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
        .collect();
    if lengths.len() != 1
        || header.lines().any(|line| {
            line.split_once(':')
                .is_some_and(|(key, _)| key.eq_ignore_ascii_case("transfer-encoding"))
        })
    {
        return Err(encryption_failure());
    }
    let length: usize = lengths[0]
        .1
        .trim()
        .parse()
        .map_err(|_| encryption_failure())?;
    if length > 24_576 {
        return Err(encryption_failure());
    }
    let offset = bytes.len();
    bytes.resize(offset + length, 0);
    stream
        .read_exact(&mut bytes[offset..])
        .map_err(|_| unavailable(FailurePhase::Connect))?;
    let (status, body) = parse_http_response(&bytes)?;
    if status == 503 {
        return Err(unavailable(FailurePhase::Connect));
    }
    let actual: PairingIdentity =
        serde_json::from_slice(&body).map_err(|_| encryption_failure())?;
    if status != 200 || actual != *expected {
        return Err(encryption_failure());
    }
    Ok(())
}
