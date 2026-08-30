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
    def test_arabic_shell_accepts_language_and_direction_without_visible_locale_copy(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            window = root / "shells" / "windows" / "MainWindow.xaml"
            window.parent.mkdir(parents=True)
            window.write_text(
                '<Window FlowDirection="RightToLeft" Language="ar-YE" />\n',
                encoding="utf-8",
            )
            with patch.object(policy, "ROOT", root):
                errors: list[str] = []
                policy.check_arabic_shell(errors)
            self.assertEqual([], errors)

    def test_arabic_shell_rejects_missing_language_and_direction(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            window = root / "shells" / "windows" / "MainWindow.xaml"
            window.parent.mkdir(parents=True)
            window.write_text("<Window />\n", encoding="utf-8")
            with patch.object(policy, "ROOT", root):
                errors: list[str] = []
                policy.check_arabic_shell(errors)
            self.assertEqual(2, len(errors))
            self.assertTrue(all("Arabic/RTL marker" in error for error in errors))

    def test_text_files_ignores_tracked_paths_deleted_by_the_change(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            existing = root / "current.rs"
            missing = root / "deleted.rs"
            existing.write_text("fn current() {}\n", encoding="utf-8")
            with patch.object(policy, "tracked_files", return_value=[existing, missing]):
                self.assertEqual([existing], policy.text_files())

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

    def test_migration_inventory_rejects_base_changes_and_allows_new_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            existing = root / "server" / "control-plane" / "migrations" / "0001_initial.sql"
            added = root / "server" / "control-plane" / "migrations" / "0002_added.sql"
            existing.parent.mkdir(parents=True)
            existing.write_text("changed\n", encoding="utf-8")
            added.write_text("new\n", encoding="utf-8")
            manifest = root / "deploy" / "migrations.sha256"
            manifest.parent.mkdir()
            manifest.write_text(
                "\n".join(
                    f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.relative_to(root).as_posix()}"
                    for path in (existing, added)
                ) + "\n",
                encoding="utf-8",
            )
            base_path = "server/control-plane/migrations/0001_initial.sql"
            with (
                patch.object(policy, "ROOT", root),
                patch.object(policy, "MIGRATION_MANIFEST", manifest),
                patch.object(policy, "migration_paths_at_revision", return_value=[base_path]),
                patch.object(policy, "migration_bytes_at_revision", return_value=b"original\n"),
            ):
                errors: list[str] = []
                policy.check_migrations(errors, "base")
            self.assertEqual([f"released migration changed or was deleted: {base_path}"], errors)

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

    def test_shell_authority_rejects_config_access_in_each_shell_language(self) -> None:
        cases = {
            "windows/App.cs": 'Environment.GetEnvironmentVariable("SECRET")',
            "linux/main.rs": 'std::env::var("SECRET")',
            "macos/App.swift": "ProcessInfo.processInfo.environment",
        }
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            for relative, content in cases.items():
                source = root / "shells" / relative
                source.parent.mkdir(parents=True)
                source.write_text(content, encoding="utf-8")
            with patch.object(policy, "ROOT", root):
                errors: list[str] = []
                policy.check_shell_authority(errors)
            self.assertEqual(3, len(errors))
            self.assertTrue(all("direct configuration access" in error for error in errors))

    def test_unsafe_logging_rejects_secret_field_without_redaction(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            source = root / "service.rs"
            source.write_text('tracing::info!(token = token, "issued");', encoding="utf-8")
            with patch.object(policy, "ROOT", root), patch.object(policy, "text_files", return_value=[source]):
                errors: list[str] = []
                policy.check_unsafe_logging(errors)
            self.assertEqual(1, len(errors))

    def test_unsafe_logging_rejects_csharp_structured_token(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            source = root / "Service.cs"
            source.write_text('logger.LogInformation("Issued {Token}", token);', encoding="utf-8")
            with patch.object(policy, "ROOT", root), patch.object(policy, "text_files", return_value=[source]):
                errors: list[str] = []
                policy.check_unsafe_logging(errors)
            self.assertEqual(1, len(errors))


if __name__ == "__main__":
    unittest.main()
