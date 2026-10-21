//! Post-installation Windows administration through the existing protected elevation channel.
use super::{Configuration, MainError, Outcome, Saved, Status, native};
use eitmad_contracts::backup::{
    ManageServerBackup, ServerBackupAction as Action, ServerBackupManagement,
};
use eitmad_contracts::setup::SetupBackupDestinations;
use std::{
    io::{Read as _, Write as _},
    os::windows::process::CommandExt as _,
    path::Path,
    process::{Command, Stdio},
};

fn request() -> Result<ManageServerBackup, MainError> {
    let mut input = Vec::new();
    std::io::stdin()
        .take(65_537)
        .read_to_end(&mut input)
        .map_err(|_| MainError::Input)?;
    if input.len() > 65_536 {
        return Err(MainError::Input);
    }
    let request: ManageServerBackup =
        serde_json::from_slice(&input).map_err(|_| MainError::Input)?;
    validate_request(&request)?;
    Ok(request)
}

fn validate_request(request: &ManageServerBackup) -> Result<(), MainError> {
    if request.version != 1
        || (request.action == Action::SaveDestinations) != request.destinations.is_some()
    {
        return Err(MainError::Input);
    }
    Ok(())
}

pub(crate) fn elevate_management() -> Result<(), MainError> {
    let request = request()?;
    let executable = std::env::current_exe().map_err(|_| MainError::Backup)?;
    let script = executable
        .parent()
        .ok_or(MainError::Backup)?
        .join("deploy/windows/installation.ps1");
    let system = std::env::var_os("SystemRoot").ok_or(MainError::Backup)?;
    let mut child =
        Command::new(Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"))
            .args(["-NoProfile", "-NonInteractive", "-File"])
            .arg(script)
            .args(["-Action", "BackupElevate", "-Backend"])
            .arg(executable)
            .env_remove("PSModulePath")
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|_| MainError::Backup)?;
    let mut input = child.stdin.take().ok_or(MainError::Backup)?;
    serde_json::to_writer(&mut input, &request).map_err(|_| MainError::Backup)?;
    input.write_all(b"\n").map_err(|_| MainError::Backup)?;
    drop(input);
    if child.wait().map_err(|_| MainError::Backup)?.success() {
        Ok(())
    } else {
        Err(MainError::Backup)
    }
}

fn invoke<T: serde::de::DeserializeOwned>(
    backend: &Path,
    state: &Path,
    action: &str,
    configuration: Option<&Configuration>,
) -> Result<(T, bool), MainError> {
    let mut child = Command::new(backend)
        .args(["windows-backup", action])
        .arg(state)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .map_err(|_| MainError::Backup)?;
    if let Some(configuration) = configuration {
        serde_json::to_writer(child.stdin.take().ok_or(MainError::Backup)?, configuration)
            .map_err(|_| MainError::Backup)?;
    } else {
        drop(child.stdin.take());
    }
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .ok_or(MainError::Backup)?
        .take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|_| MainError::Backup)?;
    if bytes.len() > 65_536 {
        let _ = child.kill();
        let _ = child.wait();
        return Err(MainError::Backup);
    }
    let success = child.wait().map_err(|_| MainError::Backup)?.success();
    Ok((
        serde_json::from_slice(&bytes).map_err(|_| MainError::Backup)?,
        success,
    ))
}

pub(crate) fn manage() -> Result<(), MainError> {
    let request = request()?;
    let state = std::env::var_os("ProgramData")
        .map(std::path::PathBuf::from)
        .ok_or(MainError::Backup)?
        .join("EitmadBackupState");
    // This OS guard precedes every private read, including an inspect request.
    if native("Guard", &state, &serde_json::json!({}))?.outcome != Outcome::Succeeded {
        return Err(MainError::Backup);
    }
    let saved: Saved = serde_json::from_reader(
        std::fs::File::open(state.join("state.json"))
            .map_err(|_| MainError::Backup)?
            .take(65_537),
    )
    .map_err(|_| MainError::Backup)?;
    let backend = saved.backend.as_deref().ok_or(MainError::Backup)?;
    // Reuse the retained scheduler backend. A new temporary release path must not rebind its task.
    if native(
        "ManagementBackend",
        &state,
        &serde_json::json!({"backend":backend}),
    )?
    .outcome
        != Outcome::Succeeded
    {
        return Err(MainError::Backup);
    }
    let (mut configuration, ok): (Configuration, _) =
        invoke(backend, &state, "configuration", None)?;
    if !ok {
        return Err(MainError::Backup);
    }
    let mut action_outcome = Outcome::Succeeded;
    if request.action != Action::Inspect {
        let action = match request.action {
            Action::Capture => "run",
            Action::RepairSchedule => "schedule",
            Action::SaveDestinations => {
                let destinations = request.destinations.ok_or(MainError::Input)?;
                configuration.local_destination = destinations.local;
                configuration.external_destination = destinations.external;
                "configure"
            }
            Action::Inspect => return Err(MainError::Input),
        };
        let result = invoke::<Status>(
            backend,
            &state,
            action,
            (action == "configure").then_some(&configuration),
        );
        action_outcome = match result {
            Ok((status, true)) => {
                let _ = status;
                Outcome::Succeeded
            }
            Ok((status, false)) => match request.action {
                Action::Capture if status.local_outcome == Outcome::Succeeded => {
                    status.external_outcome
                }
                Action::Capture => status.local_outcome,
                _ => Outcome::ScheduleFailed,
            },
            Err(_) if request.action == Action::SaveDestinations => Outcome::DestinationUnavailable,
            Err(_) if request.action == Action::RepairSchedule => Outcome::ScheduleFailed,
            Err(_) => Outcome::CaptureFailed,
        };
    }
    let (configuration, ok): (Configuration, _) = invoke(backend, &state, "configuration", None)?;
    if !ok {
        return Err(MainError::Backup);
    }
    let (status, ok): (Status, _) = invoke(backend, &state, "status", None)?;
    if !ok {
        return Err(MainError::Backup);
    }
    let response = ServerBackupManagement {
        version: 1,
        destinations: SetupBackupDestinations {
            local: configuration.local_destination,
            external: configuration.external_destination,
        },
        interval_hours: configuration.interval_hours,
        retention_count: configuration.retention_count,
        status,
        action_outcome,
    };
    serde_json::to_writer(std::io::stdout().lock(), &response).map_err(|_| MainError::Backup)?;
    std::io::stdout()
        .write_all(b"\n")
        .map_err(|_| MainError::Backup)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn management_rejects_account_authority_and_unrelated_configuration() {
        for json in [
            r#"{"version":1,"action":"capture","destinations":null,"owner":true}"#,
            r#"{"version":1,"action":"saveDestinations","destinations":{"local":"D:\\Backup","external":null},"hostRoot":"C:\\Other"}"#,
        ] {
            assert!(serde_json::from_str::<ManageServerBackup>(json).is_err());
        }
        for (version, action, destinations) in [
            (2, Action::Inspect, None),
            (1, Action::SaveDestinations, None),
            (
                1,
                Action::Capture,
                Some(SetupBackupDestinations {
                    local: "D:\\Backup".into(),
                    external: None,
                }),
            ),
        ] {
            assert!(
                validate_request(&ManageServerBackup {
                    version,
                    action,
                    destinations
                })
                .is_err()
            );
        }
    }
}
