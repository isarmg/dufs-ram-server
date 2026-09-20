import { spawnSync } from "node:child_process";
import { realpathSync } from "node:fs";

const target = "x86_64-unknown-linux-gnu";
const printPath = process.argv[2] === "--print-path";
if (printPath && process.argv.length !== 3) {
  throw new Error("--print-path does not accept browser runner arguments");
}

const build = spawnSync(
  "cargo",
  ["build", "--locked", "--target", target, "--message-format=json-render-diagnostics"],
  {
    encoding: "utf8",
    env: process.env,
    maxBuffer: 64 * 1024 * 1024,
  },
);
process.stderr.write(build.stderr ?? "");
if (build.error) {
  throw build.error;
}
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

const candidates = new Set();
for (const line of build.stdout.split("\n")) {
  if (!line) continue;
  const message = JSON.parse(line);
  if (message.reason === "compiler-message" && message.message?.rendered) {
    process.stderr.write(message.message.rendered);
  }
  if (
    message.reason === "compiler-artifact" &&
    message.target?.name === "dufs" &&
    message.target.kind?.includes("bin") &&
    message.executable
  ) {
    candidates.add(realpathSync(message.executable));
  }
}

if (candidates.size !== 1) {
  throw new Error(`cargo reported ${candidates.size} Dufs executable candidates; expected exactly one`);
}
const [binary] = candidates;
if (printPath) {
  process.stdout.write(`${binary}\n`);
} else {
  process.env.DUFS_FRONTEND_BINARY = binary;
  await import("./run-frontend-prepared.mjs");
}
