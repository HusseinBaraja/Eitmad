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
import stat
import struct
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

# These paths are the later setup program's role boundary. Client has no server dependency.
OFFLINE_COMPONENTS = {
    "setup": ("setup", ("EitmadSetup.exe", "EitmadSetup.dll", "EitmadSetup.runtimeconfig.json", "coreclr.dll", "hostfxr.dll", "hostpolicy.dll", "PresentationFramework.dll")),
    "desktop": ("client/desktop", ("install-client.ps1", "package.psm1", "release.json")),
    "application": ("server/application", ("eitmad-server.exe", "install-server.ps1", "release.json",
        "deploy/windows/prepare-workshop.ps1", "deploy/windows/host.psm1",
        "deploy/windows/installation.ps1", "deploy/windows/installation.psm1", "deploy/windows/installation-channel.cs",
        "deploy/windows/local-backup.ps1", "deploy/windows/local-backup.psm1", "deploy/windows/backup-tools.psm1",
        "deploy/windows/backup.psm1", "deploy/windows/restore-backup.ps1",
        "tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe",
        "tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.dll",
        "tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.runtimeconfig.json",
        "tools/lan-certificate/bin/Release/net8.0-windows/coreclr.dll",
        "tools/lan-certificate/bin/Release/net8.0-windows/hostfxr.dll",
        "tools/lan-certificate/bin/Release/net8.0-windows/hostpolicy.dll")),
    "postgresql": ("server/postgresql", ("bin/postgres.exe", "bin/initdb.exe", "bin/pg_ctl.exe",
        "bin/psql.exe", "bin/pg_dump.exe", "bin/pg_restore.exe", "bin/pg_basebackup.exe",
        "bin/pg_verifybackup.exe", "lib", "share/postgres.bki")),
    "winsw": ("server/winsw", ("WinSW.NET461.exe",)),
    "powershell": ("server/powershell", ("pwsh.exe", "coreclr.dll", "hostfxr.dll", "hostpolicy.dll", "pwsh.runtimeconfig.json")),
    "vc-redist": ("server/vc-redist", ("vc_redist.x64.exe",)),
    "netfx": ("server/netfx", ("ndp48-x86-x64-allos-enu.exe",)),
}
OFFLINE_OS_REQUIREMENTS = ["Supported patched x64 Windows build 19041+ on local NTFS",
    "Windows PowerShell 5.1", "Windows curl.exe (Schannel)", "MSIX permitted by Windows policy"]
EXECUTABLE_SUFFIXES = {".exe", ".dll", ".ps1", ".psm1", ".msix"}
CRT_FILES = {"concrt140.dll", "msvcp140.dll", "msvcp140_1.dll", "msvcp140_2.dll",
    "msvcp140_atomic_wait.dll", "msvcp140_codecvt_ids.dll", "vccorlib140.dll", "vcruntime140.dll", "vcruntime140_1.dll"}


def plain_path(path: Path) -> None:
    """Reject links and Windows junctions before resolving or traversing input."""
    for item in (path, *path.parents):
        if item.is_symlink() or (item.exists() and getattr(item.lstat(), "st_file_attributes", 0) & 0x400):
            raise ValueError("Payload paths cannot use links or reparse points")


def safe_relative(value: str) -> str:
    if not isinstance(value, str) or not value or any(
        part in {"", ".", ".."} or part.endswith((" ", ".")) or
        re.search(r'[<>:"\\|?*\x00-\x1f]', part) or
        re.fullmatch(r"(?i)(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\..*)?", part)
        for part in value.split("/")
    ):
        raise ValueError("Unsafe payload path")
    return value


def operational_path(relative: str) -> None:
    for part in Path(relative).parts:
        lower = part.lower()
        if lower in {"data", "cluster", "pgdata", "base", "global", "pg_wal", "pg_tblspc", "private",
                     "credentials.json", "secrets.json", "connection.json", "desktop-device.json",
                     "server.json", "host.json", "prepared-workshop.json", "active-tls.json",
                     "pg_version", "postmaster.pid", "pgpass.conf", ".pgpass"} or Path(lower).suffix in {
                         ".db", ".sqlite", ".sqlite3", ".pem", ".pfx", ".p12", ".key", ".crt", ".cer", ".pdb", ".dump", ".backup"}:
            raise ValueError("Operational state, certificates, identities, or debug symbols cannot enter an offline payload")


