//! Bundle-to-installation composition for the native setup. No shell-owned settings or secrets.
use super::{installation, preparation::native::plain_path};
use crate::MainError;
use eitmad_contracts::{
    ipc::DesktopSignInRequest,
    setup::{
        GuidedSetupAction, GuidedWindowsSetup, InstallWindows, InstallWindowsClient,
        InstallWindowsServer, PrepareWindowsWorkshop, PreparedWindowsWorkshop, SetupDesktopHandoff,
        WindowsInstallPackage, WindowsSetupBundle,
    },
    transport::{CorrelationId, RequestId},
};
use eitmad_server_connection::DesktopEnrollment;
use std::{
    io::Read as _,
    net::ToSocketAddrs as _,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

fn local_root() -> Result<PathBuf, MainError> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or(MainError::Setup)
}
fn enrollment() -> Result<DesktopEnrollment, MainError> {
    let directory = local_root()?.join("Eitmad/engine");
    plain_path(&directory, false)?;
    std::fs::create_dir_all(&directory).map_err(|_| MainError::Setup)?;
    let store = eitmad_storage::AuthorityStore::open(&directory).map_err(|_| MainError::Setup)?;
    let secrets =
        eitmad_secret_storage::SecretStore::open(&directory, None).map_err(|_| MainError::Setup)?;
    Ok(DesktopEnrollment::new(directory, store, secrets))
}

fn require_empty_desktop(directory: &Path) -> Result<(), MainError> {
    plain_path(directory, false)?;
    if directory.try_exists().map_err(|_| MainError::Setup)?
        && std::fs::read_dir(directory)
            .map_err(|_| MainError::Setup)?
            .next()
            .is_some()
    {
        return Err(MainError::Setup);
    }
    Ok(())
}

pub(super) async fn execute() -> Result<(), MainError> {
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| MainError::Input)?;
    if bytes.len() > 65536 {
        return Err(MainError::Input);
    }
    let request: GuidedWindowsSetup =
        serde_json::from_slice(&bytes).map_err(|_| MainError::Input)?;
    if request.version != 1 {
        return Err(MainError::Input);
    }
    let progress = local_root()?.join("EitmadSetupProgress");
    if matches!(request.action, GuidedSetupAction::Inspect) {
        let state = if progress.try_exists().map_err(|_| MainError::Setup)? {
            Some(installation::read_state(&progress)?)
        } else {
            None
        };
        println!(
            "{}",
            serde_json::to_string(&state).map_err(|_| MainError::Setup)?
        );
        return Ok(());
    }
    let install = compose(request, progress)?;
    installation::install(&install, &mut installation::native::WindowsPlatform).await
}

fn compose(request: GuidedWindowsSetup, progress: PathBuf) -> Result<InstallWindows, MainError> {
    let bundle = Path::new(&request.bundle_directory);
    plain_path(bundle, true)?;
    let metadata = bundle.join("setup-bundle.json");
    plain_path(&metadata, true)?;
    let profile: WindowsSetupBundle = serde_json::from_reader(
        std::fs::File::open(metadata)
            .map_err(|_| MainError::Input)?
            .take(16385),
    )
    .map_err(|_| MainError::Input)?;
    if profile.version != 1 {
        return Err(MainError::Input);
    }
    let text = |path: PathBuf| path.to_string_lossy().into_owned();
    let client = InstallWindowsClient {
        version: 1,
        progress_directory: text(progress),
        intended_user_sid: request.intended_user_sid,
        desktop: WindowsInstallPackage {
            directory: text(bundle.join("client/desktop")),
            release_sha256: profile.desktop_release_sha256,
            publisher: profile.publisher.clone(),
        },
    };
    installation::validate_client(&client)?;
    let install = if matches!(request.action, GuidedSetupAction::Client) {
        InstallWindows::Client(client)
    } else {
        let workshop = request.workshop.ok_or(MainError::Input)?;
        let program_data = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .ok_or(MainError::Setup)?;
        let application = bundle.join("server/application");
        let server_name = std::env::var("COMPUTERNAME").map_err(|_| MainError::Input)?;
        let address = (server_name.as_str(), 58443)
            .to_socket_addrs()
            .map_err(|_| MainError::Input)?
            .find_map(|a| match a.ip() {
                std::net::IpAddr::V4(ip) if ip.is_private() => Some(ip.to_string()),
                _ => None,
            })
            .ok_or(MainError::Input)?;
        let mut server = InstallWindowsServer {
            version: 1,
            client,
            preparation: PrepareWindowsWorkshop {
                version: 1,
                state_directory: text(program_data.join("EitmadPreparedWorkshop")),
                platform_script: text(application.join("deploy/windows/prepare-workshop.ps1")),
                postgres_bin: text(bundle.join("server/postgresql/bin")),
                certificate_executable: text(application.join(
                    "tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe",
                )),
                server_name,
                listen_address: address,
                server_port: 58443,
                database_port: 55439,
                workshop,
            },
            server: WindowsInstallPackage {
                directory: text(application),
                release_sha256: profile.server_release_sha256,
                publisher: profile.publisher,
            },
            service_state_directory: text(program_data.join("EitmadHost")),
            service_name: "Eitmad".into(),
            server_progress_directory: text(program_data.join("EitmadServerSetupProgress")),
            winsw_executable: text(bundle.join("server/winsw/WinSW.NET461.exe")),
            winsw_sha256: profile.winsw_sha256,
            first_desktop: None,
            backup: request.backup.map(|mut backup| {
                if backup.local.is_empty() {
                    backup.local = text(program_data.join("EitmadBackups"));
                }
                backup
            }),
        };
        installation::validate_server(&server)?;
        // Reject existing user data before opening or migrating an authority store.
        require_empty_desktop(&local_root()?.join("Eitmad/engine"))?;
        let owner = &server.preparation.workshop;
        server.first_desktop = Some(
            enrollment()?
                .first_desktop_request(
                    eitmad_contracts::server::TenantCode::parse("setup-pending")
                        .map_err(|_| MainError::Input)?,
                    owner.owner_username.clone(),
                    owner.owner_password.expose_secret().into(),
                )
                .map_err(|_| MainError::Setup)?,
        );
        InstallWindows::Server(Box::new(server))
    };
    Ok(install)
}

