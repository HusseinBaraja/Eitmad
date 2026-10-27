"""Repeat isolated setup proofs. Does not install packages, services, firewall rules or runtimes.

This development runner needs build tools. Clean operator PCs use test-offline-machine.ps1.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

CHECKS = ("offline-payload", "package", "installation", "host", "discovery-interfaces", "offline-machine-probes",
          "setup-journal", "discovery", "server-build", "dev-certificate", "lan-certificate",
          "fresh-preparation", "tls-accounts-pairing-two-clients")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--postgres-bin", required=True, type=Path)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--from-check", choices=CHECKS, default=CHECKS[0],
                        help="Start at a failed check using a new evidence root; earlier checks are recorded as not run")
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("Windows is required")
    workspace = Path(__file__).resolve().parents[2]
    pg = args.postgres_bin.resolve()
    for name in ("initdb", "pg_ctl", "psql", "createdb"):
        if not (pg / f"{name}.exe").is_file():
            parser.error(f"Missing PostgreSQL test tool: {name}")
    root = args.root.absolute()
    if shutil.disk_usage(root.parent).free < 5 * 1024**3:
        parser.error("Keep at least 5 GiB free for isolated test clusters and the shared build cache")
    root.mkdir(exist_ok=False)
    # Logs can contain synthetic database input. Keep them private; never copy live state.
    subprocess.run(["icacls.exe", str(root), "/inheritance:r", "/grant:r", f"{os.getlogin()}:(OI)(CI)F"],
                   check=True, stdout=subprocess.DEVNULL)
    environment = dict(os.environ, CARGO_TARGET_DIR=str(workspace / "target/toolchain-1.85.1"), CARGO_INCREMENTAL="0")
    # Windows PowerShell children must not load PowerShell 7 modules inherited from a developer host.
    environment["PSModulePath"] = str(Path(os.environ["SystemRoot"]) / "System32/WindowsPowerShell/v1.0/Modules")
    compiler = subprocess.check_output(["rustup", "which", "rustc", "--toolchain", "1.85.1"], text=True).strip()
    environment.update(RUSTC=compiler, PATH=str(Path(compiler).parent) + os.pathsep + environment["PATH"])
    passed = []
    report = {"passed": passed, "notRun": [], "failed": None, "fullAcceptance": False, "liveInstallationChanged": False,
              "unverified": ["trusted signed bundle on clean offline Server and ordinary-user Client PCs",
                             "native discovery/pairing/staff sign-in on separate PCs", "effective Private/Public firewall",
                             "boot without Windows sign-in", "physical DHCP changes", "replacement-PC credential/media recovery"]}

    def check(name, command, timeout=1200):
        if CHECKS.index(name) < CHECKS.index(args.from_check):
            report["notRun"].append(name)
            return
        report["failed"] = name
        with (root / f"{name}.log").open("wb") as log:
            result = subprocess.run([str(item) for item in command], cwd=workspace, env=environment,
                                    stdout=log, stderr=subprocess.STDOUT, timeout=timeout)
        if result.returncode:
            raise RuntimeError(f"{name} failed; inspect its private log")
        passed.append(name)
        report["failed"] = None
        print("PASS:", name, flush=True)

    cargo = ["rustup", "run", "1.85.1", "cargo"]
    try:
        check("offline-payload", [sys.executable, "-m", "unittest", "discover", "-s", "scripts/ci/tests", "-p", "test_offline_payload.py"])
        for name in ("package", "installation", "host", "discovery-interfaces", "offline-machine-probes"):
            check(name, ["powershell", "-NoProfile", "-File", workspace / f"deploy/windows/test-{name}.ps1"])
        check("setup-journal", cargo + ["test", "--locked", "-p", "eitmad-server", "--bin", "eitmad-server", "windows_setup"])
        check("discovery", cargo + ["test", "--locked", "-p", "eitmad-server-discovery"])
        check("server-build", cargo + ["build", "--locked", "-p", "eitmad-server", "--bin", "eitmad-server"])
        for name in ("dev-certificate", "lan-certificate"):
            project = "Eitmad.DevCertificate" if name == "dev-certificate" else "Eitmad.LanCertificate"
            check(name, ["dotnet", "build", workspace / f"tools/{name}/{project}.csproj", "--configuration", "Release", "--nologo"])
        server = workspace / "target/toolchain-1.85.1/debug/eitmad-server.exe"
        certificate = workspace / "tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe"
        check("fresh-preparation", [sys.executable, workspace / "deploy/windows/test-workshop-preparation.py",
              "--postgres-bin", pg, "--server", server, "--certificate-executable", certificate, "--root", root / "preparation"])
        check("tls-accounts-pairing-two-clients", [sys.executable, workspace / "deploy/windows/test-runtime.py",
              "--postgres-bin", pg, "--server", server, "--certificate-tool",
              workspace / "tools/dev-certificate/bin/Release/net8.0/Eitmad.DevCertificate.dll",
              "--root", root / "runtime", "--verify-workshop-setup", "--verify-client-reconnect", "--verify-setup-security"])
    finally:
        (root / "results.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
