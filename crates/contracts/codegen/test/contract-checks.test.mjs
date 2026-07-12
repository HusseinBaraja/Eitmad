import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import { findGeneratedDrift, missingBindingIdentifiers } from "../contract-checks.mjs";

test("reports missing and stale generated outputs", () => {
  const root = mkdtempSync(join(tmpdir(), "eitmad-contract-check-"));
  const generated = join(root, "generated");
  const repository = join(root, "repository");
  try {
    write(join(generated, "schema.json"), "authority\n");
    write(join(generated, "binding.cs"), "authority\n");
    write(join(repository, "binding.cs"), "stale\n");

    assert.deepEqual(
      findGeneratedDrift(repository, generated, [
        ["schema.json", "schema.json"],
        ["binding.cs", "binding.cs"],
      ]),
      ["schema.json is missing", "binding.cs differs from Rust authority"],
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("accepts content that differs only by line endings or trailing whitespace", () => {
  const root = mkdtempSync(join(tmpdir(), "eitmad-contract-check-"));
  const generated = join(root, "generated");
  const repository = join(root, "repository");
  try {
    write(join(generated, "binding.swift"), "line 1\nline 2\n");
    write(join(repository, "binding.swift"), "line 1  \r\nline 2\r\n");

    assert.deepEqual(
      findGeneratedDrift(repository, generated, [["binding.swift", "binding.swift"]]),
      [],
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("reports Rust registry identifiers absent from a platform binding", () => {
  const registry = { commands: ["eitmad.command.one.v1"], capabilities: ["eitmad.cap.one.v1"] };

  assert.deepEqual(missingBindingIdentifiers(registry, '"eitmad.command.one.v1"'), [
    "eitmad.cap.one.v1",
  ]);
});

function write(path, content) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content);
}
