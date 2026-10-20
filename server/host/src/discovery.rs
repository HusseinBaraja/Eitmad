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
        let task = tokio::spawn(async move {
            if !advertise || listen.ip().is_loopback() {
                return;
            }
            loop {
                if *cancellation.borrow() {
                    return;
                }
                if eitmad_server_discovery::advertise(id, listen, cancellation.clone())
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
impl Drop for Advertisement {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
        self.task.abort();
    }
}
