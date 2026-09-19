import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const failures = [];
const cargo = readFileSync(resolve(root, "Cargo.toml"), "utf8");
if (/\bpath\s*=\s*["']/u.test(cargo)) {
  failures.push("Cargo.toml: local path dependencies are not allowed");
}

for (const name of ["package.json", "package-lock.json"]) {
  const value = JSON.parse(readFileSync(resolve(root, name), "utf8"));
  visit(value, name);
}

function visit(value, location) {
  if (typeof value === "string") {
    if (/^(?:file|link):/u.test(value) || /(?:^|\/)\.\.(?:\/|$)/u.test(value)) {
      failures.push(`${location}: local workspace dependency is not allowed: ${value}`);
    }
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((entry, index) => visit(entry, `${location}[${index}]`));
    return;
  }
  if (value && typeof value === "object") {
    for (const [key, entry] of Object.entries(value)) {
      visit(entry, `${location}.${key}`);
    }
  }
}

if (failures.length) {
  process.stderr.write(`${failures.join("\n")}\n`);
  process.exit(1);
}
process.stdout.write("Dependency declarations are independent of adjacent workspaces\n");
