use std::process::ExitCode;

use eitmad_contracts::server::TenantCode;
use eitmad_contracts::transport::CorrelationId;
use eitmad_control_plane::{BootstrapInput, ControlDatabase, ControlPlane};
use eitmad_server::{ServerCommand, ServerConfig, ServerState};
use eitmad_sync_plane::{DomainRegistry, SyncCoordinator, SyncDatabase};

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .without_time()
        .with_target(false)
        .init();
    match execute().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error = error.code(), "server command failed");
            ExitCode::FAILURE
        }
    }
}

async fn execute() -> Result<(), MainError> {
    let arguments = std::env::args().collect::<Vec<_>>();
    let command = ServerCommand::from_arguments(&arguments);
    if command == ServerCommand::Help {
        print_help();
        return Ok(());
    }
    if command == ServerCommand::Usage {
        print_help();
        return Err(MainError::Input);
    }
    let config = ServerConfig::from_environment().map_err(|_| MainError::Configuration)?;
    if command == ServerCommand::CheckConfig {
        println!("server configuration is valid");
        return Ok(());
    }
    let control_database =
        ControlDatabase::connect(&config.database_url, config.maximum_database_connections)
            .await
            .map_err(|_| MainError::Database)?;
    control_database
        .migrate()
        .await
        .map_err(|_| MainError::Migration)?;
    let sync_database =
        SyncDatabase::connect(&config.database_url, config.maximum_database_connections)
            .await
            .map_err(|_| MainError::Database)?;
    sync_database
        .migrate()
        .await
        .map_err(|_| MainError::Migration)?;
    if command == ServerCommand::Migrate {
        println!("server migrations are current");
        return Ok(());
    }
    let control = ControlPlane::new(control_database.pool(), config.token_key.clone());
    if let ServerCommand::Bootstrap {
        tenant_code,
        tenant_name,
        organization_name,
        owner_username,
    } = command
    {
        let result = control
            .identity
            .bootstrap(
                &BootstrapInput {
                    tenant_code: TenantCode::parse(tenant_code).map_err(|_| MainError::Input)?,
                    tenant_display_name: tenant_name,
                    organization_display_name: organization_name,
                    owner_username,
                },
                CorrelationId::new(uuid::Uuid::new_v4()),
                eitmad_control_plane::unix_millis_now(),
            )
            .await
            .map_err(|_| MainError::Bootstrap)?;
        println!("tenantId={}", result.tenant_id.value());
        println!("organizationId={}", result.organization_id.value());
        println!("accountId={}", result.account_id.value());
        println!("activationToken={}", result.invite_token);
        println!("expiresAt={}", result.expires_at.0);
        return Ok(());
    }
    let domains = DomainRegistry::new(std::iter::empty()).map_err(|_| MainError::Configuration)?;
    let sync = SyncCoordinator::new(&sync_database, domains);
    let state = ServerState::new(control, sync);
    tracing::info!(listen = %config.listen, "server ready");
    eitmad_server::run(&config, state)
        .await
        .map_err(|_| MainError::Runtime)
}

fn print_help() {
    println!("eitmad-server [serve|migrate|check-config]");
    println!(
        "eitmad-server bootstrap <tenant-code> <tenant-name> <organization-name> <owner-username>"
    );
}

#[derive(Clone, Copy, Debug)]
enum MainError {
    Configuration,
    Input,
    Database,
    Migration,
    Bootstrap,
    Runtime,
}

impl MainError {
    const fn code(self) -> &'static str {
        match self {
            Self::Configuration => "eitmad.error.server-config-invalid.v1",
            Self::Input => "eitmad.error.contract-invalid.v1",
            Self::Database => "eitmad.error.server-database-unavailable.v1",
            Self::Migration => "eitmad.error.server-migration-failed.v1",
            Self::Bootstrap => "eitmad.error.server-bootstrap-failed.v1",
            Self::Runtime => "eitmad.error.server-runtime-failed.v1",
        }
    }
}
