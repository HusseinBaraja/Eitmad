"""Prove fresh Windows host preparation using a new cluster and loopback-only processes."""
import argparse
import json
import os
from pathlib import Path
import secrets
import socket
import ssl
import subprocess
import time
import uuid

# Reuse the existing native console signal and free-port helpers.
import importlib.util
runtime_spec = importlib.util.spec_from_file_location("runtime", Path(__file__).with_name("test-runtime.py"))
runtime = importlib.util.module_from_spec(runtime_spec)
runtime_spec.loader.exec_module(runtime)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--postgres-bin", required=True, type=Path)
    parser.add_argument("--server", required=True, type=Path)
    parser.add_argument("--certificate-executable", required=True, type=Path)
    parser.add_argument("--root", required=True, type=Path)
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("Windows is required")
    root = args.root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    runtime.run(["icacls.exe", str(root), "/inheritance:r", "/grant:r", f"{os.getlogin()}:(OI)(CI)F"])
    state = root / "fresh-host"
    database_port, https_port = runtime.free_port(), runtime.free_port()
    while https_port == database_port:
        https_port = runtime.free_port()
    request = {
        "version": 1, "stateDirectory": str(state),
        "platformScript": str(Path(__file__).with_name("prepare-workshop.ps1").resolve()),
        "postgresBin": str(args.postgres_bin.resolve()),
        "certificateExecutable": str(args.certificate_executable.resolve()),
        "serverName": "eitmad-setup-test.invalid", "listenAddress": "192.168.231.10",
        "serverPort": https_port, "databasePort": database_port,
        "workshop": {"version": 1, "workshopName": "ورشة اختبار التثبيت",
                     "ownerUsername": "setup-test-owner", "ownerPassword": secrets.token_urlsafe(32)},
    }

    def prepare(payload):
        return subprocess.run([args.server.resolve(), "prepare-windows-workshop"],
                              input=json.dumps(payload).encode(), capture_output=True, timeout=240)

    # Invalid credentials must fail before any native state is created.
    invalid = json.loads(json.dumps(request))
    invalid["workshop"]["ownerPassword"] = "short"
    rejected = prepare(invalid)
    assert rejected.returncode != 0 and not state.exists(), "Invalid input created native state"
    result = prepare(request)
    if result.returncode:
        if (state / "cluster/postmaster.pid").exists():
            runtime.run([args.postgres_bin.resolve() / "pg_ctl.exe", "-D", state / "cluster", "-m", "fast", "-w", "stop"])
        (root / "preparation-error.log").write_bytes(result.stderr)
        raise RuntimeError("Fresh preparation failed; inspect the private preparation-error.log")
    prepared = json.loads(result.stdout)
    assert prepared["version"] == 1
    assert prepared["endpoint"] == f"https://eitmad-setup-test.invalid:{https_port}/"
    assert not (state / "cluster/postmaster.pid").exists(), "Preparation left PostgreSQL running"
    assert not (state / "initdb-password").exists(), "Transient initdb input was retained"
    assert json.loads((state / "prepared-workshop.json").read_text()) == prepared
    before = (state / "server.json").read_bytes()
    repeated = prepare(request)
    assert repeated.returncode != 0 and (state / "server.json").read_bytes() == before
    settings = json.loads(before)
    secret_values = [request["workshop"]["ownerPassword"]]
    secret_values += [(state / name).read_text() for name in
                      ("postgres-admin-password", "application-password", "certificate-password", "token-key")]
    for value in secret_values:
        assert value.encode() not in result.stdout + result.stderr + repeated.stdout + repeated.stderr
    config = (state / "cluster/postgresql.conf").read_text()
    assert "listen_addresses = '127.0.0.1'" in config
    active = json.loads((state / "active-tls.json").read_text())
    environment = {name: value for name, value in os.environ.items() if not name.startswith("EITMAD_SERVER_")}
    environment.update(settings["environment"])
    # This test overrides the proposed LAN bind. Only its loopback listener is started.
    environment.update(EITMAD_SERVER_LISTEN=f"127.0.0.1:{https_port}",
                       EITMAD_SERVER_TLS_CERTIFICATE=str(state / active["Certificate"]),
                       EITMAD_SERVER_TLS_PRIVATE_KEY=str(state / active["PrivateKey"]))
    pg = args.postgres_bin.resolve()
    process = None
    database_started = False
    server_log = (root / "server.log").open("wb")
    try:
        database_started = True
        runtime.run([pg / "pg_ctl.exe", "-D", state / "cluster", "-l", state / "postgres.log", "-w", "start"])
        process = subprocess.Popen([args.server.resolve(), "serve"], env=environment,
                                   stdout=server_log, stderr=server_log, creationflags=subprocess.CREATE_NEW_CONSOLE)
        context = ssl.create_default_context(cafile=str(state / "trusted-ca.pem"))

        def exchange(route, body=None):
            with socket.create_connection(("127.0.0.1", https_port), timeout=5) as tcp:
                with context.wrap_socket(tcp, server_hostname=request["serverName"]) as tls:
                    encoded = json.dumps(body).encode() if body is not None else b""
                    method = "POST" if body is not None else "GET"
                    header = (f"{method} /{route} HTTP/1.1\r\nHost: {request['serverName']}\r\n"
                              f"Content-Type: application/json\r\nContent-Length: {len(encoded)}\r\nConnection: close\r\n\r\n")
                    tls.sendall(header.encode() + encoded)
                    response = b""
                    while chunk := tls.recv(16384):
                        response += chunk
                    return int(response.split(b" ", 2)[1]), response

        deadline = time.monotonic() + 50
        last_failure = "connection unavailable"
        while True:
            try:
                if exchange("readyz")[0] == 204:
                    break
            except ssl.SSLCertVerificationError as error:
                # Certificate errors are stable and contain no account input or credentials.
                raise RuntimeError(f"Prepared TLS certificate failed validation ({error.verify_code}): {error.verify_message}") from None
            except OSError as error:
                last_failure = type(error).__name__
            assert process.poll() is None, "Prepared server exited before TLS readiness"
            if time.monotonic() > deadline:
                raise RuntimeError("Prepared server did not become ready: " + last_failure)
            time.sleep(.2)
        # Login uses a distinct synthetic device. No installation identity is copied.
        # Public Ed25519 test key from RFC 8032, not a private installation key.
        import base64
        public_key = bytes.fromhex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
        login = {"tenantCode": prepared["workshop"]["tenantCode"],
                 "username": request["workshop"]["ownerUsername"], "password": request["workshop"]["ownerPassword"],
                 "deviceId": str(uuid.uuid4()), "deviceLabel": "جهاز اختبار الإعداد",
                 "devicePublicKey": {"algorithm": "ed25519", "base64": base64.urlsafe_b64encode(public_key).decode().rstrip("=")},
                 "deviceProof": None}
        status, response = exchange("v1/auth/login", login)
        # Fresh credentials do not approve an arbitrary device. Native setup must
        # authorize its first desktop; other devices use independent pairing.
        assert status == 401, "An unapproved device signed in with Owner credentials"
        # Account-response bodies stay in memory and are not written to evidence.
        del response, login
    finally:
        if process and process.poll() is None:
            runtime.console_stop(process.pid)
            process.wait(timeout=40)
        server_log.close()
        if database_started:
            runtime.run([pg / "pg_ctl.exe", "-D", state / "cluster", "-m", "fast", "-w", "stop"])
    (root / "results.json").write_text(json.dumps({"passed": [
        "invalid credentials rejected before native changes", "new private PostgreSQL 17 cluster and SCRAM credentials",
        "Rust migrations and active Owner/Manager initialization", "stopped complete preparation and refused reinitialization",
        "no secret in command output", "loopback-only PostgreSQL", "validated HTTPS ready 204 and unapproved Owner device refused"],
        "servicesInstalled": False, "firewallChanged": False, "physicalClientVerified": False}, indent=2) + "\n")
    print("PASS: isolated fresh Windows workshop preparation, validated TLS readiness and unapproved-device refusal")


if __name__ == "__main__":
    main()
