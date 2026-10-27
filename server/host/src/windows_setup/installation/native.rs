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

pub(in crate::windows_setup) struct WindowsPlatform;

fn invoke(
    action: &str,
    request: &impl serde::Serialize,
    progress: Option<&mut Progress>,
    original: Option<&eitmad_contracts::server::LoginRequest>,
) -> Result<Option<eitmad_contracts::setup::SetupDesktopHandoff>, MainError> {
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
            .env_remove("PSModulePath")
            .arg("-Backend")
            .arg(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|_| MainError::Setup)?;
    let mut errors = child.stderr.take().ok_or(MainError::Setup)?;
    let diagnostics = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = errors.by_ref().take(8192).read_to_end(&mut bytes);
        let _ = std::io::copy(&mut errors, &mut std::io::sink());
        String::from_utf8_lossy(&bytes).into_owned()
    });
    let result = (|| {
        let mut input = Some(child.stdin.take().ok_or(MainError::Setup)?);
        let stream = input.as_mut().ok_or(MainError::Setup)?;
        stream.write_all(&bytes).map_err(|_| MainError::Setup)?;
        if action == "Elevate" {
            stream.write_all(b"\n").map_err(|_| MainError::Setup)?;
            stream.flush().map_err(|_| MainError::Setup)?;
        } else {
            input = None;
        }
        read_progress(&mut child, progress, input.as_mut(), original)
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let details = diagnostics.join().unwrap_or_default();
    result.map_err(|error| {
        if details.is_empty() {
            return error;
        }
        let failure = if let MainError::SetupCheck(failure) = &error {
            Some(*failure)
        } else {
            None
        };
        MainError::SetupDiagnostic(failure, details)
    })
}

fn read_progress(
    child: &mut std::process::Child,
    mut progress: Option<&mut Progress>,
    mut input: Option<&mut std::process::ChildStdin>,
    original: Option<&eitmad_contracts::server::LoginRequest>,
) -> Result<Option<eitmad_contracts::setup::SetupDesktopHandoff>, MainError> {
    let mut reader = BufReader::new(child.stdout.take().ok_or(MainError::Setup)?);
    let mut sequence = 0;
    let mut ready = false;
    let mut complete = false;
    let mut handoff = None;
    let mut proof_sent = false;
    loop {
        let mut line = Vec::new();
        // Only typed bounded progress can cross back from the elevated worker.
        let read = reader
            .by_ref()
            .take(32769)
            .read_until(b'\n', &mut line)
            .map_err(|_| MainError::Setup)?;
        if read == 0 {
            break;
        }
        if read > 32768 {
            return Err(MainError::Setup);
        }
        if complete {
            if handoff.is_some() {
                return Err(MainError::Setup);
            }
            handoff = Some(
                serde_json::from_slice::<eitmad_contracts::setup::SetupDesktopHandoff>(&line)
                    .map_err(|_| MainError::Setup)?,
            );
            continue;
        }
        let event: WindowsInstallProgress =
            serde_json::from_slice(&line).map_err(|_| MainError::Setup)?;
        if event.version != 1 || event.sequence != sequence + 1 {
            return Err(MainError::Setup);
        }
        sequence = event.sequence;
        let target = progress.as_deref_mut().ok_or(MainError::Setup)?;
        if event.stage == Stage::EnrollDesktop && event.status == Status::Running {
            if !ready || proof_sent {
                return Err(MainError::Setup);
            }
            // This process still runs as the original desktop user. The
            // elevated worker receives only the fresh public proof.
            let proof =
                super::super::guided::refresh_first_desktop(original.ok_or(MainError::Input)?)?;
            let bytes = Zeroizing::new(serde_json::to_vec(&proof).map_err(|_| MainError::Setup)?);
            let input = input.as_mut().ok_or(MainError::Setup)?;
            input.write_all(&bytes).map_err(|_| MainError::Setup)?;
            input.write_all(b"\n").map_err(|_| MainError::Setup)?;
            input.flush().map_err(|_| MainError::Setup)?;
            proof_sent = true;
        }
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
            ready |= event.stage == Stage::VerifyHttps && event.status == Status::Running;
            target.failure = event.failure;
            target.details = event.details;
            target.record(event.stage, event.status)?;
        }
    }
    let status = child.wait().map_err(|_| MainError::Setup)?;
    if !status.success() {
        use eitmad_contracts::setup::WindowsSetupFailure as Failure;
        return Err(match status.code() {
            Some(10) => MainError::SetupCheck(Failure::DesktopUser),
            Some(11) => MainError::SetupCheck(Failure::DesktopPackage),
            Some(12) => MainError::SetupCheck(Failure::ServerPackage),
            Some(13) => MainError::SetupCheck(Failure::ProtectedPaths),
            Some(14) => MainError::SetupCheck(Failure::Prerequisites),
            Some(15) => MainError::SetupCheck(Failure::RetainedState),
            _ => MainError::Setup,
        });
    }
    if progress.is_some() && !complete {
        return Err(MainError::Setup);
    }
    Ok(handoff)
}

impl Platform for WindowsPlatform {
    fn client_preflight(&mut self, request: &InstallWindowsClient) -> Result<(), MainError> {
        invoke("ClientPreflight", request, None, None).map(|_| ())
    }
    fn server_preflight(&mut self, request: &InstallWindowsServer) -> Result<(), MainError> {
        invoke("ServerPreflight", request, None, None).map(|_| ())
    }
    fn elevate(
        &mut self,
        request: &InstallWindowsServer,
        progress: &mut Progress,
    ) -> Result<Option<eitmad_contracts::setup::SetupDesktopHandoff>, MainError> {
        invoke(
            "Elevate",
            request,
            Some(progress),
            request.first_desktop.as_ref(),
        )
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
        invoke(action, request, None, None)?;
        if matches!(action, "Install" | "ResumeInstall") {
            crate::windows_tls::configure_setup(
                &request.service_state_directory,
                &request.preparation.state_directory,
                &request.preparation.certificate_executable,
            )?;
        }
        Ok(())
    }
    fn desktop(&mut self, request: &InstallWindowsClient) -> Result<(), MainError> {
        invoke("Desktop", request, None, None).map(|_| ())
    }
}
