//! Privileged CLI for Rust-owned renewal. All OS observations are bounded and sanitized.
use crate::MainError;
use eitmad_server::tls_renewal::{self, Outcome, Pair, Platform, State};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{Read as _, Write as _},
    os::windows::{fs::OpenOptionsExt as _, process::CommandExt as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

struct Windows {
    root: PathBuf,
}

impl Windows {
    fn native(&self, action: &str, payload: &Value) -> Result<Value, Outcome> {
        let executable = std::env::current_exe().map_err(|_| Outcome::RenewalFailed)?;
        let script = executable
            .parent()
            .ok_or(Outcome::RenewalFailed)?
            .join("deploy/windows/tls-renewal.ps1");
        let system = std::env::var_os("SystemRoot").ok_or(Outcome::RenewalFailed)?;
        let mut child =
            Command::new(Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"))
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "AllSigned",
                    "-File",
                ])
                .arg(script)
                .args(["-Action", action, "-StateDirectory"])
                .arg(&self.root)
                .arg("-Backend")
                .arg(&executable)
                .env_remove("PSModulePath")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(0x0800_0000)
                .spawn()
                .map_err(|_| Outcome::RenewalFailed)?;
        let result = (|| {
            serde_json::to_writer(child.stdin.take().ok_or(Outcome::RenewalFailed)?, payload)
                .map_err(|_| Outcome::RenewalFailed)?;
            let mut bytes = Vec::new();
            child
                .stdout
                .take()
                .ok_or(Outcome::RenewalFailed)?
                .take(65_537)
                .read_to_end(&mut bytes)
                .map_err(|_| Outcome::RenewalFailed)?;
            if bytes.len() > 65_536 {
                return Err(Outcome::RenewalFailed);
            }
            if !child.wait().map_err(|_| Outcome::RenewalFailed)?.success() {
                return Err(Outcome::RenewalFailed);
            }
            let value: Value =
                serde_json::from_slice(&bytes).map_err(|_| Outcome::RenewalFailed)?;
            match value["outcome"].as_str() {
                Some("succeeded") => Ok(value["result"].clone()),
                Some("credentialUnavailable") => Err(Outcome::CredentialUnavailable),
                _ => Err(Outcome::RenewalFailed),
            }
        })();
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        result
    }
}