def tree_files(root: Path) -> dict[str, Path]:
    plain_path(root)
    if not root.is_dir():
        raise ValueError("Missing payload directory")
    result = {}
    seen = set()
    def fail_walk(error: OSError) -> None:
        raise error
    for directory, directories, files in os.walk(root, followlinks=False, onerror=fail_walk):
        if Path(directory) != root and not directories and not files:
            raise ValueError("Unexpected empty payload directory")
        for name in directories + files:
            path = Path(directory) / name
            plain_path(path)
            relative = safe_relative(path.relative_to(root).as_posix())
            operational_path(relative)
            if relative.casefold() in seen:
                raise ValueError("Duplicate Windows payload path")
            seen.add(relative.casefold())
            if name in files:
                if not stat.S_ISREG(path.stat().st_mode) or path.stat().st_nlink != 1:
                    raise ValueError("Payload files must be regular files without hard links")
                result[relative] = path
    return result


def read_json(path: Path) -> dict:
    def unique(pairs: list) -> dict:
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("Duplicate JSON key in payload metadata")
            result[key] = value
        return result
    return json.loads(path.read_text(encoding="utf-8-sig"), object_pairs_hook=unique)


def pe_architecture(path: Path) -> str:
    """Inspect PE headers, including IL-only AnyCPU; never execute a prerequisite."""
    with path.open("rb") as stream:
        def read_at(offset: int, size: int) -> bytes:
            stream.seek(offset)
            value = stream.read(size)
            if len(value) != size:
                raise ValueError("Truncated PE payload")
            return value
        if read_at(0, 2) != b"MZ":
            raise ValueError("Payload executable has no PE header")
        header = struct.unpack("<I", read_at(60, 4))[0]
        if read_at(header, 4) != b"PE\0\0":
            raise ValueError("Invalid PE payload")
        machine, sections = struct.unpack("<HH", read_at(header + 4, 4))
        optional_size = struct.unpack("<H", read_at(header + 20, 2))[0]
        optional = header + 24
        magic = struct.unpack("<H", read_at(optional, 2))[0]
        if machine == 0x8664 and magic == 0x20b:
            return "x64"
        if machine != 0x14c or magic != 0x10b:
            raise ValueError("Unsupported payload PE architecture")
        if optional_size >= 216 and struct.unpack("<I", read_at(optional + 92, 4))[0] >= 15:
            clr_rva, clr_size = struct.unpack("<II", read_at(optional + 208, 8))
            for index in range(sections):
                section = optional + optional_size + index * 40
                virtual_size, rva, size, raw = struct.unpack("<IIII", read_at(section + 8, 16))
                if clr_size >= 20 and rva <= clr_rva and clr_rva - rva + 20 <= min(size, virtual_size):
                    flags = struct.unpack("<I", read_at(raw + clr_rva - rva + 16, 4))[0]
                    if flags & 1 and not flags & (2 | 0x20000):
                        return "anycpu"
        return "x86"


def verify_payload_signatures(root: Path, files: dict[str, dict]) -> None:
    requests = [{"path": str((root / name).absolute()), "publisher": entry["publisher"]}
                for name, entry in files.items() if "publisher" in entry]
    if not requests:
        return
    if os.name != "nt":
        raise ValueError("Trusted payload signature validation requires Windows")
    # Input travels as JSON on stdin; neither paths nor publisher names become code.
    script = "$ErrorActionPreference='Stop'; $items=[Console]::In.ReadToEnd() | ConvertFrom-Json; " \
        "foreach($item in $items) { $s=Get-AuthenticodeSignature -LiteralPath $item.path; " \
        "if($s.Status -ne 'Valid' -or -not $s.TimeStamperCertificate -or " \
        "$s.SignerCertificate.Subject -cne $item.publisher) { throw 'Untrusted payload signature or publisher' } }"
    subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", script],
                   input=json.dumps(requests), text=True, check=True, capture_output=True)


