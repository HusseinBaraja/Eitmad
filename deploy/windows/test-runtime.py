"""Run isolated PostgreSQL/HTTPS failure checks without installing Windows services."""
import argparse
import base64
import json
import os
from pathlib import Path
import secrets
import socket
import ssl
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid


def run(command, env=None, timeout=90):
    # pg_ctl starts children that inherit handles. A pipe would wait for those children at EOF.
    with tempfile.TemporaryFile() as output:
        result = subprocess.run(command, env=env, stdout=output, stderr=output, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"{Path(str(command[0])).name} failed ({result.returncode}); output was withheld to protect synthetic credentials")


def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def console_stop(pid):
    # Like WinSW, send CTRL_C to the child's own console. Never signal the calling console.
    code = """
import ctypes, sys, time
k = ctypes.WinDLL('kernel32', use_last_error=True)
k.FreeConsole()
if not k.AttachConsole(int(sys.argv[1])): sys.exit(2)
k.SetConsoleCtrlHandler(None, True)
if not k.GenerateConsoleCtrlEvent(0, 0): sys.exit(3)
time.sleep(.3)
k.FreeConsole()
"""
    run([sys.executable, "-c", code, str(pid)])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--postgres-bin", required=True, type=Path)
    parser.add_argument("--server", required=True, type=Path)
    parser.add_argument("--certificate-tool", required=True, type=Path)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--verify-client-reconnect", action="store_true")
    parser.add_argument("--verify-desktop-enrollment-engine", type=Path,
                        help="Run existing TLS enrollment/credential recovery coverage with this packaged engine")
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("Windows is required")
    root = args.root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    # Synthetic secrets and cluster are private even when the parent inherits broad permissions.
    run(["icacls.exe", str(root), "/inheritance:r", "/grant:r", f"{os.getlogin()}:(OI)(CI)F"])
    cluster = root / "postgres"
    database_port, server_port = free_port(), free_port()
    while server_port == database_port:
        server_port = free_port()
    admin_password, app_password = secrets.token_hex(32), secrets.token_hex(32)
    password_file = root / "initdb-password"
    password_file.write_text(admin_password, encoding="utf-8")
    pg = args.postgres_bin.resolve()
    run([pg / "initdb.exe", "-D", cluster, "-U", "postgres", "-A", "scram-sha-256",
         "--encoding=UTF8", "--locale=C", f"--pwfile={password_file}"])
    password_file.unlink()
    db_env = dict(os.environ, PGPASSWORD=admin_password)
    psql = [pg / "psql.exe", "-X", "-h", "127.0.0.1", "-p", str(database_port), "-U", "postgres",
            "-d", "postgres", "-v", "ON_ERROR_STOP=1"]
    started = False
    server = None
    log = (root / "server.log").open("wb")
    checks = []

    def start_database():
        nonlocal started
        started = True  # Ensure cleanup also runs when the startup tool fails after launching PostgreSQL.
        run([pg / "pg_ctl.exe", "-D", cluster, "-l", root / "postgres.log", "-o",
             f"-h 127.0.0.1 -p {database_port} -c log_statement=none -c log_min_error_statement=panic", "-w", "start"])

    def stop_database():
        nonlocal started
        run([pg / "pg_ctl.exe", "-D", cluster, "-m", "fast", "-w", "stop"])
        started = False

    context = None
    last_probe = ["not probed"]

    def status(route):
        try:
            with urllib.request.urlopen(f"https://localhost:{server_port}/{route}", context=context, timeout=4) as response:
                return response.status
        except urllib.error.HTTPError as error:
            return error.code
        except (OSError, urllib.error.URLError) as error:
            last_probe[0] = str(error)
            return 0

    def wait_ready():
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if server.poll() is not None:
                raise RuntimeError("Server exited before readiness; inspect private server.log")
            if status("readyz") == 204:
                return
            time.sleep(.2)
        raise RuntimeError("Readiness deadline elapsed: " + last_probe[0])

    try:
        start_database()
        role_sql = root / "create-role.sql"
        role_sql.write_text(f"CREATE ROLE eitmad_runtime LOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE PASSWORD '{app_password}';", encoding="utf-8")
        try:
            run(psql + ["-f", role_sql], db_env)
        finally:
            role_sql.unlink()
        run([pg / "createdb.exe", "-h", "127.0.0.1", "-p", str(database_port), "-U", "postgres",
             "-O", "eitmad_runtime", "eitmad_runtime"], db_env)
        certificate = root / "certificates"
        run(["dotnet", args.certificate_tool.resolve(), certificate])
        context = ssl.create_default_context(cafile=str(certificate / "trusted-ca.pem"))
        # The repository's synthetic .NET fixture has no AKI. Retain CA and hostname validation,
        # using Python's pre-3.13 verification profile, as the existing Rust TLS tests do.
        context.verify_flags &= ~ssl.VERIFY_X509_STRICT
        (root / "database-url").write_text(f"postgresql://eitmad_runtime:{app_password}@127.0.0.1:{database_port}/eitmad_runtime", encoding="utf-8")
        (root / "token-key").write_text(base64.urlsafe_b64encode(secrets.token_bytes(32)).decode().rstrip("="), encoding="utf-8")
        (root / "manifests").mkdir()
        environment = {key: value for key, value in os.environ.items() if not key.startswith("EITMAD_SERVER_")}
        environment.update({
            "EITMAD_SERVER_DATABASE_URL_FILE": str(root / "database-url"),
            "EITMAD_SERVER_TOKEN_KEY_FILE": str(root / "token-key"),
            "EITMAD_SERVER_LISTEN": f"127.0.0.1:{server_port}",
            "EITMAD_SERVER_TLS_CERTIFICATE": str(certificate / "server-cert.pem"),
            "EITMAD_SERVER_TLS_PRIVATE_KEY": str(certificate / "server-key.pem"),
            "EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY": str(root / "manifests"),
            "EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID": str(uuid.uuid4()),
            "EITMAD_SERVER_UPDATE_KEY_ID": "synthetic-runtime-check",
            "EITMAD_SERVER_UPDATE_PUBLIC_KEY": "11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=",
        })
        stop_database()
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = 0
        server = subprocess.Popen([args.server.resolve(), "serve"], env=environment, stdout=log, stderr=log,
                                  creationflags=subprocess.CREATE_NEW_CONSOLE, startupinfo=startup)
        time.sleep(1)
        assert server.poll() is None and status("readyz") == 0, "Startup incorrectly required a live database or announced readiness"
        start_database()
        wait_ready()
        checks.append("database starts late: HTTPS becomes ready with secret files and a non-superuser role")
        stop_database()
        assert status("livez") == 204 and status("readyz") == 503, "Liveness/readiness did not distinguish database loss"
        assert server.poll() is None, "Database interruption killed the host"
        start_database()
        wait_ready()
        checks.append("database interruption: live 204, ready 503, then ready 204 with the same host PID")
        conflict = subprocess.run([args.server.resolve(), "serve"], env=environment, capture_output=True, timeout=30)
        assert conflict.returncode != 0 and server.poll() is None, "Port conflict did not fail without disturbing the owner"
        checks.append("HTTPS port conflict: second process fails; original host remains ready")
        console_stop(server.pid)
        assert server.wait(timeout=40) == 0, "CTRL_C shutdown was not graceful"
        server = None
        checks.append("CTRL_C: host exits successfully within the wrapper stop budget")
        if args.verify_client_reconnect or args.verify_desktop_enrollment_engine:
            run([pg / "createdb.exe", "-h", "127.0.0.1", "-p", str(database_port), "-U", "postgres",
                 "-O", "eitmad_runtime", "eitmad_reconnect"], db_env)
            reconnect_environment = dict(os.environ)
            reconnect_environment.update({
                "EITMAD_DIRECT_TEST_DATABASE_URL": f"postgresql://eitmad_runtime:{app_password}@127.0.0.1:{database_port}/eitmad_reconnect",
                "EITMAD_DIRECT_TEST_CERTIFICATE": str(certificate / "server-cert.pem"),
                "EITMAD_DIRECT_TEST_PRIVATE_KEY": str(certificate / "server-key.pem"),
                "EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE": str(certificate / "trusted-ca.pem"),
                "EITMAD_DIRECT_TEST_WRONG_CERTIFICATE": str(certificate / "wrong-ca.pem"),
            })
            scenario = "real_server_authentication_tls_sync_and_reconnect"
            if args.verify_desktop_enrollment_engine:
                reconnect_environment["EITMAD_DESKTOP_ENROLLMENT_ENGINE"] = str(args.verify_desktop_enrollment_engine.resolve())
                scenario = "desktop_enrollment_recovers_interruption_expiry_and_credential_loss"
            run(["rustup", "run", "1.85.1", "cargo", "test", "--locked", "-p", "eitmad-server-connection",
                 "--test", "direct_route", scenario, "--", "--ignored", "--exact"],
                reconnect_environment, timeout=300)
            checks.append("packaged engine: TLS enrollment, reopen, named-pipe controller, retained work, and credential recovery" if args.verify_desktop_enrollment_engine
                          else "existing Rust client: authenticated TLS sync, transport loss, and reconnect after host restart")
        (root / "results.json").write_text(json.dumps({"passed": checks, "servicesInstalled": False, "rebootVerified": False}, indent=2), encoding="utf-8")
        for check in checks:
            print("PASS:", check)
        print("Synthetic evidence:", root)
    finally:
        if server and server.poll() is None:
            try:
                console_stop(server.pid)
                server.wait(timeout=40)
            except (RuntimeError, subprocess.TimeoutExpired):
                server.terminate()
                server.wait(timeout=10)
        log.close()
        if started:
            stop_database()


if __name__ == "__main__":
    main()
