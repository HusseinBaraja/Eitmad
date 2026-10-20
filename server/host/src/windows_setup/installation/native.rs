use super::{
    InstallWindowsClient, InstallWindowsServer, MainError, Platform, Progress, Stage, Status,
    WindowsInstallProgress, plain_path,
};
use std::{
    io::{BufRead as _, BufReader, Read as _, Write as _},
    os::windows::process::CommandExt as _,
    path::Path,
    process::{Command, Stdio},
};
use zeroize::Zeroizing;

pub(super) struct WindowsPlatform;

fn invoke(
    action: &str,
    request: &impl serde::Serialize,
    mut progress: Option<&mut Progress>,
) -> Result<(), MainError> {
    let executable = std::env::current_exe().map_err(|_| MainError::Setup)?;
    let script = executable
        .parent()
        .ok_or(MainError::Setup)?
        .join("deploy/windows/installation.ps1");
    plain_path(&script, true)?;
    let system = std::env::var_os("SystemRoot").ok_or(MainError::Setup)?;
    let bytes = Zeroizing::new(serde_json::to_vec(request).map_err(|_| MainError::Input)?);
    let mut child =
        Command::new(Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"))
            .args(["-NoProfile", "-NonInteractive", "-File"])
            .arg(script)
            .args(["-Action", action])
            .arg("-Backend")
            .arg(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|_| MainError::Setup)?;
    let result = (|| {
        child
            .stdin
            .take()
            .ok_or(MainError::Setup)?
            .write_all(&bytes)
            .map_err(|_| MainError::Setup)?;
        let mut reader = BufReader::new(child.stdout.take().ok_or(MainError::Setup)?);
        let mut sequence = 0;
        let mut ready = false;
        let mut complete = false;
        loop {
            let mut line = Vec::new();
            // Only typed bounded progress can cross back from the elevated worker.
            let read = reader
                .by_ref()
                .take(1025)
                .read_until(b'\n', &mut line)
                .map_err(|_| MainError::Setup)?;
            if read == 0 {
                break;
            }
            if read > 1024 || complete {
                return Err(MainError::Setup);
            }
            let event: WindowsInstallProgress =
                serde_json::from_slice(&line).map_err(|_| MainError::Setup)?;
            if event.version != 1 || event.sequence != sequence + 1 {
                return Err(MainError::Setup);
            }
            sequence = event.sequence;
            let target = progress.as_deref_mut().ok_or(MainError::Setup)?;
            if event.stage == Stage::Complete {
                if !ready || event.status != Status::Succeeded {
                    return Err(MainError::Setup);
                }
                complete = true;
                target.record(Stage::VerifyHttps, Status::Succeeded)?;
            } else {
                if matches!(event.stage, Stage::Elevation | Stage::RegisterDesktop) {
                    return Err(MainError::Setup);
                }
                ready = event.stage == Stage::VerifyHttps && event.status == Status::Running;
                target.record(event.stage, event.status)?;
            }
        }
        if !child.wait().map_err(|_| MainError::Setup)?.success()
            || (progress.is_some() && !complete)
        {
            return Err(MainError::Setup);
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

impl Platform for WindowsPlatform {
    fn client_preflight(&mut self, request: &InstallWindowsClient) -> Result<(), MainError> {
        invoke("ClientPreflight", request, None)
    }
    fn server_preflight(&mut self, request: &InstallWindowsServer) -> Result<(), MainError> {
        invoke("ServerPreflight", request, None)
    }
    fn elevate(
        &mut self,
        request: &InstallWindowsServer,
        progress: &mut Progress,
    ) -> Result<(), MainError> {
        invoke("Elevate", request, Some(progress))
    }
    async fn prepare(
        &mut self,
        request: &InstallWindowsServer,
        progress: &mut Progress,
    ) -> Result<(), MainError> {
        super::preparation::run(&request.preparation, &mut |stage| progress.begin(stage)).await?;
        Ok(())
    }
    fn host(&mut self, request: &InstallWindowsServer, action: &str) -> Result<(), MainError> {
        invoke(action, request, None)
    }
    fn desktop(&mut self, request: &InstallWindowsClient) -> Result<(), MainError> {
        invoke("Desktop", request, None)
    }
}
