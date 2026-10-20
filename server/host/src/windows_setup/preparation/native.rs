use super::{MainError, PrepareWindowsWorkshop, Secrets};
use std::os::windows::{fs::MetadataExt as _, process::CommandExt as _};
use std::{
    io::Write as _,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

pub(super) fn plain_path(path: &Path, required: bool) -> Result<(), MainError> {
    let text = path.to_string_lossy();
    if !path.is_absolute() || text.starts_with(r"\\") || text.contains(['"', '%', '\r', '\n']) {
        return Err(MainError::Input);
    }
    if required && !path.try_exists().map_err(|_| MainError::Setup)? {
        return Err(MainError::Input);
    }
    for ancestor in path.ancestors() {
        match ancestor.symlink_metadata() {
            Ok(metadata) if metadata.file_attributes() & 0x400 != 0 => {
                return Err(MainError::Setup);
            }
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(MainError::Setup),
        }
    }
    Ok(())
}

pub(super) fn invoke(
    request: &PrepareWindowsWorkshop,
    action: &str,
    secrets: Option<&Secrets>,
) -> Result<(), MainError> {
    let system = std::env::var_os("SystemRoot").ok_or(MainError::Setup)?;
    let mut command =
        Command::new(Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"));
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-File",
            &request.platform_script,
            "-Action",
            action,
            "-StateDirectory",
            &request.state_directory,
            "-PostgresBin",
            &request.postgres_bin,
            "-CertificateExecutable",
            &request.certificate_executable,
            "-ServerName",
            &request.server_name,
            "-DatabasePort",
            &request.database_port.to_string(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0000);
    // Ambient libpq settings cannot redirect this setup's local database tools.
    for (name, _) in
        std::env::vars_os().filter(|(name, _)| name.to_string_lossy().starts_with("PG"))
    {
        command.env_remove(name);
    }
    let mut child = command.spawn().map_err(|_| MainError::Setup)?;
    let write = if let Some(secrets) = secrets {
        let bytes = Zeroizing::new(serde_json::to_vec(&serde_json::json!({
            "adminPassword": secrets.admin.as_str(), "certificatePassword": secrets.certificate.as_str(),
        })).map_err(|_| MainError::Setup)?);
        child
            .stdin
            .take()
            .ok_or(MainError::Setup)?
            .write_all(&bytes)
            .map_err(|_| MainError::Setup)
    } else {
        drop(child.stdin.take());
        Ok(())
    };
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(status) = child.try_wait().map_err(|_| MainError::Setup)? {
            write?;
            return if status.success() {
                Ok(())
            } else {
                Err(MainError::Setup)
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(MainError::Setup);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
