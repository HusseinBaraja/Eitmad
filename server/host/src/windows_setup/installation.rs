//! Local installer authority. OS operations are delegated to the packaged Windows adapter.
use super::preparation::{self, native::plain_path};
use crate::MainError;
use eitmad_contracts::setup::{
    InstallWindows, InstallWindowsClient, InstallWindowsServer, ReadWindowsInstallation,
    WindowsInstallPackage, WindowsInstallProgress, WindowsInstallStage as Stage,
    WindowsInstallStatus as Status, WindowsInstallationState,
};
use eitmad_server::ServerCommand;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    os::windows::fs::OpenOptionsExt as _,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

pub(super) mod native;
pub(super) mod recovery;
#[cfg(test)]
mod tests;

pub(super) async fn execute(command: ServerCommand) -> Result<(), MainError> {
    let mut source = std::io::stdin().lock();
    if command == ServerCommand::InstallWindowsServer {
        // Read one bounded JSON value without waiting for EOF. The authenticated
        // elevation channel supplies a second value only after HTTPS is ready.
        let mut request: InstallWindowsServer = read_private(&mut source)?;
        let root = super::payload::bundle_root().ok();
        let _payload = root.as_deref().map(super::payload::verify).transpose()?;
        if let Some(root) = root {
            super::payload::bind_server(&mut request, &root)?;
        }
        validate_server(&request)?;
        return install_server(&request, &mut native::WindowsPlatform, &mut source).await;
    }
    let mut input = Zeroizing::new(Vec::new());
    source
        .take(65537)
        .read_to_end(&mut input)
        .map_err(|_| MainError::Input)?;
    if input.len() > 65536 {
        return Err(MainError::Input);
    }
    if command == ServerCommand::WindowsInstallStatus {
        let request: ReadWindowsInstallation =
            serde_json::from_slice(&input).map_err(|_| MainError::Input)?;
        if request.version != 1 {
            return Err(MainError::Input);
        }
        return publish(&read_state(Path::new(&request.progress_directory))?);
    }
    let request: InstallWindows = serde_json::from_slice(&input).map_err(|_| MainError::Input)?;
    match &request {
        InstallWindows::Client(client) => validate_client(client)?,
        InstallWindows::Server(server) => {
            validate_client(&server.client)?;
            validate_server(server)?;
        }
    }
    install(&request, &mut native::WindowsPlatform).await
}

fn read_private<T: serde::de::DeserializeOwned>(
    source: &mut impl std::io::Read,
) -> Result<T, MainError> {
    T::deserialize(&mut serde_json::Deserializer::from_reader(
        source.take(65536),
    ))
    .map_err(|_| MainError::Input)
}

fn publish(value: &impl serde::Serialize) -> Result<(), MainError> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value).map_err(|_| MainError::Setup)?;
    output
        .write_all(b"\n")
        .and_then(|()| output.flush())
        .map_err(|_| MainError::Setup)
}

fn validate_package(package: &WindowsInstallPackage) -> Result<(), MainError> {
    installation_path(&package.directory)?;
    plain_path(Path::new(&package.directory), true)?;
    if package.release_sha256.len() != 64
        || !package
            .release_sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
        || package.publisher.is_empty()
        || package.publisher.len() > 1024
    {
        return Err(MainError::Input);
    }
    Ok(())
}

fn fresh_path(value: &str) -> Result<(), MainError> {
    installation_path(value)?;
    let path = Path::new(value);
    plain_path(path, false)?;
    if path.try_exists().map_err(|_| MainError::Setup)? || !path.parent().is_some_and(Path::is_dir)
    {
        return Err(MainError::Setup);
    }
    Ok(())
}

fn installation_path(value: &str) -> Result<(), MainError> {
    if Path::new(value)
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(MainError::Input);
    }
    Ok(())
}

pub(super) fn validate_client(request: &InstallWindowsClient) -> Result<(), MainError> {
    if request.version != 1
        || !request.intended_user_sid.starts_with("S-1-5-")
        || request.intended_user_sid.len() > 184
    {
        return Err(MainError::Input);
    }
    installation_path(&request.progress_directory)?;
    plain_path(Path::new(&request.progress_directory), false)?;
    validate_package(&request.desktop)
}

