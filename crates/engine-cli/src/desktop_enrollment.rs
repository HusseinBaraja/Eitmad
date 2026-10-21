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

pub(super) fn check_recovery_access(
    directory: &std::path::Path,
    actor: &eitmad_contracts::identity::AuthorizationContext,
) -> Result<(), eitmad_engine_runtime::RecoveryError> {
    use eitmad_engine_runtime::{DesktopAuthenticator, RecoveryError};
    let store = AuthorityStore::open_existing(directory).map_err(|_| RecoveryError::Unavailable)?;
    let check = || {
        let settings = eitmad_server_connection::DesktopConnectionSettings::load(directory)
            .map_err(|_| DesktopAuthenticationError::Unavailable)?;
        // Only unconnected debug fixtures retain local maintenance authentication.
        if cfg!(debug_assertions)
            && settings.is_none()
            && !directory
                .join("desktop-device.json")
                .try_exists()
                .map_err(|_| DesktopAuthenticationError::Unavailable)?
        {
            return Ok(());
        }
        DesktopOnlineAuthentication {
            directory: directory.into(),
            store: store.clone(),
            routes_configured: true,
        }
        .check_access(actor)
    };
    check().map_err(|error| {
        let _ = DesktopAuthenticator::new(store).sign_out(
            actor,
            eitmad_contracts::transport::CorrelationId::new(uuid::Uuid::new_v4()),
            crate::current_time(),
        );
        match error {
            DesktopAuthenticationError::Failed => RecoveryError::Denied,
            DesktopAuthenticationError::Unavailable => RecoveryError::Unavailable,
        }
    })
}

impl DesktopServerAuthentication for DesktopOnlineAuthentication {
    fn onboarding(
        &self,
        action: &eitmad_contracts::onboarding::DesktopOnboardingAction,
        actor: Option<&eitmad_contracts::identity::AuthorizationContext>,
    ) -> Result<eitmad_contracts::onboarding::DesktopOnboardingState, DesktopAuthenticationError>
    {
        use eitmad_contracts::onboarding::{
            DesktopOnboardingAction as Action, DesktopOnboardingState,
        };

        let secrets = eitmad_secret_storage::SecretStore::open(&self.directory, None)
            .map_err(|_| DesktopAuthenticationError::Failed)?;
        let enrollment = eitmad_server_connection::DesktopEnrollment::new(
            self.directory.clone(),
            self.store.clone(),
            secrets,
        );
        let mut state = DesktopOnboardingState {
            paired: enrollment
                .paired_connection()
                .map_err(|_| DesktopAuthenticationError::Failed)?
                .is_some()
                || eitmad_server_connection::DesktopConnectionSettings::load(&self.directory)
                    .map_err(|_| DesktopAuthenticationError::Failed)?
                    .is_some(),
            code: None,
            expires_at: None,
            pending_pairing_server: None,
        };
        match action {
            Action::State => {
                if !state.paired {
                    state.pending_pairing_server = enrollment
                        .pending_pairing_server()
                        .map_err(|_| DesktopAuthenticationError::Failed)?;
                }
            }
            Action::Pair { candidate, code } => {
                if actor.is_some() || state.paired {
                    return Err(DesktopAuthenticationError::Failed);
                }
                enrollment
                    .pair(candidate, code)
                    .map_err(|_| DesktopAuthenticationError::Failed)?;
                state.paired = true;
                state.pending_pairing_server = None;
            }
            Action::DiscardPendingPairing => {
                if actor.is_some() || state.paired {
                    return Err(DesktopAuthenticationError::Failed);
                }
                enrollment
                    .discard_pending_pairing()
                    .map_err(|_| DesktopAuthenticationError::Failed)?;
                state.pending_pairing_server = None;
            }
            Action::OpenPairing => {
                let actor = actor.ok_or(DesktopAuthenticationError::Failed)?;
                self.check_access(actor)?;
                let settings =
                    eitmad_server_connection::DesktopConnectionSettings::load(&self.directory)
                        .map_err(|_| DesktopAuthenticationError::Failed)?;
                let client =
                    crate::accounts::client(&self.store, &self.directory, settings.as_ref())
                        .map_err(|()| DesktopAuthenticationError::Failed)?
                        .ok_or(DesktopAuthenticationError::Unavailable)?;
                let opened = client
                    .open_pairing(actor)
                    .map_err(|_| DesktopAuthenticationError::Failed)?;
                state.code = Some(opened.code);
                state.expires_at = Some(opened.offer.expires_at);
            }
        }
        Ok(state)
    }
    fn check_access(
        &self,
        authorization: &eitmad_contracts::identity::AuthorizationContext,
    ) -> Result<(), DesktopAuthenticationError> {
        let settings = eitmad_server_connection::DesktopConnectionSettings::load(&self.directory)
            .map_err(|_| DesktopAuthenticationError::Unavailable)?;
        let client = crate::accounts::client(&self.store, &self.directory, settings.as_ref())
            .map_err(|()| DesktopAuthenticationError::Unavailable)?
            .ok_or(DesktopAuthenticationError::Unavailable)?;
        // The profile route verifies the account and this device's installation assignment.
        let mut actor = authorization.clone();
        actor.scope.kind = eitmad_contracts::identity::ScopeKind::parse("organization")
            .map_err(|_| DesktopAuthenticationError::Unavailable)?;
        actor.scope.id = eitmad_contracts::identity::ScopeId::new(actor.tenant_id.value());
        client
            .watch(&actor, 0)
            .map(|_| ())
            .map_err(|error| match error {
                eitmad_engine_runtime::DesktopAccountError::Denied => {
                    DesktopAuthenticationError::Failed
                }
                _ => DesktopAuthenticationError::Unavailable,
            })
    }

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
