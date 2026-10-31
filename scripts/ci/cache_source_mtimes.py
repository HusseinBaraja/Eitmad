"""Preserve Cargo cache freshness only for byte-identical tracked source files."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def tracked_files(root: Path):
    """Use current Git paths, never paths supplied by a restored cache."""
    root = root.resolve()
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=root)
    for raw in paths.split(b"\0"):
        if not raw:
            continue
        relative = raw.decode("utf-8")
        path = root / relative
        if (path.is_file() and not path.is_symlink()
                and path.resolve().is_relative_to(root) and path.stat().st_nlink == 1):
            yield relative, path


def save(root: Path, manifest: Path) -> int:
    files = {}
    for relative, path in tracked_files(root):
        files[relative] = {
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "mtime_ns": path.stat().st_mtime_ns,
        }
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(json.dumps({"version": 1, "files": files}), encoding="utf-8")
    return len(files)


def restore(root: Path, manifest: Path) -> tuple[int, int]:
    try:
        data = json.loads(manifest.read_text(encoding="utf-8"))
        if data.get("version") != 1 or not isinstance(data.get("files"), dict):
            return 0, 0
    except (OSError, ValueError, AttributeError):
        return 0, 0
    restored = changed = 0
    for relative, path in tracked_files(root):
        previous = data["files"].get(relative)
        if not isinstance(previous, dict):
            continue
        current = path.stat()
        stamp = previous.get("mtime_ns")
        if (hashlib.sha256(path.read_bytes()).hexdigest() == previous.get("sha256")
                and type(stamp) is int and 0 < stamp <= time.time_ns()):
            os.utime(path, ns=(current.st_atime_ns, stamp))
            restored += 1
        else:
            # A preserved/old checkout timestamp must never hide changed content.
            os.utime(path, ns=(current.st_atime_ns, time.time_ns()))
            changed += 1
    return restored, changed


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("save", "restore"))
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    if args.operation == "save":
        print(f"Recorded source timestamps for {save(root, args.manifest)} tracked files.")
    else:
        restored, changed = restore(root, args.manifest)
        print(f"Restored {restored} unchanged source timestamps; refreshed {changed} changed inputs.")


if __name__ == "__main__":
    main()
