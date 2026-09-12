import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const read = path => readFileSync(resolve(root, path));
const manifest = JSON.parse(read("package.json"));
const lock = JSON.parse(read("package-lock.json"));
for (const [name, version] of Object.entries({ react: "19.2.8", "react-dom": "19.2.8" })) {
  assert.equal(manifest.dependencies[name], version);
  assert.equal(lock.packages[`node_modules/${name}`].version, version);
  assert.match(lock.packages[`node_modules/${name}`].integrity, /^sha512-/u);
}
for (const [name, version] of Object.entries({ "@types/react": "19.2.18", "@types/react-dom": "19.2.5", "@vitejs/plugin-react": "4.7.0" })) {
  assert.equal(manifest.devDependencies[name], version);
  assert.equal(lock.packages[`node_modules/${name}`].version, version);
}
for (const name of ["admin-web", "admin-shell", "admin-ui", "contracts", "http-client", "design-tokens", "web-fonts", "web-toolchain"]) {
  const packageName = `@sarmg/${name}`;
  const url = `https://github.com/isarmg/sarmg-foundation-server/releases/download/v0.7.5/sarmg-${name}-0.7.5.tgz`;
  assert.equal(manifest.dependencies[packageName], url);
  const installed = JSON.parse(read(`node_modules/${packageName}/package.json`));
  assert.equal(installed.version, "0.7.5");
  assert.equal(lock.packages[`node_modules/${packageName}`].resolved, url);
  assert.match(lock.packages[`node_modules/${packageName}`].integrity, /^sha512-/u);
}
const fonts = "node_modules/@sarmg/web-fonts/dist";
const provenance = JSON.parse(read(`${fonts}/provenance.json`));
for (const [name, digest] of Object.entries(provenance.assets)) {
  const bytes = read(`${fonts}/${name}`);
  assert.equal(createHash("sha256").update(bytes).digest("hex"), digest, name);
}
assert.equal(provenance.latin.handwriting, false);
for (const page of ["index.html", "login.html"]) {
  assert.match(read(`clients/web/${page}`).toString(), /data-sarmg-appearance="content-blocks"/u);
}
assert.match(read("sarmg-product.toml").toString(), /web_profile = "web-react-admin"/u);
console.log("Foundation 0.7.5 immutable React Web and compact font bootstrap verified");
