import assert from "node:assert/strict";

export function collectUnions(schema) {
  const definitions = schemaDefinitions(schema);
  const unions = [];
  for (const [name, definition] of Object.entries(definitions)) {
    if (!Array.isArray(definition.oneOf) || definition.oneOf.length === 0) {
      continue;
    }
    const branches = definition.oneOf;
    const isTaggedEnvelope = branches.every(
      (branch) =>
        branch &&
        typeof branch === "object" &&
        typeof branch.properties?.kind?.const === "string" &&
        typeof branch.properties?.payload?.$ref === "string",
    );
    if (!isTaggedEnvelope) {
      continue;
    }
    unions.push(unionFromBranches(name, branches, definitions));
  }
  return unions;
}

export function reduceSchema(schema, unions) {
  const reduced = structuredClone(schema);
  const definitions = schemaDefinitions(reduced);
  const payloadReferences = {};
  for (const union of unions) {
    definitions[union.name] = {
      type: "object",
      description: `Discriminated union rendered in platform-specific bindings; every kind maps to a distinct typed payload.`,
    };
    for (const variant of union.variants) {
      payloadReferences[union.name + "_" + variant.pascal] = { $ref: "#/definitions/" + variant.type };
    }
  }
  reduced.properties = {
    ...(reduced.properties ?? {}),
    unionPayloadKeepAlive: { type: "object", properties: payloadReferences },
  };
  reduced.required = [
    ...new Set([...(reduced.required ?? []), "unionPayloadKeepAlive"]),
  ];
  return reduced;
}

export function collectEmptyPayloads(schema, unions) {
  const definitions = schemaDefinitions(schema);
  const names = new Set();
  for (const union of unions) {
    for (const variant of union.variants) {
      const definition = definitions[variant.type];
      const isEmptyObject =
        definition &&
        definition.type === "object" &&
        !definition.properties &&
        !definition.oneOf &&
        !definition.anyOf &&
        !definition.allOf &&
        !definition.$ref &&
        !definition.enum;
      if (isEmptyObject) {
        names.add(variant.type);
      }
    }
  }
  return [...names].sort();
}

function unionFromBranches(name, branches, definitions) {
  const variants = branches.map((branch) => variantFromBranch(branch, definitions));
  const labels = new Set(variants.map((variant) => variant.label));
  assert.equal(
    labels.size,
    variants.length,
    `union ${name} has ambiguous kind-derived member names`,
  );
  for (const variant of variants) {
    assert.ok(variant.kind.length > 0, `union ${name} declares an empty kind`);
  }
  return { name, variants };
}

function variantFromBranch(branch, definitions) {
  const kind = branch.properties.kind.const;
  const reference = branch.properties.payload.$ref;
  const match = /#\/definitions\/([A-Za-z0-9_]+)$/.exec(reference);
  assert.notEqual(match, null, `union branch references unexpected payload ${reference}`);
  const type = match[1];
  assert.ok(type in definitions, `union payload ${type} is not defined`);
  const label = kind.replace(/^eitmad\./, "").replace(/\.v\d+$/, "");
  return { kind, type, label, pascal: pascalCase(label), camel: camelCase(label) };
}

export function renderCsharpUnions(unions, emptyPayloads = []) {
  const lines = [
    "// Generated from Rust contracts. Do not edit.",
    "#nullable enable",
    "using System.Text.Json;",
    "using System.Text.Json.Serialization;",
    "",
    "namespace Eitmad.Contracts;",
  ];
  for (const union of unions) {
    lines.push("", renderCsharpUnion(union));
  }
  for (const name of emptyPayloads) {
    lines.push("", `public partial class ${name}`, "{", "}");
  }
  return `${lines.join("\n")}\n`;
}

function renderCsharpUnion(union) {
  const lines = [
    `public partial class ${union.name}`,
    "{",
    "    [JsonPropertyName(\"kind\")]",
    "    public string Kind { get; set; } = string.Empty;",
    "",
    "    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]",
    "    [JsonPropertyName(\"payload\")]",
    "    public object? Payload { get; set; }",
  ];
  for (const variant of union.variants) {
    lines.push(
      "",
      `    public const string ${variant.pascal}Kind = "${variant.kind}";`,
      "",
      `    public static ${union.name} For${variant.pascal}(${variant.type} payload) =>`,
      `        new() { Kind = ${variant.pascal}Kind, Payload = payload };`,
      "",
      `    public ${variant.type}? As${variant.pascal}() =>`,
      `        Kind == ${variant.pascal}Kind ? PayloadAs<${variant.type}>() : null;`,
    );
  }
  lines.push(
    "",
    "    internal T? PayloadAs<T>() => Payload switch",
    "    {",
    "        T typed => typed,",
    "        JsonElement element => element.Deserialize<T>(Converter.Settings),",
    "        _ => default,",
    "    };",
    "}",
  );
  return lines.join("\n");
}

export function renderSwiftUnions(unions, emptyPayloads = []) {
  const blocks = unions.map(renderSwiftUnion);
  const structs = emptyPayloads.map(
    (name) => `\npublic struct ${name}: Codable, Sendable {}`,
  );
  return `// Generated from Rust contracts. Do not edit.\nimport Foundation\n\n${blocks.join("\n")}${structs.join("")}\n`;
}

function renderSwiftUnion(union) {
  const cases = union.variants.map((variant) => `    case ${variant.camel}(${variant.type})`);
  const rawKinds = union.variants.map((variant) => `        case ${variant.camel} = "${variant.kind}"`);
  const decodes = union.variants.map(
    (variant) =>
      `        case .${variant.camel}: self = .${variant.camel}(try container.decode(${variant.type}.self, forKey: .payload))`,
  );
  const encodes = union.variants.map(
    (variant) => [
      `        case .${variant.camel}(let payload):`,
      `            try container.encode(Kind.${variant.camel}, forKey: .kind)`,
      `            try container.encode(payload, forKey: .payload)`,
    ].join("\n"),
  );
  return [
    `public enum ${union.name}: Codable, Sendable {`,
    ...cases,
    "",
    "    private enum Kind: String, Codable, Sendable {",
    ...rawKinds,
    "    }",
    "",
    "    private enum CodingKeys: String, CodingKey {",
    "        case kind",
    "        case payload",
    "    }",
    "",
    "    public init(from decoder: Decoder) throws {",
    "        let container = try decoder.container(keyedBy: CodingKeys.self)",
    "        switch try container.decode(Kind.self, forKey: .kind) {",
    ...decodes,
    "        }",
    "    }",
    "",
    "    public func encode(to encoder: Encoder) throws {",
    "        var container = encoder.container(keyedBy: CodingKeys.self)",
    "        switch self {",
    ...encodes,
    "        }",
    "    }",
    "}",
  ].join("\n");
}

function pascalCase(value) {
  return value
    .split(/[^a-zA-Z0-9]+/)
    .filter(Boolean)
    .map((word) => word[0].toUpperCase() + word.slice(1))
    .join("");
}

function camelCase(value) {
  const pascal = pascalCase(value);
  return pascal[0].toLowerCase() + pascal.slice(1);
}

function schemaDefinitions(target) {
  if (target.definitions) {
    return target.definitions;
  }
  if (target.$defs) {
    return target.$defs;
  }
  throw new Error("schema lacks a definitions container");
}