def load_payload_review(path: Path, expected: str, version: str) -> dict:
    plain_path(path)
    if not re.fullmatch(r"[a-f0-9]{64}", expected) or sha256(path) != expected:
        raise ValueError("Review manifest differs from its authenticated SHA-256")
    review = read_json(path)
    if set(review) != {"schemaVersion", "version", "architecture", "components"} or \
            review["schemaVersion"] != 1 or review["version"] != version or review["architecture"] != "x64" or \
            set(review["components"]) != set(OFFLINE_COMPONENTS):
        raise ValueError("Unsupported or incomplete offline payload review")
    for name, component in review["components"].items():
        if set(component) != {"version", "provenance", "licenses", "files"} or not component["files"]:
            raise ValueError("Incomplete component review")
        selected = component["version"]
        if not isinstance(selected, str) or not re.fullmatch(r"[0-9]+(?:\.[0-9]+){1,3}(?:-[0-9A-Za-z.-]+)?", selected):
            raise ValueError("Invalid reviewed component version")
        parts = tuple(int(part) for part in selected.split("-")[0].split("."))
        if ((name in {"desktop", "application", "setup"} and selected != version) or
            (name == "postgresql" and (len(parts) != 2 or parts[0] != 17)) or
            (name == "winsw" and selected != "2.12.0") or
            (name == "powershell" and (parts[0] != 7 or parts < (7, 4))) or
            (name == "vc-redist" and (parts[0] != 14 or parts < (14, 40))) or
            (name == "netfx" and parts != (4, 8))):
            raise ValueError("Unsupported reviewed prerequisite version")
        provenance = component["provenance"]
        if set(provenance) != {"url", "sha256"} or not isinstance(provenance["url"], str) or \
                not re.fullmatch(r"https://[^\s]+", provenance["url"]) or \
                not re.fullmatch(r"[a-f0-9]{64}", provenance["sha256"]):
            raise ValueError("Component requires HTTPS acquisition provenance and source artifact SHA-256")
        for relative, entry in component["files"].items():
            safe_relative(relative)
            operational_path(relative)
            if not set(entry) <= {"sha256", "architecture", "publisher", "unsignedReason"} or \
                    not re.fullmatch(r"[a-f0-9]{64}", entry.get("sha256", "")):
                raise ValueError("Invalid reviewed file digest or metadata")
            suffix = Path(relative).suffix.lower()
            if suffix in EXECUTABLE_SUFFIXES:
                signed = isinstance(entry.get("publisher"), str) and bool(entry["publisher"].strip())
                unsigned = name in {"postgresql", "winsw"} and bool(entry.get("unsignedReason", "").strip())
                if ("publisher" in entry) == ("unsignedReason" in entry) or not (signed or unsigned):
                    raise ValueError("Executable requires a trusted publisher or reviewed upstream unsigned exception")
            elif "publisher" in entry or "unsignedReason" in entry:
                raise ValueError("Signature policy only applies to executable content")
            if suffix in {".exe", ".dll"}:
                allowed = {"x64", "anycpu"} if name != "netfx" else {"x86", "x64"}
                if name in {"postgresql", "vc-redist"} or relative in {
                    "eitmad-server.exe", "pwsh.exe", "coreclr.dll",
                    "tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe"}:
                    allowed = {"x64"}
                if entry.get("architecture") not in allowed:
                    raise ValueError("Executable requires a supported reviewed PE architecture")
            elif "architecture" in entry:
                raise ValueError("PE architecture only applies to EXE and DLL content")
        if not component["licenses"]:
            raise ValueError("Component redistribution licenses are required")
        for license in component["licenses"]:
            if set(license) != {"name", "version", "expression", "files"} or \
                    not all(license.get(key) for key in ("name", "version", "expression", "files")) or \
                    not all(file in component["files"] for file in license["files"]):
                raise ValueError("License terms must be included in the reviewed files")
    return review


def check_release_tree(stage: Path, target: str, version: str, *, msix: bool = False) -> dict:
    inventory = read_json(stage / "release.json")
    if inventory.get("schemaVersion") != 1 or inventory.get("target") != target or \
            inventory.get("version") != version or inventory.get("architecture") != "x64" or \
            inventory.get("windowsVersion") != windows_version(version):
        raise ValueError("Eitmad package identity differs from the reviewed release")
    files = tree_files(stage)
    expected = set(inventory["files"]) | {"release.json"}
    # These files are produced by makeappx and signing after release.json is written.
    if msix:
        expected |= {"AppxBlockMap.xml", "[Content_Types].xml", "AppxSignature.p7x"}
        if "AppxMetadata/CodeIntegrity.cat" in files:
            expected.add("AppxMetadata/CodeIntegrity.cat")
    if set(files) != expected:
        raise ValueError("Eitmad package contains missing or unexpected files")
    for relative, digest in inventory["files"].items():
        safe_relative(relative)
        if sha256(files[relative]) != digest:
            raise ValueError("Eitmad package differs from its release inventory")
    return inventory


