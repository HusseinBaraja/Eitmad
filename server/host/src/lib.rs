//! Composition root for the initially combined Eitmad server.

mod config;
mod http;
mod planes;

pub use config::{
    MigrationConfig, ServerCommand, ServerConfig, ServerConfigError, pool_connection_budget,
};
pub use http::{ServerState, router, run};
pub use planes::{
    MetadataRelayRouter, ServerPlaneSecurity, ServerRelayMetrics, ServerSupportExecutor,
};
