//! Server control-plane authority.
//!
//! This crate owns remote identity, authentication, registered devices,
//! and update-channel assignment. The deployable host composes
//! it with the sync plane but does not reach into its private modules.

mod access;
mod authentication;
mod branches;
mod database;
mod identity;
mod update_assignment;

pub use access::{AccessError, AccessRequirement, ServerAccessService};
pub use authentication::{AuthenticationError, AuthenticationService, TokenKey, unix_millis_now};
pub use branches::{BranchError, BranchService};
pub use database::{ControlDatabase, ControlDatabaseError};
pub use identity::{BootstrapInput, BootstrapResult, IdentityError, IdentityService};
pub use update_assignment::{UpdateAssignmentError, UpdateAssignmentService};

use sqlx::PgPool;

#[derive(Clone)]
pub struct ControlPlane {
    pub access: ServerAccessService,
    pub authentication: AuthenticationService,
    pub branches: BranchService,
    pub identity: IdentityService,
    pub update_assignments: UpdateAssignmentService,
}

impl ControlPlane {
    #[must_use]
    pub fn new(pool: PgPool, token_key: TokenKey) -> Self {
        Self {
            access: ServerAccessService::new(pool.clone()),
            authentication: AuthenticationService::new(pool.clone(), token_key.clone()),
            branches: BranchService::new(pool.clone()),
            identity: IdentityService::new(pool.clone(), token_key),
            update_assignments: UpdateAssignmentService::new(pool),
        }
    }
}
