"""Exercise a new physical PostgreSQL cluster and the two-client recovery scenario.

Only synthetic data is created. This is a data-plane proof, not Windows BMR acceptance.
"""
import argparse
from importlib.util import module_from_spec, spec_from_file_location
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import time

spec = spec_from_file_location("runtime_check", Path(__file__).resolve().parents[1] / "deploy/windows/test-runtime.py")
runtime_check = module_from_spec(spec)
spec.loader.exec_module(runtime_check)
run, free_port = runtime_check.run, runtime_check.free_port


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--postgres-bin", required=True, type=Path)
    parser.add_argument("--root", required=True, type=Path)
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("Windows is required")
    root = args.root.resolve()
    if shutil.disk_usage(root.parent).free < 3 * 1024**3:
        parser.error("At least 3 GiB of free space is required")
    root.mkdir(parents=True, exist_ok=False)
    run(["icacls.exe", str(root), "/inheritance:r", "/grant:r", f"{os.getlogin()}:(OI)(CI)F"])
    pg = args.postgres_bin.resolve()
    source = root / "source-cluster"
    password, application, backup_password = (secrets.token_hex(32) for _ in range(3))
    port, replacement_port = free_port(), free_port()
    while replacement_port == port:
        replacement_port = free_port()
    pwfile = root / "initdb-password"
    pwfile.write_text(password, encoding="utf-8")
    try:
        run([pg / "initdb.exe", "-D", source, "-U", "postgres", "-A", "scram-sha-256",
             "--encoding=UTF8", "--locale=C", f"--pwfile={pwfile}"])
    finally:
        pwfile.unlink()
    started = time.monotonic()
    try:
        run([pg / "pg_ctl.exe", "-D", source, "-l", root / "source-postgres.log", "-o",
             f"-h 127.0.0.1 -p {port} -c log_statement=none -c log_min_error_statement=panic", "-w", "start"])
        sql = root / "role.sql"
        sql.write_text(
            f"CREATE ROLE eitmad_test LOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE PASSWORD '{application}';\n"
            f"CREATE ROLE eitmad_backup LOGIN REPLICATION NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE PASSWORD '{backup_password}';\n",
            encoding="utf-8")
        db_env = dict(os.environ, PGPASSWORD=password)
        try:
            run([pg / "psql.exe", "-X", "-h", "127.0.0.1", "-p", str(port), "-U", "postgres",
                 "-d", "postgres", "-v", "ON_ERROR_STOP=1", "-f", sql], db_env)
        finally:
            sql.unlink()
        run([pg / "createdb.exe", "-h", "127.0.0.1", "-p", str(port), "-U", "postgres", "-O", "eitmad_test", "eitmad_recovery"], db_env)
        workspace = Path(__file__).resolve().parents[1]
        certificate = root / "certificates"
        run(["dotnet", "run", "--project", workspace / "tools/dev-certificate/Eitmad.DevCertificate.csproj", "--configuration", "Release", "--", certificate])
        environment = dict(os.environ,
            EITMAD_DIRECT_TEST_DATABASE_URL=f"postgresql://eitmad_test:{application}@127.0.0.1:{port}/eitmad_recovery",
            EITMAD_DIRECT_TEST_CERTIFICATE=str(certificate / "server-cert.pem"),
            EITMAD_DIRECT_TEST_PRIVATE_KEY=str(certificate / "server-key.pem"),
            EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE=str(certificate / "trusted-ca.pem"),
            EITMAD_RECOVERY_ARTIFACT_DIRECTORY=str(root), EITMAD_RECOVERY_PG_BIN=str(pg),
            EITMAD_RECOVERY_ADMIN_PASSWORD=password, EITMAD_RECOVERY_REPLACEMENT_PORT=str(replacement_port),
            EITMAD_RECOVERY_BACKUP_USER="eitmad_backup", EITMAD_RECOVERY_BACKUP_PASSWORD=backup_password,
            CARGO_INCREMENTAL="0")
        # Select the compiler for Cargo and subprocesses, retaining the stable shared cache.
        environment["CARGO_TARGET_DIR"] = str(workspace / "target/toolchain-1.85.1")
        run(["rustup", "run", "1.85.1", "cargo", "test", "--locked", "-p", "eitmad-server-connection",
             "--test", "direct_route", "manual_two_client_recovery", "--", "--ignored", "--nocapture"], environment, timeout=1200)
        result = json.loads((root / "result.json").read_text(encoding="utf-8"))
        result["drillSecondsIncludingBuild"] = round(time.monotonic() - started, 3)
        result["windowsCredentialRecoveryProved"] = False
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(result, indent=2))
    finally:
        for cluster in (root / "replacement-cluster", source):
            if (cluster / "postmaster.pid").exists():
                run([pg / "pg_ctl.exe", "-D", cluster, "-m", "fast", "-w", "stop"])


if __name__ == "__main__":
    main()
