use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    transport::{CapabilityId, SchemaId},
    updates::ReleaseVersion,
};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SupportedProtocol {
    pub major: u16,
    pub minimum_minor: u16,
    pub maximum_minor: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SchemaSupport {
    pub schema_id: SchemaId,
    pub minimum_version: u32,
    pub maximum_version: u32,
    pub required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PeerKind {
    Engine,
    Shell,
    Server,
    DiagnosticClient,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PeerHello {
    pub peer_kind: PeerKind,
    pub product_version: ReleaseVersion,
    pub protocols: Vec<SupportedProtocol>,
    pub capabilities: Vec<CapabilityId>,
    pub required_capabilities: Vec<CapabilityId>,
    pub schemas: Vec<SchemaSupport>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RequiredBy {
    Local,
    Remote,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum NegotiationRejection {
    NoCommonProtocol,
    MissingCapability {
        capability: CapabilityId,
        required_by: RequiredBy,
    },
    IncompatibleSchema {
        schema_id: SchemaId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NegotiatedSchema {
    pub schema_id: SchemaId,
    pub version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NegotiatedSession {
    pub protocol: ProtocolVersion,
    pub capabilities: Vec<CapabilityId>,
    pub schemas: Vec<NegotiatedSchema>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", content = "payload", rename_all = "camelCase")]
pub enum NegotiationOutcome {
    Accepted(NegotiatedSession),
    Rejected(NegotiationRejection),
}

#[must_use]
pub fn negotiate(local: &PeerHello, remote: &PeerHello) -> NegotiationOutcome {
    let Some(protocol) = common_protocol(&local.protocols, &remote.protocols) else {
        return NegotiationOutcome::Rejected(NegotiationRejection::NoCommonProtocol);
    };

    let local_capabilities = local.capabilities.iter().cloned().collect::<BTreeSet<_>>();
    let remote_capabilities = remote.capabilities.iter().cloned().collect::<BTreeSet<_>>();

    for required in &local.required_capabilities {
        if !remote_capabilities.contains(required) {
            return NegotiationOutcome::Rejected(NegotiationRejection::MissingCapability {
                capability: required.clone(),
                required_by: RequiredBy::Local,
            });
        }
    }
    for required in &remote.required_capabilities {
        if !local_capabilities.contains(required) {
            return NegotiationOutcome::Rejected(NegotiationRejection::MissingCapability {
                capability: required.clone(),
                required_by: RequiredBy::Remote,
            });
        }
    }

    let capabilities = local_capabilities
        .intersection(&remote_capabilities)
        .cloned()
        .collect();

    match common_schemas(&local.schemas, &remote.schemas) {
        Ok(schemas) => NegotiationOutcome::Accepted(NegotiatedSession {
            protocol,
            capabilities,
            schemas,
        }),
        Err(schema_id) => {
            NegotiationOutcome::Rejected(NegotiationRejection::IncompatibleSchema { schema_id })
        }
    }
}

fn common_protocol(
    local: &[SupportedProtocol],
    remote: &[SupportedProtocol],
) -> Option<ProtocolVersion> {
    local
        .iter()
        .flat_map(|left| {
            remote.iter().filter_map(move |right| {
                let minimum = left.minimum_minor.max(right.minimum_minor);
                let maximum = left.maximum_minor.min(right.maximum_minor);
                (left.major == right.major && minimum <= maximum).then_some(ProtocolVersion {
                    major: left.major,
                    minor: maximum,
                })
            })
        })
        .max()
}

fn common_schemas(
    local: &[SchemaSupport],
    remote: &[SchemaSupport],
) -> Result<Vec<NegotiatedSchema>, SchemaId> {
    let local_by_id = local
        .iter()
        .map(|schema| (schema.schema_id.clone(), schema))
        .collect::<BTreeMap<_, _>>();
    let remote_by_id = remote
        .iter()
        .map(|schema| (schema.schema_id.clone(), schema))
        .collect::<BTreeMap<_, _>>();
    let all_ids = local_by_id
        .keys()
        .chain(remote_by_id.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut negotiated = Vec::new();

    for schema_id in all_ids {
        match (local_by_id.get(&schema_id), remote_by_id.get(&schema_id)) {
            (Some(left), Some(right)) => {
                let minimum = left.minimum_version.max(right.minimum_version);
                let maximum = left.maximum_version.min(right.maximum_version);
                if minimum <= maximum {
                    negotiated.push(NegotiatedSchema {
                        schema_id,
                        version: maximum,
                    });
                } else if left.required || right.required {
                    return Err(schema_id);
                }
            }
            (Some(schema), None) | (None, Some(schema)) if schema.required => {
                return Err(schema_id);
            }
            _ => {}
        }
    }

    Ok(negotiated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(protocols: Vec<SupportedProtocol>) -> PeerHello {
        PeerHello {
            peer_kind: PeerKind::Engine,
            product_version: ReleaseVersion::new(semver::Version::new(1, 0, 0)),
            protocols,
            capabilities: Vec::new(),
            required_capabilities: Vec::new(),
            schemas: Vec::new(),
        }
    }

    #[test]
    fn chooses_highest_mutual_minor() {
        let local = hello(vec![SupportedProtocol {
            major: 1,
            minimum_minor: 0,
            maximum_minor: 4,
        }]);
        let remote = hello(vec![SupportedProtocol {
            major: 1,
            minimum_minor: 2,
            maximum_minor: 3,
        }]);

        let NegotiationOutcome::Accepted(session) = negotiate(&local, &remote) else {
            panic!("expected compatible peers");
        };
        assert_eq!(session.protocol, ProtocolVersion { major: 1, minor: 3 });
    }

    #[test]
    fn rejects_incompatible_major() {
        let local = hello(vec![SupportedProtocol {
            major: 1,
            minimum_minor: 0,
            maximum_minor: 0,
        }]);
        let remote = hello(vec![SupportedProtocol {
            major: 2,
            minimum_minor: 0,
            maximum_minor: 0,
        }]);
        assert_eq!(
            negotiate(&local, &remote),
            NegotiationOutcome::Rejected(NegotiationRejection::NoCommonProtocol)
        );
    }

    #[test]
    fn rejects_missing_required_capability() {
        let required = CapabilityId::parse("eitmad.capability.sync.v1").unwrap();
        let mut local = hello(vec![SupportedProtocol {
            major: 1,
            minimum_minor: 0,
            maximum_minor: 0,
        }]);
        local.required_capabilities.push(required.clone());
        let remote = local.clone();
        local.capabilities.push(required.clone());

        assert_eq!(
            negotiate(&local, &remote),
            NegotiationOutcome::Rejected(NegotiationRejection::MissingCapability {
                capability: required,
                required_by: RequiredBy::Local,
            })
        );
    }
}
