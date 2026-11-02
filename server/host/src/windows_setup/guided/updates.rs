//! New releases get separate journals. The installed host remains the data authority.
use super::*;
use eitmad_contracts::setup::{
    WindowsInstallProgress, WindowsInstallStage, WindowsInstallStatus, WindowsInstallationState,
    WindowsSetupRecovery,
};

fn release_key(profile: &WindowsSetupBundle) -> Result<String, MainError> {
    for digest in [
        &profile.desktop_release_sha256,
        &profile.server_release_sha256,
    ] {
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(MainError::Input);
        }
    }
    let role = match profile.role {
        WindowsSetupRole::Server => "server",
        WindowsSetupRole::Client => "client",
    };
    Ok(format!(
        "EitmadSetupUpdate-{role}-{}-{}",
        profile.desktop_release_sha256, profile.server_release_sha256
    ))
}

pub(super) fn progress_path(profile: &WindowsSetupBundle) -> Result<PathBuf, MainError> {
    Ok(local_root()?.join(release_key(profile)?))
}

fn same_release(request: &InstallWindows, profile: &WindowsSetupBundle) -> bool {
    let client = match request {
        InstallWindows::Client(client) => client,
        InstallWindows::Server(server) => {
            if profile.role != WindowsSetupRole::Server
                || server.server.release_sha256 != profile.server_release_sha256
            {
                return false;
            }
            &server.client
        }
    };
    client.desktop.publisher == profile.publisher
        && client.desktop.release_sha256 == profile.desktop_release_sha256
        && (matches!(request, InstallWindows::Server(_))
            || profile.role == WindowsSetupRole::Client)
}

pub(super) fn inspect(
    original: &Path,
    profile: &WindowsSetupBundle,
) -> Result<Option<WindowsInstallationState>, MainError> {
    let update = progress_path(profile)?;
    inspect_journals(
        original,
        &update,
        profile.role == WindowsSetupRole::Server && host_root()?.is_dir(),
        profile,
    )
}

fn inspect_journals(
    original: &Path,
    update: &Path,
    host_exists: bool,
    profile: &WindowsSetupBundle,
) -> Result<Option<WindowsInstallationState>, MainError> {
    if update.try_exists().map_err(|_| MainError::Setup)? {
        return installation::read_state(update).map(Some);
    }
    if original.try_exists().map_err(|_| MainError::Setup)? {
        let mut state = installation::read_state(original)?;
        let retained = installation::recovery::load(original)?;
        if state.recovery != WindowsSetupRecovery::Complete {
            if !same_release(&retained, profile) {
                state.recovery = WindowsSetupRecovery::InspectManually;
            }
            return Ok(Some(state));
        }
        if same_release(&retained, profile) {
            return Ok(Some(state));
        }
    } else if !host_exists {
        return Ok(None);
    }
    Ok(Some(WindowsInstallationState {
        version: 1,
        progress: WindowsInstallProgress {
            version: 1,
            sequence: 0,
            stage: WindowsInstallStage::Preflight,
            status: WindowsInstallStatus::Succeeded,
            failure: None,
            details: None,
        },
        interrupted: false,
        recovery: WindowsSetupRecovery::Update,
    }))
}

fn program_data() -> Result<PathBuf, MainError> {
    std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .ok_or(MainError::Setup)
}

fn host_root() -> Result<PathBuf, MainError> {
    Ok(program_data()?.join("EitmadHost"))
}

