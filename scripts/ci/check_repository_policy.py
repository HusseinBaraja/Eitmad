#!/usr/bin/env python3
"""Fail closed on repository ownership, safety, migration, and shell rules."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import re
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
MIGRATION_MANIFEST = ROOT / "deploy" / "migrations.sha256"
TEXT_SUFFIXES = {".cs", ".json", ".md", ".ps1", ".py", ".rs", ".sql", ".swift", ".toml", ".xaml", ".yml", ".yaml"}
SKIP_PARTS = {".git", "bin", "node_modules", "obj", "target"}


def tracked_files() -> list[Path]:
    output = subprocess.check_output(["git", "ls-files"], cwd=ROOT, text=True)
    return [ROOT / line for line in output.splitlines() if line]


def text_files() -> list[Path]:
    return [
        path
        for path in tracked_files()
        if path.is_file()
        and path.suffix.lower() in TEXT_SUFFIXES
        and not SKIP_PARTS.intersection(path.parts)
    ]


def migration_entries() -> list[tuple[str, str]]:
    entries = []
    for path in sorted(ROOT.glob("server/*/migrations/*.sql")):
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        entries.append((digest, path.relative_to(ROOT).as_posix()))
    return entries


def write_migration_manifest() -> None:
    lines = [
        "# Update with: python scripts/ci/check_repository_policy.py --write-migration-manifest",
        "# Paths use forward slashes and are sorted. Released migration bytes are immutable.",
    ]
    lines.extend(f"{digest}  {path}" for digest, path in migration_entries())
    MIGRATION_MANIFEST.write_text("\n".join(lines) + "\n", encoding="utf-8")


def migration_paths_at_revision(revision: str) -> list[str]:
    output = subprocess.check_output(
        ["git", "ls-tree", "-r", "--name-only", revision, "--", "server"],
        cwd=ROOT,
        text=True,
    )
    return sorted(
        path
        for path in output.splitlines()
        if re.fullmatch(r"server/[^/]+/migrations/[^/]+\.sql", path)
    )


def migration_bytes_at_revision(revision: str, path: str) -> bytes:
    return subprocess.check_output(["git", "show", f"{revision}:{path}"], cwd=ROOT)


def check_migrations(errors: list[str], base: str | None = None) -> None:
    actual = [f"{digest}  {path}" for digest, path in migration_entries()]
    recorded = [line for line in MIGRATION_MANIFEST.read_text(encoding="utf-8").splitlines() if line and not line.startswith("#")]
    if actual != recorded:
        errors.append("server migration inventory or checksum drifted; add an immutable migration and refresh deploy/migrations.sha256")
    numbers = sorted(int(Path(path).name.split("_", 1)[0]) for _, path in migration_entries())
    if numbers != list(range(1, len(numbers) + 1)):
        errors.append("server migration numbers must be globally unique and contiguous")
    if base:
        current = {path: digest for digest, path in migration_entries()}
        for path in migration_paths_at_revision(base):
            digest = hashlib.sha256(migration_bytes_at_revision(base, path)).hexdigest()
            if current.get(path) != digest:
                errors.append(f"released migration changed or was deleted: {path}")


def check_shell_authority(errors: list[str]) -> None:
    forbidden = {
        "Microsoft.Data.Sqlite": "database client",
        "System.Data.SqlClient": "database client",
        "Npgsql": "database client",
        "Environment.GetEnvironmentVariable": "direct configuration access",
        "std::env::var": "direct configuration access",
        "ProcessInfo.processInfo.environment": "direct configuration access",
        "ConfigurationManager": "direct configuration access",
        "IConfiguration": "direct configuration access",
        "PasswordVault": "direct secret access",
        "ProtectedData": "direct secret access",
    }
    for path in (ROOT / "shells").rglob("*"):
        if (
            not path.is_file()
            or path.suffix.lower() not in {".cs", ".swift", ".rs"}
            or "generated" in path.parts
            or "tests" in path.parts
        ):
            continue
        content = path.read_text(encoding="utf-8")
        for token, reason in forbidden.items():
            if token in content:
                errors.append(f"{path.relative_to(ROOT)} contains forbidden {reason}: {token}")


def check_unsafe_logging(errors: list[str]) -> None:
    secret_name = re.compile(r"(?i)(password|private[_-]?key|secret|token|database_url|connection_string)")
    log_call = re.compile(
        r"(?s)(tracing::(?:trace|debug|info|warn|error)!|Console\.WriteLine|"
        r"\bLog(?:Trace|Debug|Information|Warning|Error|Critical))\s*\((.*?)\)"
    )
    for path in text_files():
        if path.suffix.lower() not in {".cs", ".rs"} or "tests" in path.parts:
            continue
        content = path.read_text(encoding="utf-8")
        for match in log_call.finditer(content):
            call = match.group(2)
            if secret_name.search(call) and "[REDACTED]" not in call and "error.code()" not in call:
                line = content.count("\n", 0, match.start()) + 1
                errors.append(f"{path.relative_to(ROOT)}:{line} may log secret-bearing data")


def check_secret_literals(errors: list[str]) -> None:
    assignment = re.compile(r"(?i)(password|private[_-]?key|secret|api[_-]?key)\s*[:=]\s*['\"]([^'\"]{12,})['\"]")
    for path in text_files():
        if path.name in {"Cargo.lock", "package-lock.json"} or "generated" in path.parts:
            continue
        content = path.read_text(encoding="utf-8")
        for match in assignment.finditer(content):
            value = match.group(2)
            if not any(marker in value.lower() for marker in ("example", "synthetic", "redacted", "placeholder")):
                errors.append(f"{path.relative_to(ROOT)} contains a possible committed secret literal")


def check_arabic_shell(errors: list[str]) -> None:
    window = (ROOT / "shells" / "windows" / "MainWindow.xaml").read_text(encoding="utf-8")
    required = ('FlowDirection="RightToLeft"', 'Language="ar-YE"', "العربية (اليمن)")
    for marker in required:
        if marker not in window:
            errors.append(f"Windows shell lost required Arabic/RTL marker: {marker}")


def check_ownership(errors: list[str]) -> None:
    required = [ROOT / "server", ROOT / "shells", ROOT / "platform-adapters", ROOT / "tests", ROOT / "deploy"]
    required.extend(path for path in (ROOT / "crates").iterdir() if path.is_dir())
    for path in required:
        if not (path / "OWNERSHIP.md").is_file():
            errors.append(f"{path.relative_to(ROOT)}/OWNERSHIP.md is missing")


def check_release_plan(errors: list[str]) -> None:
    with (ROOT / "deploy" / "release-plan.toml").open("rb") as stream:
        plan = tomllib.load(stream)
    if plan.get("schema_version") != 1:
        errors.append("release plan schema_version must be 1")
    artifacts = plan.get("artifacts", {})
    for name in ("windows_desktop", "macos_desktop", "linux_desktop", "server"):
        if name not in artifacts:
            errors.append(f"release plan is missing {name}")
    for name in ("macos_desktop", "linux_desktop"):
        if artifacts.get(name, {}).get("enabled") is not False:
            errors.append(f"{name} cannot be enabled before its shell and updater validation exist")


def changed_files(base: str) -> set[str]:
    output = subprocess.check_output(["git", "diff", "--name-only", f"{base}...HEAD"], cwd=ROOT, text=True)
    return set(output.splitlines())


def check_documentation_impact(base: str | None, errors: list[str]) -> None:
    if not base:
        return
    changed = changed_files(base)
    behavior = any(path.startswith(("crates/", "server/", "shells/", "platform-adapters/", "deploy/", ".github/")) for path in changed)
    docs = any(path.startswith("docs/") for path in changed)
    if behavior and not docs:
        errors.append("behavior, delivery, or CI changed without a documentation change")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", help="base commit for documentation impact")
    parser.add_argument("--write-migration-manifest", action="store_true")
    arguments = parser.parse_args()
    if arguments.write_migration_manifest:
        write_migration_manifest()
        return 0
    errors: list[str] = []
    check_migrations(errors, arguments.base)
    check_shell_authority(errors)
    check_unsafe_logging(errors)
    check_secret_literals(errors)
    check_arabic_shell(errors)
    check_ownership(errors)
    check_release_plan(errors)
    check_documentation_impact(arguments.base, errors)
    if errors:
        print("Repository policy failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print("Repository policy passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