def check_desktop_payload(stage: Path, version: str) -> dict:
    inventory = check_release_tree(stage, "windows-desktop", version)
    packages = list(stage.glob("*.msix"))
    if len(packages) != 1 or any(name not in {"release.json", "package.psm1", "install-client.ps1", packages[0].name}
                                  and not name.startswith("docs/") for name in inventory["files"]):
        raise ValueError("Client payload may contain only the existing desktop package and its installation tools")
    with tempfile.TemporaryDirectory(prefix="eitmad-msix-inspect-") as temporary:
        unpacked = Path(temporary)
        with zipfile.ZipFile(packages[0]) as archive:
            seen = set()
            for entry in archive.infolist():
                relative = safe_relative(entry.filename.rstrip("/"))
                operational_path(relative)
                if relative.casefold() in seen or stat.S_ISLNK(entry.external_attr >> 16):
                    raise ValueError("Duplicate or linked MSIX payload")
                seen.add(relative.casefold())
                if entry.is_dir():
                    continue
                destination = unpacked / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                with archive.open(entry) as source, destination.open("xb") as output:
                    shutil.copyfileobj(source, output)
        if sha256(unpacked / "release.json") != inventory.get("payloadInventorySha256"):
            raise ValueError("Desktop MSIX inventory differs from the authenticated outer package")
        inner = check_release_tree(unpacked, "windows-desktop", version, msix=True)
        required = CRT_FILES | {"coreclr.dll", "hostfxr.dll", "hostpolicy.dll", "PresentationFramework.dll",
            "Eitmad.WindowsShell.exe", "eitmad-engine-cli.exe", "visual-cpp-runtime.json", "sbom.cdx.json"}
        if not required <= set(inner["files"]):
            raise ValueError("Desktop must retain its self-contained .NET and app-local C++ runtime")
        runtime = read_json(unpacked / "visual-cpp-runtime.json")
        if runtime.get("architecture") != "x64" or runtime.get("deployment") != "app-local" or \
                tuple(int(p) for p in runtime["version"].split(".")) < (14, 40) or \
                not CRT_FILES <= set(runtime["files"]):
            raise ValueError("Unsupported desktop app-local C++ runtime")
        for relative, digest in runtime["files"].items():
            if inner["files"].get(relative) != digest:
                raise ValueError("Desktop C++ runtime differs from its release inventory")
        for relative in inner["files"]:
            path = unpacked / relative
            if path.suffix.lower() in {".exe", ".dll"} and pe_architecture(path) not in {"x64", "anycpu"}:
                raise ValueError("Desktop contains a non-x64 runtime")
            if path.name.lower() in {"eitmad-server.exe", "postgres.exe", "initdb.exe", "pwsh.exe", "winsw.net461.exe"}:
                raise ValueError("Client cannot include server components")
        return {"releaseInventorySha256": sha256(stage / "release.json"),
                "msixInventorySha256": sha256(unpacked / "release.json"), "visualCppRuntime": runtime,
                "dotnetRuntimes": self_contained_runtime(unpacked / "Eitmad.WindowsShell.runtimeconfig.json", desktop=True)}


def self_contained_runtime(path: Path, *, desktop: bool = False) -> list[dict]:
    options = read_json(path)["runtimeOptions"]
    frameworks = options.get("includedFrameworks", [])
    names = {item["name"] for item in frameworks}
    if "framework" in options or "frameworks" in options or "Microsoft.NETCore.App" not in names or \
            (desktop and "Microsoft.WindowsDesktop.App" not in names):
        raise ValueError("Offline .NET tools must retain their self-contained runtimes")
    return frameworks


