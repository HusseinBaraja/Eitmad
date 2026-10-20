//! Windows catalog verification precedes setup composition and elevated payload execution.
use super::preparation::native::plain_path;
use crate::MainError;
use eitmad_contracts::setup::{InstallWindowsServer, WindowsSetupBundle};
use std::{
    fs::{File, OpenOptions},
    io::Write as _,
    os::windows::{fs::OpenOptionsExt as _, process::CommandExt as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(super) fn bundle_root() -> Result<PathBuf, MainError> {
    let exe = std::env::current_exe().map_err(|_| MainError::Setup)?;
    let application = exe.parent().ok_or(MainError::Setup)?;
    let server = application.parent().ok_or(MainError::Setup)?;
    if application
        .file_name()
        .is_none_or(|name| name != "application")
        || server.file_name().is_none_or(|name| name != "server")
    {
        return Err(MainError::Setup);
    }
    server
        .parent()
        .map(Path::to_path_buf)
        .ok_or(MainError::Setup)
}

fn retain(path: &Path, handles: &mut Vec<File>) -> Result<(), MainError> {
    plain_path(path, true)?;
    let directory = path.is_dir();
    handles.push(
        OpenOptions::new()
            .read(true)
            .access_mode(if directory { 0 } else { 0x8000_0000 })
            .share_mode(if directory { 3 } else { 1 })
            .custom_flags(if directory { 0x0200_0000 } else { 0 })
            .open(path)
            .map_err(|_| MainError::Setup)?,
    );
    if directory {
        for entry in std::fs::read_dir(path).map_err(|_| MainError::Setup)? {
            retain(&entry.map_err(|_| MainError::Setup)?.path(), handles)?;
        }
    }
    Ok(())
}

pub(super) fn verify(root: &Path) -> Result<Vec<File>, MainError> {
    let mut handles = Vec::new();
    retain(root, &mut handles)?;
    let system = std::env::var_os("SystemRoot").ok_or(MainError::Setup)?;
    let mut child =
        Command::new(Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"))
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(include_str!(
                "../../../../deploy/windows/verify-offline.ps1"
            ))
            .env_remove("PSModulePath")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|_| MainError::Setup)?;
    let input = serde_json::to_vec(&serde_json::json!({
        "root": root, "anchor": std::env::current_exe().map_err(|_| MainError::Setup)?, "publisher": "",
    })).map_err(|_| MainError::Setup)?;
    child
        .stdin
        .take()
        .ok_or(MainError::Setup)?
        .write_all(&input)
        .map_err(|_| MainError::Setup)?;
    if !child.wait().map_err(|_| MainError::Setup)?.success() {
        return Err(MainError::Setup);
    }
    Ok(handles)
}

/// An elevated worker consumes only its protected, reverified copy, never caller-selected tools.
pub(super) fn bind_server(
    request: &mut InstallWindowsServer,
    root: &Path,
) -> Result<(), MainError> {
    let original = Path::new(&request.server.directory)
        .parent()
        .and_then(Path::parent)
        .ok_or(MainError::Input)?
        .to_path_buf();
    let expected = |relative: &str| original.join(relative).to_string_lossy().into_owned();
    if request.server.directory != expected("server/application")
        || request.client.desktop.directory != expected("client/desktop")
        || request.preparation.postgres_bin != expected("server/postgresql/bin")
        || request.preparation.platform_script
            != expected("server/application/deploy/windows/prepare-workshop.ps1")
        || request.preparation.certificate_executable
            != expected(
                "server/application/tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe",
            )
        || request.winsw_executable != expected("server/winsw/WinSW.NET461.exe")
    {
        return Err(MainError::Input);
    }
    let metadata: WindowsSetupBundle = serde_json::from_reader(
        File::open(root.join("setup-bundle.json")).map_err(|_| MainError::Setup)?,
    )
    .map_err(|_| MainError::Input)?;
    if metadata.version != 1
        || request.server.publisher != metadata.publisher
        || request.client.desktop.publisher != metadata.publisher
        || request.server.release_sha256 != metadata.server_release_sha256
        || request.client.desktop.release_sha256 != metadata.desktop_release_sha256
        || request.winsw_sha256 != metadata.winsw_sha256
    {
        return Err(MainError::Input);
    }
    let path = |relative: &str| root.join(relative).to_string_lossy().into_owned();
    request.server.directory = path("server/application");
    request.client.desktop.directory = path("client/desktop");
    request.preparation.postgres_bin = path("server/postgresql/bin");
    request.preparation.platform_script =
        path("server/application/deploy/windows/prepare-workshop.ps1");
    request.preparation.certificate_executable = path(
        "server/application/tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe",
    );
    request.winsw_executable = path("server/winsw/WinSW.NET461.exe");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn retained_payload_cannot_be_overwritten_or_removed_until_released() {
        let outer = std::env::temp_dir().join(format!("eitmad-payload-{}", uuid::Uuid::new_v4()));
        let directory = outer.join("payload");
        std::fs::create_dir_all(&directory).unwrap();
        let file = directory.join("synthetic.txt");
        std::fs::write(&file, b"retained").unwrap();
        let mut handles = Vec::new();
        super::retain(&directory, &mut handles).unwrap();
        assert!(std::fs::write(&file, b"changed").is_err());
        assert!(std::fs::remove_file(&file).is_err());
        assert!(std::fs::rename(&outer, outer.with_extension("moved")).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"retained");
        drop(handles);
        std::fs::remove_file(&file).unwrap();
        std::fs::remove_dir(directory).unwrap();
        std::fs::remove_dir(outer).unwrap();
    }
}
