//! Uses operational-tool output, rather than regenerating localhost test trust.
use super::*;
use rustls::{ServerConfig, ServerConnection, pki_types::PrivateKeyDer};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ActiveTls {
    certificate: PathBuf,
    private_key: PathBuf,
}

fn bundle(directory: &Path, manifest: &str) -> Arc<ServerConfig> {
    let active: ActiveTls =
        serde_json::from_slice(&std::fs::read(directory.join(manifest)).unwrap()).unwrap();
    let certificates = CertificateDer::pem_file_iter(directory.join(active.certificate))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = PrivateKeyDer::from_pem_file(directory.join(active.private_key)).unwrap();
    Arc::new(
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(certificates, key)
            .unwrap(),
    )
}

fn client(directory: &Path) -> Arc<ClientConfig> {
    DirectServerConfig::new(
        "https://workshop.test:8443/",
        ScopeRef {
            kind: eitmad_contracts::identity::ScopeKind::parse("organization").unwrap(),
            id: eitmad_contracts::identity::ScopeId::new(uuid::Uuid::new_v4()),
        },
        SchemaId::parse("eitmad.schema.lan-test.v1").unwrap(),
        1,
        &directory.join("trusted-ca.pem"),
    )
    .unwrap()
    .tls
}

fn handshake(
    client: Arc<ClientConfig>,
    server: Arc<ServerConfig>,
    name: &'static str,
) -> Result<(), rustls::Error> {
    let mut client = ClientConnection::new(client, ServerName::try_from(name).unwrap())?;
    let mut server = ServerConnection::new(server)?;
    for _ in 0..16 {
        let mut bytes = Vec::new();
        client.write_tls(&mut bytes).unwrap();
        server.read_tls(&mut bytes.as_slice()).unwrap();
        server.process_new_packets()?;
        bytes.clear();
        server.write_tls(&mut bytes).unwrap();
        client.read_tls(&mut bytes.as_slice()).unwrap();
        client.process_new_packets()?;
        if !client.is_handshaking() && !server.is_handshaking() {
            return Ok(());
        }
    }
    panic!("bounded in-memory TLS handshake did not finish");
}

#[test]
#[ignore = "requires synthetic output from Eitmad.LanCertificate.Tests in EITMAD_LAN_TEST_ROOT"]
fn operational_tls_retains_trust_across_renewal_and_rejects_wrong_name_and_ca() {
    let root =
        PathBuf::from(std::env::var_os("EITMAD_LAN_TEST_ROOT").expect("TLS evidence required"));
    let directory = root.join("server");
    let trusted = client(&directory);
    let first = bundle(&directory, "previous-tls.json");
    let renewed = bundle(&directory, "active-tls.json");
    // One retained Rust client configuration accepts both immutable leaf bundles.
    assert!(handshake(Arc::clone(&trusted), first, "workshop.test").is_ok());
    assert!(handshake(Arc::clone(&trusted), Arc::clone(&renewed), "workshop.test").is_ok());
    assert!(handshake(Arc::clone(&trusted), Arc::clone(&renewed), "other.test").is_err());
    assert!(handshake(Arc::clone(&trusted), Arc::clone(&renewed), "192.168.50.10").is_err());
    assert!(handshake(client(&root.join("wrong")), renewed, "workshop.test").is_err());
    // Reopening configuration after process restart uses the same saved CA.
    assert!(
        handshake(
            client(&directory),
            bundle(&directory, "active-tls.json"),
            "workshop.test"
        )
        .is_ok()
    );
    let ip_directory = root.join("ip");
    assert!(
        handshake(
            client(&ip_directory),
            bundle(&ip_directory, "active-tls.json"),
            "192.168.50.10"
        )
        .is_ok()
    );
}
