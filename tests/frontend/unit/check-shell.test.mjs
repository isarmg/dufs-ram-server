import assert from "node:assert/strict";
import { copyFileSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");

test("Shell checks work in a release archive without Git metadata", () => {
  const fixture = mkdtempSync(join(tmpdir(), "dufs-shell-check-"));
  try {
    mkdirSync(join(fixture, "scripts"));
    mkdirSync(join(fixture, "tests"));
    copyFileSync(
      join(root, "scripts/check-shell.sh"),
      join(fixture, "scripts/check-shell.sh"),
    );
    const candidate = join(fixture, "tests/candidate.sh");
    writeFileSync(candidate, "#!/usr/bin/env bash\ntrue\n");

    const valid = spawnSync("bash", ["scripts/check-shell.sh"], {
      cwd: fixture,
      encoding: "utf8",
    });
    assert.equal(valid.status, 0, valid.stderr);
    assert.match(valid.stdout, /tests\/candidate\.sh/u);

    writeFileSync(candidate, "#!/usr/bin/env bash\nif then\n");
    const invalid = spawnSync("bash", ["scripts/check-shell.sh"], {
      cwd: fixture,
      encoding: "utf8",
    });
    assert.notEqual(invalid.status, 0);
    assert.match(invalid.stderr, /candidate\.sh/u);
  } finally {
    rmSync(fixture, { force: true, recursive: true });
  }
});
