import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

import {
  collectEmptyPayloads,
  collectUnions,
  reduceSchema,
  renderCsharpUnions,
  renderSwiftUnions,
} from "../unions.mjs";

const repository = resolve(import.meta.dirname, "../../../..");
const schema = JSON.parse(
  readFileSync(resolve(repository, "crates/contracts/generated/contract-v1.schema.json"), "utf8"),
);
const unions = collectUnions(schema);

test("collects every tagged server envelope as a discriminated union", () => {
  const names = unions.map((union) => union.name);
  for (const expected of [
    "ServerClientMessage",
    "ServerMessage",
    "SyncMessage",
    "IpcClientMessage",
    "IpcServerMessage",
  ]) {
    assert.ok(names.includes(expected), `missing union ${expected}`);
  }
});

test("every ServerClientMessage and ServerMessage kind maps to its own payload type", () => {
  const byName = new Map(unions.map((union) => [union.name, union]));
  const client = byName.get("ServerClientMessage");
  assert.deepEqual(
    client.variants.map((variant) => [variant.kind, variant.type]),
    [
      ["eitmad.server.hello.v1", "ServerConnectionHello"],
      ["eitmad.server.sync.v1", "SyncTransportFrame"],
      ["eitmad.server.subscribe.v1", "ServerSubscriptionRequest"],
      ["eitmad.server.acknowledge.v1", "ServerSubscriptionAcknowledgement"],
    ],
  );
  const server = byName.get("ServerMessage");
  assert.deepEqual(
    server.variants.map((variant) => [variant.kind, variant.type]),
    [
      ["eitmad.server.hello-accepted.v1", "PeerHello"],
      ["eitmad.server.sync-message.v1", "SyncMessage"],
      ["eitmad.server.event.v1", "ServerSubscriptionEvent"],
      ["eitmad.server.failure.v1", "ServerFailure"],
    ],
  );
});

test("csharp bindings expose typed factories and accessors for every variant", () => {
  const source = readFileSync(
    resolve(repository, "shells/windows/generated/EitmadContracts.Unions.g.cs"),
    "utf8",
  );
  for (const union of unions) {
    for (const variant of union.variants) {
      assert.ok(
        source.includes(`public static ${union.name} For${variant.pascal}(${variant.type} payload)`),
        `${union.name}.${variant.pascal} factory missing`,
      );
      assert.ok(
        source.includes(`public ${variant.type}? As${variant.pascal}()`),
        `${union.name}.${variant.pascal} accessor missing`,
      );
      assert.ok(source.includes(`"${variant.kind}"`), `kind ${variant.kind} missing`);
    }
  }
});

test("swift bindings decode every kind into a typed associated value", () => {
  const source = readFileSync(
    resolve(repository, "shells/macos/generated/EitmadContractsUnions.generated.swift"),
    "utf8",
  );
  for (const union of unions) {
    const start = source.indexOf(`public enum ${union.name}: Codable, Sendable {`);
    assert.notEqual(start, -1, `missing swift enum ${union.name}`);
    const end = source.indexOf("\n}\n", start);
    const block = source.slice(start, end);
    for (const variant of union.variants) {
      assert.ok(
        block.includes(`case ${variant.camel}(${variant.type})`),
        `${union.name}.${variant.camel} case missing`,
      );
      assert.ok(block.includes(`"${variant.kind}"`), `kind ${variant.kind} missing`);
      assert.ok(
        block.includes(
          `case .${variant.camel}: self = .${variant.camel}(try container.decode(${variant.type}.self, forKey: .payload))`,
        ),
        `${union.name}.${variant.camel} decoder arm missing`,
      );
    }
  }
});

test("reduced schema keeps every payload type reachable for quicktype", () => {
  const reduced = reduceSchema(schema, unions);
  const keepAlive = JSON.stringify(reduced.properties.unionPayloadKeepAlive);
  for (const union of unions) {
    for (const variant of union.variants) {
      assert.ok(
        keepAlive.includes(`#/definitions/${variant.type}`),
        `${variant.type} unreachable in reduced schema`,
      );
      assert.ok(!reduced.definitions[union.name].oneOf, `${union.name} still collapses`);
    }
  }
});

test("empty-object union payloads still get rendered bindings quicktype would skip", () => {
  const empty = collectEmptyPayloads(schema, unions);
  assert.ok(empty.includes("ConfigurationChanges"), "ConfigurationChanges must be collected");
  assert.ok(!empty.includes("ListScopeRelationships"), "non-empty payloads must be skipped");

  const csharp = renderCsharpUnions(unions, empty);
  const swift = renderSwiftUnions(unions, empty);
  for (const name of empty) {
    assert.ok(csharp.includes(`public partial class ${name}`), `${name} missing from C# shims`);
    assert.ok(
      swift.includes(`public struct ${name}: Codable, Sendable {}`),
      `${name} missing from Swift shims`,
    );
  }

  const generatedCsharp = readFileSync(
    resolve(repository, "shells/windows/generated/EitmadContracts.Unions.g.cs"),
    "utf8",
  );
  const generatedSwift = readFileSync(
    resolve(repository, "shells/macos/generated/EitmadContractsUnions.generated.swift"),
    "utf8",
  );
  for (const name of empty) {
    assert.ok(generatedCsharp.includes(`public partial class ${name}`), `${name} missing in generated C#`);
    assert.ok(
      generatedSwift.includes(`public struct ${name}: Codable, Sendable {}`),
      `${name} missing in generated Swift`,
    );
  }
});
