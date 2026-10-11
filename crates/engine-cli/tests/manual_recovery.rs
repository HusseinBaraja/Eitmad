use std::{
    io::{BufRead as _, BufReader, Write as _},
    path::Path,
    process::{Command, Stdio},
};

use eitmad_storage::{AuthorityStore, RecoveryArtifactKind};
use tempfile::TempDir;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_eitmad-engine-cli")
}

fn seed(directory: &Path) {
    assert!(
        Command::new(binary())
            .args(["seed-development-accounts", "--runtime-directory"])
            .arg(directory)
            .status()
            .unwrap()
            .success()
    );
}

fn recovery(
    directory: &Path,
    operation: &str,
    path: &Path,
    credentials: &[u8],
) -> std::process::Output {
    let mut child = Command::new(binary())
        .arg(operation)
        .arg("--runtime-directory")
        .arg(directory)
        .arg(if operation == "backup" {
            "--destination"
        } else {
            "--source"
        })
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(credentials).unwrap();
    child.wait_with_output().unwrap()
}

const MANAGER: &[u8] = br#"{"username":"admin","password":"admin"}"#;
const RECEPTIONIST: &[u8] = br#"{"username":"rec","password":"rec"}"#;

#[test]
fn manual_recovery_authorizes_preserves_and_rejects_unsafe_overwrites() {
    let directory = TempDir::new().unwrap();
    seed(directory.path());
    let backup = directory.path().join("manual.sqlite3");
    let denied = recovery(directory.path(), "backup", &backup, RECEPTIONIST);
    assert_eq!(denied.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(denied.stderr).unwrap().trim(),
        "{\"code\":\"eitmad.error.authorization-denied.v1\"}"
    );
    assert!(!backup.exists());
    let created = recovery(directory.path(), "backup", &backup, MANAGER);
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    AuthorityStore::validate_backup(&backup).unwrap();
    let original = std::fs::read(&backup).unwrap();
    assert!(
        !recovery(directory.path(), "backup", &backup, MANAGER)
            .status
            .success()
    );
    assert_eq!(std::fs::read(&backup).unwrap(), original);
    assert!(
        !recovery(directory.path(), "restore", &backup, RECEPTIONIST)
            .status
            .success()
    );
    let foreign = TempDir::new().unwrap();
    seed(foreign.path());
    let foreign_backup = foreign.path().join("backup.sqlite3");
    assert!(
        recovery(foreign.path(), "backup", &foreign_backup, MANAGER)
            .status
            .success()
    );
    assert!(
        !recovery(directory.path(), "restore", &foreign_backup, MANAGER)
            .status
            .success()
    );
    let restored = recovery(directory.path(), "restore", &backup, MANAGER);
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let artifacts = AuthorityStore::recovery_artifacts(directory.path()).unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].kind, RecoveryArtifactKind::PreRestore);
    AuthorityStore::validate_backup(&artifacts[0].path).unwrap();
    AuthorityStore::check_compatible(directory.path()).unwrap();
    let live = directory.path().join(eitmad_storage::DATABASE_FILE_NAME);
    assert!(
        !recovery(directory.path(), "restore", &live, MANAGER)
            .status
            .success()
    );
    assert_eq!(std::fs::read(&backup).unwrap(), original);
    assert_recovered_engine_ready(directory.path());
}

fn assert_recovered_engine_ready(directory: &Path) {
    let diagnostic = Command::new(binary())
        .args(["diagnose", "--runtime-directory"])
        .arg(directory)
        .output()
        .unwrap();
    assert!(diagnostic.status.success());
    let mut child = Command::new(binary())
        .args(["run", "--mode", "supervised", "--supervisor-pid"])
        .arg(std::process::id().to_string())
        .arg("--runtime-directory")
        .arg(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    for expected in ["starting", "ready"] {
        let state: serde_json::Value =
            serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
        assert_eq!(state["state"], expected);
    }
    drop(child.stdin.take());
    for expected in ["stopping", "stopped"] {
        let state: serde_json::Value =
            serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
        assert_eq!(state["state"], expected);
    }
    assert!(child.wait_with_output().unwrap().status.success());
}
