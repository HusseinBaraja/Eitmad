//! Authenticated direct connection from the Rust desktop engine to the server.

use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::Path,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer as _, SigningKey};
use eitmad_contracts::{
    identity::{AccountId, DeviceId, ScopeRef},
    secrets::SecretId,
    server::{
        AuthenticationResult, DeviceProof, RefreshRequest, SERVER_API_VERSION, ServerClientMessage,
        ServerConnectionHello, ServerFailure, ServerMessage,
    },
    sync::{MAX_SYNC_BATCH_RECORDS, SyncMessage},
    sync_transport::{SyncFrameId, SyncTransportFrame, SyncTransportPayload},
    transport::{IdempotencyKey, SchemaId, UnixMillis},
    versioning::PeerHello,
};
use eitmad_secret_storage::{SecretMaterial, SecretStore};
use eitmad_sync::{
    AuthenticationIdentity, ConnectionDriver, ConnectionTarget, EstablishedConnection,
    FailurePhase, RetryAdvice, SessionSecurity, TransportAuthentication, TransportFailure,
    TransportFailureKind,
};
use rustls::{
    ClientConfig, ClientConnection, RootCertStore, StreamOwned,
    pki_types::{CertificateDer, ServerName, pem::PemObject},
};
use serde::{Deserialize, Serialize};
use tungstenite::{
    Message, WebSocket, client, client::IntoClientRequest, protocol::WebSocketConfig,
};
use url::Url;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_WIRE_BYTES: usize = 1024 * 1024;
const MAX_AUTH_RESPONSE_BYTES: usize = 16 * 1024;
const REFRESH_MARGIN_MS: i64 = 30_000;

type TlsStream = StreamOwned<ClientConnection, TcpStream>;

/// TLS trust and one scoped server sync route. The PEM file is an explicit trust anchor.
pub struct DirectServerConfig {
    endpoint: Url,
    scope: ScopeRef,
    schema_id: SchemaId,
    schema_version: u32,
    tls: Arc<ClientConfig>,
}

impl DirectServerConfig {
    /// Requires an HTTPS endpoint and a readable PEM trust anchor.
    ///
    /// # Errors
    ///
    /// Returns a sanitized encryption failure for an invalid endpoint or certificate.
    pub fn new(
        endpoint: &str,
        scope: ScopeRef,
        schema_id: SchemaId,
        schema_version: u32,
        trusted_certificate: &Path,
    ) -> Result<Self, TransportFailure> {
        let endpoint = Url::parse(endpoint).map_err(|_| encryption_failure())?;
        if endpoint.scheme() != "https"
            || endpoint.host_str().is_none()
            || endpoint.username() != ""
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.path() != "/"
            || schema_version == 0
        {
            return Err(encryption_failure());
        }
        let certificates = CertificateDer::pem_file_iter(trusted_certificate)
            .map_err(|_| encryption_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| encryption_failure())?;
        if certificates.is_empty() {
            return Err(encryption_failure());
        }
        let mut roots = RootCertStore::empty();
        for certificate in certificates {
            roots.add(certificate).map_err(|_| encryption_failure())?;
        }
        let tls = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Self {
            endpoint,
            scope,
            schema_id,
            schema_version,
            tls: Arc::new(tls),
        })
    }

    #[must_use]
    pub fn wan_endpoint(&self) -> eitmad_sync::WanEndpoint {
        eitmad_sync::WanEndpoint {
            server: self.endpoint.as_str().to_owned(),
            relay: None,
        }
    }

    fn websocket_url(&self) -> Url {
        let mut url = self.endpoint.clone();
        url.set_scheme("wss").expect("HTTPS can become WSS");
        url.set_path("/v1/connect");
        url.query_pairs_mut()
            .append_pair("scope_kind", self.scope.kind.as_str())
            .append_pair("scope_id", &self.scope.id.value().to_string())
            .append_pair("schema_id", self.schema_id.as_str())
            .append_pair("schema_version", &self.schema_version.to_string());
        url
    }

    fn refresh_url(&self) -> Url {
        let mut url = self.endpoint.clone();
        url.set_path("/v1/auth/refresh");
        url
    }
}

