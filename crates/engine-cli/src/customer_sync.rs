use eitmad_contracts::{
    identity::{ScopeKind, ScopeRef},
    transport::SchemaId,
};
use eitmad_engine_runtime::ProductDispatcher;
use eitmad_secret_storage::SecretStore;
use eitmad_server_connection::{DirectCustomerSyncClient, DirectServerConfig};
use eitmad_storage::AuthorityStore;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::watch;

pub(super) fn configure(
    store: &AuthorityStore,
    directory: &Path,
    installation: Option<&eitmad_server_connection::DesktopConnectionSettings>,
) -> Result<Option<Arc<DirectCustomerSyncClient>>, ()> {
    let Some(route) = crate::connection::ServerRoute::load("CUSTOMER", installation)? else {
        return Ok(None);
    };
    let branch = crate::connection::branch("CUSTOMER", installation)?;
    let config = DirectServerConfig::new(
        &route.endpoint,
        ScopeRef {
            kind: ScopeKind::parse("branch").map_err(|_| ())?,
            id: branch,
        },
        SchemaId::parse(eitmad_customer::CUSTOMER_SCHEMA_ID).map_err(|_| ())?,
        1,
        &route.trust,
    )
    .map_err(|_| ())?;
    Ok(Some(Arc::new(DirectCustomerSyncClient::from_config(
        config,
        SecretStore::open(directory, None).map_err(|_| ())?,
        route.credential,
        store.clone(),
    ))))
}

pub(super) fn start(
    client: Arc<DirectCustomerSyncClient>,
    dispatcher: Arc<ProductDispatcher>,
    mut cancel: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while !*cancel.borrow() {
            let client = client.clone();
            let dispatcher = dispatcher.clone();
            let result = tokio::task::spawn_blocking(move || {
                // Incoming history must run even when this installation has never edited
                // a customer. Upload failures must not disable this independent read path.
                let _ = client.receive_incoming();
                let result = client.retry_pending();
                // Projection can commit before a later exchange fails.
                let _ = dispatcher.drain_pending_publications();
                result
            })
            .await;
            if matches!(result, Ok(Ok(count)) if count > 0) {
                continue;
            }
            tokio::select! {
                () = tokio::time::sleep(Duration::from_secs(2)) => (),
                _ = cancel.changed() => break,
            }
        }
    })
}
