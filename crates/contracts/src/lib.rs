//! Canonical, versioned contracts for every external boundary.
//!
//! Rust definitions in this crate are the only source of protocol shapes and
//! identifiers. Native clients consume generated bindings and never duplicate
//! wire names by hand.

#[macro_use]
mod macros;

pub mod accounts;
pub mod administration;
pub mod authorization;
pub mod catalog;
pub mod catalog_image;
pub mod catalog_revision;
pub mod commands;
pub mod config;
pub mod customer;
pub mod errors;
pub mod events;
pub mod furniture;
pub mod identity;
pub mod ipc;
pub mod material;
pub mod observability;
pub mod part;
pub mod permissions;
pub mod pricing;
pub mod product;
pub mod queries;
pub mod quotation;
pub mod quotation_approval;
pub mod quotation_draft;
pub mod relay;
pub mod runtime;
pub mod sales_catalog;
pub mod secrets;
pub mod server;
pub mod sync;
pub mod sync_transport;
pub mod transport;
pub mod updates;
pub mod versioning;

pub use transport::PROTOCOL_VERSION;
