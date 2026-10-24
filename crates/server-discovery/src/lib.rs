//! Eitmad Discovery v1: bounded, nonce-bound UDP broadcast on Windows Private IPv4 subnets.
//! No credentials, trust anchors, customer data, or enrollment operations cross this boundary.
mod identity;
mod interfaces;
#[cfg(feature = "test-support")]
pub mod test_support;
#[cfg(test)]
mod tests;
mod wire;

pub use identity::load_or_create_identity;
use interfaces::{Interface, InterfaceProvider, WindowsInterfaces};

use eitmad_contracts::{
    discovery::{
        DiscoverServers, DiscoveredServer, DiscoveryStatus, ServerDiscoveryResult,
        ServerInstallationId,
    },
    transport::UnixMillis,
};
use std::{
    collections::BTreeMap,
    io,
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::{
    net::UdpSocket,
    sync::{Semaphore, watch},
    time::{Instant, sleep_until},
};
use uuid::Uuid;
use wire::{MAX_PACKET, Packet};

pub const DISCOVERY_PORT: u16 = 58444;
const SCAN_TIME: Duration = Duration::from_secs(8);
const PROFILE_TIME: Duration = Duration::from_secs(5);
/// Maximum duration of one discovery attempt, including profile reads and socket work.
pub const DISCOVERY_TIMEOUT: Duration = PROFILE_TIME
    .saturating_mul(2)
    .saturating_add(SCAN_TIME)
    .saturating_add(Duration::from_secs(1));
const LEASE: Duration = Duration::from_secs(15);
const REFRESH: Duration = Duration::from_secs(1);
const MAX_PACKETS: usize = 256;
const MAX_RESULTS: usize = 32;
static SCANS: Semaphore = Semaphore::const_new(1);

fn result(request: &DiscoverServers, status: DiscoveryStatus) -> ServerDiscoveryResult {
    ServerDiscoveryResult {
        request_id: request.request_id,
        correlation_id: request.correlation_id,
        status,
        servers: Vec::new(),
    }
}

/// Scans once. Dropping the future closes all sockets; cancellation returns no candidates.
/// Discovery is permitted only after the caller authenticates its local IPC bootstrap channel.
pub async fn discover(
    request: &DiscoverServers,
    cancellation: watch::Receiver<bool>,
) -> ServerDiscoveryResult {
    discover_using(
        request,
        cancellation,
        &mut WindowsInterfaces,
        DISCOVERY_PORT,
        Interface::broadcast,
        &SCANS,
    )
    .await
}

async fn discover_using(
    request: &DiscoverServers,
    mut cancellation: watch::Receiver<bool>,
    provider: &mut impl InterfaceProvider,
    port: u16,
    destination: fn(&Interface) -> Ipv4Addr,
    scans: &Semaphore,
) -> ServerDiscoveryResult {
    let Ok(_permit) = scans.try_acquire() else {
        return result(request, DiscoveryStatus::Busy);
    };
    let deadline = Instant::now() + DISCOVERY_TIMEOUT;
    let mut stop = cancellation.clone();
    let found = tokio::select! {
        biased;
        () = cancelled(&mut stop) => result(request, DiscoveryStatus::Cancelled),
        value = tokio::time::timeout_at(deadline, scan(request, provider, &mut cancellation, port, destination)) => {
            value.unwrap_or_else(|_| result(request, DiscoveryStatus::Unavailable))
        }
    };
    revalidate(found, request.selected_server_id)
}

fn revalidate(
    mut found: ServerDiscoveryResult,
    selected: Option<ServerInstallationId>,
) -> ServerDiscoveryResult {
    let Some(selected) = selected else {
        return found;
    };
    if found.status != DiscoveryStatus::Complete {
        found.servers.clear();
        return found;
    }
    found.servers.retain(|server| server.server_id == selected);
    if found.servers.iter().any(|server| server.identity_conflict) {
        found.status = DiscoveryStatus::IdentityConflict;
        found.servers.clear();
    } else {
        found
            .servers
            .retain(|server| server.expires_at.0 > unix_now().0);
        if found.servers.is_empty() {
            found.status = DiscoveryStatus::ServerMissing;
        }
    }
    found
}

async fn scan(
    request: &DiscoverServers,
    provider: &mut impl InterfaceProvider,
    cancellation: &mut watch::Receiver<bool>,
    discovery_port: u16,
    destination: fn(&Interface) -> Ipv4Addr,
) -> ServerDiscoveryResult {
    let mut output = result(request, DiscoveryStatus::Complete);
    let interfaces = tokio::select! {
        biased;
        () = cancelled(cancellation) => return result(request, DiscoveryStatus::Cancelled),
        value = tokio::time::timeout(PROFILE_TIME, provider.snapshot()) => match value {
            Ok(Ok(value)) => value,
            _ => return result(request, DiscoveryStatus::Unavailable),
        }
    };
    if interfaces.is_empty() {
        return output;
    }
    // Windows profile enumeration must not consume the packet reception window.
    let deadline = Instant::now() + SCAN_TIME;
    let nonce = Uuid::new_v4();
    let query = wire::encode(&Packet::Query { nonce });
    let mut sockets = Vec::new();
    for interface in &interfaces {
        let Ok(socket) = UdpSocket::bind((interface.address, 0)).await else {
            return result(request, DiscoveryStatus::Unavailable);
        };
        if socket
            .set_broadcast(true)
            .and_then(|()| socket.set_ttl(1))
            .is_err()
            || socket
                .send_to(&query, (destination(interface), discovery_port))
                .await
                .is_err()
        {
            return result(request, DiscoveryStatus::Unavailable);
        }
        sockets.push(socket);
    }
    let mut cache = Candidates::default();
    let mut received = 0;
    let mut next_profile = Instant::now() + REFRESH;
    let mut next_query = Instant::now() + Duration::from_secs(2);
    loop {
        // Profile reads can outlast the receive window; retain replies already queued.
        for (socket, interface) in sockets.iter().zip(&interfaces) {
            let mut bytes = [0; MAX_PACKET + 1];
            while let Ok((length, source)) = socket.try_recv_from(&mut bytes) {
                received += 1;
                if received > MAX_PACKETS {
                    return result(request, DiscoveryStatus::LimitReached);
                }
                if let SocketAddr::V4(source) = source {
                    if source.port() == discovery_port && interface.contains(*source.ip()) {
                        cache.accept(&bytes[..length], *source.ip(), nonce, Instant::now());
                    }
                }
                if cache.entries.len() > MAX_RESULTS {
                    return result(request, DiscoveryStatus::LimitReached);
                }
            }
        }
        tokio::select! {
            biased;
            () = cancelled(cancellation) => return result(request, DiscoveryStatus::Cancelled),
            () = sleep_until(deadline) => break,
            () = sleep_until(next_profile) => {
                match tokio::time::timeout(PROFILE_TIME, provider.snapshot()).await {
                    Ok(Ok(current)) if current == interfaces => (),
                    Ok(Ok(_)) => return result(request, DiscoveryStatus::NetworkChanged),
                    _ => return result(request, DiscoveryStatus::Unavailable),
                }
                next_profile = Instant::now() + REFRESH;
            }
            () = tokio::time::sleep(Duration::from_millis(20)) => {
                if Instant::now() >= next_query {
                    for (socket, interface) in sockets.iter().zip(&interfaces) {
                        if socket.send_to(&query, (destination(interface), discovery_port)).await.is_err() {
                            return result(request, DiscoveryStatus::Unavailable);
                        }
                    }
                    next_query = deadline;
                }
            }
        }
    }
    output.servers = cache.project(Instant::now(), unix_now());
    output
}

async fn cancelled(cancellation: &mut watch::Receiver<bool>) {
    loop {
        if *cancellation.borrow() || cancellation.changed().await.is_err() {
            return;
        }
    }
}

/// Runs the server responder until cancellation. No socket is opened without Private interfaces.
/// # Errors
/// Returns a local adapter or socket error. A profile-read failure closes the listener immediately.
pub async fn advertise(
    server_id: ServerInstallationId,
    listen: SocketAddr,
    boot: Uuid,
    cancellation: watch::Receiver<bool>,
) -> io::Result<()> {
    respond(
        server_id,
        listen,
        boot,
        &mut WindowsInterfaces,
        cancellation,
        DISCOVERY_PORT,
    )
    .await
}

async fn respond(
    server_id: ServerInstallationId,
    listen: SocketAddr,
    boot: Uuid,
    provider: &mut impl InterfaceProvider,
    mut cancellation: watch::Receiver<bool>,
    discovery_port: u16,
) -> io::Result<()> {
    let mut active: Option<(Vec<Interface>, UdpSocket)> = None;
    loop {
        let interfaces = tokio::select! {
            biased;
            () = cancelled(&mut cancellation) => return Ok(()),
            value = provider.snapshot() => value?,
        };
        let eligible: Vec<_> = interfaces
            .into_iter()
            .filter(|interface| listen.ip().is_unspecified() || listen.ip() == interface.address)
            .collect();
        if active
            .as_ref()
            .is_some_and(|(previous, _)| *previous != eligible)
        {
            active = None;
        }
        if eligible.is_empty() {
            tokio::select! { () = cancelled(&mut cancellation) => return Ok(()), () = tokio::time::sleep(REFRESH) => () }
            continue;
        }
        // Wildcard reception is required for Windows directed broadcasts. Never reply outside
        // a currently Private, non-overlapping local subnet. Firewall ingress is Private-only.
        if active.is_none() {
            let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, discovery_port)).await?;
            socket.set_ttl(1)?;
            active = Some((eligible, socket));
        }
        let Some((eligible, socket)) = active.as_ref() else {
            return Err(io::ErrorKind::NotConnected.into());
        };
        let until = Instant::now() + REFRESH;
        let mut packets = 0;
        loop {
            let mut bytes = [0; MAX_PACKET + 1];
            tokio::select! {
                biased;
                () = cancelled(&mut cancellation) => return Ok(()),
                () = sleep_until(until) => break,
                value = socket.recv_from(&mut bytes), if packets < 32 => {
                    let (length, source) = value?;
                    packets += 1;
                    let SocketAddr::V4(source) = source else { continue; };
                    let Some(interface) = eligible.iter().find(|interface| interface.contains(*source.ip())) else { continue; };
                    let Some(Packet::Query { nonce }) = wire::decode(&bytes[..length]) else { continue; };
                    // Recheck policy before each response. A failed or changed snapshot sends nothing.
                    let current = tokio::select! {
                        biased;
                        () = cancelled(&mut cancellation) => return Ok(()),
                        value = provider.snapshot() => value?,
                    };
                    if !current.contains(interface) { break; }
                    let reply = wire::encode(&Packet::Offer { nonce, server_id, boot, port: listen.port(), lease_seconds: 15 });
                    tokio::time::timeout(Duration::from_millis(100), socket.send_to(&reply, source)).await.map_err(|_| io::ErrorKind::TimedOut)??;
                }
            }
        }
    }
}

