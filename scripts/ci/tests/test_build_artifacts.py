from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import unquote, urlsplit

MODULE_PATH = Path(__file__).resolve().parents[2] / "release" / "build_artifacts.py"
SPEC = importlib.util.spec_from_file_location("build_artifacts", MODULE_PATH)
assert SPEC and SPEC.loader
artifacts = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(artifacts)

AUDIT_PATH = artifacts.ROOT / ".agents/skills/maintain-project-documentation/scripts/audit_docs.py"
AUDIT_SPEC = importlib.util.spec_from_file_location("audit_docs", AUDIT_PATH)
assert AUDIT_SPEC and AUDIT_SPEC.loader
docs_audit = importlib.util.module_from_spec(AUDIT_SPEC)
AUDIT_SPEC.loader.exec_module(docs_audit)


class BuildArtifactsTests(unittest.TestCase):
    def test_server_tools_have_one_complete_location(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            stage = Path(temporary)
            artifacts.copy_install_tools(stage, "windows-server")
            for name in ("host.psm1", "backup.psm1", "backup-worker.ps1", "run-backup.ps1", "restore-backup.ps1"):
                with self.subTest(name=name):
                    self.assertEqual(list(stage.rglob(name)), [stage / "deploy/windows" / name])
            self.assertTrue((stage / "run-services.ps1").is_file())
            self.assertTrue((stage / "install-server.ps1").is_file())
            self.assertTrue((stage / "package.psm1").is_file())

    def test_windows_packages_deliver_resolvable_inventoried_documentation(self) -> None:
        manuals = ("package-windows-desktop.md", "run-windows-host-services.md", "setup-lan-server.md",
                   "run-server-authority.md", "backup-lan-installation.md")
        for target in ("windows-desktop", "windows-server"):
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temporary:
                stage = Path(temporary).resolve()
                artifacts.copy_install_tools(stage, target)
                for name in manuals:
                    self.assertTrue((stage / "docs/operations" / name).is_file(), name)
                with patch.object(artifacts.subprocess, "check_output", side_effect=["abc123", b""]):
                    artifacts.release_inventory(stage, "1.2.3", target, "CN=Test")
                inventory = json.loads((stage / "release.json").read_text(encoding="utf-8"))["files"]
                for page in sorted(stage.rglob("*.md")):
                    relative = page.relative_to(stage).as_posix()
                    self.assertEqual(inventory[relative], artifacts.sha256(page), relative)
                    for raw_target in docs_audit.LINK_RE.findall(page.read_text(encoding="utf-8")):
                        link = raw_target.strip().strip("<>").split(maxsplit=1)[0]
                        url = urlsplit(link)
                        if url.scheme or url.netloc:
                            continue
                        linked = (page.parent / unquote(url.path)).resolve() if url.path else page
                        with self.subTest(page=relative, link=link):
                            self.assertTrue(linked.is_relative_to(stage), "Link escapes the package")
                            self.assertTrue(linked.is_file(), "Link target is absent from the package")
                            if url.fragment:
                                self.assertEqual(linked.suffix.lower(), ".md")
                                anchors = {docs_audit.slug(heading) for heading in docs_audit.HEADING_RE.findall(
                                    linked.read_text(encoding="utf-8"))}
                                self.assertIn(unquote(url.fragment).lower(), anchors)

    def test_rust_sbom_includes_transitive_build_inputs_but_not_test_only_roots(self) -> None:
        metadata = {
            "packages": [{"id": name, "name": name, "version": "1.0.0"} for name in ("engine", "runtime", "compiler", "leaf", "test-only", "other")],
            "resolve": {"nodes": [
                {"id": "engine", "deps": [{"pkg": "runtime", "dep_kinds": [{"kind": None}]},
                                             {"pkg": "compiler", "dep_kinds": [{"kind": "build"}]},
                                             {"pkg": "test-only", "dep_kinds": [{"kind": "dev"}]}]},
                {"id": "runtime", "deps": [{"pkg": "leaf", "dep_kinds": [{"kind": None}]}]},
                *[{"id": name, "deps": []} for name in ("compiler", "leaf", "test-only", "other")],
            ]},
        }
        self.assertEqual({item["name"] for item in artifacts.rust_components(metadata, "engine")},
                         {"engine", "runtime", "compiler", "leaf"})

    def test_release_version_cannot_escape_output_or_overflow_msix_identity(self) -> None:
        for value in ("../escape", "1.2.3/other", "65536.1.0", "01.2.3", "1.2", "1.2.3\n",
                      "1.2.3-..", "1.2.3-01", "1.2.3-rc..1"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                artifacts.windows_version(value)
        self.assertEqual(artifacts.windows_version("1.2.3-rc.1"), "1.2.3.0")

    def test_inventory_detects_payload_tampering_and_records_migration_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "server/control/migrations").mkdir(parents=True)
            migration = root / "server/control/migrations/0001.sql"
            migration.write_bytes(b"immutable migration")
            (root / "deploy").mkdir()
            (root / "deploy/migrations.sha256").write_text(f"{artifacts.sha256(migration)}  server/control/migrations/0001.sql\n")
            (root / "crates/contracts/src").mkdir(parents=True)
            (root / "crates/contracts/src/transport.rs").write_text("PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion { major: 1, minor: 28, };")
            stage = root / "stage"
            stage.mkdir()
            (stage / "eitmad-server.exe").write_bytes(b"server")
            with patch.object(artifacts, "ROOT", root), patch.object(artifacts.subprocess, "check_output", side_effect=["abc123", b""]):
                artifacts.release_inventory(stage, "1.2.3", "windows-server", "CN=Test")
            inventory = json.loads((stage / "release.json").read_text())
            self.assertEqual(inventory["files"], {"eitmad-server.exe": artifacts.sha256(stage / "eitmad-server.exe")})
            self.assertEqual(inventory["migrations"]["server/control/migrations/0001.sql"], artifacts.sha256(migration))
            migration.write_bytes(b"changed migration")
            with patch.object(artifacts, "ROOT", root), self.assertRaisesRegex(ValueError, "Migration inventory differs"):
                artifacts.release_inventory(stage, "1.2.3", "windows-server", "CN=Test")

    def test_sorted_zip_bytes_are_repeatable_after_source_timestamps_change(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stage = root / "stage"
            stage.mkdir()
            (stage / "z.exe").write_bytes(b"runtime")
            (stage / "a.json").write_bytes(b"{}")
            artifacts.deterministic_zip(stage, root / "one.zip")
            (stage / "z.exe").touch()
            artifacts.deterministic_zip(stage, root / "two.zip")
            self.assertEqual((root / "one.zip").read_bytes(), (root / "two.zip").read_bytes())

    def test_inventory_refuses_operational_state_before_packaging(self) -> None:
        for name in ("customer.db", "server-key.pem", "signing.pfx", "credentials.json", "connection.json", "app.pdb"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                stage = Path(temporary)
                (stage / name).write_bytes(b"must not ship")
                with self.assertRaisesRegex(ValueError, "cannot enter an installation package"):
                    artifacts.release_inventory(stage, "1.0.0", "windows-server", "CN=Validation")

    def test_server_package_rejects_non_linux_host_before_build(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            with patch.object(artifacts.sys, "platform", "darwin"), patch.object(artifacts, "run") as run:
                with self.assertRaisesRegex(SystemExit, "requires the declared Linux build host"):
                    artifacts.build_server(Path(temp_value), "0.0.0-test")
            run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
