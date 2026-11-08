"""Build both signed Windows installers from retained authenticated prerequisites."""
from __future__ import annotations

import argparse
import copy
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import zipfile

import build_artifacts as build

ROOT = build.ROOT
PRODUCTS = (("windows-server", "application", "server"),
            ("windows-desktop", "desktop", "desktop"), ("windows-setup", "setup", "setup"))
SETTINGS = ROOT / "artifacts/windows-build/settings.json"


def read_settings(path: Path) -> dict:
    if not path.is_file():
        raise ValueError("Configure artifacts/windows-build/settings.json; see the Windows packaging guide.")
    settings = build.read_json(path)
    required = {"previousRelease", "previousReviewSha256", "certificateThumbprint", "publisher", "compiler"}
    if set(settings) != required or not all(isinstance(value, str) and value.strip() for value in settings.values()):
        raise ValueError("Build settings require previousRelease, previousReviewSha256, certificateThumbprint, publisher, and compiler.")
    if not re.fullmatch(r"[a-fA-F0-9]{40}", settings["certificateThumbprint"]):
        raise ValueError("Invalid signing certificate thumbprint.")
    settings["previousRelease"] = ROOT / settings["previousRelease"]
    settings["compiler"] = ROOT / settings["compiler"]
    return settings


def source_revision() -> str:
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT):
        raise ValueError("Signed builds require a clean committed checkout.")
    return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()


def retained_review(settings: dict, version: str, output: Path) -> dict:
    build.plain_path(output)
    if output.exists():
        raise ValueError(f"Output already exists; no files were replaced: {output}")
    previous = settings["previousRelease"]
    build.plain_path(previous / "reviewed-inputs.json")
    raw = build.read_json(previous / "reviewed-inputs.json")
    old = build.load_payload_review(previous / "reviewed-inputs.json", settings["previousReviewSha256"], raw["version"])
    numeric = lambda value: tuple(map(int, build.windows_version(value).split(".")))
    if numeric(version) <= numeric(old["version"]):
        raise ValueError("Use a Windows version higher than the retained release.")
    return old


def check_prerequisites(previous: Path, review: dict, destination: Path | None = None) -> None:
    for name, (relative, _) in build.OFFLINE_COMPONENTS.items():
        if name in {component for _, component, _ in PRODUCTS}:
            continue
        origin = previous / "inputs" / relative
        files = build.tree_files(origin)
        expected = review["components"][name]["files"]
        if set(files) != set(expected):
            raise ValueError(f"Retained prerequisite file set differs: {name}")
        for relative_file, path in files.items():
            entry = expected[relative_file]
            if build.sha256(path) != entry["sha256"]:
                raise ValueError(f"Retained prerequisite digest differs: {name}/{relative_file}")
            if "architecture" in entry and build.pe_architecture(path) != entry["architecture"]:
                raise ValueError(f"Retained prerequisite architecture differs: {name}/{relative_file}")
        build.verify_payload_signatures(origin, expected)
        if destination is not None:
            shutil.copytree(origin, destination / relative)


def powershell(script: str, payload: object) -> object:
    result = subprocess.check_output(["pwsh.exe", "-NoProfile", "-NonInteractive", "-Command", script],
                                     input=json.dumps(payload), text=True, encoding="utf-8")
    return json.loads(result)


def check_tools(settings: dict) -> None:
    compiler = settings["compiler"]
    tools = [str(compiler), build.sdk_tool("makeappx.exe"), build.sdk_tool("signtool.exe")]
    script = r"""$ErrorActionPreference='Stop'
$settings=[Console]::In.ReadToEnd() | ConvertFrom-Json
if ($PSVersionTable.PSVersion -lt [version]'7.4') { throw 'PowerShell 7.4+ is required.' }
Import-Module $settings.module -Force
$certificate=Get-EitmadSigningCertificate
Assert-EitmadSelfSignedCertificate $certificate $settings.publisher
foreach ($path in $settings.tools) {
    $signature=Get-AuthenticodeSignature -LiteralPath $path
    if ($signature.Status -ne 'Valid') { throw ('Untrusted build tool: '+$path) }
    if ($path -ceq $settings.tools[0] -and $signature.SignerCertificate.Subject -notmatch '(^|, )CN=Pyrsys B[.]V[.](,|$)') {
        throw 'Require the authentic Inno Setup compiler from Pyrsys B.V.'
    }
}
'true'
"""
    powershell(script, {"module": str(ROOT / "deploy/windows/signing-certificate.psm1"),
                        "publisher": settings["publisher"], "tools": tools})
    version = subprocess.check_output([str(compiler), "--version"], text=True).strip()
    if not re.fullmatch(r"7\.\d+\.\d+", version) or tuple(map(int, version.split("."))) < (7, 1, 1):
        raise ValueError("Require Inno Setup 7.1.1+ within major version 7.")
    subprocess.run(["rustup", "which", "--toolchain", "1.85.1", "rustc"], check=True, capture_output=True)
    sdks = subprocess.check_output(["dotnet", "--list-sdks"], text=True)
    if not any(line.startswith("8.") for line in sdks.splitlines()):
        raise ValueError("Install the .NET 8 SDK.")


