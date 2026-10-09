import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { isAbsolute } from "node:path";

const binary = process.argv[2];
assert(binary && isAbsolute(binary), "provide the absolute shutdown fixture binary");
const child = spawn(binary, [], { stdio: ["ignore", "pipe", "pipe"] });
let stdout = "";
let stderr = "";
let signalled = false;
const deadline = setTimeout(() => child.kill("SIGKILL"), 10000);
child.stdout.setEncoding("utf8");
child.stderr.setEncoding("utf8");
child.stdout.on("data", chunk => {
  stdout += chunk;
  if (!signalled && stdout.includes("READY\n")) {
    signalled = true;
    child.kill("SIGTERM");
  }
});
child.stderr.on("data", chunk => { stderr += chunk; });
const [code, signal] = await new Promise((resolve, reject) => {
  child.once("error", reject);
  child.once("close", (code, signal) => resolve([code, signal]));
}).finally(() => clearTimeout(deadline));
assert(signalled, "fixture did not start its blocking commit");
assert.equal(signal, null, stderr);
assert.equal(code, 1, stderr);
assert(!stderr.includes("STATE_CLOSED"), "state closed before the blocked commit completed");
assert(!stderr.includes("STATE_OWNER_DROPPED"), "state owner released before process exit");
const reportLine = stderr.split("\n").find(line => line.startsWith("INCOMPLETE "));
assert(reportLine, stderr);
const report = JSON.parse(reportLine.slice("INCOMPLETE ".length));
assert.equal(report.clean, false);
assert.equal(report.unfinished_commits, 1);
assert.equal(report.unfinished_connections, 0);
console.log("Hard-deadline process contract passed: nonzero exit, commit retained, state not closed");