pub(super) fn validate_server(request: &InstallWindowsServer) -> Result<(), MainError> {
    if request.version != 1
        || request.client.version != 1
        || request.service_name.is_empty()
        || request.service_name.len() > 32
        || !request.service_name.as_bytes()[0].is_ascii_alphabetic()
        || !request
            .service_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric())
        || request.winsw_sha256.len() != 64
        || !request.winsw_sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(MainError::Input);
    }
    preparation::validate_inputs(&request.preparation)?;
    installation_path(&request.preparation.state_directory)?;
    validate_package(&request.server)?;
    plain_path(Path::new(&request.winsw_executable), true)?;
    for path in [
        &request.service_state_directory,
        &request.server_progress_directory,
    ] {
        installation_path(path)?;
        plain_path(Path::new(path), false)?;
    }
    let paths = [
        &request.client.progress_directory,
        &request.server_progress_directory,
        &request.preparation.state_directory,
        &request.service_state_directory,
    ];
    for (index, value) in paths.iter().enumerate() {
        for other in paths.iter().skip(index + 1) {
            let left = PathBuf::from(value.to_lowercase());
            let right = PathBuf::from(other.to_lowercase());
            if left.starts_with(&right) || right.starts_with(&left) {
                return Err(MainError::Input);
            }
        }
        let root = PathBuf::from(value.to_lowercase());
        for input in [
            &request.server.directory,
            &request.client.desktop.directory,
            &request.preparation.postgres_bin,
            &request.winsw_executable,
        ] {
            installation_path(input)?;
            let source = PathBuf::from(input.to_lowercase());
            if source.starts_with(&root) || root.starts_with(&source) {
                return Err(MainError::Input);
            }
        }
    }
    Ok(())
}

/// Append-only, flushed stage records survive failures and power loss without rewriting old state.
pub(super) struct Progress {
    root: PathBuf,
    _lock: File,
    sequence: u32,
    wire_sequence: u32,
    stage: Stage,
    blocked: bool,
}

impl Progress {
    fn create(root: &str) -> Result<Self, MainError> {
        fresh_path(root)?;
        fs::create_dir(root).map_err(|_| MainError::Setup)?;
        let lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(0)
            .open(Path::new(root).join("installation.lock"))
            .map_err(|_| MainError::Setup)?;
        Ok(Self {
            root: root.into(),
            _lock: lock,
            sequence: 0,
            wire_sequence: 0,
            stage: Stage::Preflight,
            blocked: false,
        })
    }

    fn record(&mut self, stage: Stage, status: Status) -> Result<(), MainError> {
        self.sequence += 1;
        self.wire_sequence += 1;
        self.stage = stage;
        self.blocked = status == Status::Blocked;
        let event = WindowsInstallProgress {
            version: 1,
            sequence: self.sequence,
            stage,
            status,
        };
        let file = self.root.join(format!("{:08}.json", self.sequence));
        let bytes = serde_json::to_vec(&event).map_err(|_| MainError::Setup)?;
        super::preparation::write_new(&file, &bytes)?;
        publish(&WindowsInstallProgress {
            sequence: self.wire_sequence,
            ..event
        })
    }

    fn begin(&mut self, stage: Stage) -> Result<(), MainError> {
        self.record(stage, Status::Running)
    }

    fn finish(&mut self, result: Result<(), MainError>) -> Result<(), MainError> {
        match result {
            Ok(()) => self.record(Stage::Complete, Status::Succeeded),
            Err(error) => {
                let _ = self.record(
                    self.stage,
                    if self.blocked {
                        Status::Blocked
                    } else {
                        Status::Failed
                    },
                );
                Err(error)
            }
        }
    }
}

pub(super) fn read_state(root: &Path) -> Result<WindowsInstallationState, MainError> {
    plain_path(root, true)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(root.join("installation.lock"));
    let active = match &lock {
        Ok(_) => false,
        Err(error) if error.raw_os_error() == Some(32) => true,
        Err(_) => return Err(MainError::Setup),
    };
    let progress = read_last(root)?.unwrap_or(WindowsInstallProgress {
        version: 1,
        sequence: 0,
        stage: Stage::Preflight,
        status: Status::Running,
    });
    let completed = progress.stage == Stage::Complete && progress.status == Status::Succeeded;
    let interrupted =
        !active && !completed && !matches!(progress.status, Status::Failed | Status::Blocked);
    Ok(WindowsInstallationState {
        version: 1,
        progress,
        interrupted,
        recovery: recovery::action(root, active, completed),
    })
}

fn read_last(root: &Path) -> Result<Option<WindowsInstallProgress>, MainError> {
    let mut files = fs::read_dir(root)
        .map_err(|_| MainError::Setup)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.len() == 13
                    && path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                    && n.as_bytes()[..8].iter().all(u8::is_ascii_digit)
            })
        })
        .collect::<Vec<_>>();
    files.sort();
    // A torn final write is not completion. Fall back to the last durable stage.
    Ok(files.iter().rev().find_map(|path| {
        plain_path(path, true).ok()?;
        let bytes = fs::read(path).ok()?;
        if bytes.len() > 1024 {
            return None;
        }
        serde_json::from_slice::<WindowsInstallProgress>(&bytes)
            .ok()
            .filter(|p| p.version == 1)
    }))
}