def validate_offline_tree(root: Path, review: dict, *, prepared: bool = False) -> dict:
    actual = tree_files(root)
    expected = {}
    for name, (directory, _) in OFFLINE_COMPONENTS.items():
        for relative, entry in review["components"][name]["files"].items():
            expected[f"{directory}/{relative}"] = entry
    metadata = {"reviewed-inputs.json", "payload.json", "setup-bundle.json"} if prepared else set()
    if set(actual) != set(expected) | metadata:
        raise ValueError("Offline payload contains missing or unexpected files")
    for relative, entry in expected.items():
        if sha256(actual[relative]) != entry["sha256"]:
            raise ValueError("Offline payload differs from its reviewed digest")
        path = actual[relative]
        if "architecture" in entry and pe_architecture(path) != entry["architecture"]:
            raise ValueError("Payload PE architecture differs from its review")
        if path.suffix.lower() in {".zip", ".7z", ".tar", ".gz", ".msi", ".cab"}:
            raise ValueError("Opaque nested archives are not allowed; review extracted runtime files")
        if path.suffix.lower() == ".msix" and not relative.startswith("client/desktop/"):
            raise ValueError("Only the reviewed desktop MSIX can be an opaque package")
    verify_payload_signatures(root, expected)
    for name, (directory, required) in OFFLINE_COMPONENTS.items():
        component_files = review["components"][name]["files"]
        for relative in required:
            if relative not in component_files and not any(p.startswith(relative + "/") for p in component_files):
                raise ValueError("Required prerequisite content is missing")
    server = root / OFFLINE_COMPONENTS["application"][0]
    check_release_tree(server, "windows-server", review["version"])
    runtimes = {"certificateTool": self_contained_runtime(server / "tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.runtimeconfig.json"),
                "powershell": self_contained_runtime(root / "server/powershell/pwsh.runtimeconfig.json")}
    desktop = check_desktop_payload(root / OFFLINE_COMPONENTS["desktop"][0], review["version"])
    runtimes["setup"] = self_contained_runtime(root / "setup/EitmadSetup.runtimeconfig.json", desktop=True)
    return {"schemaVersion": 1, "version": review["version"], "architecture": "x64",
            "productionEligible": False,
            "roles": {"Client": ["client/desktop"], "Server": ["client/desktop", "server"]},
            "osRequirements": OFFLINE_OS_REQUIREMENTS, "components": review["components"],
            "desktop": desktop, "serverDotnetRuntimes": runtimes,
            "files": {name: entry["sha256"] for name, entry in sorted(expected.items())}}


def prepare_offline_payload(source: Path, output: Path, review_path: Path, expected_review: str, version: str) -> Path:
    """Publish only a complete verified directory; never install or run payload code."""
    windows_version(version)
    source, output, review_path = source.absolute(), output.absolute(), review_path.absolute()
    plain_path(source)
    plain_path(output)
    source, output = source.resolve(), output.resolve()
    if output.exists() or output.is_relative_to(source) or source.is_relative_to(output):
        raise ValueError("Offline output must be a new directory separate from reviewed inputs")
    review = load_payload_review(review_path, expected_review, version)
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="eitmad-offline-", dir=output.parent) as temporary:
        stage = Path(temporary) / "payload"
        # Validate the source first, and validate the independent copy again before publication.
        validate_offline_tree(source, review)
        shutil.copytree(source, stage)
        inventory = validate_offline_tree(stage, review)
        shutil.copyfile(review_path, stage / "reviewed-inputs.json")
        if sha256(stage / "reviewed-inputs.json") != expected_review:
            raise ValueError("Review manifest changed during preparation")
        inventory["reviewManifestSha256"] = expected_review
        (stage / "setup-bundle.json").write_text(json.dumps(setup_bundle_metadata(stage, review), indent=2) + "\n", encoding="utf-8")
        (stage / "payload.json").write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8")
        stage.rename(output)
    return output


def verify_offline_payload(output: Path, expected_review: str, version: str) -> None:
    windows_version(version)
    plain_path(output)
    review = load_payload_review(output / "reviewed-inputs.json", expected_review, version)
    expected = validate_offline_tree(output, review, prepared=True)
    expected["reviewManifestSha256"] = expected_review
    if read_json(output / "payload.json") != expected:
        raise ValueError("Prepared payload inventory differs from the verified review")
    if read_json(output / "setup-bundle.json") != setup_bundle_metadata(output, review):
        raise ValueError("Setup metadata differs from the authenticated payload")


def setup_bundle_metadata(root: Path, review: dict) -> dict:
    """Bind the native launcher's inputs to the independently authenticated review."""
    return {"version": 1,
            "publisher": review["components"]["application"]["files"]["eitmad-server.exe"]["publisher"],
            "desktopReleaseSha256": sha256(root / "client/desktop/release.json"),
            "serverReleaseSha256": sha256(root / "server/application/release.json"),
            "winswSha256": review["components"]["winsw"]["files"]["WinSW.NET461.exe"]["sha256"]}