pub(super) fn compose(
    request: GuidedWindowsSetup,
    progress: PathBuf,
    profile: &WindowsSetupBundle,
) -> Result<InstallWindows, MainError> {
    if !inspect(&local_root()?.join("EitmadSetupProgress"), profile)?
        .is_some_and(|state| state.recovery == WindowsSetupRecovery::Update)
    {
        return Err(MainError::Setup);
    }
    let bundle = Path::new(&request.bundle_directory);
    let text = |path: PathBuf| path.to_string_lossy().into_owned();
    let client = InstallWindowsClient {
        version: 1,
        progress_directory: text(progress),
        intended_user_sid: request.intended_user_sid,
        desktop: WindowsInstallPackage {
            directory: text(bundle.join("client/desktop")),
            release_sha256: profile.desktop_release_sha256.clone(),
            publisher: profile.publisher.clone(),
        },
    };
    installation::validate_client(&client)?;
    if profile.role == WindowsSetupRole::Client {
        return Ok(InstallWindows::Client(client));
    }
    // Read credential-free original settings, including the installed network binding.
    // Elevated preflight checks them against the protected host and owned services.
    let InstallWindows::Server(mut server) =
        installation::recovery::load(&program_data()?.join("EitmadServerSetupProgress"))?
    else {
        return Err(MainError::Setup);
    };
    if Path::new(&server.service_state_directory) != host_root()? || server.service_name != "Eitmad"
    {
        return Err(MainError::Setup);
    }
    server.update_existing = true;
    server.first_desktop = None;
    server.client = client;
    server.server.directory = text(bundle.join("server/application"));
    server
        .server
        .release_sha256
        .clone_from(&profile.server_release_sha256);
    server.server.publisher.clone_from(&profile.publisher);
    server.winsw_sha256.clone_from(&profile.winsw_sha256);
    server.winsw_executable = text(bundle.join("server/winsw/WinSW.NET461.exe"));
    server.preparation.postgres_bin = text(bundle.join("server/postgresql/bin"));
    server.preparation.platform_script =
        text(bundle.join("server/application/deploy/windows/prepare-workshop.ps1"));
    server.preparation.certificate_executable = text(bundle.join("server/application/tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe"));
    server.server_progress_directory =
        text(program_data()?.join(format!("EitmadServerSetupUpdate-{}", uuid::Uuid::new_v4())));
    super::super::payload::bind_server(&mut server, bundle)?;
    installation::validate_server(&server)?;
    Ok(InstallWindows::Server(server))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        root: &Path,
        sequence: u32,
        stage: WindowsInstallStage,
        status: WindowsInstallStatus,
    ) {
        let event = WindowsInstallProgress {
            version: 1,
            sequence,
            stage,
            status,
            failure: None,
            details: None,
        };
        std::fs::write(
            root.join(format!("{sequence:08}.json")),
            serde_json::to_vec(&event).unwrap(),
        )
        .unwrap();
    }

    fn client_request(root: &Path, progress: &Path) -> InstallWindows {
        InstallWindows::Client(InstallWindowsClient {
            version: 1,
            progress_directory: progress.to_string_lossy().into_owned(),
            intended_user_sid: "S-1-5-21-1001".into(),
            desktop: WindowsInstallPackage {
                directory: root.to_string_lossy().into_owned(),
                release_sha256: "a".repeat(64),
                publisher: "CN=Synthetic".into(),
            },
        })
    }

    #[test]
    fn newer_release_offers_update_and_keeps_original_and_failed_update_journals() {
        let root =
            std::env::temp_dir().join(format!("eitmad-update-inspection-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let original = root.join("original");
        let update = root.join("update");
        let mut request = client_request(&root, &original);
        let mut profile = WindowsSetupBundle {
            version: 2,
            role: WindowsSetupRole::Client,
            publisher: "CN=Synthetic".into(),
            desktop_release_sha256: "a".repeat(64),
            server_release_sha256: "b".repeat(64),
            winsw_sha256: "c".repeat(64),
        };
        let progress = installation::Progress::open(original.to_str().unwrap(), &request).unwrap();
        record(
            &original,
            1,
            WindowsInstallStage::RegisterDesktop,
            WindowsInstallStatus::Running,
        );
        drop(progress);
        profile.desktop_release_sha256 = "d".repeat(64);
        assert_eq!(
            inspect_journals(&original, &update, false, &profile)
                .unwrap()
                .unwrap()
                .recovery,
            WindowsSetupRecovery::InspectManually
        );
        record(
            &original,
            2,
            WindowsInstallStage::Complete,
            WindowsInstallStatus::Succeeded,
        );
        let before = std::fs::read(original.join("recovery-input.json")).unwrap();
        assert_eq!(
            inspect_journals(&original, &update, false, &profile)
                .unwrap()
                .unwrap()
                .recovery,
            WindowsSetupRecovery::Update
        );
        let InstallWindows::Client(client) = &mut request else {
            unreachable!()
        };
        client.progress_directory = update.to_string_lossy().into_owned();
        client
            .desktop
            .release_sha256
            .clone_from(&profile.desktop_release_sha256);
        let progress = installation::Progress::open(update.to_str().unwrap(), &request).unwrap();
        record(
            &update,
            1,
            WindowsInstallStage::RegisterDesktop,
            WindowsInstallStatus::Running,
        );
        assert_eq!(
            inspect_journals(&original, &update, false, &profile)
                .unwrap()
                .unwrap()
                .recovery,
            WindowsSetupRecovery::Wait
        );
        drop(progress);
        record(
            &update,
            2,
            WindowsInstallStage::RegisterDesktop,
            WindowsInstallStatus::Failed,
        );
        assert_eq!(
            inspect_journals(&original, &update, false, &profile)
                .unwrap()
                .unwrap()
                .recovery,
            WindowsSetupRecovery::Retry
        );
        assert_eq!(
            std::fs::read(original.join("recovery-input.json")).unwrap(),
            before
        );
        record(
            &update,
            3,
            WindowsInstallStage::Complete,
            WindowsInstallStatus::Succeeded,
        );
        assert_eq!(
            inspect_journals(&original, &update, false, &profile)
                .unwrap()
                .unwrap()
                .recovery,
            WindowsSetupRecovery::Complete
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
