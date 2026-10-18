use eitmad_contracts::{
    identity::{AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind},
    transport::SchemaId,
};
use eitmad_engine_runtime::{DesktopAccountError, local_ipc::EventBroker};
use eitmad_server_connection::{
    DesktopConnectionSettings, DirectAccountClient, DirectServerConfig,
};
use eitmad_storage::AuthorityStore;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::watch;

pub(super) struct ConnectedAccounts {
    pub client: Arc<DirectAccountClient>,
    store: AuthorityStore,
    events: EventBroker,
}

pub(super) fn configure(
    store: &AuthorityStore,
    directory: &Path,
    settings: Option<&DesktopConnectionSettings>,
    events: &EventBroker,
) -> Result<Option<ConnectedAccounts>, ()> {
    let Some(settings) = settings else {
        return Ok(None);
    };
    let owner = store
        .existing_local_authorization_context()
        .map_err(|_| ())?;
    let config = DirectServerConfig::new(
        &settings.endpoint,
        owner.scope,
        SchemaId::parse("eitmad.schema.desktop-accounts.v1").map_err(|_| ())?,
        1,
        &settings.trusted_certificate,
    )
    .map_err(|_| ())?;
    Ok(Some(ConnectedAccounts {
        client: Arc::new(DirectAccountClient::from_config(
            config,
            eitmad_secret_storage::SecretStore::open(directory, None).map_err(|_| ())?,
            settings.credential.clone(),
            settings.organization,
            settings.branch,
        )),
        store: store.clone(),
        events: events.clone(),
    }))
}

impl ConnectedAccounts {
    pub fn start(&self, mut cancel: watch::Receiver<bool>) -> tokio::task::JoinHandle<()> {
        let client = self.client.clone();
        let store = self.store.clone();
        let events = self.events.clone();
        tokio::spawn(async move {
            let mut workers = std::collections::HashMap::new();
            while !*cancel.borrow() {
                if let Ok(owner) = store.existing_local_authorization_context() {
                    if let Ok(page) = store.list_desktop_accounts(owner.tenant_id) {
                        for account in page.accounts.into_iter().filter(|a| a.active) {
                            if workers
                                .get(&account.user_id)
                                .is_some_and(|h: &tokio::task::JoinHandle<()>| !h.is_finished())
                            {
                                continue;
                            }
                            let actor = AuthorizationContext {
                                identity: AuthenticatedIdentity {
                                    principal_id: PrincipalId::new(account.user_id.value()),
                                    principal_kind: PrincipalKind::User,
                                    device_id: owner.identity.device_id,
                                    service_id: None,
                                },
                                ..owner.clone()
                            };
                            let client = client.clone();
                            let store = store.clone();
                            let events = events.clone();
                            let mut stopping = cancel.clone();
                            workers.insert(account.user_id, tokio::spawn(async move {
                                let mut revision = 0;
                                while !*stopping.borrow() {
                                    let client = client.clone(); let actor = actor.clone();
                                    let checking = actor.clone();
                                    let result = tokio::task::spawn_blocking(move || client.watch(&checking, revision)).await;
                                    if matches!(result, Ok(Err(DesktopAccountError::Denied))) {
                                        if store.revoke_connected_desktop_account(&actor, account.revision, eitmad_control_now()).is_ok() {
                                            events.policy_changed(actor.scope);
                                        }
                                        break;
                                    }
                                    if let Ok(Ok(update)) = &result {
                                        if update.revision != revision {
                                            revision = update.revision;
                                            let _ = events.publish(actor.scope.clone(), eitmad_contracts::events::Event::DesktopAccountChanged(
                                                eitmad_contracts::accounts::DesktopAccountChangeNotice { scope: actor.scope.clone(), revision }));
                                        }
                                    }
                                    if !matches!(result, Ok(Ok(_))) {
                                        tokio::select! { ()=tokio::time::sleep(Duration::from_secs(2))=>(), _=stopping.changed()=>break }
                                    }
                                }
                            }));
                        }
                    }
                }
                tokio::select! { ()=tokio::time::sleep(Duration::from_secs(1))=>(), _=cancel.changed()=>break }
            }
            for (_, worker) in workers {
                let _ = worker.await;
            }
        })
    }
}

fn eitmad_control_now() -> eitmad_contracts::transport::UnixMillis {
    eitmad_contracts::transport::UnixMillis(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX)),
    )
}
