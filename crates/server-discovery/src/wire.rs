use eitmad_contracts::discovery::ServerInstallationId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) const MAX_PACKET: usize = 512;
const MAGIC: &[u8] = b"EITMAD-DISCOVERY/1\n";

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(super) enum Packet {
    Query {
        nonce: Uuid,
    },
    Offer {
        nonce: Uuid,
        server_id: ServerInstallationId,
        boot: Uuid,
        port: u16,
        lease_seconds: u8,
    },
}

pub(super) fn encode(packet: &Packet) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    // Packet contains only fixed-size values, so serialization cannot fail.
    bytes.extend(serde_json::to_vec(packet).expect("fixed discovery packet"));
    bytes
}

pub(super) fn decode(bytes: &[u8]) -> Option<Packet> {
    if bytes.len() > MAX_PACKET {
        return None;
    }
    serde_json::from_slice(bytes.strip_prefix(MAGIC)?).ok()
}
