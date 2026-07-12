import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

import { missingBindingIdentifiers } from "../contract-checks.mjs";

const repository = resolve(import.meta.dirname, "../../../..");
const registry = JSON.parse(
  readFileSync(resolve(repository, "crates/contracts/generated/protocol-v1.json"), "utf8"),
);

for (const [platform, relativePath] of [
  ["C#", "shells/windows/generated/ProtocolIds.g.cs"],
  ["Swift", "shells/macos/generated/ProtocolIds.generated.swift"],
]) {
  test(`${platform} binding contains every Rust-exported protocol identifier`, () => {
    const source = readFileSync(resolve(repository, relativePath), "utf8");
    assert.deepEqual(missingBindingIdentifiers(registry, source), []);
  });
}
