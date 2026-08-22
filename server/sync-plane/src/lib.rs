//! Server synchronization-plane authority.

mod audit;
mod database;
mod domain;
mod operations;
mod snapshots;
mod subscriptions;

pub use database::{SyncDatabase, SyncDatabaseError};
pub use domain::{
    AuthoritativeChangeDraft, CommandSubmission, DomainDescriptor, DomainRegistry,
    DomainRegistryError, DomainSyncHandler, DomainValidationError, LocalOperationDraft, SyncIntent,
};
pub use operations::{OperationError, OperationResult, SyncCoordinator};
pub use snapshots::{SnapshotBundle, SnapshotError};
pub use subscriptions::{SubscriptionError, SubscriptionPage};
