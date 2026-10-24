//! Both authenticated enrollment channels feed the same retained connection authority.
use super::*;
use eitmad_contracts::{
    identity::TenantId,
    pairing::{PairingIdentity, PairingReceipt},
    setup::SetupDesktopHandoff,
};
use serde::Deserialize;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RetainedEnrollment {
    version: u16,
    pub identity: PairingIdentity,
    pub tenant_id: TenantId,
    pub tenant_code: eitmad_contracts::server::TenantCode,
    pub organization: eitmad_contracts::identity::OrganizationId,
    pub branch: eitmad_contracts::identity::ScopeId,
    pub device: DeviceId,
    pub public_key: DevicePublicKey,
}

impl RetainedEnrollment {
    const FIRST_FILE: &str = "desktop-first-server.json";

    pub(crate) fn load(directory: &Path) -> Result<Option<Self>, TransportFailure> {
        let first = read_optional(&directory.join(Self::FIRST_FILE))?;
        let paired = read_optional(&directory.join("desktop-pairing.json"))?;
        let mut retained: Self = match (first, paired) {
            (Some(_), Some(_)) => return Err(encryption_failure()),
            (Some(bytes), None) => {
                serde_json::from_slice(&bytes).map_err(|_| encryption_failure())?
            }
            (None, Some(bytes)) => {
                let receipt: PairingReceipt =
                    serde_json::from_slice(&bytes).map_err(|_| encryption_failure())?;
                let binding = receipt.binding;
                let offer = binding.offer;
                Self {
                    version: offer.version,
                    identity: offer.identity,
                    tenant_id: offer.tenant_id,
                    tenant_code: offer.tenant_code,
                    organization: offer.organization_id,
                    branch: offer.branch_id,
                    device: binding.device_id,
                    public_key: binding.device_public_key,
                }
            }
            (None, None) => return Ok(None),
        };
        retained.validate()?;
        retained.identity.endpoint =
            DirectServerConfig::parse_endpoint(&retained.identity.endpoint)?.into();
        Ok(Some(retained))
    }

    fn validate(&self) -> Result<(), TransportFailure> {
        if self.version != 1
            || self.identity.server_id.value().is_nil()
            || self.tenant_id.value().is_nil()
            || self.organization.value().is_nil()
            || self.branch.value().is_nil()
            || self.device.value().is_nil()
        {
            return Err(encryption_failure());
        }
        Ok(())
    }

    fn from_handoff(
        handoff: &SetupDesktopHandoff,
        original: &LoginRequest,
    ) -> Result<Self, TransportFailure> {
        if handoff.version != 1
            || handoff.prepared.version != 1
            || handoff.prepared.workshop.version != 1
            || handoff.public_ca_pem.len() > 16_384
            || handoff.public_ca_pem.contains("PRIVATE KEY")
        {
            return Err(encryption_failure());
        }
        let workshop = &handoff.prepared.workshop;
        let expected = Self {
            version: 1,
            identity: PairingIdentity {
                server_id: handoff.server_id,
                endpoint: DirectServerConfig::parse_endpoint(&handoff.prepared.endpoint)?.into(),
                public_ca_pem: handoff.public_ca_pem.clone(),
            },
            tenant_id: workshop.tenant_id,
            tenant_code: workshop.tenant_code.clone(),
            organization: workshop.organization_id,
            branch: workshop.branch_id,
            device: original.device_id,
            public_key: original.device_public_key.clone(),
        };
        expected.validate()?;
        Ok(expected)
    }

    pub(super) fn settings(&self, directory: &Path) -> DesktopEnrollmentSettings {
        DesktopEnrollmentSettings {
            endpoint: self.identity.endpoint.clone(),
            trusted_certificate: directory
                .join("desktop-pairing-ca.pem")
                .to_string_lossy()
                .into_owned(),
            tenant_code: self.tenant_code.clone(),
            organization: self.organization,
            branch: self.branch,
        }
    }
}

impl DesktopEnrollment {
    /// Retains trust delivered by the original user's authenticated setup elevation channel.
    /// Call only after private first-desktop approval; this does not grant account access.
    /// # Errors
    /// Rejects changed trust, scope, or device identity, including on interrupted setup retry.
    pub fn retain_first_desktop(
        &self,
        handoff: &SetupDesktopHandoff,
        original: &LoginRequest,
    ) -> Result<DesktopEnrollmentSettings, TransportFailure> {
        let _owner = self
            .admission
            .lock()
            .map_err(|_| authentication_failure())?;
        if self
            .directory
            .join("desktop-pairing.json")
            .try_exists()
            .map_err(|_| encryption_failure())?
        {
            return Err(encryption_failure());
        }
        let expected = RetainedEnrollment::from_handoff(handoff, original)?;
        let retained = DesktopConnectionSettings::load(&self.directory)?;
        let (device, route) = self.device(retained.as_ref())?;
        let key = self.signing_key(device, &route)?;
        if device != expected.device
            || expected.public_key.algorithm != "ed25519"
            || expected.public_key.base64 != URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes())
        {
            return Err(authentication_failure());
        }
        let settings = expected.settings(&self.directory);
        DirectServerConfig::from_ca_pem(
            &settings.endpoint,
            ScopeRef {
                kind: ScopeKind::parse("organization").map_err(|_| encryption_failure())?,
                id: eitmad_contracts::identity::ScopeId::new(settings.organization.value()),
            },
            SchemaId::parse("eitmad.schema.desktop-enrollment.v1")
                .map_err(|_| encryption_failure())?,
            1,
            expected.identity.public_ca_pem.as_bytes(),
        )?;
        if let Some(saved) = RetainedEnrollment::load(&self.directory)? {
            if saved != expected {
                return Err(encryption_failure());
            }
        } else if retained.is_some()
            || self
                .store
                .has_local_installation()
                .map_err(|_| authentication_failure())?
        {
            return Err(authentication_failure());
        }
        // Commit identity first: a failed CA write can be retried, but cannot replace trust.
        atomic_write(
            &self.directory.join(RetainedEnrollment::FIRST_FILE),
            &serde_json::to_vec(&expected).map_err(|_| encryption_failure())?,
        )?;
        atomic_write(
            &self.directory.join("desktop-pairing-ca.pem"),
            expected.identity.public_ca_pem.as_bytes(),
        )?;
        Ok(settings)
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, TransportFailure> {
    match pairing::read_pairing_file(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(encryption_failure()),
    }
}