def run_logged(name: str, command: list, logs: Path) -> None:
    print(f"START: {name}", flush=True)
    log = logs / f"{name}.log"
    with log.open("wb") as stream:
        result = subprocess.run([str(arg) for arg in command], cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
    if result.returncode:
        raise ValueError(f"{name} failed (exit {result.returncode}); see {log}")
    print(f"PASS: {name}", flush=True)


def extract_component(archive: Path, stage: Path, version: str, head: str) -> None:
    with zipfile.ZipFile(archive) as package:
        identity = json.loads(package.read("release.json"))
        if (identity["sourceCommit"], identity["sourceDirty"], identity["version"], identity["windowsVersion"]) != (
                head, False, version, build.windows_version(version)):
            raise ValueError("Component version or source identity differs.")
        names = [build.safe_relative(item.filename) for item in package.infolist()]
        if len({name.casefold() for name in names}) != len(names):
            raise ValueError("Duplicate Windows archive path.")
        package.extractall(stage)


def signature_records(paths: list[Path]) -> list[dict]:
    script = r"""$ErrorActionPreference='Stop'
$paths=[Console]::In.ReadToEnd() | ConvertFrom-Json
$rows=@(foreach ($path in $paths) {
    $signature=Get-AuthenticodeSignature -LiteralPath $path
    if ($signature.Status -ne 'Valid') { throw ('Untrusted built file: '+$path) }
    [pscustomobject]@{path=$path; publisher=$signature.SignerCertificate.Subject;
        thumbprint=$signature.SignerCertificate.Thumbprint;
        fileVersion=([Diagnostics.FileVersionInfo]::GetVersionInfo($path).FileVersion)}
})
ConvertTo-Json -InputObject $rows -Compress
"""
    # The script SIP needs extended paths for deeply nested package inputs.
    return powershell(script, ["\\\\?\\" + str(path.resolve()) for path in paths])


def component_review(stage: Path, archive: Path, component: str, version: str) -> dict:
    files = build.tree_files(stage)
    signatures = signature_records([path for path in files.values() if path.suffix.lower() in build.EXECUTABLE_SUFFIXES])
    publishers = {Path(item["path"]): item["publisher"] for item in signatures}
    inventory = {}
    for relative, path in sorted(files.items()):
        entry = {"sha256": build.sha256(path)}
        if path.suffix.lower() in {".exe", ".dll"}:
            entry["architecture"] = build.pe_architecture(path)
        if path.suffix.lower() in build.EXECUTABLE_SUFFIXES:
            entry["publisher"] = publishers[Path("\\\\?\\" + str(path.resolve()))]
        inventory[relative] = entry
    terms = sorted(name for name in files if name.startswith(("licenses/", "docs/third-party/")))
    if not terms:
        raise ValueError(f"Redistribution terms are absent: {component}")
    return {"version": version, "provenance": {"url": "https://github.com/HusseinBaraja/Eitmad", "sha256": build.sha256(archive)},
            "licenses": [{"name": component + " redistributed inputs", "version": version,
                          "expression": "LicenseRef-Included-Third-Party-Terms", "files": terms}], "files": inventory}


def verify_outputs(output: Path, version: str, head: str, settings: dict, review_pin: str) -> None:
    records = []
    artifacts = sorted([*output.glob("*.zip"), *output.glob("*Setup.exe")])
    expected = {f"eitmad-{prefix}-{version}-win-x64-payload-signed.zip" for _, _, prefix in PRODUCTS}
    expected |= {"EitmadServerSetup.exe", "EitmadClientSetup.exe"}
    if {artifact.name for artifact in artifacts} != expected:
        raise ValueError("Expected three component archives and two setup EXEs.")
    for artifact in artifacts:
        manifest = Path(str(artifact) + ".manifest.json")
        build.verify(artifact, manifest)
        if build.read_json(manifest)["version"] != version:
            raise ValueError("Final artifact version differs.")
        records.append({"file": artifact.name, "bytes": artifact.stat().st_size, "sha256": build.sha256(artifact)})
    for target, component, _ in PRODUCTS:
        stage = output / "inputs" / build.OFFLINE_COMPONENTS[component][0]
        identity = build.check_release_tree(stage, target, version)
        if identity["sourceCommit"] != head or identity["sourceDirty"] is not False:
            raise ValueError("Final component source identity differs.")
    desktop = output / "inputs/client/desktop"
    build.check_desktop_payload(desktop, version)
    signatures = signature_records([output / "EitmadServerSetup.exe", output / "EitmadClientSetup.exe",
                                   output / "inputs/setup/EitmadSetup.exe", next(desktop.glob("*.msix")),
                                   output / "inputs/server/application/eitmad-server.exe"])
    for item in signatures:
        if (item["publisher"] != settings["publisher"] or
                item["thumbprint"].upper() != settings["certificateThumbprint"].upper()):
            raise ValueError("Final publisher or signing certificate differs.")
    if any((item["fileVersion"] or "").strip() != build.windows_version(version) for item in signatures[:3]):
        raise ValueError("Final setup EXE version differs.")
    if source_revision() != head:
        raise ValueError("Source changed during the build.")
    result = {"version": version, "windowsVersion": build.windows_version(version), "sourceCommit": head,
              "sourceDirty": False, "artifacts": records, "signatures": signatures,
              "reviewSha256": review_pin, "productionEligible": False,
              "cleanHostAcceptance": "Not performed by the build command."}
    (output / "verification.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    (output / "SHA256SUMS.txt").write_text("".join(f"{item['sha256']}  {item['file']}\n" for item in records), encoding="utf-8")
    (output / "BUILD-RECORD.md").write_text(
        f"# Windows build {build.windows_version(version)}\n\nSource: `{head}` (clean checkout).\n\n"
        "Use `EitmadServerSetup.exe` for Server and `EitmadClientSetup.exe` for Client.\n\n"
        "All component packages and both installers passed version, source, signature, and digest checks.\n"
        "See `verification.json`, `input-review-provenance.json`, and `SHA256SUMS.txt`.\n\n"
        "No application was installed. Mandatory source validation and clean-host release acceptance remain\n"
        "separate gates; signing does not grant production acceptance.\n", encoding="utf-8")


def build_release(version: str, settings: dict, old: dict, output: Path, head: str) -> None:
    logs = ROOT / "artifacts/windows-build" / build.windows_version(version)
    logs.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True)
    inputs = output / "inputs"
    check_prerequisites(settings["previousRelease"], old, inputs)
    review = copy.deepcopy(old)
    review["version"] = version
    common = ["--version", version, "--publisher", settings["publisher"], "--signing-hook",
              ROOT / "deploy/windows/sign-with-self-signed-certificate.ps1", "--output", output]
    for target, component, prefix in PRODUCTS:
        run_logged("build-" + prefix, [sys.executable, ROOT / "scripts/release/build_artifacts.py", target, *common], logs)
        archive = output / f"eitmad-{prefix}-{version}-win-x64-payload-signed.zip"
        build.verify(archive, Path(str(archive) + ".manifest.json"))
        stage = inputs / build.OFFLINE_COMPONENTS[component][0]
        extract_component(archive, stage, version, head)
        review["components"][component] = component_review(stage, archive, component, version)
    review_path = output / "reviewed-inputs.json"
    review_path.write_text(json.dumps(review, indent=2) + "\n", encoding="utf-8")
    pin = build.sha256(review_path)
    build.load_payload_review(review_path, pin, version)
    (output / "input-review-provenance.json").write_text(json.dumps({
        "retainedReviewSha256": settings["previousReviewSha256"], "retainedVersion": old["version"],
        "sourceRevision": head, "newReviewSha256": pin,
        "prerequisites": "Retained distribution bytes matched the authenticated review; no installed runtime or data was used."
    }, indent=2) + "\n", encoding="utf-8")
    run_logged("assemble-installers", [sys.executable, ROOT / "scripts/release/build_artifacts.py", "windows-installers",
               *common, "--payload-input", inputs, "--payload-review", review_path, "--expected-review-sha256", pin], logs)
    verify_outputs(output, version, head, settings, pin)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--settings", type=Path, default=SETTINGS)
    parser.add_argument("--check", action="store_true", help="Check inputs and tools without compiling or writing outputs")
    args = parser.parse_args()
    try:
        windows_version = build.windows_version(args.version)
        if os.name != "nt":
            raise ValueError("This command requires Windows.")
        settings = read_settings(args.settings)
        os.environ.update(EITMAD_SIGNING_CERTIFICATE_THUMBPRINT=settings["certificateThumbprint"],
                          EITMAD_INNO_COMPILER=str(settings["compiler"]), CARGO_INCREMENTAL="0")
        head = source_revision()
        output = ROOT / "dist" / windows_version
        old = retained_review(settings, args.version, output)
        check_tools(settings)
        check_prerequisites(settings["previousRelease"], old)
        if shutil.disk_usage(ROOT).free < 10 * 1024 ** 3:
            raise ValueError("Keep at least 10 GiB free for the Windows build; a fresh compiler cache can require more.")
        print(f"PASS: preflight for {args.version}; source {head}", flush=True)
        if not args.check:
            build_release(args.version, settings, old, output, head)
            print(f"PASS: both signed installers and build records: {output}", flush=True)
    except (ValueError, OSError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        parser.exit(1, f"Build stopped: {error}\n")


if __name__ == "__main__":
    main()
