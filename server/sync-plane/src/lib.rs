//! Server synchronization-plane authority.

mod catalog_image;
mod catalog_revision;
mod pricing;
pub use catalog_image::CatalogImageServer;
pub use pricing::PricingServer;
mod boundary_audit;
mod customer;
mod database;
mod domain;
mod operations;
mod snapshots;
mod subscriptions;

pub use customer::CustomerSyncHandler;
pub use database::{SyncDatabase, SyncDatabaseError};
pub use domain::{
    AuthoritativeChangeDraft, DomainDescriptor, DomainRegistry, DomainRegistryError,
    DomainSyncHandler, DomainValidationError, LocalOperationDraft, SyncIntent,
};
pub use operations::{
    AcknowledgeRequest, OperationError, OperationResult, PullPageRequest, SyncCoordinator,
};
pub use snapshots::{SnapshotBundle, SnapshotError, SnapshotRequest};
pub use subscriptions::{SubscriptionError, SubscriptionPage, SubscriptionPageRequest};
