import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parseDocument } from "yaml";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const workflowRoot = resolve(root, ".github", "workflows");

export function validateReleaseWorkflows(workflowSources) {
  const workflows = new Map(
    [...workflowSources].map(([name, source]) => [name, parseWorkflow(name, source)]),
  );
  const release = requiredWorkflow(workflows, "release-binary.yml");
  const formal = requiredWorkflow(workflows, "formal-release-e2e.yml");
  const audit = requiredWorkflow(workflows, "dependency-audit.yml");
  const ci = requiredWorkflow(workflows, "read-only-ci.yml");
  const performance = requiredWorkflow(workflows, "performance.yml");

  for (const [name, workflow] of workflows) {
    rejectWritePermissions(name, "workflow", workflow.permissions);
    visit(workflow, value => {
      if (typeof value.uses === "string") requirePinnedAction(name, value.uses);
    });
    if (name !== "release-binary.yml") {
      for (const [jobName, job] of Object.entries(workflow.jobs ?? {})) {
        rejectWritePermissions(name, jobName, job.permissions);
      }
    }
  }

  const releaseJobs = release.jobs ?? {};
  requiredJob(releaseJobs, "preflight");
  const verifyBuild = requiredJob(releaseJobs, "verify_build");
  const publish = requiredJob(releaseJobs, "publish");

  for (const [jobName, job] of Object.entries(releaseJobs)) {
    if (jobName !== "publish") {
      rejectWritePermissions("release-binary.yml", jobName, job.permissions);
    }
  }
  requireExactPublishPermissions(publish.permissions);

  requireValue(
    needs(verifyBuild).length === 1 && needs(verifyBuild)[0] === "preflight" &&
      !Object.hasOwn(releaseJobs, "formal_release"),
    "release-binary.yml: release build must follow preflight without the optional package E2E",
  );
  requireValue(
    needs(publish).length === 1 && needs(publish)[0] === "verify_build",
    "release-binary.yml: publish must depend only on candidate verification",
  );

  const publishSteps = Array.isArray(publish.steps) ? publish.steps : [];
  const downloads = publishSteps.filter(step =>
    typeof step?.uses === "string" && step.uses.startsWith("actions/download-artifact@"));
  requireValue(
    downloads.length === 1,
    "release-binary.yml: publish must download exactly one verified artifact",
  );
  requireValue(
    downloads[0].with?.["artifact-ids"] ===
      "${{ needs.verify_build.outputs.release_artifact_id }}",
    "release-binary.yml: publish must bind the artifact ID from verify_build",
  );
  for (const step of publishSteps) {
    if (typeof step?.uses === "string" && step.uses.startsWith("actions/checkout@")) {
      fail("release-binary.yml: the write-permission job must not check out source");
    }
    if (typeof step?.run === "string" &&
      /\bcargo\b|\bnpm\b|\brustc\b|DUFS_FRONTEND_BINARY/u.test(step.run)) {
      fail("release-binary.yml: the write-permission job must not build or run a candidate");
    }
  }

  visit(release, value => {
    if (typeof value.run === "string" && /gh run (?:list|watch|view|download)/u.test(value.run)) {
      fail("release-binary.yml: cross-workflow polling and downloads are forbidden");
    }
  });
  requireValue(
    formal.on?.workflow_dispatch !== undefined,
    "formal-release-e2e.yml: optional package verification must be manually available",
  );
  requireValue(
    hasAction(formal, "actions/upload-artifact@"),
    "formal-release-e2e.yml: formal verification must transfer its exact candidate",
  );
  requireValue(
    audit.on?.schedule !== undefined,
    "dependency-audit.yml: weekly advisory checks must remain scheduled",
  );
  requireValue(
    Array.isArray(ci.on?.push?.branches) &&
      ci.on.push.branches.length === 1 &&
      ci.on.push.branches[0] === "main",
    "read-only-ci.yml: push validation must be limited to main",
  );
  requireValue(
    performance.on?.schedule === undefined,
    "performance.yml: performance benchmarks are manual only",
  );
}

function parseWorkflow(name, source) {
  const document = parseDocument(source, { uniqueKeys: true });
  if (document.errors.length > 0) {
    fail(`${name}: invalid YAML: ${document.errors[0].message}`);
  }
  const workflow = document.toJS();
  if (!workflow || typeof workflow !== "object" || Array.isArray(workflow)) {
    fail(`${name}: workflow root must be a mapping`);
  }
  return workflow;
}

function requiredWorkflow(workflows, name) {
  const workflow = workflows.get(name);
  if (!workflow) fail(`missing workflow: ${name}`);
  return workflow;
}

function requiredJob(jobs, name) {
  const job = jobs[name];
  if (!job || typeof job !== "object" || Array.isArray(job)) {
    fail(`release-binary.yml: missing ${name} job`);
  }
  return job;
}

function needs(job) {
  if (typeof job.needs === "string") return [job.needs];
  return Array.isArray(job.needs) ? job.needs : [];
}

function rejectWritePermissions(workflow, owner, permissions) {
  if (permissions === "write-all") {
    fail(`${workflow}: ${owner} must not receive write-all permission`);
  }
  if (!permissions || typeof permissions !== "object" || Array.isArray(permissions)) return;
  const writable = Object.entries(permissions).find(([, access]) => access === "write");
  if (writable) fail(`${workflow}: ${owner} must not receive ${writable[0]}: write`);
}

function requireExactPublishPermissions(permissions) {
  const entries = permissions && typeof permissions === "object" && !Array.isArray(permissions)
    ? Object.entries(permissions)
    : [];
  requireValue(
    entries.length === 1 && entries[0][0] === "contents" && entries[0][1] === "write",
    "release-binary.yml: publish may receive only contents: write",
  );
}

function requirePinnedAction(workflow, uses) {
  if (uses.startsWith("./") || uses.startsWith("docker://")) return;
  const separator = uses.lastIndexOf("@");
  const revision = separator < 0 ? "" : uses.slice(separator + 1);
  if (!/^[0-9a-f]{40}$/u.test(revision)) {
    fail(`${workflow}: external action must use a full commit SHA: ${uses}`);
  }
}

function hasAction(workflow, prefix) {
  let found = false;
  visit(workflow, value => {
    if (typeof value.uses === "string" && value.uses.startsWith(prefix)) found = true;
  });
  return found;
}

function visit(value, callback) {
  if (!value || typeof value !== "object") return;
  callback(value);
  for (const child of Array.isArray(value) ? value : Object.values(value)) {
    visit(child, callback);
  }
}

function requireValue(condition, message) {
  if (!condition) fail(message);
}

function fail(message) {
  throw new Error(message);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const workflowSources = new Map(
    readdirSync(workflowRoot)
      .filter(name => /\.ya?ml$/u.test(name))
      .map(name => [name, readFileSync(resolve(workflowRoot, name), "utf8")]),
  );
  validateReleaseWorkflows(workflowSources);
  process.stdout.write("Workflow permission and artifact dependency policy passed\n");
}
