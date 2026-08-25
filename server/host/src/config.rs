use std::{net::SocketAddr, path::PathBuf};

use ed25519_dalek::VerifyingKey;
use eitmad_contracts::identity::TenantId;
use eitmad_contracts::updates::UpdateSigningKeyId;
use eitmad_control_plane::TokenKey;

#[derive(Clone)]
pub struct ServerConfig {
    pub database_url: String,
    pub token_key: TokenKey,
    pub listen: SocketAddr,
    pub tls_certificate: Option<PathBuf>,
    pub tls_private_key: Option<PathBuf>,
    pub allow_insecure_loopback: bool,
    pub maximum_database_connections: u32,
    pub update_manifest_directory: PathBuf,
    pub update_operator_tenant_id: TenantId,
    pub update_signing_key_id: UpdateSigningKeyId,
    pub update_verifying_key: VerifyingKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationConfig {
    pub database_url: String,
    pub maximum_database_connections: u32,
}

impl std::fmt::Debug for ServerConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServerConfig")
            .field("database_url", &"[REDACTED]")
            .field("token_key", &self.token_key)
            .field("listen", &self.listen)
            .field("tls_certificate", &self.tls_certificate)
            .field(
                "tls_private_key",
                &self.tls_private_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("allow_insecure_loopback", &self.allow_insecure_loopback)
            .field(
                "maximum_database_connections",
                &self.maximum_database_connections,
            )
            .field("update_manifest_directory", &self.update_manifest_directory)
            .field("update_operator_tenant_id", &self.update_operator_tenant_id)
            .field("update_signing_key_id", &self.update_signing_key_id)
            .field("update_verifying_key", &"[PUBLIC KEY]")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerCommand {
    Serve,
    Migrate,
    CheckConfig,
    Bootstrap {
        tenant_code: String,
        tenant_name: String,
        organization_name: String,
        owner_username: String,
    },
    Usage,
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ServerConfigError {
    #[error("required server configuration is missing")]
    Missing,
    #[error("server configuration is invalid")]
    Invalid,
    #[error("plaintext server transport is not allowed")]
    InsecureTransport,
}

/// Splits one process-wide connection budget evenly across all database
/// pools so the process never exceeds the operator's configured total.
#[must_use]
pub fn pool_connection_budget(maximum_database_connections: u32) -> u32 {
    (maximum_database_connections / 3).max(1)
}

impl ServerConfig {
    /// Reads and validates process configuration without exposing secret values.
    ///
    /// # Errors
    ///
    /// Returns a stable error for missing, invalid, or unsafe configuration.
    pub fn from_environment() -> Result<Self, ServerConfigError> {
        let database_url =
            std::env::var("EITMAD_SERVER_DATABASE_URL").map_err(|_| ServerConfigError::Missing)?;
        let token_key = TokenKey::from_base64(
            &std::env::var("EITMAD_SERVER_TOKEN_KEY").map_err(|_| ServerConfigError::Missing)?,
        )
        .map_err(|_| ServerConfigError::Invalid)?;
        let listen: SocketAddr = std::env::var("EITMAD_SERVER_LISTEN")
            .unwrap_or_else(|_| "127.0.0.1:8443".to_owned())
            .parse()
            .map_err(|_| ServerConfigError::Invalid)?;
        let tls_certificate = std::env::var_os("EITMAD_SERVER_TLS_CERTIFICATE").map(PathBuf::from);
        let tls_private_key = std::env::var_os("EITMAD_SERVER_TLS_PRIVATE_KEY").map(PathBuf::from);
        if tls_certificate.is_some() != tls_private_key.is_some() {
            return Err(ServerConfigError::Invalid);
        }
        let allow_insecure_loopback = std::env::var("EITMAD_SERVER_ALLOW_INSECURE_LOOPBACK")
            .is_ok_and(|value| value.eq_ignore_ascii_case("true"));
        if tls_certificate.is_none() && (!listen.ip().is_loopback() || !allow_insecure_loopback) {
            return Err(ServerConfigError::InsecureTransport);
        }
        let maximum_database_connections = maximum_database_connections()?;
        let update_manifest_directory = std::env::var_os("EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY")
            .map(PathBuf::from)
            .ok_or(ServerConfigError::Missing)?;
        let update_operator_tenant_id = std::env::var("EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID")
            .map_err(|_| ServerConfigError::Missing)?
            .parse::<uuid::Uuid>()
            .map_err(|_| ServerConfigError::Invalid)?;
        if update_operator_tenant_id.is_nil() {
            return Err(ServerConfigError::Invalid);
        }
        let update_signing_key_id = UpdateSigningKeyId::parse(
            std::env::var("EITMAD_SERVER_UPDATE_KEY_ID").map_err(|_| ServerConfigError::Missing)?,
        )
        .map_err(|_| ServerConfigError::Invalid)?;
        let update_key_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            std::env::var("EITMAD_SERVER_UPDATE_PUBLIC_KEY")
                .map_err(|_| ServerConfigError::Missing)?,
        )
        .map_err(|_| ServerConfigError::Invalid)?;
        let update_key_bytes: [u8; 32] = update_key_bytes
            .try_into()
            .map_err(|_| ServerConfigError::Invalid)?;
        let update_verifying_key =
            VerifyingKey::from_bytes(&update_key_bytes).map_err(|_| ServerConfigError::Invalid)?;
        Ok(Self {
            database_url,
            token_key,
            listen,
            tls_certificate,
            tls_private_key,
            allow_insecure_loopback,
            maximum_database_connections,
            update_manifest_directory,
            update_operator_tenant_id: TenantId::new(update_operator_tenant_id),
            update_signing_key_id,
            update_verifying_key,
        })
    }
}

impl MigrationConfig {
    /// Reads only the database settings required by `migrate`.
    ///
    /// # Errors
    ///
    /// Returns a stable error for missing or invalid database configuration.
    pub fn from_environment() -> Result<Self, ServerConfigError> {
        let database_url =
            std::env::var("EITMAD_SERVER_DATABASE_URL").map_err(|_| ServerConfigError::Missing)?;
        Ok(Self {
            database_url,
            maximum_database_connections: maximum_database_connections()?,
        })
    }
}

fn maximum_database_connections() -> Result<u32, ServerConfigError> {
    let value = std::env::var("EITMAD_SERVER_MAX_CONNECTIONS")
        .map_or(Ok(16), |value| value.parse())
        .map_err(|_| ServerConfigError::Invalid)?;
    if !(3..=192).contains(&value) {
        return Err(ServerConfigError::Invalid);
    }
    Ok(value)
}

impl ServerCommand {
    #[must_use]
    pub fn from_arguments(arguments: &[String]) -> Self {
        match arguments.get(1).map(String::as_str) {
            None | Some("serve") => Self::Serve,
            Some("migrate") => Self::Migrate,
            Some("check-config") => Self::CheckConfig,
            Some("bootstrap") if arguments.len() == 6 => Self::Bootstrap {
                tenant_code: arguments[2].clone(),
                tenant_name: arguments[3].clone(),
                organization_name: arguments[4].clone(),
                owner_username: arguments[5].clone(),
            },
            Some("bootstrap") => Self::Usage,
            _ => Self::Help,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_select_bounded_commands() {
        assert_eq!(
            ServerCommand::from_arguments(&["server".to_owned()]),
            ServerCommand::Serve
        );
        assert_eq!(
            ServerCommand::from_arguments(&["server".to_owned(), "migrate".to_owned()]),
            ServerCommand::Migrate
        );
        assert_eq!(
            ServerCommand::from_arguments(&["server".to_owned(), "unknown".to_owned()]),
            ServerCommand::Help
        );
        assert_eq!(
            ServerCommand::from_arguments(&[
                "server".to_owned(),
                "bootstrap".to_owned(),
                "al-eitmad".to_owned()
            ]),
            ServerCommand::Usage
        );
    }

    #[test]
    fn connection_budget_splits_evenly_and_stays_positive() {
        assert_eq!(pool_connection_budget(3), 1);
        assert_eq!(pool_connection_budget(16), 5);
        assert_eq!(pool_connection_budget(191), 63);
        assert_eq!(pool_connection_budget(192), 64);
    }
}
