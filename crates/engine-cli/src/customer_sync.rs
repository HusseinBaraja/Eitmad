use eitmad_contracts::{
    identity::{ScopeId, ScopeKind, ScopeRef},
    transport::SchemaId,
};
use eitmad_engine_runtime::ProductDispatcher;
use eitmad_secret_storage::SecretStore;
use eitmad_server_connection::{DirectCustomerSyncClient, DirectServerConfig};
use eitmad_storage::AuthorityStore;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::watch;

pub(super) fn configure(
    store: &AuthorityStore,
    directory: &Path,
) -> Result<Option<Arc<DirectCustomerSyncClient>>, ()> {
    let Some(endpoint) = std::env::var_os("EITMAD_CUSTOMER_SERVER") else {
        return Ok(None);
    };
    let trust = std::env::var_os("EITMAD_CUSTOMER_TRUST_PEM").ok_or(())?;
    let credential = std::env::var("EITMAD_CUSTOMER_CREDENTIAL_ID").map_err(|_| ())?;
    let branch = std::env::var("EITMAD_CUSTOMER_BRANCH_ID")
        .map_err(|_| ())?
        .parse()
        .map_err(|_| ())?;
    let config = DirectServerConfig::new(
        endpoint.to_str().ok_or(())?,
        ScopeRef {
            kind: ScopeKind::parse("branch").map_err(|_| ())?,
            id: ScopeId::new(branch),
        },
        SchemaId::parse(eitmad_customer::CUSTOMER_SCHEMA_ID).map_err(|_| ())?,
        1,
        &PathBuf::from(trust),
    )
    .map_err(|_| ())?;
    Ok(Some(Arc::new(DirectCustomerSyncClient::from_config(
        config,
        SecretStore::open(directory, None).map_err(|_| ())?,
        serde_json::from_str(&credential).map_err(|_| ())?,
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