/// Stores one token pair and device signing seed as a single replaceable secret.
/// The caller must have completed server activation or login with the matching public key.
///
/// # Errors
///
/// Returns a sanitized authentication failure for invalid tokens or expiry metadata.
pub fn store_session(
    store: &SecretStore,
    id: &SecretId,
    result: AuthenticationResult,
    signing_seed: [u8; 32],
) -> Result<(), TransportFailure> {
    if result.tokens.access_token.is_empty()
        || result.tokens.refresh_token.is_empty()
        || result.tokens.refresh_expires_at.0 <= unix_millis_now().0
        || result.tokens.access_expires_at.0 > result.tokens.refresh_expires_at.0
    {
        return Err(authentication_failure());
    }
    let credential = StoredCredential {
        account_id: result.session.account_id,
        device_id: result.session.device_id,
        access_token: result.tokens.access_token,
        refresh_token: result.tokens.refresh_token,
        access_expires_at: result.tokens.access_expires_at,
        refresh_expires_at: result.tokens.refresh_expires_at,
        signing_seed,
    };
    let bytes = serde_json::to_vec(&credential).map_err(|_| authentication_failure())?;
    let material = SecretMaterial::new(bytes).map_err(|_| authentication_failure())?;
    store
        .set(id, material)
        .map_err(|_| authentication_failure())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredCredential {
    account_id: AccountId,
    device_id: DeviceId,
    access_token: String,
    refresh_token: String,
    access_expires_at: UnixMillis,
    refresh_expires_at: UnixMillis,
    signing_seed: [u8; 32],
}

impl Drop for StoredCredential {
    fn drop(&mut self) {
        self.access_token.zeroize();
        self.refresh_token.zeroize();
        self.signing_seed.zeroize();
    }
}

struct PendingResponse {
    request: SyncTransportFrame,
    next_sequence: u64,
    cancelled: bool,
}

/// Blocking driver for the existing synchronous `SyncTransport` interface.
/// Run it on a Rust worker thread, never on the shell UI thread.
pub struct DirectServerDriver {
    config: DirectServerConfig,
    store: SecretStore,
    local_hello: PeerHello,
    socket: Option<WebSocket<TlsStream>>,
    pending: Option<PendingResponse>,
}

impl DirectServerDriver {
    #[must_use]
    pub fn new(config: DirectServerConfig, store: SecretStore, local_hello: PeerHello) -> Self {
        Self {
            config,
            store,
            local_hello,
            socket: None,
            pending: None,
        }
    }

    fn load_credential(&self, id: &SecretId) -> Result<StoredCredential, TransportFailure> {
        let material = self
            .store
            .get(id)
            .map_err(|_| authentication_failure())?
            .ok_or_else(authentication_failure)?;
        serde_json::from_slice(material.expose_secret()).map_err(|_| authentication_failure())
    }

    fn refresh_if_due(
        &self,
        id: &SecretId,
        credential: &mut StoredCredential,
    ) -> Result<(), TransportFailure> {
        let now = unix_millis_now();
        if credential.access_expires_at.0 > now.0.saturating_add(REFRESH_MARGIN_MS) {
            return Ok(());
        }
        if credential.refresh_expires_at.0 <= now.0 {
            let _ = self.store.delete(id);
            return Err(authentication_failure());
        }
        let mut request = RefreshRequest {
            refresh_token: credential.refresh_token.clone(),
            device_proof: device_proof(credential),
        };
        let body =
            Zeroizing::new(serde_json::to_vec(&request).map_err(|_| authentication_failure())?);
        request.refresh_token.zeroize();
        let result = self.post_refresh(&body).inspect_err(|failure| {
            if failure.kind == TransportFailureKind::AuthenticationFailed {
                let _ = self.store.delete(id);
            }
        })?;
        if result.session.account_id != credential.account_id
            || result.session.device_id != credential.device_id
        {
            let _ = self.store.delete(id);
            return Err(authentication_failure());
        }
        credential.access_token.zeroize();
        credential.refresh_token.zeroize();
        credential.access_token = result.tokens.access_token;
        credential.refresh_token = result.tokens.refresh_token;
        credential.access_expires_at = result.tokens.access_expires_at;
        credential.refresh_expires_at = result.tokens.refresh_expires_at;
        let material = SecretMaterial::new(
            serde_json::to_vec(credential).map_err(|_| authentication_failure())?,
        )
        .map_err(|_| authentication_failure())?;
        self.store.set(id, material).map_err(|_| {
            let _ = self.store.delete(id);
            authentication_failure()
        })
    }

    fn post_refresh(&self, body: &[u8]) -> Result<AuthenticationResult, TransportFailure> {
        let url = self.config.refresh_url();
        let mut stream = connect_tls(&url, &self.config.tls)?;
        let host = host_header(&url)?;
        let header = format!(
            "POST /v1/auth/refresh HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(header.as_bytes())
            .and_then(|()| stream.write_all(body))
            .and_then(|()| stream.flush())
            .map_err(|_| unavailable(FailurePhase::Authentication))?;
        let mut response = Zeroizing::new(Vec::new());
        stream
            .take((MAX_AUTH_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut response)
            .map_err(|_| unavailable(FailurePhase::Authentication))?;
        if response.len() > MAX_AUTH_RESPONSE_BYTES {
            return Err(authentication_failure());
        }
        let (status, body) = parse_http_response(&response)?;
        let body = Zeroizing::new(body);
        if status == 401 || status == 403 {
            return Err(authentication_failure());
        }
        if status != 200 {
            return Err(unavailable(FailurePhase::Authentication));
        }
        serde_json::from_slice(&body).map_err(|_| authentication_failure())
    }

    fn open_socket(
        &mut self,
        credential: &StoredCredential,
    ) -> Result<PeerHello, TransportFailure> {
        let url = self.config.websocket_url();
        let stream = connect_tls(&url, &self.config.tls)?;
        let proof = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&device_proof(credential)).map_err(|_| authentication_failure())?,
        );
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|_| encryption_failure())?;
        request.headers_mut().insert(
            "Authorization",
            format!("Bearer {}", credential.access_token)
                .parse()
                .map_err(|_| authentication_failure())?,
        );
        request.headers_mut().insert(
            "X-Eitmad-Device-Proof",
            proof.parse().map_err(|_| authentication_failure())?,
        );
        let (mut socket, _) = client::client_with_config(
            request,
            stream,
            Some(
                WebSocketConfig::default()
                    .max_message_size(Some(MAX_WIRE_BYTES))
                    .max_frame_size(Some(MAX_WIRE_BYTES)),
            ),
        )
        .map_err(|error| match error {
            tungstenite::HandshakeError::Failure(tungstenite::Error::Http(response))
                if response.status() == 401 || response.status() == 403 =>
            {
                authentication_failure()
            }
            tungstenite::HandshakeError::Failure(tungstenite::Error::Tls(_)) => {
                encryption_failure()
            }
            _ => unavailable(FailurePhase::Connect),
        })?;
        let hello = ServerClientMessage::Hello(ServerConnectionHello {
            api_version: SERVER_API_VERSION,
            peer: self.local_hello.clone(),
            resume_after: None,
        });
        write_message(&mut socket, &hello)?;
        let response = read_message(&mut socket, FailurePhase::Negotiation)?;
        let remote_hello = match response {
            ServerMessage::Hello(hello) => hello,
            ServerMessage::Failure(failure) => return Err(map_server_failure(&failure)),
            _ => return Err(unavailable(FailurePhase::Negotiation)),
        };
        socket
            .get_mut()
            .sock
            .set_read_timeout(Some(POLL_TIMEOUT))
            .map_err(|_| unavailable(FailurePhase::Connect))?;
        self.socket = Some(socket);
        Ok(remote_hello)
    }
}

