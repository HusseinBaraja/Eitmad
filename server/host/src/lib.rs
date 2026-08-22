//! Composition root for the initially combined Eitmad server.

mod config;
mod http;

pub use config::{ServerCommand, ServerConfig, ServerConfigError};
pub use http::{ServerState, router, run};
