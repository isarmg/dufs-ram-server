import { accessSync, constants, realpathSync } from "node:fs";
import { spawnSync } from "node:child_process";

const configured = process.env.XCZS_FRONTEND_BINARY;
if (!configured) {
  throw new Error("XCZS_FRONTEND_BINARY must name the prepared candidate binary");
}
const binary = realpathSync(configured);
accessSync(binary, constants.X_OK);
const version = spawnSync(binary, ["--version"], {
  encoding: "utf8",
  env: process.env,
});
if (version.status !== 0 || !version.stdout.startsWith("xczs ")) {
  throw new Error(`prepared candidate did not report a Xczs version: ${binary}`);
}
process.env.XCZS_FRONTEND_BINARY = binary;
process.stdout.write(`Browser candidate: ${binary}\n${version.stdout}`);
await import("../tests/frontend/run.mjs");
