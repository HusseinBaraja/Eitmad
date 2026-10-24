use eitmad_contracts::{
    discovery::{DiscoverServers, DiscoveryStatus, ServerDiscoveryResult},
    ipc::IpcServerMessage,
};
use tokio::{sync::watch, task::JoinSet};

#[derive(Default)]
pub(super) struct DesktopDiscovery {
    tasks: JoinSet<ServerDiscoveryResult>,
    active: Option<(DiscoverServers, watch::Sender<bool>)>,
}
impl DesktopDiscovery {
    pub(super) fn is_active(&self) -> bool {
        !self.tasks.is_empty()
    }
    pub(super) fn start(
        &mut self,
        request: DiscoverServers,
        authenticated: bool,
    ) -> Option<IpcServerMessage> {
        let status = if !authenticated {
            Some(DiscoveryStatus::AuthenticationRequired)
        } else if self.is_active() {
            Some(DiscoveryStatus::Busy)
        } else {
            None
        };
        if let Some(status) = status {
            return Some(IpcServerMessage::ServerDiscovery(ServerDiscoveryResult {
                request_id: request.request_id,
                correlation_id: request.correlation_id,
                status,
                servers: Vec::new(),
            }));
        }
        let (sender, cancellation) = watch::channel(false);
        self.active = Some((request.clone(), sender));
        self.tasks
            .spawn(async move { eitmad_server_discovery::discover(&request, cancellation).await });
        None
    }
    pub(super) fn cancel(&self, request: &DiscoverServers) {
        if let Some((active, sender)) = &self.active {
            if active == request {
                let _ = sender.send(true);
            }
        }
    }
    pub(super) async fn finish(&mut self) -> Option<IpcServerMessage> {
        let completed = self.tasks.join_next().await;
        let (request, _) = self.active.take()?;
        Some(IpcServerMessage::ServerDiscovery(completed?.unwrap_or(
            ServerDiscoveryResult {
                request_id: request.request_id,
                correlation_id: request.correlation_id,
                status: DiscoveryStatus::Unavailable,
                servers: Vec::new(),
            },
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_contracts::transport::{CorrelationId, RequestId};
    fn request() -> DiscoverServers {
        DiscoverServers {
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
            selected_server_id: None,
        }
    }
    #[tokio::test]
    async fn bootstrap_required_and_scan_cancellation_keeps_product_authority_out() {
        let mut discovery = DesktopDiscovery::default();
        let request = request();
        let Some(IpcServerMessage::ServerDiscovery(denied)) =
            discovery.start(request.clone(), false)
        else {
            panic!("denial expected")
        };
        assert_eq!(denied.status, DiscoveryStatus::AuthenticationRequired);
        assert!(!discovery.is_active());
        assert!(discovery.start(request.clone(), true).is_none());
        assert!(matches!(
            discovery.start(request.clone(), true),
            Some(IpcServerMessage::ServerDiscovery(ServerDiscoveryResult {
                status: DiscoveryStatus::Busy,
                ..
            }))
        ));
        discovery.cancel(&request);
        let Some(IpcServerMessage::ServerDiscovery(cancelled)) = discovery.finish().await else {
            panic!("completion expected")
        };
        assert_eq!(cancelled.status, DiscoveryStatus::Cancelled);
        assert!(cancelled.servers.is_empty());
        assert!(!discovery.is_active());
    }
}