impl ConnectionDriver for DirectServerDriver {
    fn establish(
        &mut self,
        target: &ConnectionTarget,
        authentication: &TransportAuthentication,
    ) -> Result<EstablishedConnection, TransportFailure> {
        self.close();
        let (account_id, device_id, id) = match (target, authentication) {
            (
                ConnectionTarget::WanServer { endpoint },
                TransportAuthentication::AccountDevice {
                    account_id,
                    device_id,
                    credential,
                },
            ) if endpoint == self.config.endpoint.as_str() => (*account_id, *device_id, credential),
            _ => return Err(authentication_failure()),
        };
        let mut credential = self.load_credential(id)?;
        if credential.account_id != account_id || credential.device_id != device_id {
            return Err(authentication_failure());
        }
        self.refresh_if_due(id, &mut credential)?;
        let start = Instant::now();
        let remote_hello = self.open_socket(&credential).inspect_err(|failure| {
            if failure.kind == TransportFailureKind::AuthenticationFailed {
                let _ = self.store.delete(id);
            }
        })?;
        Ok(EstablishedConnection {
            remote_hello,
            authenticated_as: AuthenticationIdentity::AccountDevice {
                account_id,
                device_id,
            },
            security: SessionSecurity::encrypted(),
            round_trip_ms: Some(u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)),
        })
    }

    fn send(&mut self, frame: &SyncTransportFrame) -> Result<(), TransportFailure> {
        let cancellation = matches!(frame.payload, SyncTransportPayload::Cancel(_));
        if self.pending.is_some() && !cancellation {
            return Err(TransportFailure::new(
                TransportFailureKind::RetryNotReady,
                FailurePhase::Send,
                RetryAdvice::Immediate,
            ));
        }
        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| unavailable(FailurePhase::Send))?;
        if let SyncTransportPayload::Message(SyncMessage::Pull(request)) = &frame.payload {
            if usize::try_from(request.maximum_records).unwrap_or(usize::MAX)
                > MAX_SYNC_BATCH_RECORDS
            {
                return Err(unavailable(FailurePhase::Send));
            }
        }
        if !matches!(
            frame.payload,
            SyncTransportPayload::Message(SyncMessage::Pull(_) | SyncMessage::Acknowledge(_))
                | SyncTransportPayload::Cancel(_)
        ) {
            return Err(TransportFailure::new(
                TransportFailureKind::CapabilityMismatch,
                FailurePhase::Send,
                RetryAdvice::Never,
            ));
        }
        let encoded = serde_json::to_string(&ServerClientMessage::Sync(frame.clone()))
            .map_err(|_| unavailable(FailurePhase::Send))?;
        if encoded.len() > MAX_WIRE_BYTES {
            return Err(unavailable(FailurePhase::Send));
        }
        socket
            .send(Message::Text(encoded.into()))
            .map_err(|_| unavailable(FailurePhase::Send))?;
        if let SyncTransportPayload::Cancel(cancellation) = &frame.payload {
            if let Some(pending) = &mut self.pending {
                if pending.request.stream_id == cancellation.stream_id {
                    pending.cancelled = true;
                }
            }
            return Ok(());
        }
        if let SyncTransportPayload::Message(SyncMessage::Pull(_) | SyncMessage::Acknowledge(_)) =
            &frame.payload
        {
            self.pending = Some(PendingResponse {
                request: frame.clone(),
                next_sequence: 0,
                cancelled: false,
            });
        }
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<SyncTransportFrame>, TransportFailure> {
        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| unavailable(FailurePhase::Receive))?;
        let response = match read_message(socket, FailurePhase::Receive) {
            Ok(response) => response,
            Err(failure) if failure.kind == TransportFailureKind::RetryNotReady => return Ok(None),
            Err(failure) => return Err(failure),
        };
        let message = match response {
            ServerMessage::Sync(message) => message,
            ServerMessage::Failure(failure) => return Err(map_server_failure(&failure)),
            _ => return Err(unavailable(FailurePhase::Receive)),
        };
        let pending = self
            .pending
            .as_mut()
            .ok_or_else(|| unavailable(FailurePhase::Receive))?;
        let valid_response = match (&pending.request.payload, &message, pending.next_sequence) {
            (
                SyncTransportPayload::Message(SyncMessage::Pull(_)),
                SyncMessage::Changes(_) | SyncMessage::SnapshotManifest(_),
                0,
            )
            | (
                SyncTransportPayload::Message(SyncMessage::Pull(_)),
                SyncMessage::SnapshotChunk(_) | SyncMessage::SnapshotComplete(_),
                1..,
            ) => true,
            (
                SyncTransportPayload::Message(SyncMessage::Acknowledge(expected)),
                SyncMessage::Acknowledge(actual),
                0,
            ) => expected == actual,
            _ => false,
        };
        if !valid_response {
            return Err(unavailable(FailurePhase::Receive));
        }
        let end_of_stream = matches!(
            message,
            SyncMessage::Changes(_)
                | SyncMessage::SnapshotComplete(_)
                | SyncMessage::Acknowledge(_)
        );
        if pending.cancelled {
            pending.next_sequence = pending.next_sequence.saturating_add(1);
            if end_of_stream {
                self.pending = None;
            }
            return Ok(None);
        }
        let frame = SyncTransportFrame {
            frame_id: SyncFrameId::new(Uuid::new_v4()),
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            protocol_version: pending.request.protocol_version,
            correlation_id: pending.request.correlation_id,
            stream_id: pending.request.stream_id,
            sequence: pending.next_sequence,
            end_of_stream,
            payload: SyncTransportPayload::Message(message),
        };
        pending.next_sequence = pending.next_sequence.saturating_add(1);
        if end_of_stream {
            self.pending = None;
        }
        Ok(Some(frame))
    }

    fn close(&mut self) {
        self.socket.take();
        self.pending = None;
    }
}

