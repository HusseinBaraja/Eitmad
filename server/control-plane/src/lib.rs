//! Server control-plane authority.
//!
//! This crate owns remote identity, authentication, registered devices,
//! license state, and update-channel assignment. The deployable host composes
//! it with the sync plane but does not reach into its private modules.

mod access;
mod audit;
mod authentication;
mod database;
mod identity;
mod licensing;
mod update_assignment;

pub use access::{
    AccessError, AccessRequirement, ServerAccessService, ServerAuditEntry, ServerAuditOutcome,
};
pub use authentication::{AuthenticationError, AuthenticationService, TokenKey, unix_millis_now};
pub use database::{ControlDatabase, ControlDatabaseError};
pub use identity::{
    BootstrapInput, BootstrapResult, IdentityError, IdentityService, NotificationDelivery,
    NotificationSink,
};
pub use licensing::{LicenseDecision, LicenseError, LicenseService};
pub use update_assignment::{UpdateAssignmentError, UpdateAssignmentService};

use std::sync::Arc;

use sqlx::PgPool;

#[derive(Clone)]
pub struct ControlPlane {
    pub access: ServerAccessService,
    pub authentication: AuthenticationService,
    pub identity: IdentityService,
    pub licensing: LicenseService,
    pub update_assignments: UpdateAssignmentService,
}

impl ControlPlane {
    #[must_use]
    pub fn new(pool: PgPool, token_key: TokenKey) -> Self {
        Self {
            access: ServerAccessService::new(pool.clone()),
            authentication: AuthenticationService::new(pool.clone(), token_key.clone()),
            identity: IdentityService::new(pool.clone(), token_key),
            licensing: LicenseService::new(pool.clone()),
            update_assignments: UpdateAssignmentService::new(pool),
        }
    }

    #[must_use]
    pub fn with_notification_sink(mut self, sink: Arc<dyn NotificationSink>) -> Self {
        self.identity = self.identity.with_notification_sink(sink);
        self
    }
}
