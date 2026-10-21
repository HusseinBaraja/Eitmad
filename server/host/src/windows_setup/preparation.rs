use crate::MainError;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use eitmad_contracts::setup::{PrepareWindowsWorkshop, PreparedWindowsWorkshop};
use eitmad_control_plane::{ControlPlane, TokenKey};
use rand::RngCore as _;
use std::{
    fs,
    io::{Read as _, Write as _},
    net::Ipv4Addr,
    path::Path,
};
use zeroize::Zeroizing;

mod database;
pub(super) mod native;

pub(super) async fn prepare() -> Result<(), MainError> {
    let mut input = Zeroizing::new(Vec::new());
    std::io::stdin()
        .take(16385)
        .read_to_end(&mut input)
        .map_err(|_| MainError::Input)?;
    if input.len() > 16384 {
        return Err(MainError::Input);
    }
    let request: PrepareWindowsWorkshop =
        serde_json::from_slice(&input).map_err(|_| MainError::Input)?;
    // The standalone preparation command remains fresh-only. Reuse belongs to
    // the installer, which binds the protected preparation to its retained attempt.
    validate(&request)?;
    let prepared = run(&request, &mut |_| Ok(())).await?;
    println!(
        "{}",
        serde_json::to_string(&prepared).map_err(|_| MainError::Setup)?
    );
    Ok(())
}

pub(super) async fn run(
    request: &PrepareWindowsWorkshop,
    stage: &mut dyn FnMut(eitmad_contracts::setup::WindowsInstallStage) -> Result<(), MainError>,
) -> Result<PreparedWindowsWorkshop, MainError> {
    if Path::new(&request.state_directory).exists() {
        validate_inputs(request)?;
        // Only a complete, stopped preparation has a safe import boundary. Never guess
        // whether an interrupted Owner transaction or cluster copy committed.
        stage(eitmad_contracts::setup::WindowsInstallStage::StopPreparedDatabase)?;
        let path = Path::new(&request.state_directory).join("prepared-workshop.json");
        native::plain_path(&path, true)?;
        let prepared: PreparedWindowsWorkshop = serde_json::from_reader(
            fs::File::open(&path)
                .map_err(|_| MainError::Setup)?
                .take(16385),
        )
        .map_err(|_| MainError::Setup)?;
        if prepared.version != 1
            || prepared.endpoint
                != format!("https://{}:{}/", request.server_name, request.server_port)
        {
            return Err(MainError::Setup);
        }
        native::invoke(request, "Inspect", None)?;
        return Ok(prepared);
    }
    validate(request)?;
    stage(eitmad_contracts::setup::WindowsInstallStage::PrepareWorkshop)?;
    let secrets = Secrets::new();
    // Init may start PostgreSQL before a later native operation fails. Always attempt an owned stop.
    let result = native::invoke(request, "Initialize", Some(&secrets));
    let result = match result {
        Ok(()) => initialize(request, &secrets, stage).await,
        Err(error) => Err(error),
    };
    let stop_stage = if result.is_ok() {
        stage(eitmad_contracts::setup::WindowsInstallStage::StopPreparedDatabase)
    } else {
        Ok(())
    };
    let stopped = native::invoke(request, "Stop", None);
    let prepared = result?;
    stop_stage?;
    stopped?;
    // A stopped, complete preparation is the only state that can be imported by the service adapter.
    write_new(
        Path::new(&request.state_directory)
            .join("prepared-workshop.json")
            .as_path(),
        &serde_json::to_vec(&prepared).map_err(|_| MainError::Setup)?,
    )?;
    Ok(prepared)
}

async fn initialize(
    request: &PrepareWindowsWorkshop,
    secrets: &Secrets,
    stage: &mut dyn FnMut(eitmad_contracts::setup::WindowsInstallStage) -> Result<(), MainError>,
) -> Result<PreparedWindowsWorkshop, MainError> {
    database::retain_secrets(request, secrets)?;
    let database_url = database::provision(request, secrets).await?;
    stage(eitmad_contracts::setup::WindowsInstallStage::ApplyMigrations)?;
    let (database, sync, admin) = crate::migrated_databases(&database_url, 12).await?;
    let token = TokenKey::from_base64(&secrets.token).map_err(|_| MainError::Setup)?;
    let control = ControlPlane::new(database.pool(), token);
    stage(eitmad_contracts::setup::WindowsInstallStage::InitializeOwner)?;
    let workshop = control
        .identity
        .initialize_workshop(
            &request.workshop,
            eitmad_contracts::transport::CorrelationId::new(uuid::Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .map_err(|_| MainError::Bootstrap)?;
    let endpoint = format!("https://{}:{}/", request.server_name, request.server_port);
    database::write_settings(request, secrets, &database_url, &workshop, &endpoint)?;
    // Close every pool before PostgreSQL is stopped and the fresh cluster is copied.
    database.pool().close().await;
    sync.pool().close().await;
    admin.pool().close().await;
    Ok(PreparedWindowsWorkshop {
        version: 1,
        endpoint,
        workshop,
    })
}

struct Secrets {
    admin: Zeroizing<String>,
    application: Zeroizing<String>,
    certificate: Zeroizing<String>,
    token: Zeroizing<String>,
}

impl Secrets {
    fn new() -> Self {
        Self {
            admin: random_secret(),
            application: random_secret(),
            certificate: random_secret(),
            token: random_secret(),
        }
    }
}

fn random_secret() -> Zeroizing<String> {
    let mut bytes = Zeroizing::new([0; 32]);
    rand::rng().fill_bytes(bytes.as_mut());
    Zeroizing::new(URL_SAFE_NO_PAD.encode(bytes.as_ref()))
}

pub(super) fn validate(request: &PrepareWindowsWorkshop) -> Result<(), MainError> {
    validate_inputs(request)?;
    if Path::new(&request.state_directory)
        .try_exists()
        .map_err(|_| MainError::Setup)?
    {
        return Err(MainError::Setup);
    }
    Ok(())
}

pub(super) fn validate_inputs(request: &PrepareWindowsWorkshop) -> Result<(), MainError> {
    if request.version != 1
        || request.workshop.version != 1
        || request.server_port < 1024
        || request.database_port < 1024
        || request.server_port == request.database_port
    {
        return Err(MainError::Input);
    }
    let address: Ipv4Addr = request
        .listen_address
        .parse()
        .map_err(|_| MainError::Input)?;
    if !address.is_private() {
        return Err(MainError::Input);
    }
    if request
        .server_name
        .parse::<Ipv4Addr>()
        .is_ok_and(|name| name != address)
    {
        return Err(MainError::Input);
    }
    // The native CA adapter validates DNS syntax and issues a matching SAN.
    if request.server_name.is_empty()
        || request.server_name.len() > 253
        || !request
            .server_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return Err(MainError::Input);
    }
    for input in [
        &request.platform_script,
        &request.postgres_bin,
        &request.certificate_executable,
    ] {
        native::plain_path(Path::new(input), true)?;
    }
    let root = Path::new(&request.state_directory);
    native::plain_path(root, false)?;
    if !root.parent().is_some_and(Path::is_dir) {
        return Err(MainError::Input);
    }
    // Reject invalid account input before creating any native state.
    eitmad_control_plane::validate_workshop_initialization(&request.workshop)
        .map_err(|_| MainError::Input)?;
    Ok(())
}

pub(super) fn write_new(path: &Path, content: &[u8]) -> Result<(), MainError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| MainError::Setup)?;
    file.write_all(content)
        .and_then(|()| file.sync_all())
        .map_err(|_| MainError::Setup)
}
