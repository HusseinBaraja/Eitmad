use std::io;

use serde::Deserialize;

use super::{Interface, RawInterface, eligible};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Profile {
    interface_index: u32,
    network_category: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Address {
    interface_index: u32,
    #[serde(rename = "IPAddress")]
    ip: String,
    prefix_length: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Adapter {
    interface_index: u32,
    interface_operational_status: Option<u32>,
}

fn join_policy(
    profiles: &[Profile],
    addresses: Vec<Address>,
    adapters: &[Adapter],
) -> io::Result<Vec<Interface>> {
    let mut rows = Vec::new();
    for ip in addresses {
        let mut matching = adapters
            .iter()
            .filter(|adapter| adapter.interface_index == ip.interface_index)
            .peekable();
        if matching.peek().is_none()
            || matching.any(|adapter| adapter.interface_operational_status != Some(1))
        {
            continue;
        }
        let mut matching = profiles
            .iter()
            .filter(|profile| profile.interface_index == ip.interface_index)
            .peekable();
        let private = matching.peek().is_some()
            && matching.all(|profile| profile.network_category == Some(1));
        rows.push(RawInterface {
            index: ip.interface_index,
            address: ip.ip.parse().map_err(|_| io::ErrorKind::InvalidData)?,
            prefix: ip.prefix_length,
            private,
        });
    }
    eligible(&rows)
}

#[cfg(windows)]
pub(super) async fn snapshot() -> io::Result<Vec<Interface>> {
    // Keep the COM apartment on one process-lifetime native worker. A hung call
    // cannot create more workers or keep Tokio runtime shutdown waiting.
    static WORKER: std::sync::Mutex<Option<tokio::sync::mpsc::Sender<Request>>> =
        std::sync::Mutex::new(None);
    let sender = {
        let mut worker = WORKER.lock().map_err(|_| io::ErrorKind::Other)?;
        if worker
            .as_ref()
            .is_none_or(tokio::sync::mpsc::Sender::is_closed)
        {
            *worker = Some(start_worker(native_reader)?);
        }
        worker.as_ref().ok_or(io::ErrorKind::Other)?.clone()
    };
    bounded_snapshot(&sender).await
}

struct Request {
    expires: std::time::Instant,
    reply: tokio::sync::oneshot::Sender<io::Result<Vec<Interface>>>,
}

fn start_worker<F: FnMut() -> io::Result<Vec<Interface>> + 'static>(
    prepare: impl FnOnce() -> F + Send + 'static,
) -> io::Result<tokio::sync::mpsc::Sender<Request>> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<Request>(1);
    std::thread::Builder::new()
        .name("eitmad-network-policy".into())
        .spawn(move || {
            let mut read = prepare();
            while let Some(request) = receiver.blocking_recv() {
                if request.reply.is_closed() {
                    continue;
                }
                let result = if std::time::Instant::now() >= request.expires {
                    Err(io::ErrorKind::TimedOut.into())
                } else {
                    read()
                };
                let _ = request.reply.send(result);
            }
        })?;
    Ok(sender)
}

async fn bounded_snapshot(
    sender: &tokio::sync::mpsc::Sender<Request>,
) -> io::Result<Vec<Interface>> {
    let expires = std::time::Instant::now() + crate::PROFILE_TIME;
    tokio::time::timeout(crate::PROFILE_TIME, async {
        let (reply, receiver) = tokio::sync::oneshot::channel();
        sender
            .send(Request { expires, reply })
            .await
            .map_err(|_| io::ErrorKind::BrokenPipe)?;
        receiver.await.map_err(|_| io::ErrorKind::Other)?
    })
    .await
    .map_err(|_| io::ErrorKind::TimedOut)?
}

#[cfg(windows)]
fn native_reader() -> impl FnMut() -> io::Result<Vec<Interface>> {
    let com = wmi::COMLibrary::new().map_err(|_| io::ErrorKind::PermissionDenied);
    let mut connection = None;
    move || {
        let com = com.as_ref().map_err(|kind| io::Error::from(*kind))?;
        if connection.is_none() {
            connection = Some(
                wmi::WMIConnection::with_namespace_path("ROOT\\StandardCimv2", *com)
                    .map_err(|_| io::ErrorKind::PermissionDenied)?,
            );
        }
        let result = read_policy(connection.as_ref().ok_or(io::ErrorKind::Other)?);
        if result.is_err() {
            connection = None;
        }
        result
    }
}

