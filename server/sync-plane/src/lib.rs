//! Server synchronization-plane authority.

mod boundary_audit;
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
pub use operations::{
    AcknowledgeRequest, OperationError, OperationResult, PullPageRequest, SyncCoordinator,
};
pub use snapshots::{SnapshotBundle, SnapshotError, SnapshotRequest};
pub use subscriptions::{SubscriptionError, SubscriptionPage};
