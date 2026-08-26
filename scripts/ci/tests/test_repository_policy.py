from __future__ import annotations

import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

MODULE_PATH = Path(__file__).resolve().parents[1] / "check_repository_policy.py"
SPEC = importlib.util.spec_from_file_location("repository_policy", MODULE_PATH)
assert SPEC and SPEC.loader
policy = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(policy)


class RepositoryPolicyTests(unittest.TestCase):
    def test_migration_inventory_rejects_changed_released_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            migration = root / "server" / "control-plane" / "migrations" / "0001_initial.sql"
            migration.parent.mkdir(parents=True)
            migration.write_text("CREATE TABLE example(id integer);\n", encoding="utf-8")
            manifest = root / "deploy" / "migrations.sha256"
            manifest.parent.mkdir()
            digest = hashlib.sha256(migration.read_bytes()).hexdigest()
            manifest.write_text(f"{digest}  server/control-plane/migrations/0001_initial.sql\n", encoding="utf-8")
            with patch.object(policy, "ROOT", root), patch.object(policy, "MIGRATION_MANIFEST", manifest):
                errors: list[str] = []
                policy.check_migrations(errors)
                self.assertEqual([], errors)
                migration.write_text("CREATE TABLE changed(id integer);\n", encoding="utf-8")
                policy.check_migrations(errors)
                self.assertTrue(any("checksum drifted" in error for error in errors))

    def test_shell_authority_rejects_direct_config_but_ignores_negative_tests(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            source = root / "shells" / "windows" / "App.cs"
            source.parent.mkdir(parents=True)
            source.write_text('Environment.GetEnvironmentVariable("SECRET")', encoding="utf-8")
            negative_test = root / "shells" / "windows" / "tests" / "Program.cs"
            negative_test.parent.mkdir()
            negative_test.write_text('"Environment.GetEnvironmentVariable"', encoding="utf-8")
            with patch.object(policy, "ROOT", root):
                errors: list[str] = []
                policy.check_shell_authority(errors)
            self.assertEqual(1, len(errors))
            self.assertIn("direct configuration access", errors[0])

    def test_unsafe_logging_rejects_secret_field_without_redaction(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            source = root / "service.rs"
            source.write_text('tracing::info!(token = token, "issued");', encoding="utf-8")
            with patch.object(policy, "ROOT", root), patch.object(policy, "text_files", return_value=[source]):
                errors: list[str] = []
                policy.check_unsafe_logging(errors)
            self.assertEqual(1, len(errors))


if __name__ == "__main__":
    unittest.main()