#[cfg(windows)]
fn read_policy(connection: &wmi::WMIConnection) -> io::Result<Vec<Interface>> {
    let profiles = query(
        connection,
        "SELECT InterfaceIndex, NetworkCategory FROM MSFT_NetConnectionProfile",
    )?;
    let addresses = query(
        connection,
        "SELECT InterfaceIndex, IPAddress, PrefixLength FROM MSFT_NetIPAddress WHERE AddressFamily=2 AND AddressState=4",
    )?;
    let adapters = query(
        connection,
        "SELECT InterfaceIndex, InterfaceOperationalStatus FROM MSFT_NetAdapter",
    )?;
    join_policy(&profiles, addresses, &adapters)
}

#[cfg(windows)]
fn query<T: serde::de::DeserializeOwned>(
    connection: &wmi::WMIConnection,
    query: &str,
) -> io::Result<Vec<T>> {
    let results = connection
        .exec_query(query)
        .map_err(|_| io::ErrorKind::Other)?;
    let mut rows = Vec::new();
    for result in results.take(257) {
        if rows.len() == 256 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        rows.push(
            result
                .and_then(wmi::IWbemClassWrapper::into_desr)
                .map_err(|_| io::ErrorKind::InvalidData)?,
        );
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn timed_out_native_read_does_not_accumulate_workers_or_publish_late_policy() {
        let (started, running) = tokio::sync::oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let worker = start_worker(move || {
            let mut started = Some(started);
            move || {
                if observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    let _ = started.take().unwrap().send(());
                    blocked.recv().unwrap();
                }
                Ok(Vec::new())
            }
        })
        .unwrap();
        let first = worker.clone();
        let pending = tokio::spawn(async move { bounded_snapshot(&first).await });
        running.await.unwrap();
        tokio::time::pause();
        tokio::time::advance(crate::PROFILE_TIME).await;
        assert_eq!(
            pending.await.unwrap().unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        let retry = bounded_snapshot(&worker);
        assert_eq!(retry.await.unwrap_err().kind(), io::ErrorKind::TimedOut);
        release.send(()).unwrap();
        tokio::time::resume();
        assert!(bounded_snapshot(&worker).await.unwrap().is_empty());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
        drop(worker);
    }

    #[test]
    fn requires_matching_up_adapter_and_only_private_profiles() {
        let address = || Address {
            interface_index: 7,
            ip: "192.168.42.10".into(),
            prefix_length: 24,
        };
        let up = Adapter {
            interface_index: 7,
            interface_operational_status: Some(1),
        };
        let private = Profile {
            interface_index: 7,
            network_category: Some(1),
        };
        assert_eq!(
            join_policy(&[private], vec![address()], &[up])
                .unwrap()
                .len(),
            1
        );
        for category in [Some(0), Some(2), None] {
            let profiles = [
                Profile {
                    interface_index: 7,
                    network_category: Some(1),
                },
                Profile {
                    interface_index: 7,
                    network_category: category,
                },
            ];
            let adapters = [Adapter {
                interface_index: 7,
                interface_operational_status: Some(1),
            }];
            assert!(
                join_policy(&profiles, vec![address()], &adapters)
                    .unwrap()
                    .is_empty()
            );
        }
        assert!(join_policy(&[], vec![address()], &[]).unwrap().is_empty());
        assert!(
            join_policy(
                &[Profile {
                    interface_index: 7,
                    network_category: Some(1)
                }],
                vec![address()],
                &[Adapter {
                    interface_index: 7,
                    interface_operational_status: None
                }]
            )
            .unwrap()
            .is_empty()
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "reads live Windows network policy; run explicitly for native adapter proof"]
    async fn live_native_snapshots() {
        let start = std::time::Instant::now();
        for _ in 0..20 {
            snapshot().await.expect("complete live network policy");
        }
        eprintln!(
            "20 fresh native snapshots completed in {:?}",
            start.elapsed()
        );
    }
}
