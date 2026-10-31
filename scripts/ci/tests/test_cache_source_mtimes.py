"""Verify that compiler cache metadata cannot hide a changed source input."""

import importlib.util
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

SPEC = importlib.util.spec_from_file_location(
    "cache_source_mtimes", Path(__file__).resolve().parents[1] / "cache_source_mtimes.py"
)
cache = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(cache)


class SourceTimestampTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="eitmad-source-cache-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        self.source = self.root / "source.rs"
        self.source.write_text("fn main() {}", encoding="utf-8")
        subprocess.run(["git", "add", "source.rs"], cwd=self.root, check=True)
        self.manifest = self.root / "cache.json"
        self.original_stamp = time.time_ns() - 10_000_000_000
        os.utime(self.source, ns=(self.original_stamp, self.original_stamp))
        self.original_stamp = self.source.stat().st_mtime_ns
        cache.save(self.root, self.manifest)

    def test_identical_content_recovers_its_timestamp_after_a_fresh_checkout(self):
        self.source.write_text("fn main() {}", encoding="utf-8")
        self.assertEqual(cache.restore(self.root, self.manifest), (1, 0))
        self.assertEqual(self.source.stat().st_mtime_ns, self.original_stamp)

    def test_changed_content_cannot_keep_a_cached_timestamp(self):
        self.source.write_text("fn main() { changed(); }", encoding="utf-8")
        os.utime(self.source, ns=(self.original_stamp, self.original_stamp))
        self.assertEqual(cache.restore(self.root, self.manifest), (0, 1))
        self.assertGreater(self.source.stat().st_mtime_ns, self.original_stamp)

    def test_cache_paths_cannot_modify_untracked_files(self):
        untracked = self.root / "untracked.rs"
        untracked.write_text("saved work", encoding="utf-8")
        stamp = untracked.stat().st_mtime_ns
        data = json.loads(self.manifest.read_text(encoding="utf-8"))
        data["files"]["untracked.rs"] = {
            "sha256": hashlib.sha256(untracked.read_bytes()).hexdigest(),
            "mtime_ns": self.original_stamp,
        }
        self.manifest.write_text(json.dumps(data), encoding="utf-8")
        cache.restore(self.root, self.manifest)
        self.assertEqual(untracked.stat().st_mtime_ns, stamp)

    def test_missing_or_invalid_metadata_leaves_current_source_untouched(self):
        self.manifest.write_text("invalid", encoding="utf-8")
        self.assertEqual(cache.restore(self.root, self.manifest), (0, 0))
        self.assertEqual(self.source.stat().st_mtime_ns, self.original_stamp)
        self.manifest.unlink()
        self.assertEqual(cache.restore(self.root, self.manifest), (0, 0))


if __name__ == "__main__":
    unittest.main()
