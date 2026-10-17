use super::*;
use std::sync::Mutex;

// Native keyrings outlive TempDir. Retain exact test-created IDs until all live
// scenarios in this process finish; parallel scenarios must not delete each other's sessions.
struct Registry {
    active: usize,
    entries: Vec<(SecretStore, SecretId)>,
}
static CREDENTIALS: Mutex<Registry> = Mutex::new(Registry {
    active: 0,
    entries: Vec::new(),
});

pub(super) struct TestCredentials;
impl TestCredentials {
    pub(super) fn new() -> Self {
        CREDENTIALS.lock().unwrap().active += 1;
        Self
    }
}
impl Drop for TestCredentials {
    fn drop(&mut self) {
        let mut registry = CREDENTIALS.lock().unwrap();
        registry.active -= 1;
        if registry.active == 0 {
            for (store, id) in registry.entries.drain(..) {
                let result = store.delete(&id);
                if !std::thread::panicking() {
                    assert!(result.is_ok(), "Cannot remove a disposable test credential");
                }
            }
        }
    }
}

pub(super) fn store_session(
    store: &SecretStore,
    id: &SecretId,
    authentication: AuthenticationResult,
    seed: [u8; 32],
) -> Result<(), eitmad_sync::TransportFailure> {
    eitmad_server_connection::store_session(store, id, authentication, seed)?;
    CREDENTIALS
        .lock()
        .unwrap()
        .entries
        .push((store.clone(), id.clone()));
    Ok(())
}

pub(super) fn store_account_session(
    store: &SecretStore,
    route: &SecretId,
    authentication: AuthenticationResult,
    seed: [u8; 32],
) -> Result<(), eitmad_sync::TransportFailure> {
    let id = eitmad_server_connection::account_credential_id(
        route,
        PrincipalId::new(authentication.session.user_id.value()),
    );
    store_session(store, &id, authentication, seed)
}
