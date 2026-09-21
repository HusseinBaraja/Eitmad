import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

import { missingBindingIdentifiers } from "../contract-checks.mjs";

const repository = resolve(import.meta.dirname, "../../../..");
const registry = JSON.parse(
  readFileSync(resolve(repository, "crates/contracts/generated/protocol-v1.json"), "utf8"),
);
const fixture = JSON.parse(
  readFileSync(resolve(repository, "tests/contract-compatibility/fixtures/protocol-v1.json"), "utf8"),
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

test("platform bindings expose the Rust-exported protocol version", () => {
  const { major, minor } = fixture.query.protocolVersion;
  const csharp = readFileSync(
    resolve(repository, "shells/windows/generated/ProtocolIds.g.cs"),
    "utf8",
  );
  const swift = readFileSync(
    resolve(repository, "shells/macos/generated/ProtocolIds.generated.swift"),
    "utf8",
  );

  assert.match(csharp, new RegExp(`public const long Major = ${major};`));
  assert.match(csharp, new RegExp(`public const long Minor = ${minor};`));
  assert.match(swift, new RegExp(`public static let major = ${major}`));
  assert.match(swift, new RegExp(`public static let minor = ${minor}`));
});
