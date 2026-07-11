from __future__ import annotations

import datetime as dt
import tempfile
import unittest
from pathlib import Path

from audit_docs import check_file


FRONTMATTER = """---
title: "Test page"
description: "A focused test page."
audience: "developer"
page_type: "reference"
status: "active"
owner: "test maintainers"
last_verified: "2026-07-11"
---
"""


class AuditDocsTests(unittest.TestCase):
    def test_accepts_valid_page_and_relative_anchor(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            target = root / "target.md"
            target.write_text(FRONTMATTER + "\n# Target\n\n## Exact section\n", encoding="utf-8")
            source = root / "source.md"
            source.write_text(
                FRONTMATTER + "\n# Source\n\n[Read more](target.md#exact-section)\n",
                encoding="utf-8",
            )

            self.assertEqual(check_file(source, root, dt.date(2026, 7, 11)), [])

    def test_reports_missing_metadata_placeholder_and_broken_link(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            page = root / "bad.md"
            page.write_text(
                "---\ntitle: \"Bad page\"\nstatus: \"active\"\n---\n"
                "\n# Bad page\n\nTODO: finish. [Missing](missing.md)\n",
                encoding="utf-8",
            )

            errors = check_file(page, root, dt.date(2026, 7, 11))

            self.assertTrue(any(error.startswith("missing metadata:") for error in errors))
            self.assertIn("active page contains a placeholder", errors)
            self.assertIn("broken relative link: missing.md", errors)

    def test_reports_duplicate_heading_anchor(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            page = root / "duplicate.md"
            page.write_text(FRONTMATTER + "\n# Page\n\n## Same\n\n## Same\n", encoding="utf-8")

            errors = check_file(page, root, dt.date(2026, 7, 11))

            self.assertIn("duplicate heading anchors: same", errors)


if __name__ == "__main__":
    unittest.main()