#[derive(Default)]
struct Candidates {
    entries: BTreeMap<(Uuid, Ipv4Addr, u16), (Uuid, Instant)>,
}
impl Candidates {
    fn accept(&mut self, bytes: &[u8], source: Ipv4Addr, expected: Uuid, now: Instant) {
        let Some(Packet::Offer {
            nonce,
            server_id,
            boot,
            port,
            lease_seconds,
        }) = wire::decode(bytes)
        else {
            return;
        };
        if nonce != expected
            || server_id.value().is_nil()
            || boot.is_nil()
            || port == 0
            || !(1..=15).contains(&lease_seconds)
        {
            return;
        }
        self.entries.retain(|_, (_, expiry)| *expiry > now);
        let key = (server_id.value(), source, port);
        // Keep a second claimant on the same address/port visible as a conflict as well.
        if let Some((previous, _)) = self
            .entries
            .get(&key)
            .filter(|(previous, _)| *previous != boot)
        {
            self.entries
                .insert((server_id.value(), source, 0), (*previous, now + LEASE));
        }
        self.entries.insert(
            key,
            (boot, now + Duration::from_secs(u64::from(lease_seconds))),
        );
    }
    fn project(&self, now: Instant, wall: UnixMillis) -> Vec<DiscoveredServer> {
        self.entries
            .iter()
            .filter(|((_, _, port), (_, expiry))| *port != 0 && *expiry > now)
            .map(|((id, address, port), (boot, expiry))| {
                let identity_conflict =
                    self.entries
                        .iter()
                        .any(|((other_id, _, _), (other_boot, other_expiry))| {
                            other_id == id && other_boot != boot && *other_expiry > now
                        });
                DiscoveredServer {
                    server_id: ServerInstallationId::new(*id),
                    address: address.to_string(),
                    port: *port,
                    expires_at: UnixMillis(wall.0.saturating_add(
                        i64::try_from(expiry.duration_since(now).as_millis()).unwrap_or(15_000),
                    )),
                    identity_conflict,
                }
            })
            .collect()
    }
}

fn unix_now() -> UnixMillis {
    UnixMillis(
        i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        )
        .unwrap_or(i64::MAX),
    )
}
