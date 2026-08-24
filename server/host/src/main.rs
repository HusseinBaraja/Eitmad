use std::{process::ExitCode, sync::Arc};

use eitmad_admin_plane::{AdminDatabase, AdministrationService, PostgresAdministrationDataSource};
use eitmad_contracts::server::TenantCode;
use eitmad_contracts::transport::CorrelationId;
use eitmad_control_plane::{BootstrapInput, ControlDatabase, ControlPlane};
use eitmad_relay_plane::RelayCoordinator;
use eitmad_release_policy::TrustedUpdateKeys;
use eitmad_server::{
    MetadataRelayRouter, ServerCommand, ServerConfig, ServerPlaneSecurity, ServerRelayMetrics,
    ServerState, ServerSupportExecutor,
};
use eitmad_sync_plane::{DomainRegistry, SyncCoordinator, SyncDatabase};
use eitmad_update_plane::{FileManifestRepository, UpdateCatalog};

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
    let pool_budget = eitmad_server::pool_connection_budget(config.maximum_database_connections);
    let control_database = ControlDatabase::connect(&config.database_url, pool_budget)
        .await
        .map_err(|_| MainError::Database)?;
    control_database
        .migrate()
        .await
        .map_err(|_| MainError::Migration)?;
    let sync_database = SyncDatabase::connect(&config.database_url, pool_budget)
        .await
        .map_err(|_| MainError::Database)?;
    sync_database
        .migrate()
        .await
        .map_err(|_| MainError::Migration)?;
    let admin_database = AdminDatabase::connect(&config.database_url, pool_budget)
        .await
        .map_err(|_| MainError::Database)?;
    admin_database
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
    if domains.descriptors().is_empty() {
        tracing::error!("no sync domains are registered; refusing to report ready");
        return Err(MainError::Configuration);
    }
    let sync = SyncCoordinator::new(&sync_database, domains);
    let state = compose_server_state(&config, control, sync, &admin_database)?;
    tracing::info!(listen = %config.listen, "server ready");
    eitmad_server::run(&config, state)
        .await
        .map_err(|_| MainError::Runtime)
}

fn compose_server_state(
    config: &ServerConfig,
    control: ControlPlane,
    sync: SyncCoordinator,
    admin_database: &AdminDatabase,
) -> Result<ServerState, MainError> {
    let security = Arc::new(ServerPlaneSecurity::new(control.access.clone()));
    let relay = RelayCoordinator::new(security.clone(), Arc::new(MetadataRelayRouter));
    let support = Arc::new(ServerSupportExecutor::new(
        relay.clone(),
        control.access.clone(),
    ));
    let administration = AdministrationService::new(
        security.clone(),
        Arc::new(PostgresAdministrationDataSource::new(
            admin_database.pool(),
            support,
            Arc::new(ServerRelayMetrics::new(relay.clone())),
        )),
    );
    let mut trusted_update_keys = TrustedUpdateKeys::new();
    trusted_update_keys.insert(
        config.update_signing_key_id.clone(),
        config.update_verifying_key,
    );
    let repository = FileManifestRepository::open(&config.update_manifest_directory)
        .map_err(|_| MainError::Configuration)?;
    let updates = UpdateCatalog::new(security, Arc::new(repository), trusted_update_keys);
    Ok(ServerState::new(control, sync).with_planes(relay, updates, administration))
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