fn device_proof(credential: &StoredCredential) -> DeviceProof {
    let issued_at = unix_millis_now();
    let nonce = Uuid::new_v4().to_string();
    let message = format!("{}\n{}\n{nonce}", credential.device_id.value(), issued_at.0);
    let signing = SigningKey::from_bytes(&credential.signing_seed);
    DeviceProof {
        device_id: credential.device_id,
        nonce,
        issued_at,
        signature_base64: URL_SAFE_NO_PAD.encode(signing.sign(message.as_bytes()).to_bytes()),
    }
}

fn connect_tls(url: &Url, config: &Arc<ClientConfig>) -> Result<TlsStream, TransportFailure> {
    let host = url.host_str().ok_or_else(encryption_failure)?;
    let port = url.port_or_known_default().ok_or_else(encryption_failure)?;
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|_| unavailable(FailurePhase::Connect))?;
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(IO_TIMEOUT))
                    .and_then(|()| stream.set_write_timeout(Some(IO_TIMEOUT)))
                    .map_err(|_| unavailable(FailurePhase::Connect))?;
                let name =
                    ServerName::try_from(host.to_owned()).map_err(|_| encryption_failure())?;
                let mut connection = ClientConnection::new(Arc::clone(config), name)
                    .map_err(|_| encryption_failure())?;
                let mut stream = stream;
                connection.complete_io(&mut stream).map_err(|error| {
                    if error
                        .get_ref()
                        .is_some_and(|source| source.is::<rustls::Error>())
                    {
                        encryption_failure()
                    } else {
                        unavailable(FailurePhase::Connect)
                    }
                })?;
                return Ok(StreamOwned::new(connection, stream));
            }
            Err(error) => last_error = Some(error),
        }
    }
    let _ = last_error;
    Err(unavailable(FailurePhase::Connect))
}

