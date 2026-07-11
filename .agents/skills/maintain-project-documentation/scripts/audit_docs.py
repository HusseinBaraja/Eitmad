#!/usr/bin/env python3
"""Audit migrated Markdown documentation without third-party dependencies."""

from __future__ import annotations

import argparse
import datetime as dt
import re
import sys
from pathlib import Path
from urllib.parse import unquote


ALLOWED = {
    "audience": {"user", "developer", "operations", "api", "architecture", "support"},
    "page_type": {"tutorial", "task", "reference", "explanation", "troubleshooting", "decision", "release"},
    "status": {"draft", "active", "deprecated", "historical"},
}
REQUIRED = {"title", "description", "audience", "page_type", "status", "owner", "last_verified"}
LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
HEADING_RE = re.compile(r"^#{1,6}\s+(.+?)\s*$", re.MULTILINE)
PLACEHOLDER_RE = re.compile(r"\b(?:TODO|TBD|FIXME)\b|\{[^}\n]+\}", re.IGNORECASE)


def parse_frontmatter(text: str) -> tuple[dict[str, str], str | None]:
    if not text.startswith("---\n"):
        return {}, "missing YAML frontmatter"
    end = text.find("\n---\n", 4)
    if end == -1:
        return {}, "unterminated YAML frontmatter"
    data: dict[str, str] = {}
    for raw in text[4:end].splitlines():
        if not raw or raw[0].isspace() or raw.lstrip().startswith("-"):
            continue
        if ":" not in raw:
            continue
        key, value = raw.split(":", 1)
        data[key.strip()] = value.strip().strip('"\'')
    return data, None


def slug(value: str) -> str:
    value = re.sub(r"[`*_~]", "", value).strip().lower()
    value = re.sub(r"[^\w\-\s\u0600-\u06ff]", "", value)
    return re.sub(r"[\s_]+", "-", value).strip("-")


def check_file(path: Path, root: Path, today: dt.date) -> list[str]:
    errors: list[str] = []
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return ["must be valid UTF-8"]

    meta, frontmatter_error = parse_frontmatter(text)
    if frontmatter_error:
        errors.append(frontmatter_error)
    else:
        missing = sorted(REQUIRED - meta.keys())
        if missing:
            errors.append("missing metadata: " + ", ".join(missing))
        for key, values in ALLOWED.items():
            if key in meta and meta[key] not in values:
                errors.append(f"invalid {key}: {meta[key]}")
        if "last_verified" in meta:
            try:
                verified = dt.date.fromisoformat(meta["last_verified"])
                if verified > today:
                    errors.append("last_verified is in the future")
            except ValueError:
                errors.append("last_verified must use YYYY-MM-DD")
        if meta.get("status") == "active" and PLACEHOLDER_RE.search(text):
            errors.append("active page contains a placeholder")

    headings = [slug(item) for item in HEADING_RE.findall(text)]
    duplicates = sorted({item for item in headings if item and headings.count(item) > 1})
    if duplicates:
        errors.append("duplicate heading anchors: " + ", ".join(duplicates))

    for raw_target in LINK_RE.findall(text):
        target = raw_target.strip().strip("<>").split(maxsplit=1)[0]
        if target.startswith(("http://", "https://", "mailto:", "#")):
            continue
        path_part, _, anchor = unquote(target).partition("#")
        linked = (path.parent / path_part).resolve() if path_part else path
        try:
            linked.relative_to(root.resolve())
        except ValueError:
            errors.append(f"relative link escapes docs root: {target}")
            continue
        if not linked.exists():
            errors.append(f"broken relative link: {target}")
            continue
        if anchor and linked.suffix.lower() == ".md":
            linked_text = linked.read_text(encoding="utf-8")
            anchors = {slug(item) for item in HEADING_RE.findall(linked_text)}
            if anchor.lower() not in anchors:
                errors.append(f"broken heading anchor: {target}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("docs"))
    parser.add_argument("--files", nargs="*", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    files = args.files or sorted(args.root.rglob("*.md"))
    failures = 0
    checked = 0
    for candidate in files:
        path = candidate.resolve()
        if not path.exists():
            print(f"ERROR {candidate}: file does not exist")
            failures += 1
            continue
        checked += 1
        for problem in check_file(path, root, dt.date.today()):
            print(f"ERROR {path.relative_to(Path.cwd())}: {problem}")
            failures += 1
    if failures:
        print(f"Documentation audit failed: {failures} problem(s) in {checked} file(s).")
        return 1
    print(f"Documentation audit passed: {checked} file(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
