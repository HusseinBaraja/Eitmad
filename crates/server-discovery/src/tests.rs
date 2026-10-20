use super::*;
use eitmad_contracts::transport::{CorrelationId, RequestId};

fn request() -> DiscoverServers {
    DiscoverServers {
        request_id: RequestId::new(Uuid::new_v4()),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
    }
}
fn offer(nonce: Uuid, server_id: ServerInstallationId, boot: Uuid) -> Vec<u8> {
    wire::encode(&Packet::Offer {
        nonce,
        server_id,
        boot,
        port: 58443,
        lease_seconds: 15,
    })
}

#[test]
fn installation_identity_survives_reopen_and_preserves_invalid_data() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("server-identity");
    let first = load_or_create_identity(&file).unwrap();
    assert_eq!(first, load_or_create_identity(&file).unwrap());
    assert_ne!(
        first,
        load_or_create_identity(&directory.path().join("other")).unwrap()
    );
    std::fs::write(&file, b"interrupted").unwrap();
    assert!(load_or_create_identity(&file).is_err());
    assert_eq!(std::fs::read(&file).unwrap(), b"interrupted");
}

#[test]
fn discovers_multiple_servers_and_refuses_stale_or_malformed_offers() {
    let mut cache = Candidates::default();
    let now = Instant::now();
    let nonce = Uuid::new_v4();
    let id = ServerInstallationId::new(Uuid::new_v4());
    let source = Ipv4Addr::new(192, 168, 40, 2);
    cache.accept(
        &offer(Uuid::new_v4(), id, Uuid::new_v4()),
        source,
        nonce,
        now,
    );
    cache.accept(&[b'x'; MAX_PACKET + 1], source, nonce, now);
    assert!(cache.project(now, UnixMillis(100)).is_empty());
    cache.accept(&offer(nonce, id, Uuid::new_v4()), source, nonce, now);
    let other = ServerInstallationId::new(Uuid::new_v4());
    cache.accept(
        &offer(nonce, other, Uuid::new_v4()),
        Ipv4Addr::new(192, 168, 40, 3),
        nonce,
        now,
    );
    let values = cache.project(now, UnixMillis(100));
    assert_eq!(values.len(), 2);
    assert!(
        values
            .iter()
            .all(|value| !value.identity_conflict && value.expires_at.0 == 15_100)
    );
    assert!(cache.project(now + LEASE, UnixMillis(15_100)).is_empty());
}

#[test]
fn copied_identity_conflicts_but_one_multihomed_server_does_not() {
    let mut cache = Candidates::default();
    let now = Instant::now();
    let nonce = Uuid::new_v4();
    let id = ServerInstallationId::new(Uuid::new_v4());
    let boot = Uuid::new_v4();
    let one = Ipv4Addr::new(192, 168, 40, 2);
    let two = Ipv4Addr::new(192, 168, 50, 2);
    cache.accept(&offer(nonce, id, boot), one, nonce, now);
    cache.accept(&offer(nonce, id, boot), two, nonce, now);
    assert!(
        cache
            .project(now, UnixMillis(100))
            .iter()
            .all(|value| !value.identity_conflict)
    );
    cache.accept(&offer(nonce, id, Uuid::new_v4()), two, nonce, now);
    assert!(
        cache
            .project(now, UnixMillis(100))
            .iter()
            .all(|value| value.identity_conflict)
    );
}

struct Profiles(Vec<Interface>);
impl InterfaceProvider for Profiles {
    async fn snapshot(&mut self) -> io::Result<Vec<Interface>> {
        Ok(self.0.clone())
    }
}

fn loopback_interface() -> Interface {
    Interface {
        index: 1,
        address: Ipv4Addr::LOCALHOST,
        prefix: 8,
    }
}

async fn free_port() -> u16 {
    UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test]
async fn real_udp_scan_and_responder_cleanup_use_only_isolated_loopback() {
    let port = free_port().await;
    let id = ServerInstallationId::new(Uuid::new_v4());
    let (stop_server, cancellation) = watch::channel(false);
    let server = tokio::spawn(async move {
        respond(
            id,
            "0.0.0.0:58443".parse().unwrap(),
            Uuid::new_v4(),
            &mut Profiles(vec![loopback_interface()]),
            cancellation,
            port,
        )
        .await
    });
    tokio::task::yield_now().await;
    let (_stop_scan, mut cancellation) = watch::channel(false);
    let found = scan(
        &request(),
        &mut Profiles(vec![loopback_interface()]),
        &mut cancellation,
        port,
        |interface| interface.address,
    )
    .await;
    assert_eq!(found.status, DiscoveryStatus::Complete);
    assert_eq!(found.servers.len(), 1);
    assert_eq!(found.servers[0].server_id, id);
    assert!(!found.servers[0].identity_conflict);
    stop_server.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    // A new listener can immediately own the responder's port after cancellation.
    let reclaimed = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))
        .await
        .unwrap();
    assert_eq!(reclaimed.local_addr().unwrap().port(), port);
}

struct ChangingProfiles {
    calls: usize,
    replacement: Vec<Interface>,
}
impl InterfaceProvider for ChangingProfiles {
    async fn snapshot(&mut self) -> io::Result<Vec<Interface>> {
        self.calls += 1;
        Ok(if self.calls == 1 {
            vec![loopback_interface()]
        } else {
            self.replacement.clone()
        })
    }
}