fn host_header(url: &Url) -> Result<String, TransportFailure> {
    let host = url.host_str().ok_or_else(encryption_failure)?;
    Ok(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    })
}

fn write_message(
    socket: &mut WebSocket<TlsStream>,
    message: &ServerClientMessage,
) -> Result<(), TransportFailure> {
    let bytes = serde_json::to_string(message).map_err(|_| unavailable(FailurePhase::Send))?;
    if bytes.len() > MAX_WIRE_BYTES {
        return Err(unavailable(FailurePhase::Send));
    }
    socket
        .send(Message::Text(bytes.into()))
        .map_err(|_| unavailable(FailurePhase::Send))
}

fn read_message(
    socket: &mut WebSocket<TlsStream>,
    phase: FailurePhase,
) -> Result<ServerMessage, TransportFailure> {
    loop {
        match socket.read() {
            Ok(Message::Text(text)) => {
                return serde_json::from_str(&text).map_err(|_| unavailable(phase));
            }
            Ok(Message::Ping(_) | Message::Pong(_)) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Err(TransportFailure::new(
                    TransportFailureKind::RetryNotReady,
                    phase,
                    RetryAdvice::Immediate,
                ));
            }
            Ok(_) | Err(_) => return Err(unavailable(phase)),
        }
    }
}