impl Platform for Windows {
    fn observe(&mut self) -> Result<(Pair, bool), Outcome> {
        let value = self.native("Observe", &json!({}))?;
        Ok((
            serde_json::from_value(value["pair"].clone()).map_err(|_| Outcome::RenewalFailed)?,
            value["running"].as_bool().ok_or(Outcome::RenewalFailed)?,
        ))
    }
    fn stage(&mut self) -> Result<Pair, Outcome> {
        serde_json::from_value(self.native("Stage", &json!({}))?)
            .map_err(|_| Outcome::RenewalFailed)
    }
    fn activate(&mut self, pair: &Pair) -> Result<(), Outcome> {
        self.native("Activate", &json!({"pair": pair})).map(|_| ())
    }
    fn restart(&mut self) -> Result<(), Outcome> {
        self.native("Restart", &json!({})).map(|_| ())
    }
    fn ready(&mut self, pair: &Pair) -> Result<(), Outcome> {
        self.native("Ready", &json!({"pair": pair})).map(|_| ())
    }
    fn save(&mut self, state: &State, event: &str) -> Result<(), Outcome> {
        let directory = self.root.join("renewal");
        let temporary = directory.join(format!("{}.tmp", uuid::Uuid::new_v4()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| Outcome::RenewalFailed)?;
        serde_json::to_writer(&mut file, state).map_err(|_| Outcome::RenewalFailed)?;
        file.sync_all().map_err(|_| Outcome::RenewalFailed)?;
        drop(file);
        // The audit contains no PEM, credentials, raw native errors, or deployment settings.
        let mut audit = OpenOptions::new()
            .append(true)
            .create(true)
            .open(directory.join("audit.jsonl"))
            .map_err(|_| Outcome::RenewalFailed)?;
        serde_json::to_writer(
            &mut audit,
            &json!({"event": event, "at": state.last_attempt,
            "outcome": state.outcome}),
        )
        .map_err(|_| Outcome::RenewalFailed)?;
        audit.write_all(b"\n").map_err(|_| Outcome::RenewalFailed)?;
        audit.sync_all().map_err(|_| Outcome::RenewalFailed)?;
        std::fs::rename(temporary, directory.join("state.json")).map_err(|_| Outcome::RenewalFailed)
    }
}

pub(super) fn configure_setup(root: &str, source: &str, tool: &str) -> Result<(), MainError> {
    execute(
        "configure",
        root,
        Some(&json!({"lanDirectory": source, "certificateExecutable": tool})),
    )
}

pub(super) fn dispatch(arguments: &[String]) -> Option<Result<(), MainError>> {
    if arguments.get(1)?.as_str() != "windows-tls" {
        return None;
    }
    Some(if arguments.len() == 4 {
        execute(&arguments[2], &arguments[3], None)
    } else {
        Err(MainError::Input)
    })
}

fn execute(action: &str, root: &str, configuration: Option<&Value>) -> Result<(), MainError> {
    if !["configure", "tick", "status"].contains(&action) {
        return Err(MainError::Input);
    }
    let mut platform = Windows {
        root: PathBuf::from(root),
    };
    platform
        .native("Guard", &json!({}))
        .map_err(|_| MainError::Tls)?;
    // The host-wide lock also excludes installation, upgrades, stops, and maintenance write barriers.
    let _lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(platform.root.join("host.lock"))
        .map_err(|_| MainError::Tls)?;
    let path = platform.root.join("renewal/state.json");
    let mut state: State = if path.try_exists().map_err(|_| MainError::Tls)? {
        serde_json::from_reader(File::open(path).map_err(|_| MainError::Tls)?.take(65_537))
            .map_err(|_| MainError::Tls)?
    } else {
        State::default()
    };
    if state.version != 1 {
        return Err(MainError::Tls);
    }
    let result = match action {
        "configure" => {
            if state.pending.is_some() {
                return Err(MainError::Tls);
            }
            let mut bytes = Zeroizing::new(Vec::new());
            let mut input;
            let payload = if let Some(configuration) = configuration {
                configuration
            } else {
                std::io::stdin()
                    .take(16_385)
                    .read_to_end(&mut bytes)
                    .map_err(|_| MainError::Input)?;
                if bytes.len() > 16_384 {
                    return Err(MainError::Input);
                }
                input = serde_json::from_slice::<Value>(&bytes).map_err(|_| MainError::Input)?;
                &mut input
            };
            platform.native("Configure", payload).and_then(|_| {
                platform.native(
                    "Schedule",
                    &json!({"intervalSeconds": tls_renewal::CHECK_INTERVAL_SECONDS}),
                )?;
                state.schedule_healthy = true;
                platform.save(&state, "configured")
            })
        }
        "tick" => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| MainError::Tls)?
                .as_secs();
            tls_renewal::tick(&mut platform, &mut state, now)
        }
        "status" => {
            state.schedule_healthy = platform
                .native(
                    "ScheduleStatus",
                    &json!({
                        "intervalSeconds": tls_renewal::CHECK_INTERVAL_SECONDS
                    }),
                )
                .is_ok();
            Ok(())
        }
        _ => return Err(MainError::Input),
    };
    if configuration.is_none() {
        serde_json::to_writer(std::io::stdout().lock(), &state).map_err(|_| MainError::Tls)?;
    }
    result.map_err(|_| MainError::Tls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durable_state_replaces_existing_state_and_keeps_a_sanitized_audit() {
        let root = std::env::temp_dir().join(format!("eitmad-tls-state-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("renewal")).unwrap();
        let mut platform = Windows { root: root.clone() };
        let mut state = State::default();
        platform.save(&state, "configured").unwrap();
        state.last_attempt = Some(1234);
        state.outcome = Outcome::CredentialUnavailable;
        platform.save(&state, "failed").unwrap();
        let retained: State =
            serde_json::from_reader(File::open(root.join("renewal/state.json")).unwrap()).unwrap();
        assert_eq!(retained.outcome, Outcome::CredentialUnavailable);
        assert_eq!(retained.last_attempt, Some(1234));
        let events = std::fs::read_to_string(root.join("renewal/audit.jsonl")).unwrap();
        let events = events
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[1],
            json!({"event":"failed","at":1234,"outcome":"credentialUnavailable"})
        );
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(0)
            .open(root.join("host.lock"))
            .unwrap();
        assert!(
            OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(root.join("host.lock"))
                .is_err()
        );
        drop(lock);
    }
}
