//! Physical `PostgreSQL` recovery onto a new cluster, including SCRAM role verifiers.
use super::*;
use std::{ffi::OsStr, process::Command};

fn run(bin: &Path, tool: &str, arguments: &[&OsStr], password: &str) {
    let mut command = Command::new(bin.join(format!("{tool}.exe")));
    for (name, _) in env::vars_os().filter(|(name, _)| name.to_string_lossy().starts_with("PG")) {
        command.env_remove(name);
    }
    let status = command
        .args(arguments)
        .env("PGPASSWORD", password)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "{tool} failed; diagnostics withheld");
}

pub(super) fn recover(root: &Path, port: &str) -> String {
    let original = url::Url::parse(&env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap()).unwrap();
    assert_eq!(original.host_str(), Some("127.0.0.1"));
    let bin = required_path("EITMAD_RECOVERY_PG_BIN");
    let password = env::var("EITMAD_RECOVERY_ADMIN_PASSWORD").unwrap();
    let backup_user = env::var("EITMAD_RECOVERY_BACKUP_USER").unwrap();
    let backup_password = env::var("EITMAD_RECOVERY_BACKUP_PASSWORD").unwrap();
    let backup = root.join("cluster-backup");
    let cluster = root.join("replacement-cluster");
    let source_port = original.port().unwrap().to_string();
    run(
        &bin,
        "pg_basebackup",
        &[
            "--host=127.0.0.1".as_ref(),
            "--port".as_ref(),
            source_port.as_ref(),
            "--username".as_ref(),
            backup_user.as_ref(),
            "--no-password".as_ref(),
            "--format=plain".as_ref(),
            "--wal-method=stream".as_ref(),
            "--checkpoint=fast".as_ref(),
            "--manifest-checksums=SHA256".as_ref(),
            "--pgdata".as_ref(),
            backup.as_os_str(),
        ],
        &backup_password,
    );
    run(&bin, "pg_verifybackup", &[backup.as_os_str()], &password);
    copy_tree(&backup, &cluster);
    let options =
        format!("-h 127.0.0.1 -p {port} -c log_statement=none -c log_min_error_statement=panic");
    run(
        &bin,
        "pg_ctl",
        &[
            "-D".as_ref(),
            cluster.as_os_str(),
            "-l".as_ref(),
            root.join("replacement-postgres.log").as_os_str(),
            "-o".as_ref(),
            options.as_ref(),
            "-w".as_ref(),
            "start".as_ref(),
        ],
        &password,
    );
    let mut restored = original.clone();
    restored.set_port(Some(port.parse().unwrap())).unwrap();
    let mut admin_source = original;
    admin_source.set_username("postgres").unwrap();
    admin_source.set_password(Some(&password)).unwrap();
    let mut admin_restored = admin_source.clone();
    admin_restored
        .set_port(Some(port.parse().unwrap()))
        .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        assert_eq!(
            postgres_rows(admin_source.as_str()).await,
            postgres_rows(admin_restored.as_str()).await
        );
        // The application password must work in a new cluster with restored roles and RLS.
        let pool = sqlx::PgPool::connect(restored.as_str()).await.unwrap();
        let restricted: bool = sqlx::query_scalar(
            "SELECT NOT rolsuper AND NOT rolbypassrls AND NOT rolcreatedb AND NOT rolcreaterole
             FROM pg_roles WHERE rolname=current_user",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(restricted);
        pool.close().await;
    });
    restored.to_string()
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        assert!(!kind.is_symlink());
        let destination = target.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).unwrap();
        }
    }
}
