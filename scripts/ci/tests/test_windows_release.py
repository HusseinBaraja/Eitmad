"""Protect the full build's authenticated-input and publication boundaries."""
import importlib.util
import json
from pathlib import Path
import shutil
import sys
import unittest
from unittest.mock import patch
import zipfile

import test_offline_payload as offline

RELEASE = Path(__file__).resolve().parents[2] / "release"
with patch.object(sys, "path", [str(RELEASE), *sys.path]):
    SPEC = importlib.util.spec_from_file_location("windows_release", RELEASE / "build_windows.py")
    release = importlib.util.module_from_spec(SPEC)
    SPEC.loader.exec_module(release)


class WindowsReleaseTests(unittest.TestCase):
    def setUp(self):
        self.fixture = offline.OfflinePayloadTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root = self.fixture.root
        self.previous = self.root / "previous"
        self.previous.mkdir()
        shutil.copytree(self.fixture.source, self.previous / "inputs")
        shutil.copy2(self.fixture.review_path, self.previous / "reviewed-inputs.json")
        self.settings = {"previousRelease": self.previous, "previousReviewSha256": self.fixture.digest,
                         "publisher": "CN=Synthetic Test Publisher"}
        self.output = self.root / "new-release"

    def test_authenticated_review_is_required_before_any_copy(self):
        review = self.previous / "reviewed-inputs.json"
        review.write_text(review.read_text() + " ")
        with self.assertRaisesRegex(ValueError, "authenticated SHA-256"):
            release.retained_review(self.settings, "1.2.4", self.output)
        self.assertFalse(self.output.exists())

    def test_existing_outputs_and_reused_windows_versions_are_preserved(self):
        for version in ("1.2.3", "1.2.3-beta", "1.2.2"):
            with self.subTest(version=version), self.assertRaisesRegex(ValueError, "higher"):
                release.retained_review(self.settings, version, self.output)
        self.output.mkdir()
        marker = self.output / "saved-work"
        marker.write_bytes(b"preserve")
        with self.assertRaisesRegex(ValueError, "already exists"):
            release.retained_review(self.settings, "1.2.4", self.output)
        self.assertEqual(marker.read_bytes(), b"preserve")

    def test_retained_runtime_tampering_stops_before_copy(self):
        (self.previous / "inputs/server/postgresql/bin/postgres.exe").write_bytes(b"changed")
        with patch.object(release.build, "verify_payload_signatures"), self.assertRaisesRegex(ValueError, "digest differs"):
            release.check_prerequisites(self.previous, self.fixture.review, self.output)
        self.assertFalse(self.output.exists())

    def test_retained_signature_failure_stops_before_copy(self):
        with patch.object(release.build, "verify_payload_signatures", side_effect=ValueError("untrusted signature")):
            with self.assertRaisesRegex(ValueError, "untrusted signature"):
                release.check_prerequisites(self.previous, self.fixture.review, self.output)
        self.assertFalse(self.output.exists())

    def make_archive(self, identity=None, extra=None):
        archive = self.root / "component.zip"
        identity = identity or {"sourceCommit": "b" * 40, "sourceDirty": False,
                                "version": "1.2.4", "windowsVersion": "1.2.4.0"}
        with zipfile.ZipFile(archive, "w") as package:
            package.writestr("release.json", json.dumps(identity))
            for name in extra or []:
                package.writestr(name, b"synthetic")
        return archive

    def test_archive_escape_and_windows_path_collisions_are_rejected_before_extracting(self):
        for names, message in ((["../outside.exe"], "Unsafe"), (["file.exe", "FILE.exe"], "Duplicate")):
            with self.subTest(names=names), self.assertRaisesRegex(ValueError, message):
                release.extract_component(self.make_archive(extra=names), self.output, "1.2.4", "b" * 40)
            self.assertFalse(self.output.exists())

    def test_component_from_another_revision_is_rejected_before_extracting(self):
        archive = self.make_archive(identity={"sourceCommit": "c" * 40, "sourceDirty": False,
                                            "version": "1.2.4", "windowsVersion": "1.2.4.0"})
        with self.assertRaisesRegex(ValueError, "identity differs"):
            release.extract_component(archive, self.output, "1.2.4", "b" * 40)
        self.assertFalse(self.output.exists())

    def test_failed_compilation_does_not_assemble_or_record_success(self):
        with patch.object(release, "ROOT", self.root), patch.object(release.build, "verify_payload_signatures"), \
                patch.object(release, "run_logged", side_effect=ValueError("compiler failed")) as command:
            with self.assertRaisesRegex(ValueError, "compiler failed"):
                release.build_release("1.2.4", self.settings, self.fixture.review, self.output, "b" * 40)
        self.assertEqual(command.call_count, 1)
        self.assertFalse((self.output / "verification.json").exists())
        self.assertFalse((self.output / "EitmadServerSetup.exe").exists())

    def test_built_components_form_a_valid_authenticated_installer_input(self):
        def signatures(paths):
            return [{"path": "\\\\?\\" + str(path.resolve()), "publisher": self.settings["publisher"]} for path in paths]

        def command(name, arguments, logs):
            if name == "assemble-installers":
                review_path = Path(arguments[arguments.index("--payload-review") + 1])
                pin = arguments[arguments.index("--expected-review-sha256") + 1]
                review = release.build.load_payload_review(review_path, pin, self.fixture.version)
                release.build.validate_offline_tree(self.output / "inputs", review)
                return
            target, component, prefix = next(item for item in release.PRODUCTS if name == "build-" + item[2])
            stage = self.fixture.source / release.build.OFFLINE_COMPONENTS[component][0]
            license_path = stage / ("docs/third-party/license.txt" if component == "desktop" else "licenses/license.txt")
            license_path.parent.mkdir(parents=True, exist_ok=True)
            license_path.write_text("Synthetic redistribution terms.")
            identity = json.loads((stage / "release.json").read_text())
            identity["files"] = {path.relative_to(stage).as_posix(): release.build.sha256(path)
                                 for path in stage.rglob("*") if path.is_file() and path.name != "release.json"}
            (stage / "release.json").write_text(json.dumps(identity))
            archive = self.output / f"eitmad-{prefix}-{self.fixture.version}-win-x64-payload-signed.zip"
            release.build.deterministic_zip(stage, archive)
            release.build.write_manifest(self.output, archive, self.fixture.version, target)

        with patch.object(release, "ROOT", self.root), patch.object(release.build, "verify_payload_signatures"), \
                patch.object(release, "signature_records", side_effect=signatures), \
                patch.object(release, "run_logged", side_effect=command), patch.object(release, "verify_outputs") as final:
            release.build_release(self.fixture.version, self.settings, self.fixture.review, self.output, "b" * 40)
        final.assert_called_once()
        review = json.loads((self.output / "reviewed-inputs.json").read_text())
        self.assertEqual(review["components"]["postgresql"], self.fixture.review["components"]["postgresql"])
