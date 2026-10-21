//! Privileged CLI orchestration. Product policy stays in `eitmad_server::backup`.
mod management;

use crate::MainError;
use eitmad_contracts::backup::{
    ServerBackupConfiguration as Configuration, ServerBackupOutcome as Outcome,
    ServerBackupStatus as Status,
};
use eitmad_server::backup;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs::{File, OpenOptions},
    io::Read as _,
    os::windows::{fs::OpenOptionsExt as _, process::CommandExt as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) fn dispatch(arguments: &[String]) -> Option<Result<(), MainError>> {
    match arguments.get(1).map(String::as_str)? {
        "windows-backup-management" if arguments.len() == 2 => {
            Some(management::elevate_management())
        }
        "windows-backup-management-worker" if arguments.len() == 2 => Some(management::manage()),
        "windows-backup" if arguments.len() == 4 => Some(execute(&arguments[2], &arguments[3])),
        "windows-backup" | "windows-backup-management" | "windows-backup-management-worker" => {
            Some(Err(MainError::Input))
        }
        _ => None,
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Saved {
    #[serde(default)]
    backend: Option<PathBuf>,
    configuration: Configuration,
    status: Status,
    generation: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Observation {
    outcome: Outcome,
    #[serde(default)]
    generations: Vec<String>,
}

/// Reuses the privileged CLI and destination checks without emitting backup state as installer progress.
pub(super) fn configure_setup(
    state: &Path,
    configuration: &Configuration,
) -> Result<(), MainError> {
    let mut configuration = configuration.clone();
    if state
        .join("state.json")
        .try_exists()
        .map_err(|_| MainError::Backup)?
    {
        let guard = native("Guard", state, &json!({}))?;
        if guard.outcome != Outcome::Succeeded {
            return Err(MainError::Backup);
        }
        let saved: Saved = serde_json::from_reader(
            File::open(state.join("state.json"))
                .map_err(|_| MainError::Backup)?
                .take(65_537),
        )
        .map_err(|_| MainError::Backup)?;
        if saved.generation.is_none() && saved.status.local_outcome == Outcome::Succeeded {
            return Err(MainError::Backup);
        }
        configuration = retained_setup_configuration(configuration, &saved.configuration)?;
    }
    let configuration = &configuration;
    backup::validate(configuration).map_err(|_| MainError::Input)?;
    let observation = native(
        "SetupDestination",
        state,
        &json!({"configuration":configuration}),
    )?;
    if observation.outcome != Outcome::Succeeded {
        return Err(MainError::Backup);
    }
    let executable = std::env::current_exe().map_err(|_| MainError::Backup)?;
    let mut capture_action = "run";
    for step in ["status", "configure", "schedule", "capture"] {
        let action = if step == "capture" {
            capture_action
        } else {
            step
        };
        if action == "status"
            && !state
                .join("state.json")
                .try_exists()
                .map_err(|_| MainError::Backup)?
        {
            continue;
        }
        let mut child = Command::new(&executable)
            .args(["windows-backup", action])
            .arg(state)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|_| MainError::Backup)?;
        if action == "configure" {
            serde_json::to_writer(child.stdin.take().ok_or(MainError::Backup)?, &configuration)
                .map_err(|_| MainError::Backup)?;
        } else {
            drop(child.stdin.take());
        }
        let output = child.wait_with_output().map_err(|_| MainError::Backup)?;
        if !output.status.success() || output.stdout.len() > 65536 {
            return Err(MainError::Backup);
        }
        let status: Status =
            serde_json::from_slice(&output.stdout).map_err(|_| MainError::Backup)?;
        if action == "status" {
            capture_action = setup_capture_action(&status);
        }
        if action == "status"
            && status.schedule_outcome == Outcome::Succeeded
            && status.local_outcome == Outcome::Succeeded
            && (configuration.external_destination.is_none()
                || status.external_outcome == Outcome::Succeeded)
        {
            return Ok(());
        }
        if action == "schedule" && status.schedule_outcome != Outcome::Succeeded
            || matches!(action, "run" | "tick")
                && (status.local_outcome != Outcome::Succeeded
                    || configuration.external_destination.is_some()
                        && status.external_outcome != Outcome::Succeeded)
        {
            return Err(MainError::Backup);
        }
    }
    Ok(())
}

fn setup_capture_action(status: &Status) -> &'static str {
    if status.local_outcome == Outcome::Succeeded && status.last_local_success_at.is_some() {
        // Tick retries the external copy using the retained cut and captures a
        // new local generation only when it is due. Run would always recapture.
        "tick"
    } else {
        "run"
    }
}

fn retained_setup_configuration(
    mut requested: Configuration,
    saved: &Configuration,
) -> Result<Configuration, MainError> {
    requested.deployment_id = saved.deployment_id;
    if serde_json::to_value(&requested).map_err(|_| MainError::Backup)?
        != serde_json::to_value(saved).map_err(|_| MainError::Backup)?
    {
        return Err(MainError::Backup);
    }
    Ok(requested)
}

fn now() -> Result<u64, MainError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| MainError::Backup)
}

