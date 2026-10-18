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
fn connected_recovery_cannot_use_a_saved_verifier_without_server_settings() {
    let directory = TempDir::new().unwrap();
    seed(directory.path());
    std::fs::write(directory.path().join("desktop-device.json"), b"null").unwrap();
    let backup = directory.path().join("denied.sqlite3");
    let denied = recovery(directory.path(), "backup", &backup, MANAGER);
    assert_eq!(denied.status.code(), Some(1));
    assert!(!backup.exists());
    let store = AuthorityStore::open_existing(directory.path()).unwrap();
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    let open_sessions: i64 = connection.query_row(
        "SELECT COUNT(*) FROM identity_sessions WHERE closed_at IS NULL AND principal_kind='\"user\"' AND session_id IN (SELECT session_id FROM mutation_audit WHERE operation='eitmad.desktop.session.sign-in.v1')",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(open_sessions, 0);
}

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
    assert_restore_session_evidence(directory.path(), &artifacts[0].path);
    let live = directory.path().join(eitmad_storage::DATABASE_FILE_NAME);
    assert!(
        !recovery(directory.path(), "restore", &live, MANAGER)
            .status
            .success()
    );
    assert_eq!(std::fs::read(&backup).unwrap(), original);
    assert_recovered_engine_ready(directory.path());
}

fn assert_restore_session_evidence(directory: &Path, previous: &Path) {
    let live =
        rusqlite::Connection::open(directory.join(eitmad_storage::DATABASE_FILE_NAME)).unwrap();
    let session: String = live.query_row(
        "SELECT session_id FROM mutation_audit WHERE operation = 'eitmad.storage.restore.v1' AND outcome = '\"succeeded\"'",
        [], |row| row.get(0),
    ).unwrap();
    let attribution: (String, String, i64) = live.query_row(
        "SELECT s.user_id, s.account_id, s.closed_at FROM identity_sessions s
         JOIN identity_accounts a ON a.account_id=s.account_id AND a.user_id=s.user_id AND a.tenant_id=s.tenant_id
         JOIN identity_users u ON u.user_id=s.user_id WHERE s.session_id=?1",
        [&session], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    let old = rusqlite::Connection::open(previous).unwrap();
    let original: (String, String, Option<i64>) = old
        .query_row(
            "SELECT user_id, account_id, closed_at FROM identity_sessions WHERE session_id=?1",
            [&session],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!((&attribution.0, &attribution.1), (&original.0, &original.1));
    assert_eq!(original.2, None);
    let sign_in = |connection: &rusqlite::Connection| -> Vec<rusqlite::types::Value> {
        connection.query_row(
            "SELECT * FROM mutation_audit WHERE session_id=?1 AND operation='eitmad.desktop.session.sign-in.v1'",
            [&session], |row| (0..row.as_ref().column_count()).map(|column| row.get(column)).collect(),
        ).unwrap()
    };
    assert_eq!(sign_in(&live), sign_in(&old));
}

#[test]
fn manual_recovery_preserves_new_operator_evidence_without_new_credentials_or_permissions() {
    use argon2::{Argon2, PasswordHasher as _, password_hash::SaltString};
    use eitmad_contracts::{
        identity::{AccountId, UserId},
        transport::UnixMillis,
    };
    use eitmad_storage::{DesktopAccount, DesktopRole};
    use uuid::Uuid;

    let directory = TempDir::new().unwrap();
    seed(directory.path());
    let backup = directory.path().join("before-operator.sqlite3");
    assert!(
        recovery(directory.path(), "backup", &backup, MANAGER)
            .status
            .success()
    );
    let store = AuthorityStore::open(directory.path()).unwrap();
    let owner = store.local_authorization_context(UnixMillis(1)).unwrap();
    let salt = SaltString::encode_b64(b"synthetic-recovery").unwrap();
    let account = DesktopAccount {
        account_id: AccountId::new(Uuid::new_v4()),
        user_id: UserId::new(Uuid::new_v4()),
        tenant_id: owner.tenant_id,
        organization_id: store.local_organization_id().unwrap(),
        password_hash: Argon2::default()
            .hash_password(b"synthetic recovery password", &salt)
            .unwrap()
            .to_string(),
        role: DesktopRole::Manager,
    };
    store
        .provision_desktop_account(&owner, &account, "operator", UnixMillis(2))
        .unwrap();
    let restored = recovery(
        directory.path(),
        "restore",
        &backup,
        br#"{"username":"operator","password":"synthetic recovery password"}"#,
    );
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let artifacts = AuthorityStore::recovery_artifacts(directory.path()).unwrap();
    assert_restore_session_evidence(directory.path(), &artifacts[0].path);
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    let credentials: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM desktop_accounts WHERE account_id=?1",
            [account.account_id.value().to_string()],
            |row| row.get(0),
        )
        .unwrap();
    let permissions: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM scope_relationships WHERE principal_id=?1",
            [account.user_id.value().to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!((credentials, permissions), (0, 0));
    assert!(
        store
            .desktop_account(owner.tenant_id, "operator")
            .unwrap()
            .is_none()
    );
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
