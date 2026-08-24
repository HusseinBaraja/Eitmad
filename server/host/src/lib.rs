//! Composition root for the initially combined Eitmad server.

mod config;
mod http;

pub use config::{ServerCommand, ServerConfig, ServerConfigError, pool_connection_budget};
pub use http::{ServerState, router, run};
