use super::{MainError, PrepareWindowsWorkshop, Secrets, write_new};
use base64::Engine as _;
use eitmad_contracts::setup::InitializedWorkshop;
use eitmad_postgres_support as sqlx;
use sqlx::ConnectOptions as _;
use std::path::Path;
use zeroize::Zeroizing;

pub(super) fn retain_secrets(
    request: &PrepareWindowsWorkshop,
    secrets: &Secrets,
) -> Result<(), MainError> {
    let root = Path::new(&request.state_directory);
    write_new(
        &root.join("certificate-password"),
        secrets.certificate.as_bytes(),
    )?;
    write_new(
        &root.join("postgres-admin-password"),
        secrets.admin.as_bytes(),
    )?;
    write_new(
        &root.join("application-password"),
        secrets.application.as_bytes(),
    )?;
    write_new(&root.join("token-key"), secrets.token.as_bytes())
}

pub(super) async fn provision(
    request: &PrepareWindowsWorkshop,
    secrets: &Secrets,
) -> Result<Zeroizing<String>, MainError> {
    let admin_url = Zeroizing::new(format!(
        "postgresql://postgres:{}@127.0.0.1:{}/postgres",
        secrets.admin.as_str(),
        request.database_port
    ));
    let options = admin_url
        .parse::<sqlx::PgConnectOptions>()
        .map_err(|_| MainError::Setup)?
        .disable_statement_logging();
    let pool = sqlx::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|_| MainError::Setup)?;
    // Password material is generated from a fixed URL-safe alphabet, never from user SQL input.
    let statement = Zeroizing::new(format!(
        "CREATE ROLE eitmad_app LOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE NOREPLICATION PASSWORD '{}'",
        secrets.application.as_str()
    ));
    let result = async {
        sqlx::query(&statement)
            .execute(&pool)
            .await
            .map_err(|_| MainError::Setup)?;
        sqlx::query("CREATE DATABASE eitmad OWNER eitmad_app ENCODING 'UTF8' TEMPLATE template0")
            .execute(&pool)
            .await
            .map_err(|_| MainError::Setup)?;
        sqlx::query("REVOKE ALL ON DATABASE eitmad FROM PUBLIC")
            .execute(&pool)
            .await
            .map_err(|_| MainError::Setup)?;
        Ok(())
    }
    .await;
    pool.close().await;
    result?;
    Ok(Zeroizing::new(format!(
        "postgresql://eitmad_app:{}@127.0.0.1:{}/eitmad",
        secrets.application.as_str(),
        request.database_port
    )))
}

pub(super) fn write_settings(
    request: &PrepareWindowsWorkshop,
    secrets: &Secrets,
    database_url: &str,
    workshop: &InitializedWorkshop,
    endpoint: &str,
) -> Result<(), MainError> {
    let root = Path::new(&request.state_directory);
    let manifests = root.join("manifests");
    std::fs::create_dir(&manifests).map_err(|_| MainError::Setup)?;
    // Offline setup creates no update assignment or signing authority. An empty catalog stays empty.
    let mut update_seed = Zeroizing::new([0; 32]);
    rand::RngCore::fill_bytes(&mut rand::rng(), update_seed.as_mut());
    let update_key = ed25519_dalek::SigningKey::from_bytes(&update_seed).verifying_key();
    let public_key = base64::engine::general_purpose::STANDARD.encode(update_key.as_bytes());
    let settings = Zeroizing::new(serde_json::to_vec(&serde_json::json!({
        "version": 1, "executable": std::env::current_exe().map_err(|_| MainError::Setup)?,
        "address": request.listen_address, "port": request.server_port, "endpoint": endpoint,
        "environment": {
            "EITMAD_SERVER_DATABASE_URL": database_url, "EITMAD_SERVER_TOKEN_KEY": secrets.token.as_str(),
            "EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY": manifests,
            "EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID": workshop.tenant_id.value(),
            "EITMAD_SERVER_UPDATE_KEY_ID": "eitmad.offline.unassigned.v1",
            "EITMAD_SERVER_UPDATE_PUBLIC_KEY": public_key, "EITMAD_SERVER_MAX_CONNECTIONS": "12",
        }
    })).map_err(|_| MainError::Setup)?);
    write_new(&root.join("server.json"), &settings)
}
