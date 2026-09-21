use std::{
    io::{BufRead as _, BufReader},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use tempfile::TempDir;

use eitmad_contracts::{
    identity::{AuthenticatedIdentity, PrincipalId, PrincipalKind},
    transport::{CorrelationId, UnixMillis},
};
use eitmad_engine_runtime::DesktopAuthenticator;
use eitmad_storage::{AuthorityStore, DesktopRole};
use uuid::Uuid;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_eitmad-engine-cli")
}

fn spawn_supervised(
    runtime_directory: &Path,
    include_supervisor: bool,
) -> (Child, Receiver<Value>) {
    let mut command = Command::new(binary());
    command
        .args(["run", "--mode", "supervised", "--runtime-directory"])
        .arg(runtime_directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if include_supervisor {
        command
            .arg("--supervisor-pid")
            .arg(std::process::id().to_string());
    }
    let mut child = command.spawn().expect("spawn supervised engine");
    let stdout = child.stdout.take().expect("child stdout");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let line = line.expect("read lifecycle line");
            sender
                .send(serde_json::from_str(&line).expect("valid lifecycle JSON"))
                .expect("send lifecycle line");
        }
    });
    (child, receiver)
}

fn next_state(receiver: &Receiver<Value>) -> String {
    receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("lifecycle output before timeout")["state"]
        .as_str()
        .expect("state string")
        .to_owned()
}

fn wait_for_exit(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            return status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill timed-out child");
            panic!("engine process did not exit before timeout");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn supervised_engine_reports_readiness_and_clean_shutdown() {
    let directory = TempDir::new().expect("temp directory");
    let (mut child, output) = spawn_supervised(directory.path(), true);

    assert_eq!(next_state(&output), "starting");
    assert_eq!(next_state(&output), "ready");
    drop(child.stdin.take());
    assert_eq!(next_state(&output), "stopping");
    assert_eq!(next_state(&output), "stopped");

    assert!(wait_for_exit(&mut child).success());
}

#[test]
fn missing_supervisor_identity_fails_structurally() {
    let directory = TempDir::new().expect("temp directory");
    let (mut child, output) = spawn_supervised(directory.path(), false);

    assert_eq!(next_state(&output), "starting");
    assert_eq!(next_state(&output), "failed");
    assert_eq!(wait_for_exit(&mut child).code(), Some(1));
}

#[test]
fn diagnostic_mode_emits_one_healthy_report() {
    let directory = TempDir::new().expect("temp directory");
    let output = Command::new(binary())
        .args(["diagnose", "--runtime-directory"])
        .arg(directory.path())
        .output()
        .expect("run diagnostics");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let lines = String::from_utf8(output.stdout)
        .expect("UTF-8 diagnostics")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    let report: Value = serde_json::from_str(&lines[0]).expect("diagnostic JSON");
    assert_eq!(report["status"], "healthy");
    assert_eq!(report["readyToStart"], true);
}

#[test]
fn authority_lock_blocks_concurrent_process_and_allows_replacement() {
    let directory = TempDir::new().expect("temp directory");
    let (mut first, first_output) = spawn_supervised(directory.path(), true);
    assert_eq!(next_state(&first_output), "starting");
    assert_eq!(next_state(&first_output), "ready");

    let (mut duplicate, duplicate_output) = spawn_supervised(directory.path(), true);
    assert_eq!(next_state(&duplicate_output), "starting");
    let duplicate_failure = duplicate_output
        .recv_timeout(Duration::from_secs(10))
        .expect("duplicate failure");
    assert_eq!(duplicate_failure["state"], "failed");
    assert_eq!(
        duplicate_failure["error"]["code"],
        "eitmad.error.engine-already-running.v1"
    );
    assert_eq!(wait_for_exit(&mut duplicate).code(), Some(1));

    drop(first.stdin.take());
    assert_eq!(next_state(&first_output), "stopping");
    assert_eq!(next_state(&first_output), "stopped");
    assert!(wait_for_exit(&mut first).success());

    let (mut replacement, replacement_output) = spawn_supervised(directory.path(), true);
    assert_eq!(next_state(&replacement_output), "starting");
    assert_eq!(next_state(&replacement_output), "ready");
    drop(replacement.stdin.take());
    assert_eq!(next_state(&replacement_output), "stopping");
    assert_eq!(next_state(&replacement_output), "stopped");
    assert!(wait_for_exit(&mut replacement).success());
}

#[test]
fn invalid_cli_usage_exits_two() {
    let output = Command::new(binary())
        .arg("unknown")
        .output()
        .expect("run invalid command");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn debug_seed_provisions_distinct_accounts_and_is_idempotent() {
    let directory = TempDir::new().expect("temp directory");
    for _ in 0..2 {
        let output = Command::new(binary())
            .args(["seed-development-accounts", "--runtime-directory"])
            .arg(directory.path())
            .output()
            .expect("seed development accounts");
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }

    let store = AuthorityStore::open(directory.path()).expect("open authority store");
    let owner = store
        .local_authorization_context(eitmad_contracts::transport::UnixMillis(1))
        .expect("local authority");
    let manager = store
        .desktop_account(owner.tenant_id, "test.manager")
        .expect("manager lookup")
        .expect("manager account");
    let receptionist = store
        .desktop_account(owner.tenant_id, "test.receptionist")
        .expect("receptionist lookup")
        .expect("receptionist account");

    assert_eq!(manager.role, DesktopRole::Manager);
    assert_eq!(receptionist.role, DesktopRole::Receptionist);
    assert_ne!(manager.account_id, receptionist.account_id);
    assert_ne!(manager.user_id, receptionist.user_id);
    assert!(!manager.password_hash.contains("Eitmad-Manager-2026!"));
    assert!(
        !receptionist
            .password_hash
            .contains("Eitmad-Reception-2026!")
    );

    let device_id = owner.identity.device_id.expect("installation device");
    let mut process = owner;
    process.identity = AuthenticatedIdentity {
        principal_id: PrincipalId::new(device_id.value()),
        principal_kind: PrincipalKind::Device,
        device_id: Some(device_id),
        service_id: None,
    };
    let authenticator = DesktopAuthenticator::new(store);
    let manager_session = authenticator
        .sign_in(
            &process,
            "test.manager",
            "Eitmad-Manager-2026!",
            CorrelationId::new(Uuid::new_v4()),
            UnixMillis(2_000_000_000_000),
        )
        .expect("manager sign-in");
    let receptionist_session = authenticator
        .sign_in(
            &process,
            "test.receptionist",
            "Eitmad-Reception-2026!",
            CorrelationId::new(Uuid::new_v4()),
            UnixMillis(2_000_000_000_001),
        )
        .expect("receptionist sign-in");
    assert_ne!(
        manager_session
            .authorization
            .expect("manager authorization")
            .identity
            .principal_id,
        receptionist_session
            .authorization
            .expect("receptionist authorization")
            .identity
            .principal_id
    );
}
