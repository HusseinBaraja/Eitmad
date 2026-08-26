//! Runtime ownership and diagnostic health for the `SQLite` authority store.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use eitmad_contracts::runtime::{HealthCheckId, HealthCheckImpact, HealthStatus};
use eitmad_storage::{AuthorityStore, CorruptionCheck, DATABASE_FILE_NAME};

use crate::{ComponentFailure, ComponentFuture, HealthCheck, HealthCheckFuture, RuntimeComponent};

#[derive(Clone, Default)]
pub struct AuthorityStoreHandle {
    store: Arc<Mutex<Option<AuthorityStore>>>,
}

impl AuthorityStoreHandle {
    /// Returns the migrated store after its runtime component has started.
    ///
    /// # Errors
    ///
    /// Returns a sanitized component failure before startup or after shutdown.
    pub fn store(&self) -> Result<AuthorityStore, ComponentFailure> {
        self.store
            .lock()
            .map_err(|_| ComponentFailure::new())?
            .clone()
            .ok_or_else(ComponentFailure::new)
    }
}

pub struct AuthorityStoreComponent {
    runtime_directory: PathBuf,
    handle: AuthorityStoreHandle,
}

impl AuthorityStoreComponent {
    #[must_use]
    pub fn new(runtime_directory: impl Into<PathBuf>, handle: AuthorityStoreHandle) -> Self {
        Self {
            runtime_directory: runtime_directory.into(),
            handle,
        }
    }
}

impl RuntimeComponent for AuthorityStoreComponent {
    fn start(&mut self) -> ComponentFuture<'_> {
        Box::pin(async move {
            let store = AuthorityStore::open(&self.runtime_directory)
                .map_err(|_| ComponentFailure::new())?;
            *self
                .handle
                .store
                .lock()
                .map_err(|_| ComponentFailure::new())? = Some(store);
            Ok(())
        })
    }

    fn stop(&mut self) -> ComponentFuture<'_> {
        Box::pin(async move {
            *self
                .handle
                .store
                .lock()
                .map_err(|_| ComponentFailure::new())? = None;
            Ok(())
        })
    }
}

pub struct AuthorityStoreHealthCheck {
    source: AuthorityStoreHealthSource,
}

enum AuthorityStoreHealthSource {
    Diagnostic(PathBuf),
    Started(AuthorityStoreHandle),
}

impl AuthorityStoreHealthCheck {
    #[must_use]
    pub fn new(runtime_directory: impl Into<PathBuf>) -> Self {
        Self {
            source: AuthorityStoreHealthSource::Diagnostic(runtime_directory.into()),
        }
    }

    #[must_use]
    pub const fn started(handle: AuthorityStoreHandle) -> Self {
        Self {
            source: AuthorityStoreHealthSource::Started(handle),
        }
    }
}

impl HealthCheck for AuthorityStoreHealthCheck {
    fn id(&self) -> HealthCheckId {
        HealthCheckId::parse("eitmad.health.authority-store.v1")
            .expect("authority store health ID is valid")
    }

    fn impact(&self) -> HealthCheckImpact {
        HealthCheckImpact::RequiredForReadiness
    }

    fn check(&self) -> HealthCheckFuture<'_> {
        Box::pin(async move {
            match &self.source {
                AuthorityStoreHealthSource::Diagnostic(runtime_directory) => {
                    let database = runtime_directory.join(DATABASE_FILE_NAME);
                    if !database.exists()
                        || AuthorityStore::check_compatible(runtime_directory).is_ok()
                    {
                        HealthStatus::Healthy
                    } else {
                        HealthStatus::Unhealthy
                    }
                }
                AuthorityStoreHealthSource::Started(handle) => handle
                    .store()
                    .and_then(|store| {
                        store
                            .verify_integrity(CorruptionCheck::Quick)
                            .map_err(|_| ComponentFailure::new())
                    })
                    .map_or(HealthStatus::Unhealthy, |()| HealthStatus::Healthy),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn runtime_component_opens_and_releases_the_migrated_store() {
        let directory = TempDir::new().unwrap();
        let handle = AuthorityStoreHandle::default();
        let mut component = AuthorityStoreComponent::new(directory.path(), handle.clone());
        assert!(handle.store().is_err());
        component.start().await.unwrap();
        assert!(handle.store().unwrap().path().is_file());
        component.stop().await.unwrap();
        assert!(handle.store().is_err());
    }

    #[tokio::test]
    async fn diagnostic_check_is_read_only_and_accepts_an_absent_store() {
        let directory = TempDir::new().unwrap();
        let check = AuthorityStoreHealthCheck::new(directory.path());
        assert_eq!(check.check().await, HealthStatus::Healthy);
        assert!(!directory.path().join(DATABASE_FILE_NAME).exists());
    }

    #[tokio::test]
    async fn started_check_reuses_the_open_store_without_migration_preflight() {
        let directory = TempDir::new().unwrap();
        let handle = AuthorityStoreHandle::default();
        let mut component = AuthorityStoreComponent::new(directory.path(), handle.clone());
        component.start().await.unwrap();
        let check = AuthorityStoreHealthCheck::started(handle);

        assert_eq!(check.check().await, HealthStatus::Healthy);
    }
}
