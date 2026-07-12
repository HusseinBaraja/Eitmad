import { spawnSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { findGeneratedDrift, normalize } from "./contract-checks.mjs";

const mode = process.argv[2];
if (mode !== "generate" && mode !== "check") {
  throw new Error("usage: node generate.mjs <generate|check>");
}

const codegenDirectory = dirname(fileURLToPath(import.meta.url));
const repository = resolve(codegenDirectory, "../../..");
const temporary = mkdtempSync(join(tmpdir(), "eitmad-contracts-"));

try {
  run("cargo", [
    "run",
    "-q",
    "-p",
    "eitmad-contracts",
    "--bin",
    "export_contracts",
    "--",
    temporary,
  ]);

  const schema = join(temporary, "contract-v1.schema.json");
  const csharp = join(temporary, "EitmadContracts.g.cs");
  const swift = join(temporary, "EitmadContracts.generated.swift");

  runQuicktype([
    "--src-lang",
    "schema",
    "--lang",
    "cs",
    "--framework",
    "SystemTextJson",
    "--namespace",
    "Eitmad.Contracts",
    "--csharp-version",
    "6",
    "--features",
    "complete",
    "--check-required",
    "--top-level",
    "EitmadContractSchema",
    "--out",
    csharp,
    schema,
  ]);
  runQuicktype([
    "--src-lang",
    "schema",
    "--lang",
    "swift",
    "--access-level",
    "public",
    "--swift-5-support",
    "--sendable",
    "--top-level",
    "EitmadContractSchema",
    "--out",
    swift,
    schema,
  ]);

  prependGeneratedHeader(csharp, "// Generated from Rust contracts. Do not edit.\n");
  prependGeneratedHeader(swift, "// Generated from Rust contracts. Do not edit.\n");

  const registry = JSON.parse(readFileSync(join(temporary, "protocol-v1.json"), "utf8"));
  writeFileSync(join(temporary, "ProtocolIds.g.cs"), renderCsharpIds(registry));
  writeFileSync(join(temporary, "ProtocolIds.generated.swift"), renderSwiftIds(registry));

  const outputs = new Map([
    ["contract-v1.schema.json", "crates/contracts/generated/contract-v1.schema.json"],
    ["protocol-v1.json", "crates/contracts/generated/protocol-v1.json"],
    ["protocol-v1.fixture.json", "tests/contract-compatibility/fixtures/protocol-v1.json"],
    ["contracts-v1.md", "docs/_generated/contracts-v1.md"],
    ["EitmadContracts.g.cs", "shells/windows/generated/EitmadContracts.g.cs"],
    ["ProtocolIds.g.cs", "shells/windows/generated/ProtocolIds.g.cs"],
    ["EitmadContracts.generated.swift", "shells/macos/generated/EitmadContracts.generated.swift"],
    ["ProtocolIds.generated.swift", "shells/macos/generated/ProtocolIds.generated.swift"],
  ]);

  for (const [sourceName, targetName] of outputs) {
    const generated = normalize(readFileSync(join(temporary, sourceName), "utf8"));
    const target = join(repository, targetName);
    if (mode === "generate") {
      mkdirSync(dirname(target), { recursive: true });
      writeFileSync(target, generated);
    }
  }

  checkForHandwrittenProtocolStrings();
  const drift = mode === "check" ? findGeneratedDrift(repository, temporary, outputs) : [];
  if (drift.length > 0) {
    throw new Error(`contract drift detected:\n- ${drift.join("\n- ")}`);
  }
} finally {
  rmSync(temporary, { recursive: true, force: true });
}

function runQuicktype(arguments_) {
  const executable = join(codegenDirectory, "node_modules", "quicktype", "dist", "index.js");
  run(process.execPath, [executable, "--telemetry", "disable", ...arguments_]);
}

function run(command, arguments_) {
  const result = spawnSync(command, arguments_, {
    cwd: repository,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} failed (${result.status ?? "no exit code"})\n${result.error ?? ""}\n${result.stdout ?? ""}${result.stderr ?? ""}`,
    );
  }
}

function prependGeneratedHeader(path, header) {
  const content = normalize(readFileSync(path, "utf8"));
  writeFileSync(path, `${header}${content}`);
}

function protocolGroups(registry) {
  return [
    ["Commands", registry.commands],
    ["Queries", registry.queries],
    ["Subscriptions", registry.subscriptions],
    ["Events", registry.events],
    ["SyncMessages", registry.syncMessages],
    ["Capabilities", registry.capabilities],
    ["Permissions", registry.permissions],
    ["ConfigKeys", registry.configKeys],
    ["SchemaIds", registry.schemaIds],
    ["ErrorCodes", registry.errorCodes],
    ["MessageIds", registry.messageIds],
    ["ErrorParameterNames", registry.errorParameterNames],
  ];
}

function constantName(identifier) {
  const words = identifier.split(/[^A-Za-z0-9]+/).filter(Boolean);
  const value = words.map((word) => word[0].toUpperCase() + word.slice(1)).join("");
  return /^\d/.test(value) ? `Value${value}` : value;
}

function swiftConstantName(identifier) {
  const pascal = constantName(identifier);
  return pascal[0].toLowerCase() + pascal.slice(1);
}

function renderCsharpIds(registry) {
  const lines = [
    "// Generated from Rust contracts. Do not edit.",
    "using System;",
    "using System.Linq;",
    "",
    "namespace Eitmad.Contracts;",
    "",
    "public static class ProtocolIds",
    "{",
  ];
  for (const [group, identifiers] of protocolGroups(registry)) {
    lines.push(`    public static class ${group}`, "    {");
    for (const identifier of identifiers) {
      lines.push(`        public const string ${constantName(identifier)} = \"${identifier}\";`);
    }
    lines.push("    }", "");
  }
  lines.push(
    "}",
    "",
    "public readonly record struct OpenProtocolId(string Value)",
    "{",
    "    public static bool TryParse(string value, out OpenProtocolId identifier)",
    "    {",
    "        var valid = value is { Length: >= 3 and <= 128 }",
    "            && char.IsAsciiLetterLower(value[0])",
    "            && (char.IsAsciiLetterLower(value[^1]) || char.IsAsciiDigit(value[^1]))",
    "            && value.All(character => char.IsAsciiLetterLower(character)",
    "                || char.IsAsciiDigit(character) || character is '.' or '-' or '_')",
    "            && !value.Contains(\"..\") && !value.Contains(\"--\") && !value.Contains(\"__\");",
    "        identifier = valid ? new OpenProtocolId(value) : default;",
    "        return valid;",
    "    }",
    "}",
  );
  return `${lines.join("\n")}\n`;
}

