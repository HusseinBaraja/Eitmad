use super::{MainError, PrepareWindowsWorkshop, Secrets};
use std::os::windows::{fs::MetadataExt as _, process::CommandExt as _};
use std::{
    io::{BufRead as _, BufReader, Read, Write as _},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

pub(in crate::windows_setup) fn plain_path(path: &Path, required: bool) -> Result<(), MainError> {
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
        .stderr(Stdio::piped())
        .creation_flags(0x0800_0000);
    // Ambient libpq settings cannot redirect this setup's local database tools.
    for (name, _) in
        std::env::vars_os().filter(|(name, _)| name.to_string_lossy().starts_with("PG"))
    {
        command.env_remove(name);
    }
    let mut child = command.spawn().map_err(|_| MainError::Setup)?;
    let diagnostics = read_diagnostic(child.stderr.take().ok_or(MainError::Setup)?);
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
            if !status.success() {
                let details = diagnostics
                    .recv_timeout(Duration::from_millis(250))
                    .unwrap_or_default();
                return Err(MainError::SetupDiagnostic(
                    None,
                    format!("Workshop preparation {action} exited with {status}. {details}"),
                ));
            }
            write?;
            return Ok(());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(MainError::SetupDiagnostic(
                None,
                format!("Workshop preparation {action} exceeded its 120-second limit."),
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn read_diagnostic(errors: impl Read + Send + 'static) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        // pg_ctl can leave stderr open in PostgreSQL. A complete diagnostic line
        // is sufficient; never join this reader or wait for EOF on success.
        let _ = BufReader::new(errors)
            .take(8192)
            .read_until(b'\n', &mut bytes);
        let _ = sender.send(String::from_utf8_lossy(&bytes).into_owned());
    });
    receiver
}

#[cfg(test)]
mod tests {
    use super::read_diagnostic;
    use std::{io::Write as _, net::TcpListener, time::Duration};

    #[test]
    fn diagnostic_arrives_while_inherited_handle_remains_open() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut writer = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (reader, _) = listener.accept().unwrap();
        let diagnostics = read_diagnostic(reader);
        let message = "{\"message\":\"The database port is already used. ورشة اختبار\"}\n";
        writer.write_all(message.as_bytes()).unwrap();
        assert_eq!(
            diagnostics.recv_timeout(Duration::from_secs(2)).unwrap(),
            message
        );
        // The writer stays open through the assertion, as PostgreSQL's handle does.
        drop(writer);
    }

    #[test]
    fn native_diagnostic_is_bounded_without_a_line_ending() {
        let diagnostics = read_diagnostic(std::io::Cursor::new(vec![b'x'; 10000]));
        assert_eq!(
            diagnostics.recv_timeout(Duration::from_secs(2)).unwrap(),
            "x".repeat(8192)
        );
    }
}