def run(*command: str, env: dict | None = None) -> None:
    subprocess.run(command, cwd=ROOT, check=True, env=env)


def windows_version(version: str) -> str:
    prerelease = r"(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    match = re.fullmatch(
        rf"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-{prerelease}(?:\.{prerelease})*)?",
        version,
    )
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


def deterministic_zip(source: Path, destination: Path) -> None:
    with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in sorted(item for item in source.rglob("*") if item.is_file()):
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
    for path in sorted(stage.rglob("visual-cpp-runtime.json")):
        runtime = json.loads(path.read_text(encoding="utf-8"))
        component = {"type": "library", "name": "Microsoft Visual C++ Runtime", "version": runtime["version"],
                     "properties": [{"name": "eitmad:deployment", "value": "app-local x64"}]}
        if component not in components:
            components.append(component)
    payload = {"bomFormat": "CycloneDX", "specVersion": "1.6", "version": 1,
               "metadata": {"component": {"type": "application", "name": package, "version": version},
                            "properties": [{"name": "eitmad:inventory-scope", "value": "Resolved Rust normal/build inputs and published .NET libraries; excludes test-only roots. Payload hashes are in release.json."}]},
               "components": components}
    (stage / "sbom.cdx.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def copy_install_tools(stage: Path, target: str) -> None:
    names = ["package.psm1", "install-client.ps1" if target == "windows-desktop" else "install-server.ps1"]
    for name in names:
        shutil.copy2(ROOT / "deploy/windows" / name, stage / name)
    if target == "windows-server":
        shutil.copy2(ROOT / "run-services.ps1", stage)
        tools = stage / "deploy/windows"
        tools.mkdir(parents=True)
        for name in ("host.psm1", "prepare-workshop.ps1", "installation.ps1", "installation.psm1", "installation-channel.cs", "backup.psm1", "backup-worker.ps1", "backup-tools.psm1", "local-backup.ps1", "local-backup.psm1", "server-backup.example.json", "run-backup.ps1", "restore-backup.ps1", "backup.example.json"):
            shutil.copy2(ROOT / "deploy/windows" / name, tools)
    # Preserve the canonical documentation tree so all relative links work offline.
    shutil.copytree(ROOT / "docs", stage / "docs")


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


def find_vc_runtime() -> Path:
    installer = Path(os.environ.get("ProgramFiles(x86)", "C:/Program Files (x86)")) / "Microsoft Visual Studio/Installer/vswhere.exe"
    if not installer.is_file():
        raise ValueError("Use --vc-runtime with the reviewed x64 Microsoft.VC143.CRT redistribution directory")
    installations = json.loads(subprocess.check_output(
        [str(installer), "-all", "-products", "*", "-format", "json", "-utf8"], text=True, encoding="utf-8-sig",
    ))
    candidates = []
    for installation in installations:
        root = Path(installation["installationPath"]) / "VC/Redist/MSVC"
        for path in root.glob("*/x64/Microsoft.VC143.CRT"):
            if re.fullmatch(r"\d+\.\d+\.\d+", path.parts[-3]):
                candidates.append(path)
    if not candidates:
        raise ValueError("Install the Visual Studio x64 CRT redistribution files on the build host, or use --vc-runtime")
    return max(candidates, key=lambda path: tuple(int(part) for part in path.parts[-3].split(".")))


def copy_vc_runtime(stage: Path, source: Path | None) -> None:
    """Use reviewed app-local redistribution files, never DLLs from System32."""
    source = source if source is not None else find_vc_runtime()
    metadata = json.loads(subprocess.check_output(
        ["powershell.exe", "-NoProfile", "-File", str(ROOT / "deploy/windows/verify-vc-runtime.ps1"),
         "-Directory", str(source.absolute())], cwd=ROOT, text=True, encoding="utf-8",
    ))
    for name in metadata["files"]:
        # Refuse accidental replacement of files from a different published runtime.
        target = stage / name
        if target.exists() and sha256(target) != sha256(source / name):
            raise ValueError("Published runtime conflicts with the reviewed Microsoft CRT")
        shutil.copy2(source / name, target)
        if sha256(target) != metadata["hashes"][name]:
            raise ValueError("Copied Microsoft CRT differs from its authenticated input")
    (stage / "visual-cpp-runtime.json").write_text(json.dumps({
        "version": metadata["version"], "architecture": "x64", "deployment": "app-local",
        "files": metadata["hashes"],
    }, indent=2) + "\n", encoding="utf-8")


def build_windows(output: Path, version: str, publisher: str = "CN=Eitmad Unsigned Validation", hook: Path | None = None,
                  vc_runtime: Path | None = None) -> Path:
    if os.name != "nt":
        raise SystemExit("windows-desktop packaging requires Windows")
    plan = load_plan()["artifacts"]["windows_desktop"]
    binaries = build_rust(plan["engine_package"], version)
    with tempfile.TemporaryDirectory(prefix="eitmad-package-") as temp_value:
        stage = Path(temp_value) / "Eitmad"
        publish(plan["shell_project"], stage, version)
        copy_vc_runtime(stage, vc_runtime)
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
        bundle = Path(temp_value) / "installer"
        bundle.mkdir()
        shutil.copy2(package, bundle)
        copy_install_tools(bundle, "windows-desktop")
        sign_path(bundle, publisher, hook)
        release_inventory(bundle, version, "windows-desktop", publisher, sha256(stage / "release.json"))
        destination = output / f"eitmad-desktop-{version}-win-x64-{'payload-signed' if hook else 'unsigned'}.zip"
        deterministic_zip(bundle, destination)
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
        # Keep the existing LAN tool's relative assembly path; it loads into PowerShell 7.4+.
        (stage / "tools").mkdir()
        shutil.copy2(ROOT / "tools/lan-server.ps1", stage / "tools")
        publish("tools/lan-certificate/Eitmad.LanCertificate.csproj",
                stage / "tools/lan-certificate/bin/Release/net8.0-windows", version)
        write_sbom(stage, load_plan()["artifacts"]["windows_server"]["rust_package"], version)
        sign_path(stage, publisher, hook)
        release_inventory(stage, version, "windows-server", publisher)
        destination = output / f"eitmad-server-{version}-win-x64-{'payload-signed' if hook else 'unsigned'}.zip"
        deterministic_zip(stage, destination)
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
    parser.add_argument("target", choices=("windows-desktop", "windows-server", "windows-offline", "server"))
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--publisher", default="CN=Eitmad Unsigned Validation")
    parser.add_argument("--signing-hook", type=Path, help="Reviewed hardware-backed signing script accepting -Path; no private key is packaged")
    parser.add_argument("--vc-runtime", type=Path, help="Reviewed x64 Microsoft.VC143.CRT redistribution directory for offline Windows builds")
    parser.add_argument("--payload-input", type=Path, help="Reviewed extracted offline component tree; never an installed data directory")
    parser.add_argument("--payload-review", type=Path, help="Authenticated offline component review JSON")
    parser.add_argument("--expected-review-sha256", help="Independent authenticated SHA-256 of the component review")
    parser.add_argument("--verify-payload", action="store_true", help="Recheck an existing offline output directory without installing it")
    arguments = parser.parse_args()
    windows_version(arguments.version)
    if arguments.target == "windows-offline":
        if not arguments.expected_review_sha256 or arguments.signing_hook or arguments.vc_runtime:
            parser.error("Offline preparation requires --expected-review-sha256 and prebuilt reviewed packages")
        if arguments.verify_payload:
            if arguments.payload_input or arguments.payload_review:
                parser.error("Verification uses the reviewed-inputs.json already in --output")
            verify_offline_payload(arguments.output, arguments.expected_review_sha256, arguments.version)
        else:
            if not arguments.payload_input or not arguments.payload_review:
                parser.error("Offline preparation requires --payload-input and --payload-review")
            prepare_offline_payload(arguments.payload_input, arguments.output, arguments.payload_review,
                                    arguments.expected_review_sha256, arguments.version)
        print(arguments.output)
        return
    if arguments.payload_input or arguments.payload_review or arguments.expected_review_sha256 or arguments.verify_payload:
        parser.error("Payload review options apply only to windows-offline")
    if arguments.target != "windows-desktop" and arguments.vc_runtime is not None:
        parser.error("--vc-runtime currently applies only to windows-desktop")
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
        options = {"vc_runtime": arguments.vc_runtime} if arguments.target == "windows-desktop" else {}
        artifact = builder(arguments.output.resolve(), arguments.version, arguments.publisher, arguments.signing_hook, **options)
    manifest = write_manifest(arguments.output, artifact, arguments.version, arguments.target)
    verify(artifact, manifest)
    print(artifact)
    print(manifest)


if __name__ == "__main__":
    main()
