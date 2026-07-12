import { readFileSync } from "node:fs";
import { join } from "node:path";

export function normalize(content) {
  const lines = content.replaceAll("\r\n", "\n").split("\n");
  return `${lines.map((line) => line.trimEnd()).join("\n").trimEnd()}\n`;
}

export function findGeneratedDrift(repository, generatedDirectory, outputs) {
  const drift = [];
  for (const [sourceName, targetName] of outputs) {
    const generated = normalize(readFileSync(join(generatedDirectory, sourceName), "utf8"));
    let current;
    try {
      current = normalize(readFileSync(join(repository, targetName), "utf8"));
    } catch {
      drift.push(`${targetName} is missing`);
      continue;
    }
    if (current !== generated) {
      drift.push(`${targetName} differs from Rust authority`);
    }
  }
  return drift;
}

export function missingBindingIdentifiers(registry, bindingSource) {
  return Object.values(registry)
    .flat()
    .filter((identifier) => !bindingSource.includes(`\"${identifier}\"`));
}