pub(super) async fn approve_first_desktop(
    request: &InstallWindowsServer,
) -> Result<SetupDesktopHandoff, MainError> {
    let root = Path::new(&request.service_state_directory);
    let database_url = Zeroizing::new(
        std::fs::read_to_string(root.join("private/database-url")).map_err(|_| MainError::Setup)?,
    );
    let database = eitmad_control_plane::ControlDatabase::connect(&database_url, 1)
        .await
        .map_err(|_| MainError::Database)?;
    let prepared: PreparedWindowsWorkshop = serde_json::from_reader(
        std::fs::File::open(
            Path::new(&request.preparation.state_directory).join("prepared-workshop.json"),
        )
        .map_err(|_| MainError::Setup)?
        .take(16385),
    )
    .map_err(|_| MainError::Setup)?;
    let mut login = request.first_desktop.clone().ok_or(MainError::Input)?;
    login.tenant_code = prepared.workshop.tenant_code.clone();
    let result = eitmad_control_plane::PairingService::new(database.pool())
        .enroll_first_desktop(
            &login,
            CorrelationId::new(uuid::Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await;
    zeroize::Zeroize::zeroize(&mut login.password);
    result.map_err(|_| MainError::Setup)?;
    let public_ca_pem = std::fs::read_to_string(root.join("private/trusted-ca.pem"))
        .map_err(|_| MainError::Setup)?;
    if public_ca_pem.len() > 16384 || public_ca_pem.contains("PRIVATE KEY") {
        return Err(MainError::Setup);
    }
    Ok(SetupDesktopHandoff {
        version: 1,
        prepared,
        public_ca_pem,
    })
}

pub(super) fn enroll_desktop(
    request: &InstallWindowsServer,
    handoff: SetupDesktopHandoff,
) -> Result<(), MainError> {
    if handoff.version != 1
        || handoff.prepared.version != 1
        || handoff.public_ca_pem.len() > 16384
        || handoff.public_ca_pem.contains("PRIVATE KEY")
    {
        return Err(MainError::Setup);
    }
    let ca = Path::new(&request.client.progress_directory).join("first-desktop-ca.pem");
    super::preparation::write_new(&ca, handoff.public_ca_pem.as_bytes())?;
    let owner = &request.preparation.workshop;
    let coordinates = handoff.prepared.workshop;
    let mut sign_in = DesktopSignInRequest {
        request_id: RequestId::new(uuid::Uuid::new_v4()),
        correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
        username: owner.owner_username.clone(),
        password: owner.owner_password.expose_secret().into(),
        server_login: true,
        connection: Some(eitmad_contracts::ipc::DesktopEnrollmentSettings {
            endpoint: handoff.prepared.endpoint,
            trusted_certificate: ca.to_string_lossy().into_owned(),
            tenant_code: coordinates.tenant_code,
            organization: coordinates.organization_id,
            branch: coordinates.branch_id,
        }),
    };
    let result = enrollment()?.sign_in(&sign_in);
    zeroize::Zeroize::zeroize(&mut sign_in.password);
    result.map(|_| ()).map_err(|_| MainError::Setup)
}

pub(super) fn configure_backups(request: &InstallWindowsServer) -> Result<(), MainError> {
    let destinations = request.backup.as_ref().ok_or(MainError::Input)?;
    let program_data = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .ok_or(MainError::Setup)?;
    let state = program_data.join("EitmadBackupState");
    let configuration = eitmad_contracts::backup::ServerBackupConfiguration {
        version: 1,
        deployment_id: uuid::Uuid::new_v4(),
        host_root: request.service_state_directory.clone(),
        service_name: request.service_name.clone(),
        recovery_directory: request.preparation.state_directory.clone(),
        local_destination: destinations.local.clone(),
        external_destination: destinations.external.clone(),
        interval_hours: 24,
        retention_count: 7,
        reserve_bytes: 1_073_741_824,
    };
    crate::windows_backup::configure_setup(&state, &configuration)
}

#[cfg(test)]
mod tests {
    #[test]
    fn new_server_refuses_existing_desktop_data_before_opening_the_store() {
        let directory =
            std::env::temp_dir().join(format!("eitmad-guided-{}", uuid::Uuid::new_v4()));
        assert!(super::require_empty_desktop(&directory).is_ok());
        std::fs::create_dir(&directory).unwrap();
        assert!(super::require_empty_desktop(&directory).is_ok());
        let saved = directory.join("synthetic-saved-work");
        std::fs::write(&saved, b"preserved synthetic data").unwrap();
        assert!(super::require_empty_desktop(&directory).is_err());
        assert_eq!(std::fs::read(&saved).unwrap(), b"preserved synthetic data");
        std::fs::remove_file(saved).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
