import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { parse, stringify } from "yaml";
import { validateReleaseWorkflows } from "../../../scripts/check-release-workflow.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const workflowRoot = resolve(root, ".github", "workflows");

function workflowSources(mutate = () => {}) {
  const sources = new Map(
    readdirSync(workflowRoot)
      .filter(name => /\.ya?ml$/u.test(name))
      .map(name => [name, readFileSync(resolve(workflowRoot, name), "utf8")]),
  );
  const release = parse(sources.get("release-binary.yml"));
  mutate(release);
  sources.set("release-binary.yml", stringify(release));
  return sources;
}

test("workflow policy is independent of display names and YAML formatting", () => {
  const sources = workflowSources(release => {
    release.jobs.publish.name = "A renamed publishing job";
    release.jobs.verify_build.name = "A renamed verification job";
  });
  assert.doesNotThrow(() => validateReleaseWorkflows(sources));
});

test("workflow policy rejects write permission on a non-publishing job", () => {
  const sources = workflowSources(release => {
    release.jobs.verify_build.permissions.contents = "write";
  });
  assert.throws(
    () => validateReleaseWorkflows(sources),
    /verify_build must not receive contents: write/u,
  );
});

test("workflow policy rejects write permission on an added job", () => {
  const sources = workflowSources(release => {
    release.jobs.unexpected = {
      "runs-on": "ubuntu-24.04",
      permissions: { contents: "write" },
      steps: [],
    };
  });
  assert.throws(
    () => validateReleaseWorkflows(sources),
    /unexpected must not receive contents: write/u,
  );
});

test("workflow policy rejects extra publishing permissions", () => {
  const sources = workflowSources(release => {
    release.jobs.publish.permissions.actions = "write";
  });
  assert.throws(
    () => validateReleaseWorkflows(sources),
    /publish may receive only contents: write/u,
  );
});

test("workflow policy rejects an artifact ID from another job", () => {
  const sources = workflowSources(release => {
    const download = release.jobs.publish.steps.find(step =>
      step.uses?.startsWith("actions/download-artifact@"));
    download.with["artifact-ids"] = "${{ needs.preflight.outputs.release_artifact_id }}";
  });
  assert.throws(
    () => validateReleaseWorkflows(sources),
    /bind the artifact ID from verify_build/u,
  );
});
