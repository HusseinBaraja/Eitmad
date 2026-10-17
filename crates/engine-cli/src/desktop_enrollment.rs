use eitmad_engine_runtime::{DesktopAuthenticationError, DesktopServerAuthentication};
use eitmad_storage::AuthorityStore;
use std::path::PathBuf;

pub(super) struct DesktopOnlineAuthentication {
    pub directory: PathBuf,
    pub store: AuthorityStore,
    routes_configured: bool,
}

pub(super) fn authenticator(
    directory: &std::path::Path,
    store: &AuthorityStore,
) -> eitmad_engine_runtime::DesktopAuthenticator {
    eitmad_engine_runtime::DesktopAuthenticator::new(store.clone()).with_server(
        std::sync::Arc::new(DesktopOnlineAuthentication {
            directory: directory.into(),
            store: store.clone(),
            routes_configured: eitmad_server_connection::DesktopConnectionSettings::load(directory)
                .ok()
                .flatten()
                .is_some(),
        }),
    )
}

impl DesktopServerAuthentication for DesktopOnlineAuthentication {
    fn sign_in(
        &self,
        request: &eitmad_contracts::ipc::DesktopSignInRequest,
    ) -> Result<bool, DesktopAuthenticationError> {
        let secrets = eitmad_secret_storage::SecretStore::open(&self.directory, None)
            .map_err(|_| DesktopAuthenticationError::Unavailable)?;
        eitmad_server_connection::DesktopEnrollment::new(
            self.directory.clone(),
            self.store.clone(),
            secrets,
        )
        .sign_in(request)
        .map(|restart| restart || !self.routes_configured)
        .map_err(|_| DesktopAuthenticationError::Failed)
    }
}
