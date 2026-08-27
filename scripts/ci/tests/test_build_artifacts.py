from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

MODULE_PATH = Path(__file__).resolve().parents[2] / "release" / "build_artifacts.py"
SPEC = importlib.util.spec_from_file_location("build_artifacts", MODULE_PATH)
assert SPEC and SPEC.loader
artifacts = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(artifacts)


class BuildArtifactsTests(unittest.TestCase):
    def test_windows_zip_excludes_debug_symbols(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            root = Path(temp_value)
            source = root / "stage"
            source.mkdir()
            (source / "Eitmad.WindowsShell.dll").write_bytes(b"runtime")
            (source / "Eitmad.WindowsShell.pdb").write_bytes(b"debug paths")
            destination = root / "bundle.zip"

            artifacts.deterministic_zip(source, destination, frozenset({".pdb"}))

            with zipfile.ZipFile(destination) as archive:
                self.assertEqual(archive.namelist(), ["Eitmad.WindowsShell.dll"])

    def test_server_package_rejects_non_linux_host_before_build(self) -> None:
        with tempfile.TemporaryDirectory() as temp_value:
            with patch.object(artifacts.sys, "platform", "darwin"), patch.object(artifacts, "run") as run:
                with self.assertRaisesRegex(SystemExit, "requires the declared Linux build host"):
                    artifacts.build_server(Path(temp_value), "0.0.0-test")
            run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
