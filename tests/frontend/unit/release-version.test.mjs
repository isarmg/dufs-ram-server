import assert from "node:assert/strict";
import test from "node:test";
import { releaseVersion } from "../../../scripts/release-version.mjs";

const source = "a".repeat(40);
const foundation = "b".repeat(40);
const line = revision => `source = "git+https://github.com/isarmg/xcss.git?rev=${revision}#${revision}"\n`;

test("release version binds software, full source and the one locked xcss revision", () => {
  assert.equal(releaseVersion("0.53.5", source, line(foundation).repeat(2)),
    `xczs 0.53.5 (git ${source}) xcss=${foundation}`);
});

test("release version rejects missing, mixed and mismatched xcss revisions", () => {
  assert.throws(() => releaseVersion("0.53.5", source, ""), /Missing/);
  assert.throws(() => releaseVersion("0.53.5", source, line(foundation) + line("c".repeat(40))), /Mixed/);
  assert.throws(() => releaseVersion("0.53.5", source, line(foundation).replace(`#${foundation}`, `#${source}`)), /Invalid locked/);
  assert.throws(() => releaseVersion("0.53.5", "unbound", line(foundation)), /Invalid release source/);
});
