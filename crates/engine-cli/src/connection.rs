use std::path::PathBuf;

use eitmad_contracts::{identity::ScopeId, secrets::SecretId};
use eitmad_server_connection::DesktopConnectionSettings;

pub(super) struct ServerRoute {
    pub endpoint: String,
    pub trust: PathBuf,
    pub credential: SecretId,
}

impl ServerRoute {
    pub fn load(
        prefix: &str,
        installation: Option<&DesktopConnectionSettings>,
    ) -> Result<Option<Self>, ()> {
        if let Some(settings) = installation {
            return Ok(Some(Self {
                endpoint: settings.endpoint.clone(),
                trust: settings.trusted_certificate.clone(),
                credential: settings.credential.clone(),
            }));
        }
        let Some(endpoint) = std::env::var_os(format!("EITMAD_{prefix}_SERVER")) else {
            return Ok(None);
        };
        Ok(Some(Self {
            endpoint: endpoint.into_string().map_err(|_| ())?,
            trust: std::env::var_os(format!("EITMAD_{prefix}_TRUST_PEM"))
                .ok_or(())?
                .into(),
            credential: serde_json::from_str(
                &std::env::var(format!("EITMAD_{prefix}_CREDENTIAL_ID")).map_err(|_| ())?,
            )
            .map_err(|_| ())?,
        }))
    }
}

pub(super) fn branch(
    prefix: &str,
    installation: Option<&DesktopConnectionSettings>,
) -> Result<ScopeId, ()> {
    installation.map_or_else(
        || {
            std::env::var(format!("EITMAD_{prefix}_BRANCH_ID"))
                .map_err(|_| ())?
                .parse()
                .map(ScopeId::new)
                .map_err(|_| ())
        },
        |settings| Ok(settings.branch),
    )
}
