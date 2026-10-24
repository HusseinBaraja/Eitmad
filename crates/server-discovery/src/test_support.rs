//! Synthetic loopback discovery for cross-crate deadline and cancellation regressions.
use super::{
    DiscoverServers, Duration, Interface, InterfaceProvider, Ipv4Addr, MAX_PACKET, Packet,
    Semaphore, ServerDiscoveryResult, ServerInstallationId, UdpSocket, Uuid, discover_using, io,
    watch, wire,
};

struct SlowProfiles {
    peer: UdpSocket,
    server_id: ServerInstallationId,
    https_port: u16,
    reads: usize,
}

impl InterfaceProvider for SlowProfiles {
    async fn snapshot(&mut self) -> io::Result<Vec<Interface>> {
        self.reads += 1;
        if self.reads == 2 {
            // Answer the real query during the first profile recheck. The scan must
            // drain this queued offer after enumeration completes.
            let mut bytes = [0; MAX_PACKET + 1];
            let (length, client) = self.peer.recv_from(&mut bytes).await?;
            let Some(Packet::Query { nonce }) = wire::decode(&bytes[..length]) else {
                return Err(io::ErrorKind::InvalidData.into());
            };
            self.peer
                .send_to(
                    &wire::encode(&Packet::Offer {
                        nonce,
                        server_id: self.server_id,
                        boot: Uuid::new_v4(),
                        port: self.https_port,
                        lease_seconds: 15,
                    }),
                    client,
                )
                .await?;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(vec![Interface {
            index: 1,
            address: Ipv4Addr::LOCALHOST,
            prefix: 8,
        }])
    }
}

/// Runs the production discovery boundary with three-second profile snapshots and
/// a nonce-bound UDP offer. All sockets use disposable loopback ports.
/// # Panics
/// Panics if a disposable loopback socket cannot be opened.
pub async fn discover_slow_profiles(
    request: DiscoverServers,
    cancellation: watch::Receiver<bool>,
    server_id: ServerInstallationId,
    https_port: u16,
) -> ServerDiscoveryResult {
    let peer = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let port = peer.local_addr().unwrap().port();
    discover_using(
        &request,
        cancellation,
        &mut SlowProfiles {
            peer,
            server_id,
            https_port,
            reads: 0,
        },
        port,
        |_| Ipv4Addr::LOCALHOST,
        &Semaphore::new(1),
    )
    .await
}