fn parse_http_response(response: &[u8]) -> Result<(u16, Vec<u8>), TransportFailure> {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(authentication_failure)?;
    let head = std::str::from_utf8(&response[..split]).map_err(|_| authentication_failure())?;
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
        .ok_or_else(authentication_failure)?;
    let body = &response[split + 4..];
    if head
        .lines()
        .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"))
    {
        return Ok((status, parse_chunked(body)?));
    }
    if let Some(length) = head.lines().find_map(|line| {
        line.to_ascii_lowercase()
            .strip_prefix("content-length:")
            .and_then(|value| value.trim().parse::<usize>().ok())
    }) {
        if length != body.len() {
            return Err(authentication_failure());
        }
    }
    Ok((status, body.to_vec()))
}

fn parse_chunked(mut input: &[u8]) -> Result<Vec<u8>, TransportFailure> {
    let mut output = Vec::new();
    loop {
        let line_end = input
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(authentication_failure)?;
        let length = std::str::from_utf8(&input[..line_end])
            .ok()
            .and_then(|value| value.split(';').next())
            .and_then(|value| usize::from_str_radix(value, 16).ok())
            .ok_or_else(authentication_failure)?;
        input = &input[line_end + 2..];
        if length == 0 {
            return Ok(output);
        }
        if length > MAX_AUTH_RESPONSE_BYTES.saturating_sub(output.len())
            || input.len() < length + 2
            || &input[length..length + 2] != b"\r\n"
        {
            return Err(authentication_failure());
        }
        output.extend_from_slice(&input[..length]);
        input = &input[length + 2..];
    }
}

fn unix_millis_now() -> UnixMillis {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    UnixMillis(i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
}

fn authentication_failure() -> TransportFailure {
    TransportFailure::new(
        TransportFailureKind::AuthenticationFailed,
        FailurePhase::Authentication,
        RetryAdvice::Never,
    )
}

fn map_server_failure(failure: &ServerFailure) -> TransportFailure {
    let code = failure.code.as_str();
    if code == "eitmad.error.server-token-expired.v1" {
        return unavailable(FailurePhase::Authentication);
    }
    if matches!(
        code,
        "eitmad.error.server-authentication-failed.v1"
            | "eitmad.error.server-device-proof-invalid.v1"
    ) {
        return authentication_failure();
    }
    if code == "eitmad.error.server-client-incompatible.v1" {
        return TransportFailure::new(
            TransportFailureKind::CapabilityMismatch,
            FailurePhase::Negotiation,
            RetryAdvice::Never,
        );
    }
    unavailable(FailurePhase::Receive)
}

fn encryption_failure() -> TransportFailure {
    TransportFailure::new(
        TransportFailureKind::EncryptionRequired,
        FailurePhase::Encryption,
        RetryAdvice::Never,
    )
}

fn unavailable(phase: FailurePhase) -> TransportFailure {
    TransportFailure::new(
        TransportFailureKind::ServerUnavailable,
        phase,
        RetryAdvice::After { delay_ms: 250 },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_contracts::{server::ServerErrorCode, transport::CorrelationId};

    #[test]
    fn expired_access_token_retries_but_revoked_session_stops() {
        let failure = |code| ServerFailure {
            code: ServerErrorCode::parse(code).unwrap(),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            retry_after_ms: None,
        };
        let expired = map_server_failure(&failure("eitmad.error.server-token-expired.v1"));
        assert_eq!(expired.kind, TransportFailureKind::ServerUnavailable);
        assert!(matches!(expired.retry, RetryAdvice::After { .. }));
        let revoked = map_server_failure(&failure("eitmad.error.server-authentication-failed.v1"));
        assert_eq!(revoked.kind, TransportFailureKind::AuthenticationFailed);
        assert_eq!(revoked.retry, RetryAdvice::Never);
    }

    #[test]
    fn refresh_response_parser_accepts_bounded_chunked_json_and_rejects_truncation() {
        let response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n{\"a\r\n3\r\n\":1\r\n1\r\n}\r\n0\r\n\r\n";
        assert_eq!(
            parse_http_response(response).unwrap(),
            (200, br#"{"a":1}"#.to_vec())
        );
        let truncated = b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n{}";
        assert!(parse_http_response(truncated).is_err());
    }
}