pub(super) trait Platform {
    fn client_preflight(&mut self, request: &InstallWindowsClient) -> Result<(), MainError>;
    fn server_preflight(&mut self, request: &InstallWindowsServer) -> Result<(), MainError>;
    fn elevate(
        &mut self,
        request: &InstallWindowsServer,
        progress: &mut Progress,
    ) -> Result<Option<eitmad_contracts::setup::SetupDesktopHandoff>, MainError>;
    async fn prepare(
        &mut self,
        request: &InstallWindowsServer,
        progress: &mut Progress,
    ) -> Result<(), MainError>;
    fn host(&mut self, request: &InstallWindowsServer, action: &str) -> Result<(), MainError>;
    fn desktop(&mut self, request: &InstallWindowsClient) -> Result<(), MainError>;
}

pub(super) async fn install(
    request: &InstallWindows,
    platform: &mut impl Platform,
) -> Result<(), MainError> {
    let client = match request {
        InstallWindows::Client(client) => client,
        InstallWindows::Server(server) => &server.client,
    };
    let mut progress = Progress::open(&client.progress_directory, request)?;
    let result = async {
        progress.begin(Stage::Preflight)?;
        platform.client_preflight(client)?;
        if let InstallWindows::Server(server) = request {
            progress.begin(Stage::Elevation)?;
            let handoff = platform.elevate(server, &mut progress)?;
            if server.first_desktop.is_some() {
                progress.begin(Stage::EnrollDesktop)?;
                super::guided::enroll_desktop(server, handoff.ok_or(MainError::Setup)?)?;
            }
        }
        progress.begin(Stage::RegisterDesktop)?;
        platform.desktop(client)?;
        Ok(())
    }
    .await;
    progress.finish(result)
}

async fn install_server(
    request: &InstallWindowsServer,
    platform: &mut impl Platform,
    proof_input: &mut impl std::io::Read,
) -> Result<(), MainError> {
    if Path::new(&request.preparation.state_directory)
        .try_exists()
        .map_err(|_| MainError::Setup)?
        || Path::new(&request.service_state_directory)
            .try_exists()
            .map_err(|_| MainError::Setup)?
    {
        let bound =
            recovery::load(Path::new(&request.server_progress_directory)).and_then(|retained| {
                Ok(recovery::binding(&retained)?
                    == recovery::binding(&InstallWindows::Server(Box::new(request.clone())))?)
            });
        if !matches!(bound, Ok(true)) {
            publish(&WindowsInstallProgress {
                version: 1,
                sequence: 1,
                stage: Stage::Preflight,
                status: Status::Blocked,
            })?;
            return Err(MainError::Setup);
        }
    }
    // Validate elevation and protected ancestors before creating any server-owned state.
    if let Err(error) = platform.server_preflight(request) {
        let _ = publish(&WindowsInstallProgress {
            version: 1,
            sequence: 1,
            stage: Stage::Preflight,
            status: if Path::new(&request.service_state_directory).exists() {
                Status::Blocked
            } else {
                Status::Failed
            },
        });
        return Err(error);
    }
    let mut progress = Progress::open(
        &request.server_progress_directory,
        &InstallWindows::Server(Box::new(request.clone())),
    )?;
    let mut handoff = None;
    let result = async {
        progress.begin(Stage::Preflight)?;
        // Preflight has inspected the retained host and its source binding. Once
        // imported, the service database is authoritative; never import the stale copy again.
        let imported = Path::new(&request.service_state_directory)
            .try_exists()
            .map_err(|_| MainError::Setup)?;
        if !imported {
            let retained = Path::new(&request.preparation.state_directory)
                .try_exists()
                .map_err(|_| MainError::Setup)?;
            if let Err(error) = platform.prepare(request, &mut progress).await {
                if retained {
                    progress.record(Stage::PrepareWorkshop, Status::Blocked)?;
                }
                return Err(error);
            }
        }
        for (stage, action) in [
            (
                Stage::InstallServices,
                if imported { "ResumeInstall" } else { "Install" },
            ),
            (Stage::ConfigureFirewall, "Firewall"),
            (Stage::VerifyHttps, "Start"),
        ] {
            progress.begin(stage)?;
            platform.host(request, action)?;
        }
        if request.first_desktop.is_some() {
            progress.begin(Stage::EnrollDesktop)?;
            let proof = read_private(proof_input)?;
            handoff = Some(super::guided::approve_first_desktop(request, proof).await?);
        }
        if request.backup.is_some() {
            progress.begin(Stage::ConfigureBackups)?;
            super::guided::configure_backups(request)?;
        }
        Ok(())
    }
    .await;
    progress.finish(result)?;
    if let Some(handoff) = handoff {
        publish(&handoff)?;
    }
    Ok(())
}
