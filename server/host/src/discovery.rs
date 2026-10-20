use std::{io, net::SocketAddr, path::Path};
use tokio::{sync::watch, task::JoinHandle};

/// Owns responder cleanup, including failed HTTPS startup.
pub(crate) struct Advertisement {
    stop: watch::Sender<bool>,
    task: JoinHandle<()>,
}
impl Advertisement {
    pub(crate) async fn start(
        directory: &Path,
        listen: SocketAddr,
        advertise: bool,
    ) -> io::Result<Self> {
        let identity = directory.join("server-identity");
        let id = tokio::task::spawn_blocking(move || {
            eitmad_server_discovery::load_or_create_identity(&identity)
        })
        .await??;
        let (stop, cancellation) = watch::channel(false);
        let boot = uuid::Uuid::new_v4();
        let task = tokio::spawn(async move {
            if !advertise || listen.ip().is_loopback() {
                return;
            }
            loop {
                if *cancellation.borrow() {
                    return;
                }
                if eitmad_server_discovery::advertise(id, listen, boot, cancellation.clone())
                    .await
                    .is_ok()
                {
                    return;
                }
                tracing::warn!("server discovery unavailable; retrying");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        });
        Ok(Self { stop, task })
    }
}

/// Paired LAN servers retain their TLS origin while DHCP changes the transport address.
/// The installed Private + `LocalSubnet` firewall rules remain the ingress boundary.
pub(crate) fn listener_address(listen: SocketAddr, paired: bool) -> SocketAddr {
    if paired && listen.is_ipv4() && !listen.ip().is_loopback() {
        SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), listen.port())
    } else {
        listen
    }
}

impl Drop for Advertisement {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paired_lan_listener_survives_address_changes_without_broadening_loopback() {
        for ip in ["192.168.40.2", "192.168.40.7"] {
            assert_eq!(
                listener_address(format!("{ip}:58443").parse().unwrap(), true),
                "0.0.0.0:58443".parse().unwrap()
            );
        }
        for (address, paired) in [
            ("127.0.0.1:443", true),
            ("192.168.40.2:443", false),
            ("[::1]:443", true),
        ] {
            let address = address.parse().unwrap();
            assert_eq!(listener_address(address, paired), address);
        }
    }
}
