import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const workflowRoot = resolve(root, ".github", "workflows");
const workflows = new Map(readdirSync(workflowRoot)
  .filter(name => /\.ya?ml$/u.test(name))
  .map(name => [name, readFileSync(resolve(workflowRoot, name), "utf8")]));
const release = requiredWorkflow("release-binary.yml");
const formal = requiredWorkflow("formal-release-e2e.yml");
const audit = requiredWorkflow("dependency-audit.yml");
const ci = requiredWorkflow("read-only-ci.yml");
const performance = requiredWorkflow("performance.yml");

for (const [name, source] of workflows) {
  if (/^\s*contents:\s*write\s*$/mu.test(source) && name !== "release-binary.yml") {
    fail(`${name}: only the release workflow may grant contents: write`);
  }
  for (const match of source.matchAll(/^\s*uses:\s*([^\s#]+)@([^\s#]+)(?:\s+#.*)?$/gmu)) {
    if (!/^[0-9a-f]{40}$/u.test(match[2])) {
      fail(`${name}: external action must use a full commit SHA: ${match[1]}`);
    }
  }
}

const formalJob = job(release, "formal_release");
const verifyJob = job(release, "verify_build");
const publishJob = job(release, "publish");
requirePattern(formalJob, /^\s*uses:\s*\.\/\.github\/workflows\/formal-release-e2e\.yml\s*$/mu,
  "release-binary.yml: formal verification must be an explicit reusable-workflow dependency");
requirePattern(verifyJob, /^\s*needs:\s*formal_release\s*$/mu,
  "release-binary.yml: candidate build must depend on formal verification");
requirePattern(publishJob, /^\s*needs:\s*verify_build\s*$/mu,
  "release-binary.yml: publish must depend on the candidate build");
requirePattern(publishJob, /^\s*contents:\s*write\s*$/mu,
  "release-binary.yml: only publish receives contents: write");
requirePattern(publishJob, /actions\/download-artifact@[0-9a-f]{40}/u,
  "release-binary.yml: publish must download the verified artifact");
requirePattern(publishJob, /artifact-ids:\s*\$\{\{ needs\.verify_build\.outputs\.release_artifact_id \}\}/u,
  "release-binary.yml: publish must bind the artifact ID from verify_build");
if (/actions\/checkout|\bcargo\b|\bnpm\b|\brustc\b|DUFS_FRONTEND_BINARY/u.test(publishJob)) {
  fail("release-binary.yml: the write-permission job must not check out or rebuild source");
}
if (/gh run (?:list|watch|view|download)/u.test(release)) {
  fail("release-binary.yml: cross-workflow polling and downloads are forbidden");
}
requirePattern(formal, /^\s*workflow_call:\s*$/mu,
  "formal-release-e2e.yml: formal verification must be reusable by release");
requirePattern(formal, /actions\/upload-artifact@[0-9a-f]{40}/u,
  "formal-release-e2e.yml: formal verification must transfer its exact candidate");
requirePattern(audit, /^\s*schedule:\s*$/mu,
  "dependency-audit.yml: weekly advisory checks must remain scheduled");
requirePattern(ci, /^\s{4}branches:\s*\n\s{6}- main\s*$/mu,
  "read-only-ci.yml: push validation must be limited to main");
if (/^\s*schedule:\s*$/mu.test(performance)) {
  fail("performance.yml: performance benchmarks are manual only");
}

process.stdout.write("Workflow permission and artifact dependency policy passed\n");

function requiredWorkflow(name) {
  const source = workflows.get(name);
  if (!source) fail(`missing workflow: ${name}`);
  return source;
}

function job(source, name) {
  const marker = `  ${name}:\n`;
  const start = source.indexOf(marker);
  if (start < 0) fail(`release-binary.yml: missing ${name} job`);
  const rest = source.slice(start + marker.length);
  const next = /^  [A-Za-z0-9_]+:\s*$/mu.exec(rest);
  return source.slice(start, next ? start + marker.length + next.index : undefined);
}

function requirePattern(source, pattern, message) {
  if (!pattern.test(source)) fail(message);
}

function fail(message) {
  throw new Error(message);
}
