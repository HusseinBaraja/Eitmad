from __future__ import annotations

from copy import deepcopy
import importlib.util
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

SPEC = importlib.util.spec_from_file_location("offline_artifacts", Path(__file__).resolve().parents[2] / "release/build_artifacts.py")
assert SPEC and SPEC.loader
artifacts = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(artifacts)


def pe(machine: int = 0x8664, *, managed_flags: int | None = None) -> bytes:
    value = bytearray(1024)
    value[:2] = b"MZ"
    struct.pack_into("<I", value, 60, 128)
    value[128:132] = b"PE\0\0"
    struct.pack_into("<HH", value, 132, machine, 1)
    struct.pack_into("<H", value, 148, 224)
    struct.pack_into("<H", value, 152, 0x20b if machine == 0x8664 else 0x10b)
    if managed_flags is not None:
        struct.pack_into("<I", value, 244, 16)
        struct.pack_into("<II", value, 360, 0x2000, 72)
        struct.pack_into("<IIII", value, 384, 128, 0x2000, 128, 512)
        struct.pack_into("<I", value, 528, managed_flags)
    return bytes(value)


class OfflinePayloadTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source = self.root / "inputs"
        self.output = self.root / "verified"
        self.review_path = self.root / "review.json"
        self.version = "1.2.3"
        self.review = {"schemaVersion": 1, "version": self.version, "architecture": "x64", "components": {}}
        versions = {"setup": self.version, "desktop": self.version, "application": self.version, "postgresql": "17.6",
                    "winsw": "2.12.0", "powershell": "7.4.13", "vc-redist": "14.44.35211", "netfx": "4.8"}
        for name, (directory, required) in artifacts.OFFLINE_COMPONENTS.items():
            stage = self.source / directory
            stage.mkdir(parents=True)
            for relative in required:
                if relative == "release.json":
                    continue
                if relative == "lib":
                    relative = "lib/plugin.dll"
                path = stage / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(pe() if path.suffix in {".exe", ".dll"} else b"synthetic input")
                if path.name.endswith(".runtimeconfig.json"):
                    path.write_text(json.dumps({"runtimeOptions": {"includedFrameworks": [
                        {"name": "Microsoft.NETCore.App", "version": "8.0.0"},
                        *([{"name": "Microsoft.WindowsDesktop.App", "version": "8.0.0"}] if name == "setup" else [])]}}))
            (stage / "LICENSE.txt").write_text("Synthetic redistribution terms only.")
            if name in {"setup", "application"}:
                (stage / "sbom.cdx.json").write_text(json.dumps({"bomFormat": "CycloneDX", "metadata": {"component": {"version": self.version}}, "components": []}))
            if name == "desktop":
                self.make_desktop(stage)
            elif name == "application":
                self.release(stage, "windows-server")
            elif name == "setup":
                self.release(stage, "windows-setup")
            files = {}
            for path in sorted(p for p in stage.rglob("*") if p.is_file()):
                entry = {"sha256": artifacts.sha256(path)}
                if path.suffix in artifacts.EXECUTABLE_SUFFIXES:
                    entry["publisher"] = "CN=Synthetic Test Publisher"
                if path.suffix in {".exe", ".dll"}:
                    entry["architecture"] = "x64"
                files[path.relative_to(stage).as_posix()] = entry
            self.review["components"][name] = {
                "version": versions[name], "provenance": {"url": f"https://example.invalid/{name}/{versions[name]}", "sha256": "a" * 64},
                "licenses": [{"name": name, "version": versions[name], "expression": "LicenseRef-Synthetic", "files": ["LICENSE.txt"]}],
                "files": files}
        self.save_review()

    def release(self, stage: Path, target: str, **extra: object) -> None:
        value = {"schemaVersion": 1, "version": self.version, "windowsVersion": "1.2.3.0", "architecture": "x64",
                 "publisher": "CN=Synthetic Test Publisher",
                 "sourceCommit": "b" * 40, "sourceDirty": False,
                 "target": target, "files": {p.relative_to(stage).as_posix(): artifacts.sha256(p)
                                               for p in stage.rglob("*") if p.is_file() and p.name != "release.json"}, **extra}
        (stage / "release.json").write_text(json.dumps(value))

    def make_desktop(self, stage: Path) -> None:
        inner = self.root / "msix"
        inner.mkdir()
        required = artifacts.CRT_FILES | {"coreclr.dll", "hostfxr.dll", "hostpolicy.dll", "PresentationFramework.dll",
                                         "Eitmad.WindowsShell.exe", "eitmad-engine-cli.exe"}
        for name in required:
            (inner / name).write_bytes(pe())
        runtime = {"version": "14.44.35211.0", "architecture": "x64", "deployment": "app-local",
                   "files": {name: artifacts.sha256(inner / name) for name in artifacts.CRT_FILES}}
        (inner / "visual-cpp-runtime.json").write_text(json.dumps(runtime))
        (inner / "sbom.cdx.json").write_text('{"bomFormat":"CycloneDX"}')
        (inner / "AppxManifest.xml").write_text('<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"><Identity Name="Eitmad.Desktop" Publisher="CN=Synthetic Test Publisher" Version="1.2.3.0" ProcessorArchitecture="x64"/></Package>')
        (inner / "Eitmad.WindowsShell.runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {"includedFrameworks": [
            {"name": "Microsoft.NETCore.App", "version": "8.0.0"}, {"name": "Microsoft.WindowsDesktop.App", "version": "8.0.0"}]}}))
        self.release(inner, "windows-desktop")
        for name in ("AppxBlockMap.xml", "[Content_Types].xml", "AppxSignature.p7x"):
            (inner / name).write_bytes(b"synthetic packaging metadata")
        artifacts.deterministic_zip(inner, stage / "desktop.msix")
        # A real desktop bundle has only its MSIX, two scripts, and docs.
        (stage / "LICENSE.txt").unlink()
        (stage / "docs").mkdir()
        (stage / "docs/LICENSE.txt").write_text("Synthetic terms.")
        self.release(stage, "windows-desktop", payloadInventorySha256=artifacts.sha256(inner / "release.json"))

    def save_review(self) -> None:
        # Desktop license lives in the existing documentation tree.
        self.review["components"]["desktop"]["licenses"][0]["files"] = ["docs/LICENSE.txt"]
        self.review_path.write_text(json.dumps(self.review))
        self.digest = artifacts.sha256(self.review_path)

    def prepare(self) -> Path:
        return artifacts.prepare_offline_payload(self.source, self.output, self.review_path, self.digest, self.version)

    def test_complete_payload_is_separate_reusable_and_reverified(self) -> None:
        # Synthetic PE files have no real signatures. Mock only the OS trust boundary.
        with patch.object(artifacts, "verify_payload_signatures") as signatures:
            self.assertEqual(self.prepare(), self.output)
            artifacts.verify_offline_payload(self.output, self.digest, self.version)
            self.assertEqual(signatures.call_count, 3)
        inventory = json.loads((self.output / "payload.json").read_text())
        self.assertEqual(inventory["roles"]["Client"], ["client/desktop"])
        self.assertTrue(all(not p.startswith("server/") for p in artifacts.tree_files(self.output / "client")))
        self.assertEqual(inventory["components"], self.review["components"])
        self.assertEqual(inventory["desktop"]["visualCppRuntime"]["version"], "14.44.35211.0")
        for relative, source in artifacts.tree_files(self.source).items():
            self.assertEqual(artifacts.sha256(source), artifacts.sha256(self.output / relative))
        with self.assertRaisesRegex(ValueError, "new directory"):
            self.prepare()

    def test_missing_tampered_and_unexpected_inputs_never_publish(self) -> None:
        path = self.source / "server/postgresql/bin/postgres.exe"
        original = path.read_bytes()
        for operation in ("missing", "tampered", "unexpected"):
            with self.subTest(operation=operation), patch.object(artifacts, "verify_payload_signatures"):
                path.write_bytes(original)
                extra = self.source / "server/unreviewed.exe"
                if operation == "missing":
                    path.unlink()
                elif operation == "tampered":
                    path.write_bytes(original + b"tampered")
                else:
                    extra.write_bytes(pe())
                with self.assertRaisesRegex(ValueError, "missing or unexpected|reviewed digest"):
                    self.prepare()
                self.assertFalse(self.output.exists())
                if extra.exists():
                    extra.unlink()

    def test_operational_state_is_rejected_even_when_reviewed(self) -> None:
        for name in ("customer.db", "cluster/PG_VERSION", "credentials.json", "leaf.pem", "desktop-device.json", "ca.cer"):
            with self.subTest(name=name):
                component = self.review["components"]["postgresql"]
                component["files"][name] = {"sha256": "a" * 64}
                self.save_review()
                with self.assertRaisesRegex(ValueError, "Operational state"):
                    self.prepare()
                self.assertFalse(self.output.exists())
                del component["files"][name]

    def test_review_pin_and_component_and_license_requirements_fail_closed(self) -> None:
        original = deepcopy(self.review)
        mutations = [lambda r: r["components"].pop("netfx"),
                     lambda r: r["components"]["winsw"].update(version="3.0.0"),
                     lambda r: r["components"]["postgresql"].update(version="16.9"),
                     lambda r: r["components"]["powershell"].update(version="7.3.0"),
                     lambda r: r["components"]["vc-redist"].update(licenses=[]),
                     lambda r: r["components"]["netfx"]["licenses"][0].update(files=["absent.txt"])]
        for mutate in mutations:
            self.review = deepcopy(original)
            mutate(self.review)
            self.save_review()
            with self.subTest(review=self.review), self.assertRaises(ValueError):
                self.prepare()
        self.review = original
        self.save_review()
        self.review_path.write_text(self.review_path.read_text() + " ")
        with self.assertRaisesRegex(ValueError, "authenticated SHA-256"):
            self.prepare()

    def test_prepared_directory_detects_tampered_metadata_missing_and_extra_files(self) -> None:
        with patch.object(artifacts, "verify_payload_signatures"):
            self.prepare()
            marker = self.output / "payload.json"
            original = marker.read_bytes()
            marker.write_text("{}")
            with self.assertRaisesRegex(ValueError, "inventory differs"):
                artifacts.verify_offline_payload(self.output, self.digest, self.version)
            marker.write_bytes(original)
            path = self.output / "server/winsw/WinSW.NET461.exe"
            path.write_bytes(b"altered")
            with self.assertRaisesRegex(ValueError, "reviewed digest"):
                artifacts.verify_offline_payload(self.output, self.digest, self.version)
            path.unlink()
            with self.assertRaisesRegex(ValueError, "missing or unexpected"):
                artifacts.verify_offline_payload(self.output, self.digest, self.version)

    def test_pe_architecture_reads_x64_anycpu_and_rejects_other_machines(self) -> None:
        path = self.root / "binary.exe"
        for contents, expected in ((pe(), "x64"), (pe(0x14c), "x86"), (pe(0x14c, managed_flags=1), "anycpu"),
                                   (pe(0x14c, managed_flags=3), "x86"), (pe(0x14c, managed_flags=0x20001), "x86")):
            path.write_bytes(contents)
            self.assertEqual(artifacts.pe_architecture(path), expected)
        for contents in (pe(0xaa64), b"not a PE", pe()[:70]):
            path.write_bytes(contents)
            with self.assertRaises(ValueError):
                artifacts.pe_architecture(path)
        path = self.source / "server/postgresql/bin/postgres.exe"
        path.write_bytes(pe(0x14c))
        self.review["components"]["postgresql"]["files"]["bin/postgres.exe"]["sha256"] = artifacts.sha256(path)
        self.save_review()
        with self.assertRaisesRegex(ValueError, "architecture differs"):
            self.prepare()

    def test_unsafe_paths_and_hard_links_are_refused(self) -> None:
        for name in ("../escape", "/root", "C:/root", "a\\b", "a:stream", "dir/CON.txt", "dir/a."):
            with self.subTest(name=name), self.assertRaises(ValueError):
                artifacts.safe_relative(name)
        # Hard links work on Windows without developer-mode symlink privileges.
        original = self.source / "server/postgresql/bin/postgres.exe"
        linked = self.source / "server/postgresql/bin/linked.exe"
        os.link(original, linked)
        with self.assertRaisesRegex(ValueError, "hard links"):
            artifacts.tree_files(self.source)

    def test_inner_msix_tampering_and_unsafe_entries_are_refused(self) -> None:
        package = self.source / "client/desktop/desktop.msix"
        for name in ("../escaped", "customer.db", "coreclr.dll"):
            with self.subTest(name=name):
                with zipfile.ZipFile(package) as archive:
                    entries = {entry.filename: archive.read(entry) for entry in archive.infolist()}
                entries[name] = b"unreviewed"
                with zipfile.ZipFile(package, "w") as archive:
                    for relative, contents in entries.items():
                        archive.writestr(relative, contents)
                self.release(package.parent, "windows-desktop", payloadInventorySha256=artifacts.sha256(self.root / "msix/release.json"))
                with self.assertRaises(ValueError):
                    artifacts.check_desktop_payload(package.parent, self.version)
                self.assertFalse((self.root / "escaped").exists())
                artifacts.deterministic_zip(self.root / "msix", package)

    def test_review_cannot_omit_required_postgresql_backup_tools(self) -> None:
        relative = "bin/pg_verifybackup.exe"
        (self.source / "server/postgresql" / relative).unlink()
        del self.review["components"]["postgresql"]["files"][relative]
        self.save_review()
        with patch.object(artifacts, "verify_payload_signatures"), self.assertRaisesRegex(ValueError, "Required prerequisite"):
            self.prepare()

    def test_reviewed_hashes_cannot_change_msix_identity_or_mix_source_revisions(self) -> None:
        inner = self.root / "msix"
        manifest = inner / "AppxManifest.xml"
        original = manifest.read_text()
        manifest.write_text(original.replace('Name="Eitmad.Desktop"', 'Name="Other.Desktop"'))
        self.release(inner, "windows-desktop")
        desktop = self.source / "client/desktop"
        artifacts.deterministic_zip(inner, desktop / "desktop.msix")
        self.release(desktop, "windows-desktop", payloadInventorySha256=artifacts.sha256(inner / "release.json"))
        with self.assertRaisesRegex(ValueError, "MSIX identity"):
            artifacts.check_desktop_payload(desktop, self.version)
        release = self.source / "server/application/release.json"
        value = json.loads(release.read_text())
        value["sourceCommit"] = "c" * 40
        release.write_text(json.dumps(value))
        self.review["components"]["application"]["files"]["release.json"]["sha256"] = artifacts.sha256(release)
        # Re-review the changed desktop bytes as well. Identity is checked separately above.
        for relative in ("release.json", "desktop.msix"):
            self.review["components"]["desktop"]["files"][relative]["sha256"] = artifacts.sha256(desktop / relative)
        self.save_review()
        with patch.object(artifacts, "verify_payload_signatures"), self.assertRaisesRegex(ValueError, "clean source revision"):
            self.prepare()

    def test_framework_dependent_tool_is_refused_even_with_reviewed_hash(self) -> None:
        relative = "pwsh.runtimeconfig.json"
        path = self.source / "server/powershell" / relative
        path.write_text(json.dumps({"runtimeOptions": {"framework": {"name": "Microsoft.NETCore.App", "version": "8.0.0"}}}))
        self.review["components"]["powershell"]["files"][relative]["sha256"] = artifacts.sha256(path)
        self.save_review()
        with patch.object(artifacts, "verify_payload_signatures"), self.assertRaisesRegex(ValueError, "self-contained runtimes"):
            self.prepare()

    def test_client_refuses_external_package_dependencies_and_server_tools(self) -> None:
        inner = self.root / "msix"
        desktop = self.source / "client/desktop"
        manifest = inner / "AppxManifest.xml"
        original = manifest.read_text()
        # Rebuild valid inventories: these are policy failures, not hash failures.
        for dependency in ("PackageDependency", "MainPackageDependency"):
            with self.subTest(dependency=dependency):
                manifest.write_text(original.replace("</Package>",
                    f'<Dependencies><{dependency} Name="External.Runtime" Publisher="CN=Vendor"/></Dependencies></Package>'))
                self.release(inner, "windows-desktop")
                artifacts.deterministic_zip(inner, desktop / "desktop.msix")
                self.release(desktop, "windows-desktop", payloadInventorySha256=artifacts.sha256(inner / "release.json"))
                with self.assertRaisesRegex(ValueError, "external package dependencies"):
                    artifacts.check_desktop_payload(desktop, self.version)
        manifest.write_text(original)
        for tool in ("pg_dump.exe", "pg_restore.exe", "pg_ctl.exe", "pg_resetwal.exe", "host-service.exe", "Eitmad.LanCertificate.exe"):
            with self.subTest(tool=tool):
                (inner / tool).write_bytes(pe())
                self.release(inner, "windows-desktop")
                artifacts.deterministic_zip(inner, desktop / "desktop.msix")
                self.release(desktop, "windows-desktop", payloadInventorySha256=artifacts.sha256(inner / "release.json"))
                with self.assertRaisesRegex(ValueError, "server components"):
                    artifacts.check_desktop_payload(desktop, self.version)
                (inner / tool).unlink()

    def test_untrusted_signature_cannot_publish_and_unsigned_exception_is_scoped(self) -> None:
        with patch.object(artifacts, "verify_payload_signatures", side_effect=ValueError("Untrusted payload signature")):
            with self.assertRaisesRegex(ValueError, "Untrusted"):
                self.prepare()
        self.assertFalse(self.output.exists())
        entry = self.review["components"]["vc-redist"]["files"]["vc_redist.x64.exe"]
        entry["unsignedReason"] = "Do not accept two trust policies"
        self.save_review()
        with self.assertRaisesRegex(ValueError, "trusted publisher"):
            self.prepare()

        entry.pop("publisher")
        entry["unsignedReason"] = "Not signed upstream"
        self.save_review()
        with self.assertRaisesRegex(ValueError, "trusted publisher"):
            self.prepare()

    def test_offline_archive_binds_metadata_sboms_and_repeatable_entries(self) -> None:
        def catalog(stage: Path, publisher: str, hook: Path | None) -> None:
            (stage / "offline.cat").write_bytes(b"Synthetic catalog: not signed or installable")
        output = self.root / "archives"
        with patch.object(artifacts, "verify_payload_signatures"), \
                patch.object(artifacts, "create_offline_catalog", side_effect=catalog), \
                patch.object(artifacts, "os", wraps=os) as host:
            host.name = "posix"
            archive = artifacts.build_windows_offline(output, self.version, self.source, self.review_path, self.digest,
                                                       "CN=Synthetic Test Publisher", None)
        with zipfile.ZipFile(archive) as package:
            self.assertIn("setup/EitmadSetup.exe", package.namelist())
            self.assertIn("server/postgresql/bin/pg_verifybackup.exe", package.namelist())
            inventory = json.loads(package.read("release.json"))
            for relative, digest in inventory["files"].items():
                self.assertEqual(artifacts.hashlib.sha256(package.read(relative)).hexdigest(), digest)
            self.assertIn("reviewed-inputs.json", inventory["files"])
            self.assertIn("setup-bundle.json", inventory["files"])
            self.assertIn("sbom.cdx.json", inventory["files"])
            self.assertTrue(all(entry.date_time == artifacts.FIXED_ZIP_TIME for entry in package.infolist()))
            unpacked = self.root / "unpacked"
            package.extractall(unpacked)  # Only the test's own generated archive.
        artifacts.deterministic_zip(unpacked, self.root / "repeat.zip")
        self.assertEqual(archive.read_bytes(), (self.root / "repeat.zip").read_bytes())
        with patch.object(artifacts, "verify_payload_signatures"), patch.object(artifacts, "verify_offline_catalog"):
            artifacts.verify_offline_payload(unpacked, self.digest, self.version)
            (unpacked / "setup-bundle.json").write_text('{}')
            with self.assertRaisesRegex(ValueError, "Setup metadata"):
                artifacts.verify_offline_payload(unpacked, self.digest, self.version)

    def test_archive_failure_does_not_publish_and_development_tools_are_rejected(self) -> None:
        output = self.root / "archives"
        with patch.object(artifacts, "verify_payload_signatures"), \
                patch.object(artifacts, "create_offline_catalog", side_effect=ValueError("Signing unavailable")):
            with self.assertRaisesRegex(ValueError, "Signing unavailable"):
                artifacts.build_windows_offline(output, self.version, self.source, self.review_path, self.digest,
                                                "CN=Synthetic Test Publisher", None)
        self.assertEqual(list(output.iterdir()), [])
        for name in ("sdk/compiler.dll", "tools/rustc.exe", "tools/MSBuild.exe"):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "Development toolchains"):
                artifacts.operational_path(name)

    @unittest.skipUnless(os.name == "nt", "Windows file catalog boundary")
    def test_windows_catalog_rejects_tampering_missing_payload_and_unsigned_catalog(self) -> None:
        stage = self.root / "catalog-input"
        stage.mkdir()
        content = stage / "synthetic.txt"
        content.write_bytes(b"synthetic payload")
        artifacts.create_offline_catalog(stage, "CN=Synthetic", None)
        # Mock only signature approval; Windows computes and compares the real catalog hashes.
        script = (artifacts.ROOT / "deploy/windows/verify-offline.ps1").read_text()
        script = script.replace("Microsoft.PowerShell.Security\\Get-AuthenticodeSignature", "Get-SyntheticSignature")
        script = "function Get-SyntheticSignature { param($LiteralPath) [pscustomobject]@{Status='Valid'; TimeStamperCertificate='synthetic'; SignerCertificate=[pscustomobject]@{Subject='CN=Synthetic'}} };\n" + script
        def check() -> subprocess.CompletedProcess:
            return subprocess.run(["pwsh.exe", "-NoProfile", "-NonInteractive", "-Command", script],
                                  input=json.dumps({"root": str(stage), "anchor": str(content), "publisher": "CN=Synthetic"}),
                                  text=True, capture_output=True)
        self.assertEqual(check().returncode, 0)
        content.write_bytes(b"tampered payload")
        self.assertNotEqual(check().returncode, 0)
        content.unlink()
        self.assertNotEqual(check().returncode, 0)
        content.write_bytes(b"synthetic payload")
        (stage / "unexpected.dll").write_bytes(pe())
        self.assertNotEqual(check().returncode, 0)
        with self.assertRaises(subprocess.CalledProcessError):
            artifacts.verify_offline_catalog(stage, "CN=Synthetic")
    @unittest.skipUnless(os.name == "nt", "Windows Authenticode boundary")
    def test_real_windows_signature_check_refuses_unsigned_synthetic_pe(self) -> None:
        with self.assertRaises(subprocess.CalledProcessError) as failure:
            artifacts.verify_payload_signatures(self.source / "server/winsw", {
                "WinSW.NET461.exe": {"publisher": "CN=Synthetic Test Publisher"}})
        self.assertIn("Untrusted payload signature or publisher", failure.exception.stderr)


if __name__ == "__main__":
    unittest.main()
