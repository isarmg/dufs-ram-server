
const printPath = process.argv[2] === "--print-path";
if (printPath && process.argv.length !== 3) {
  throw new Error("--print-path does not accept browser runner arguments");
}

const { buildWebServer } = await import("@xcss/web-toolchain/server");
const binary = buildWebServer("foundation-web-build.json", { mode: "development", noInstall: true });
if (!binary) throw new Error("Foundation builder returned no executable");
if (printPath) {
  process.stdout.write(`${binary}\n`);
} else {
  process.env.XCZS_FRONTEND_BINARY = binary;
  await import("./run-frontend-prepared.mjs");
}