fn native(
    action: &str,
    state: &Path,
    payload: &serde_json::Value,
) -> Result<Observation, MainError> {
    let executable = std::env::current_exe().map_err(|_| MainError::Backup)?;
    let script = executable
        .parent()
        .ok_or(MainError::Backup)?
        .join("deploy/windows/local-backup.ps1");
    let system = std::env::var_os("SystemRoot").ok_or(MainError::Backup)?;
    let mut child =
        Command::new(Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"))
            .args(["-NoProfile", "-NonInteractive", "-File"])
            .arg(script)
            .args(["-Action", action, "-StateDirectory"])
            .arg(state)
            .arg("-Backend")
            .arg(executable)
            // A caller launched from PowerShell 7 can otherwise load incompatible PS7 modules in 5.1.
            .env_remove("PSModulePath")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|_| MainError::Backup)?;
    let result = (|| {
        serde_json::to_writer(child.stdin.take().ok_or(MainError::Backup)?, payload)
            .map_err(|_| MainError::Backup)?;
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or(MainError::Backup)?
            .take(65_537)
            .read_to_end(&mut bytes)
            .map_err(|_| MainError::Backup)?;
        if bytes.len() > 65_536 {
            return Err(MainError::Backup);
        }
        if !child.wait().map_err(|_| MainError::Backup)?.success() {
            return Err(MainError::Backup);
        }
        serde_json::from_slice(&bytes).map_err(|_| MainError::Backup)
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn publish(status: &Status) -> Result<(), MainError> {
    serde_json::to_writer(std::io::stdout().lock(), status).map_err(|_| MainError::Backup)
}

fn save(state: &Path, saved: &Saved, event: &str) -> Result<(), MainError> {
    // Same-directory atomic publication; never truncate the previous valid state.
    let next = state.join(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&next)
        .map_err(|_| MainError::Backup)?;
    serde_json::to_writer(&mut file, saved).map_err(|_| MainError::Backup)?;
    file.sync_all().map_err(|_| MainError::Backup)?;
    drop(file);
    let result = native(
        "Publish",
        state,
        &json!({"temporary":next.file_name().and_then(|name| name.to_str()).ok_or(MainError::Backup)?, "event":event, "at":now()?, "status":saved.status}),
    )?;
    if result.outcome != Outcome::Succeeded {
        return Err(MainError::Backup);
    }
    Ok(())
}

pub(super) fn execute(action: &str, directory: &str) -> Result<(), MainError> {
    if ![
        "configure",
        "configuration",
        "status",
        "run",
        "tick",
        "schedule",
    ]
    .contains(&action)
    {
        return Err(MainError::Input);
    }
    let state = PathBuf::from(directory);
    // Guard checks administrator identity, plain paths, and protected ancestors before I/O.
    let guard = native(
        if action == "configure" {
            "Initialize"
        } else {
            "Guard"
        },
        &state,
        &json!({}),
    )?;
    if guard.outcome != Outcome::Succeeded {
        return Err(MainError::Backup);
    }
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(state.join("backup.lock"));
    let lease = match lease {
        Ok(file) => Some(file),
        Err(error)
            if matches!(action, "status" | "configuration") && error.raw_os_error() == Some(32) =>
        {
            None
        }
        Err(_) => return Err(MainError::Backup),
    };
    if action == "configure" {
        return configure(&state);
    }
    let file = File::open(state.join("state.json")).map_err(|_| MainError::Backup)?;
    let mut saved: Saved =
        serde_json::from_reader(file.take(65_537)).map_err(|_| MainError::Backup)?;
    backup::validate(&saved.configuration).map_err(|_| MainError::Input)?;
    if saved.status.deployment_id != saved.configuration.deployment_id || saved.status.version != 1
    {
        return Err(MainError::Backup);
    }
    if lease.is_some() && saved.status.local_outcome == Outcome::Running {
        saved.status.local_outcome = Outcome::Interrupted;
    }
    if lease.is_some() && saved.status.external_outcome == Outcome::Running {
        saved.status.external_outcome = Outcome::Interrupted;
    }
    backup::refresh(&saved.configuration, &mut saved.status, now()?);
    match action {
        "configuration" => serde_json::to_writer(std::io::stdout().lock(), &saved.configuration)
            .map_err(|_| MainError::Backup),
        "status" => status(&state, &mut saved),
        "schedule" => {
            let observation = native(
                "Schedule",
                &state,
                &json!({"configuration":saved.configuration}),
            )?;
            saved.status.schedule_outcome = observation.outcome;
            save(&state, &saved, "schedule")?;
            publish(&saved.status)?;
            if saved.status.schedule_outcome == Outcome::Succeeded {
                Ok(())
            } else {
                Err(MainError::Backup)
            }
        }
        _ => run(&state, saved, action == "run"),
    }
}

fn configure(state: &Path) -> Result<(), MainError> {
    let mut input = Vec::new();
    std::io::stdin()
        .take(65_537)
        .read_to_end(&mut input)
        .map_err(|_| MainError::Input)?;
    if input.len() > 65_536 {
        return Err(MainError::Input);
    }
    let configuration: Configuration =
        serde_json::from_slice(&input).map_err(|_| MainError::Input)?;
    backup::validate(&configuration).map_err(|_| MainError::Input)?;
    // State must not be captured, pruned, or placed under a source/destination.
    let state_text = state.to_string_lossy().to_ascii_lowercase();
    for path in [
        &configuration.host_root,
        &configuration.recovery_directory,
        &configuration.local_destination,
    ]
    .into_iter()
    .chain(configuration.external_destination.iter())
    {
        let path = path.to_ascii_lowercase();
        if state_text == path
            || state_text.starts_with(&format!("{path}\\"))
            || path.starts_with(&format!("{state_text}\\"))
        {
            return Err(MainError::Input);
        }
    }
    let result = native("Validate", state, &json!({"configuration":configuration}))?;
    if result.outcome != Outcome::Succeeded {
        return Err(MainError::Backup);
    }
    let mut saved = Saved {
        backend: Some(std::env::current_exe().map_err(|_| MainError::Backup)?),
        status: backup::initial(&configuration, now()?),
        configuration,
        generation: None,
    };
    if state.join("state.json").exists() {
        let previous: Saved = serde_json::from_reader(
            File::open(state.join("state.json"))
                .map_err(|_| MainError::Backup)?
                .take(65_537),
        )
        .map_err(|_| MainError::Backup)?;
        saved.backend = previous.backend.or(saved.backend);
        let old = &previous.configuration;
        if old.deployment_id != saved.configuration.deployment_id
            || old.host_root != saved.configuration.host_root
            || old.recovery_directory != saved.configuration.recovery_directory
            || old.service_name != saved.configuration.service_name
        {
            return Err(MainError::Input);
        }
        saved.status = previous.status;
        saved.generation = previous.generation;
        if old.local_destination != saved.configuration.local_destination {
            saved.generation = None;
            saved.status.last_local_success_at = None;
            saved.status.local_outcome = Outcome::NeverRun;
        }
        if old.external_destination != saved.configuration.external_destination {
            saved.status.last_external_success_at = None;
            saved.status.external_outcome = Outcome::NeverRun;
        }
        saved.status.external_configured = saved.configuration.external_destination.is_some();
        backup::refresh(&saved.configuration, &mut saved.status, now()?);
    }
    save(state, &saved, "configured")?;
    publish(&saved.status)
}

fn prune(state: &Path, config: &Configuration, external: bool) -> Result<Outcome, MainError> {
    let list = native(
        "List",
        state,
        &json!({"configuration":config, "external":external}),
    )?;
    if list.outcome != Outcome::Succeeded {
        return Ok(Outcome::RetentionFailed);
    }
    for id in backup::expired(list.generations, config.retention_count) {
        let result = native(
            "Delete",
            state,
            &json!({"configuration":config, "external":external, "id":id}),
        )?;
        if result.outcome != Outcome::Succeeded {
            return Ok(Outcome::RetentionFailed);
        }
    }
    Ok(Outcome::Succeeded)
}

fn run(state: &Path, mut saved: Saved, force: bool) -> Result<(), MainError> {
    if force || saved.status.overdue {
        let cut = now()?;
        let id = format!("{cut:020}-{}", uuid::Uuid::new_v4().simple());
        saved.status.last_attempt_at = Some(cut);
        saved.status.local_outcome = Outcome::Running;
        save(state, &saved, "captureStarted")?;
        let result = native(
            "Capture",
            state,
            &json!({"configuration":saved.configuration, "id":id}),
        )?;
        saved.status.local_outcome = result.outcome;
        if result.outcome == Outcome::Succeeded {
            saved.status.last_local_success_at = Some(cut);
            saved.generation = Some(id);
            saved.status.local_outcome = prune(state, &saved.configuration, false)?;
        }
        save(state, &saved, "captureFinished")?;
    }
    // Retry an unavailable removable destination on the next hourly tick, using the same cut.
    if saved.configuration.external_destination.is_some()
        && saved.generation.is_some()
        && (saved.status.last_external_success_at != saved.status.last_local_success_at
            || saved.status.external_outcome != Outcome::Succeeded)
    {
        saved.status.external_outcome = Outcome::Running;
        save(state, &saved, "externalStarted")?;
        let result = native(
            "Copy",
            state,
            &json!({"configuration":saved.configuration, "id":saved.generation}),
        )?;
        saved.status.external_outcome = result.outcome;
        if result.outcome == Outcome::Succeeded {
            saved.status.last_external_success_at = saved.status.last_local_success_at;
            saved.status.external_outcome = prune(state, &saved.configuration, true)?;
        }
        save(state, &saved, "externalFinished")?;
    }
    backup::refresh(&saved.configuration, &mut saved.status, now()?);
    save(state, &saved, "runFinished")?;
    publish(&saved.status)?;
    if matches!(saved.status.local_outcome, Outcome::Succeeded)
        && (!saved.status.external_configured
            || saved.status.external_outcome == Outcome::Succeeded)
    {
        Ok(())
    } else {
        Err(MainError::Backup)
    }
}

fn status(state: &Path, saved: &mut Saved) -> Result<(), MainError> {
    let schedule = native(
        "ReadSchedule",
        state,
        &json!({"configuration":saved.configuration}),
    )?;
    saved.status.schedule_outcome = schedule.outcome;
    if saved.generation.is_some() {
        let local = native(
            "Probe",
            state,
            &json!({"configuration":saved.configuration, "id":saved.generation, "external":false}),
        )?;
        if local.outcome != Outcome::Succeeded {
            saved.status.local_outcome = local.outcome;
        }
    }
    if saved.status.disk_loss_protected {
        let observed = native(
            "Probe",
            state,
            &json!({"configuration":saved.configuration, "id":saved.generation, "external":true}),
        )?;
        if observed.outcome != Outcome::Succeeded {
            saved.status.external_outcome = observed.outcome;
            saved.status.disk_loss_protected = false;
        }
    }
    publish(&saved.status)
}

#[cfg(test)]
mod setup_tests {
    use super::*;

    #[test]
    fn backup_retry_retains_deployment_and_rejects_changed_installation_or_destinations() {
        let saved = Configuration {
            version: 1,
            deployment_id: uuid::Uuid::new_v4(),
            host_root: "C:\\SyntheticHost".into(),
            service_name: "Synthetic".into(),
            recovery_directory: "C:\\SyntheticRecovery".into(),
            local_destination: "D:\\SyntheticBackups".into(),
            external_destination: None,
            interval_hours: 24,
            retention_count: 7,
            reserve_bytes: 1_073_741_824,
        };
        let mut request = saved.clone();
        let mut status = backup::initial(&saved, 100);
        assert_eq!(setup_capture_action(&status), "run");
        status.local_outcome = Outcome::Succeeded;
        status.last_local_success_at = Some(100);
        status.external_outcome = Outcome::DestinationUnavailable;
        assert_eq!(setup_capture_action(&status), "tick");
        status.local_outcome = Outcome::CaptureFailed;
        assert_eq!(setup_capture_action(&status), "run");
        request.deployment_id = uuid::Uuid::new_v4();
        assert_eq!(
            retained_setup_configuration(request.clone(), &saved)
                .unwrap()
                .deployment_id,
            saved.deployment_id
        );
        for change in 0..3 {
            let mut changed = request.clone();
            match change {
                0 => changed.host_root.push('2'),
                1 => changed.service_name.push('2'),
                _ => changed.local_destination.push('2'),
            }
            assert!(retained_setup_configuration(changed, &saved).is_err());
        }
    }
}
