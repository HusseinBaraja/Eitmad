use std::{
    fs::File,
    io::Read as _,
    path::{Path, PathBuf},
};

use eitmad_contracts::{
    identity::{OrganizationId, ScopeId},
    secrets::SecretId,
};
use serde::{Deserialize, Serialize};

/// Installation connection references. Tokens and signing keys remain in secret storage.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopConnectionSettings {
    pub version: u32,
    pub endpoint: String,
    pub trusted_certificate: PathBuf,
    pub credential: SecretId,
    pub organization: OrganizationId,
    pub branch: ScopeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_code: Option<eitmad_contracts::server::TenantCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<eitmad_contracts::discovery::ServerInstallationId>,
}

impl DesktopConnectionSettings {
    pub const FILE_NAME: &str = "connection.json";

    /// Reads bounded installation settings. Only an absent file means unconfigured.
    /// Relative certificate paths are resolved against the installation directory.
    ///
    /// # Errors
    /// Returns a sanitized failure for unreadable, malformed, or unsupported settings.
    pub fn load(directory: &Path) -> Result<Option<Self>, eitmad_sync::TransportFailure> {
        let file = match File::open(directory.join(Self::FILE_NAME)) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(super::encryption_failure()),
        };
        let mut bytes = Vec::new();
        file.take(16_385)
            .read_to_end(&mut bytes)
            .map_err(|_| super::encryption_failure())?;
        if bytes.len() > 16_384 {
            return Err(super::encryption_failure());
        }
        let mut settings: Self =
            serde_json::from_slice(&bytes).map_err(|_| super::encryption_failure())?;
        if settings.version != 1 {
            return Err(super::encryption_failure());
        }
        if settings.trusted_certificate.is_relative() {
            settings.trusted_certificate = directory.join(settings.trusted_certificate);
        }
        if let Some(id) = settings.server_id {
            settings.endpoint =
                super::DirectServerConfig::parse_endpoint(&settings.endpoint)?.into();
            let saved = super::desktop_enrollment::retained::RetainedEnrollment::load(directory)?
                .ok_or_else(super::encryption_failure)?;
            if id != saved.identity.server_id
                || settings.endpoint != saved.identity.endpoint
                || settings.organization != saved.organization
                || settings.branch != saved.branch
                || settings.tenant_code.as_ref() != Some(&saved.tenant_code)
                || super::desktop_enrollment::read_trust(&settings.trusted_certificate)?
                    != saved.identity.public_ca_pem.as_bytes()
            {
                return Err(super::encryption_failure());
            }
        }
        Ok(Some(settings))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_are_optional_but_invalid_settings_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        assert!(
            DesktopConnectionSettings::load(directory.path())
                .unwrap()
                .is_none()
        );
        let path = directory.path().join(DesktopConnectionSettings::FILE_NAME);
        for bytes in [b"{".to_vec(), vec![b' '; 16_385]] {
            std::fs::write(&path, bytes).unwrap();
            assert!(DesktopConnectionSettings::load(directory.path()).is_err());
        }
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(DesktopConnectionSettings::load(directory.path()).is_err());
    }

    #[test]
    fn retained_settings_resolve_relative_trust_and_reject_unknown_versions_or_fields() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(DesktopConnectionSettings::FILE_NAME);
        let settings = serde_json::json!({
            "version": 1,
            "endpoint": "https://localhost:58443/",
            "trustedCertificate": "trusted-ca.pem",
            "credential": {"kind": "desktop-route", "reference": uuid::Uuid::new_v4()},
            "organization": uuid::Uuid::new_v4(),
            "branch": uuid::Uuid::new_v4(),
        });
        std::fs::write(&path, serde_json::to_vec(&settings).unwrap()).unwrap();
        let loaded = DesktopConnectionSettings::load(directory.path())
            .unwrap()
            .unwrap();
        assert_eq!(
            loaded.trusted_certificate,
            directory.path().join("trusted-ca.pem")
        );
        assert_eq!(loaded.endpoint, "https://localhost:58443/");
        for (field, value) in [
            ("version", serde_json::json!(2)),
            ("accessToken", serde_json::json!("invalid")),
        ] {
            let mut invalid = settings.clone();
            invalid[field] = value;
            std::fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
            assert!(DesktopConnectionSettings::load(directory.path()).is_err());
        }
    }
}
