//! Canonical, versioned contracts for every external boundary.
//!
//! Rust definitions in this crate are the only source of protocol shapes and
//! identifiers. Native clients consume generated bindings and never duplicate
//! wire names by hand.

#[macro_use]
mod macros;

pub mod administration;
pub mod authorization;
pub mod background_jobs;
pub mod catalog;
pub mod commands;
pub mod config;
pub mod errors;
pub mod events;
pub mod identity;
pub mod ipc;
pub mod notifications;
pub mod observability;
pub mod permissions;
pub mod queries;
pub mod reference_marker;
pub mod relay;
pub mod runtime;
pub mod secrets;
pub mod server;
pub mod sync;
pub mod sync_transport;
pub mod transport;
pub mod updates;
pub mod versioning;

pub use transport::PROTOCOL_VERSION;