#[tokio::test]
async fn scan_discards_candidates_on_interface_or_profile_change() {
    let (_stop, mut cancellation) = watch::channel(false);
    for replacement in [
        Vec::new(),
        vec![Interface {
            index: 2,
            ..loopback_interface()
        }],
        vec![Interface {
            address: Ipv4Addr::new(127, 0, 0, 2),
            ..loopback_interface()
        }],
    ] {
        let output = scan(
            &request(),
            &mut ChangingProfiles {
                calls: 0,
                replacement,
            },
            &mut cancellation,
            free_port().await,
            |interface| interface.address,
        )
        .await;
        assert_eq!(output.status, DiscoveryStatus::NetworkChanged);
        assert!(output.servers.is_empty());
    }
}

#[tokio::test]
async fn responder_rechecks_profiles_before_reply_and_releases_socket_on_failure() {
    let port = free_port().await;
    let (stop, cancellation) = watch::channel(false);
    let task = tokio::spawn(async move {
        respond(
            ServerInstallationId::new(Uuid::new_v4()),
            "0.0.0.0:58443".parse().unwrap(),
            Uuid::new_v4(),
            &mut ChangingProfiles {
                calls: 0,
                replacement: Vec::new(),
            },
            cancellation,
            port,
        )
        .await
    });
    tokio::task::yield_now().await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client
        .send_to(
            &wire::encode(&Packet::Query {
                nonce: Uuid::new_v4(),
            }),
            (Ipv4Addr::LOCALHOST, port),
        )
        .await
        .unwrap();
    let mut bytes = [0; MAX_PACKET];
    assert!(
        tokio::time::timeout(Duration::from_millis(100), client.recv_from(&mut bytes))
            .await
            .is_err()
    );
    stop.send(true).unwrap();
    task.await.unwrap().unwrap();
    UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))
        .await
        .unwrap();
}

#[tokio::test]
async fn active_cancel_and_dropped_future_release_client_sockets() {
    for disconnect in [false, true] {
        let peer = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let port = peer.local_addr().unwrap().port();
        let (stop, mut cancellation) = watch::channel(false);
        let task = tokio::spawn(async move {
            scan(
                &request(),
                &mut Profiles(vec![loopback_interface()]),
                &mut cancellation,
                port,
                |interface| interface.address,
            )
            .await
        });
        let mut bytes = [0; MAX_PACKET];
        let (_, client) = tokio::time::timeout(Duration::from_secs(1), peer.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        if disconnect {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        } else {
            stop.send(true).unwrap();
            let result = tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(result.status, DiscoveryStatus::Cancelled);
            assert!(result.servers.is_empty());
        }
        UdpSocket::bind(client).await.unwrap();
    }
}

#[tokio::test]
async fn no_private_network_and_pre_cancelled_scan_need_no_sockets() {
    let (stop, mut cancellation) = watch::channel(false);
    let empty = scan(
        &request(),
        &mut Profiles(Vec::new()),
        &mut cancellation,
        DISCOVERY_PORT,
        Interface::broadcast,
    )
    .await;
    assert_eq!(empty.status, DiscoveryStatus::Complete);
    assert!(empty.servers.is_empty());
    stop.send(true).unwrap();
    let cancelled = scan(
        &request(),
        &mut Profiles(Vec::new()),
        &mut cancellation,
        DISCOVERY_PORT,
        Interface::broadcast,
    )
    .await;
    assert_eq!(cancelled.status, DiscoveryStatus::Cancelled);
}

#[tokio::test]
async fn responder_cancels_without_binding_on_non_private_networks() {
    let (stop, cancellation) = watch::channel(true);
    respond(
        ServerInstallationId::new(Uuid::new_v4()),
        "0.0.0.0:58443".parse().unwrap(),
        Uuid::new_v4(),
        &mut Profiles(Vec::new()),
        cancellation,
        DISCOVERY_PORT,
    )
    .await
    .unwrap();
    drop(stop);
}

struct MutableProfiles(std::sync::Arc<std::sync::Mutex<Vec<Interface>>>);
impl InterfaceProvider for MutableProfiles {
    async fn snapshot(&mut self) -> io::Result<Vec<Interface>> {
        Ok(self.0.lock().unwrap().clone())
    }
}

#[tokio::test]
async fn responder_rebinds_after_private_address_change_with_the_same_process_nonce() {
    let port = free_port().await;
    let id = ServerInstallationId::new(Uuid::new_v4());
    let boot = Uuid::new_v4();
    let profiles = std::sync::Arc::new(std::sync::Mutex::new(vec![loopback_interface()]));
    let mut provider = MutableProfiles(profiles.clone());
    let (stop, cancellation) = watch::channel(false);
    let server = tokio::spawn(async move {
        respond(
            id,
            "0.0.0.0:58443".parse().unwrap(),
            boot,
            &mut provider,
            cancellation,
            port,
        )
        .await
    });
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    for address in [Ipv4Addr::LOCALHOST, Ipv4Addr::new(127, 0, 0, 2)] {
        *profiles.lock().unwrap() = vec![Interface {
            address,
            ..loopback_interface()
        }];
        let nonce = Uuid::new_v4();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            client
                .send_to(
                    &wire::encode(&Packet::Query { nonce }),
                    (Ipv4Addr::LOCALHOST, port),
                )
                .await
                .unwrap();
            let mut bytes = [0; MAX_PACKET];
            if let Ok(Ok((length, _))) =
                tokio::time::timeout(Duration::from_millis(50), client.recv_from(&mut bytes)).await
            {
                let Some(Packet::Offer {
                    nonce: echoed,
                    server_id,
                    boot: observed,
                    ..
                }) = wire::decode(&bytes[..length])
                else {
                    panic!("expected offer");
                };
                assert_eq!((echoed, server_id, observed), (nonce, id, boot));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "responder did not recover its interface"
            );
        }
    }
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
    UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))
        .await
        .unwrap();
}