function renderSwiftIds(registry) {
  const lines = [
    "// Generated from Rust contracts. Do not edit.",
    "import Foundation",
    "",
    "public enum ProtocolIds {",
  ];
  for (const [group, identifiers] of protocolGroups(registry)) {
    lines.push(`    public enum ${group} {`);
    for (const identifier of identifiers) {
      lines.push(`        public static let ${swiftConstantName(identifier)} = \"${identifier}\"`);
    }
    lines.push("    }", "");
  }
  lines.push(
    "}",
    "",
    "public struct OpenProtocolId: RawRepresentable, Codable, Hashable, Sendable {",
    "    public let rawValue: String",
    "",
    "    public init?(rawValue: String) {",
    "        let allowed = CharacterSet(charactersIn: \"abcdefghijklmnopqrstuvwxyz0123456789.-_\")",
    "        guard (3...128).contains(rawValue.utf8.count),",
    "              rawValue.unicodeScalars.allSatisfy(allowed.contains),",
    "              rawValue.first?.isLowercase == true,",
    "              rawValue.last?.isLetter == true || rawValue.last?.isNumber == true,",
    "              !rawValue.contains(\"..\"), !rawValue.contains(\"--\"), !rawValue.contains(\"__\")",
    "        else { return nil }",
    "        self.rawValue = rawValue",
    "    }",
    "}",
  );
  return `${lines.join("\n")}\n`;
}

function checkForHandwrittenProtocolStrings() {
  const violations = [];
  for (const relative of ["shells/windows", "shells/macos", "shells/linux"]) {
    const directory = join(repository, relative);
    walk(directory, (path) => {
      if (!/\.(cs|swift|cpp|cc|hpp|h)$/.test(path) || path.includes(`${join("generated", "")}`)) {
        return;
      }
      if (/eitmad\.[a-z0-9]/.test(readFileSync(path, "utf8"))) {
        violations.push(path.slice(repository.length + 1));
      }
    });
  }
  if (violations.length > 0) {
    throw new Error(`handwritten protocol strings found:\n- ${violations.join("\n- ")}`);
  }
}

function walk(directory, visit) {
  let entries;
  try {
    entries = readdirSync(directory);
  } catch {
    return;
  }
  for (const entry of entries) {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) {
      walk(path, visit);
    } else {
      visit(path);
    }
  }
}
