#!/usr/bin/env python3
"""Build versioned validation artifacts; signing never implies release acceptance."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parents[2]
PLAN = ROOT / "deploy" / "release-plan.toml"
FIXED_ZIP_TIME = (2020, 1, 1, 0, 0, 0)


def run(*command: str, env: dict | None = None) -> None:
    subprocess.run(command, cwd=ROOT, check=True, env=env)


def windows_version(version: str) -> str:
    match = re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-[A-Za-z0-9.-]+)?", version)
    if not match or any(int(part) > 65535 for part in match.groups()):
        raise ValueError("Use a semantic version with three numeric components <= 65535")
    return ".".join(match.groups()) + ".0"


def build_rust(package: str, version: str) -> Path:
    # Select rustc explicitly: a PATH shim must not mix stable objects into the MSRV cache.
    environment = dict(os.environ, EITMAD_RELEASE_VERSION=version, CARGO_INCREMENTAL="0")
    compiler = subprocess.check_output(
        ["rustup", "which", "--toolchain", "1.85.1", "rustc"], cwd=ROOT, text=True
    ).strip()
    environment["RUSTC"] = compiler
    environment["CARGO_TARGET_DIR"] = str(ROOT / "target" / "toolchain-1.85.1")
    run("rustup", "run", "1.85.1", "cargo", "build", "--locked", "--release", "-p", package, env=environment)
    return Path(environment["CARGO_TARGET_DIR"]) / "release"


def load_plan() -> dict:
    with PLAN.open("rb") as stream:
        return tomllib.load(stream)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_manifest(output: Path, artifact: Path, version: str, target: str) -> Path:
    manifest = output / f"{artifact.name}.manifest.json"
    payload = {
        "schemaVersion": 1,
        "target": target,
        "version": version,
        "file": artifact.name,
        "bytes": artifact.stat().st_size,
        "sha256": sha256(artifact),
        "signed": False,
        "productionEligible": False,
    }
    manifest.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return manifest


def deterministic_zip(
    source: Path,
    destination: Path,
    excluded_suffixes: frozenset[str] = frozenset(),
) -> None:
    with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in sorted(
            item
            for item in source.rglob("*")
            if item.is_file() and item.suffix.lower() not in excluded_suffixes
        ):
            relative = path.relative_to(source).as_posix()
            info = zipfile.ZipInfo(relative, FIXED_ZIP_TIME)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            archive.writestr(info, path.read_bytes(), compresslevel=9)


def release_inventory(stage: Path, version: str, target: str, publisher: str, payload_inventory: str | None = None) -> None:
    for path in stage.rglob("*"):
        if path.is_symlink() or (path.is_file() and (
            path.suffix.lower() in {".db", ".sqlite", ".sqlite3", ".pem", ".pfx", ".key", ".pdb"}
            or path.name.lower() in {"credentials.json", "secrets.json", "connection.json", "desktop-device.json", "server.json", "host.json"}
        )):
            raise ValueError("Operational state, keys, links, and debug symbols cannot enter an installation package")
    migrations = {}
    for line in (ROOT / "deploy/migrations.sha256").read_text().splitlines():
        if line and not line.startswith("#"):
            digest, path = line.split(None, 1)
            if sha256(ROOT / path) != digest:
                raise ValueError("Migration inventory differs from source")
            migrations[path] = digest
    transport = (ROOT / "crates/contracts/src/transport.rs").read_text(encoding="utf-8")
    protocol = re.search(r"PROTOCOL_VERSION:.*?major: (\d+),\s+minor: (\d+)", transport, re.S)
    assert protocol
    payload = {
        "schemaVersion": 1, "version": version, "windowsVersion": windows_version(version),
        "target": target, "publisher": publisher, "architecture": "x64",
        "sourceCommit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "sourceDirty": bool(subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=normal"], cwd=ROOT)),
        "localProtocol": ".".join(protocol.groups()),
        "compatibility": "Install server and desktop from the same release; Rust negotiates protocols and schemas.",
        "migrations": migrations,
        "files": {p.relative_to(stage).as_posix(): sha256(p) for p in sorted(stage.rglob("*")) if p.is_file()},
        "productionEligible": False,
        "payloadInventorySha256": payload_inventory,
        "requiredAcceptance": ["trusted signatures and RFC3161 timestamps", "clean Windows lifecycle and connected work", "backup restore and migration rehearsal"],
    }
    (stage / "release.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def rust_components(metadata: dict, package: str) -> list[dict]:
    """Inventory resolved normal/build inputs without unrelated workspace or test roots."""
    packages = {item["id"]: item for item in metadata["packages"]}
    nodes = {item["id"]: item for item in metadata["resolve"]["nodes"]}
    pending = [item["id"] for item in metadata["packages"] if item["name"] == package]
    included: set[str] = set()
    while pending:
        identity = pending.pop()
        if identity in included:
            continue
        included.add(identity)
        for dependency in nodes[identity]["deps"]:
            if any(kind["kind"] != "dev" for kind in dependency["dep_kinds"]):
                pending.append(dependency["pkg"])
    return [{"type": "library", "name": packages[key]["name"], "version": packages[key]["version"],
             "purl": f"pkg:cargo/{packages[key]['name']}@{packages[key]['version']}"}
            for key in sorted(included)]


def write_sbom(stage: Path, package: str, version: str) -> None:
    compiler = subprocess.check_output(["rustup", "which", "--toolchain", "1.85.1", "rustc"], cwd=ROOT, text=True).strip()
    environment = dict(os.environ, RUSTC=compiler)
    platform = "x86_64-pc-windows-msvc" if os.name == "nt" else "x86_64-unknown-linux-gnu"
    metadata = json.loads(subprocess.check_output(
        ["rustup", "run", "1.85.1", "cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", platform],
        cwd=ROOT, env=environment,
    ))
    components = rust_components(metadata, package)
    for path in sorted(stage.rglob("*.deps.json")):
        libraries = json.loads(path.read_text(encoding="utf-8"))["libraries"]
        for identity, library in sorted(libraries.items()):
            name, component_version = identity.rsplit("/", 1)
            component = {"type": "library", "name": name, "version": component_version}
            if library["type"] != "project":
                component["purl"] = f"pkg:nuget/{name.removeprefix('runtimepack.')}@{component_version}"
            if component not in components:
                components.append(component)
    payload = {"bomFormat": "CycloneDX", "specVersion": "1.6", "version": 1,
               "metadata": {"component": {"type": "application", "name": package, "version": version},
                            "properties": [{"name": "eitmad:inventory-scope", "value": "Resolved Rust normal/build inputs and published .NET libraries; excludes test-only roots. Payload hashes are in release.json."}]},
               "components": components}
    (stage / "sbom.cdx.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def copy_install_tools(stage: Path, target: str) -> None:
    names = ["package.psm1", "install-client.ps1"] if target == "windows-desktop" else [
        "package.psm1", "install-server.ps1", "restore-backup.ps1", "backup.psm1", "host.psm1",
    ]
    for name in names:
        shutil.copy2(ROOT / "deploy/windows" / name, stage / name)
    # Include the canonical task pages, not operational examples or test harnesses.
    for name in ("package-windows-desktop.md", "run-windows-host-services.md", "setup-lan-server.md", "run-server-authority.md", "backup-lan-installation.md"):
        shutil.copy2(ROOT / "docs/operations" / name, stage / name)


def sign_path(path: Path, publisher: str, hook: Path | None) -> None:
    if hook:
        run("powershell.exe", "-NoProfile", "-File", str(ROOT / "deploy/windows/sign-windows.ps1"),
            "-Path", str(path), "-Publisher", publisher, "-SigningHook", str(hook.resolve()))


def sdk_tool(name: str) -> str:
    found = shutil.which(name)
    if found:
        return found
    kits = Path(os.environ.get("ProgramFiles(x86)", "C:/Program Files (x86)")) / "Windows Kits/10/bin"
    matches = sorted(kits.glob(f"10.*/x64/{name}"), key=lambda p: tuple(int(n) for n in p.parts[-3].split(".")))
    if not matches:
        raise SystemExit(f"Install the Windows SDK x64 tool: {name}")
    return str(matches[-1])


def publish(project: str, stage: Path, version: str) -> None:
    run("dotnet", "publish", project, "--configuration", "Release", "--runtime", "win-x64",
        "--self-contained", "true", f"-p:Version={version}", f"-p:AssemblyVersion={windows_version(version)}",
        f"-p:FileVersion={windows_version(version)}", "-p:DebugType=None", "-p:DebugSymbols=false",
        "--output", str(stage), "--nologo")


def build_windows(output: Path, version: str, publisher: str = "CN=Eitmad Unsigned Validation", hook: Path | None = None) -> Path:
    if os.name != "nt":
        raise SystemExit("windows-desktop packaging requires Windows")
    plan = load_plan()["artifacts"]["windows_desktop"]
    binaries = build_rust(plan["engine_package"], version)
    with tempfile.TemporaryDirectory(prefix="eitmad-package-") as temp_value:
        stage = Path(temp_value) / "Eitmad"
        publish(plan["shell_project"], stage, version)
        shutil.copy2(binaries / "eitmad-engine-cli.exe", stage)
        verify_binary_version(stage / "eitmad-engine-cli.exe", "eitmad-engine-cli", version)
        report = json.loads(subprocess.check_output(
            [str(stage / "eitmad-engine-cli.exe"), "diagnose", "--runtime-directory", str(Path(temp_value) / "probe")],
            text=True, encoding="utf-8",
        ))
        if report["identity"]["productVersion"] != version:
            raise ValueError("Engine runtime identity differs from the release version")
        # Native package assets are generated from a fixed vector-like mark, without local input.
        run("powershell.exe", "-NoProfile", "-File", str(ROOT / "deploy/windows/package-assets.ps1"), "-Directory", str(stage / "Assets"))
        manifest = (ROOT / "deploy/windows/AppxManifest.xml").read_text(encoding="utf-8")
        manifest = manifest.replace("@VERSION@", windows_version(version)).replace("@PUBLISHER@", escape(publisher, {'"': '&quot;'}))
        (stage / "AppxManifest.xml").write_text(manifest, encoding="utf-8")
        write_sbom(stage, plan["engine_package"], version)
        sign_path(stage, publisher, hook)
        release_inventory(stage, version, "windows-desktop", publisher)
        package = Path(temp_value) / f"eitmad-desktop-{version}-win-x64.msix"
        run(sdk_tool("makeappx.exe"), "pack", "/d", str(stage), "/p", str(package), "/o")
        sign_path(package, publisher, hook)
        bundle = Path(temp_value) / "installer"
        bundle.mkdir()
        shutil.copy2(package, bundle)
        copy_install_tools(bundle, "windows-desktop")
        sign_path(bundle, publisher, hook)
        release_inventory(bundle, version, "windows-desktop", publisher, sha256(stage / "release.json"))
        destination = output / f"eitmad-desktop-{version}-win-x64-{'payload-signed' if hook else 'unsigned'}.zip"
        deterministic_zip(bundle, destination, frozenset({".pdb"}))
    return destination


def verify_binary_version(path: Path, name: str, version: str) -> None:
    actual = subprocess.check_output([str(path), "--version"], text=True).strip()
    if actual != f"{name} {version}":
        raise ValueError(f"Packaged {name} version does not match release")


def build_windows_server(output: Path, version: str, publisher: str = "CN=Eitmad Unsigned Validation", hook: Path | None = None) -> Path:
    if os.name != "nt":
        raise SystemExit("windows-server packaging requires Windows")
    binaries = build_rust(load_plan()["artifacts"]["windows_server"]["rust_package"], version)
    with tempfile.TemporaryDirectory(prefix="eitmad-package-") as temp_value:
        stage = Path(temp_value)
        shutil.copy2(binaries / "eitmad-server.exe", stage)
        verify_binary_version(stage / "eitmad-server.exe", "eitmad-server", version)
        copy_install_tools(stage, "windows-server")
        shutil.copy2(ROOT / "run-services.ps1", stage)
        # Keep the existing LAN tool's relative assembly path; it loads into PowerShell 7.4+.
        (stage / "tools").mkdir()
        shutil.copy2(ROOT / "tools/lan-server.ps1", stage / "tools")
        publish("tools/lan-certificate/Eitmad.LanCertificate.csproj",
                stage / "tools/lan-certificate/bin/Release/net8.0-windows", version)
        (stage / "deploy/windows").mkdir(parents=True)
        for name in ("host.psm1", "backup.psm1", "backup-worker.ps1", "run-backup.ps1", "restore-backup.ps1", "backup.example.json"):
            shutil.copy2(ROOT / "deploy/windows" / name, stage / "deploy/windows")
        write_sbom(stage, load_plan()["artifacts"]["windows_server"]["rust_package"], version)
        sign_path(stage, publisher, hook)
        release_inventory(stage, version, "windows-server", publisher)
        destination = output / f"eitmad-server-{version}-win-x64-{'payload-signed' if hook else 'unsigned'}.zip"
        deterministic_zip(stage, destination, frozenset({".pdb"}))
    return destination


def build_server(output: Path, version: str) -> Path:
    if sys.platform != "linux":
        raise SystemExit("server packaging requires the declared Linux build host")
    plan = load_plan()["artifacts"]["server"]
    executable = build_rust(plan["rust_package"], version) / "eitmad-server"
    destination = output / f"eitmad-server-{version}-{plan['runtime']}.tar.gz"
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "1577836800"))
    tar_bytes = io.BytesIO()
    with tarfile.open(fileobj=tar_bytes, mode="w", format=tarfile.PAX_FORMAT) as archive:
        info = archive.gettarinfo(str(executable), arcname="eitmad-server")
        info.mtime = epoch
        info.uid = 0
        info.gid = 0
        info.uname = "root"
        info.gname = "root"
        with executable.open("rb") as stream:
            archive.addfile(info, stream)
    with destination.open("wb") as output_stream:
        with gzip.GzipFile(filename="", mode="wb", fileobj=output_stream, mtime=epoch, compresslevel=9) as compressor:
            compressor.write(tar_bytes.getvalue())
    return destination


def verify(artifact: Path, manifest: Path) -> None:
    payload = json.loads(manifest.read_text(encoding="utf-8"))
    if payload["file"] != artifact.name or payload["bytes"] != artifact.stat().st_size:
        raise SystemExit("artifact metadata does not match")
    if payload["sha256"] != sha256(artifact):
        raise SystemExit("artifact digest does not match")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("target", choices=("windows-desktop", "windows-server", "server"))
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--publisher", default="CN=Eitmad Unsigned Validation")
    parser.add_argument("--signing-hook", type=Path, help="Reviewed hardware-backed signing script accepting -Path; no private key is packaged")
    arguments = parser.parse_args()
    windows_version(arguments.version)
    if arguments.signing_hook:
        if arguments.publisher == "CN=Eitmad Unsigned Validation":
            parser.error("Signing requires the exact approved certificate-profile subject")
        if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT):
            parser.error("Signed release builds require a clean reviewed checkout")
    arguments.output.mkdir(parents=True, exist_ok=True)
    if arguments.target == "server":
        if arguments.signing_hook:
            parser.error("The Windows signing hook cannot sign Linux artifacts")
        artifact = build_server(arguments.output.resolve(), arguments.version)
    else:
        builder = build_windows if arguments.target == "windows-desktop" else build_windows_server
        artifact = builder(arguments.output.resolve(), arguments.version, arguments.publisher, arguments.signing_hook)
    manifest = write_manifest(arguments.output, artifact, arguments.version, arguments.target)
    verify(artifact, manifest)
    print(artifact)
    print(manifest)


if __name__ == "__main__":
    main()
