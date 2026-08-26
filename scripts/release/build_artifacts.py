#!/usr/bin/env python3
"""Build deterministic unsigned validation artifacts from the release plan."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
PLAN = ROOT / "deploy" / "release-plan.toml"
FIXED_ZIP_TIME = (2020, 1, 1, 0, 0, 0)


def run(*command: str) -> None:
    subprocess.run(command, cwd=ROOT, check=True)


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


def build_windows(output: Path, version: str) -> Path:
    if os.name != "nt":
        raise SystemExit("windows-desktop packaging requires Windows")
    plan = load_plan()["artifacts"]["windows_desktop"]
    run("cargo", "build", "--locked", "--release", "-p", plan["engine_package"])
    with tempfile.TemporaryDirectory(prefix="eitmad-package-") as temp_value:
        stage = Path(temp_value) / "Eitmad"
        run(
            "dotnet", "publish", plan["shell_project"], "--configuration", "Release",
            "--runtime", plan["runtime"], "--self-contained", "false",
            f"-p:Version={version}", "--output", str(stage), "--nologo",
        )
        shutil.copy2(ROOT / "target" / "release" / "eitmad-engine-cli.exe", stage)
        destination = output / f"eitmad-desktop-{version}-win-x64-unsigned.zip"
        deterministic_zip(stage, destination)
    return destination


def build_server(output: Path, version: str) -> Path:
    plan = load_plan()["artifacts"]["server"]
    if os.name == "nt":
        raise SystemExit("server packaging requires the declared Linux build host")
    run("cargo", "build", "--locked", "--release", "-p", plan["rust_package"])
    executable = ROOT / "target" / "release" / ("eitmad-server.exe" if os.name == "nt" else "eitmad-server")
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
    parser.add_argument("target", choices=("windows-desktop", "server"))
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    arguments = parser.parse_args()
    arguments.output.mkdir(parents=True, exist_ok=True)
    artifact = (build_windows if arguments.target == "windows-desktop" else build_server)(
        arguments.output.resolve(), arguments.version
    )
    manifest = write_manifest(arguments.output, artifact, arguments.version, arguments.target)
    verify(artifact, manifest)
    print(artifact)
    print(manifest)


if __name__ == "__main__":
    main()
