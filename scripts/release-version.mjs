import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export function releaseVersion(version, sourceRevision, lock) {
  if (!/^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:[-+][0-9A-Za-z.+-]+)?$/.test(version)
      || !/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(sourceRevision)) {
    throw new Error("Invalid release source identity");
  }
  const sources = [...lock.matchAll(/^source = "(git\+https:\/\/github\.com\/isarmg\/xcss\.git[^"]*)"$/gm)];
  if (sources.length === 0) throw new Error("Missing locked Foundation identity");
  const revisions = new Set(sources.map(([, source]) => {
    const match = /^git\+https:\/\/github\.com\/isarmg\/xcss\.git\?rev=([0-9a-f]{40})#([0-9a-f]{40})$/.exec(source);
    if (!match || match[1] !== match[2]) throw new Error("Invalid locked Foundation identity");
    return match[1];
  }));
  if (revisions.size !== 1) throw new Error("Mixed locked Foundation identities");
  return `xczs ${version} (git ${sourceRevision}) foundation=${[...revisions][0]}`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  if (process.argv.length !== 5) throw new Error("Expected VERSION SOURCE_REVISION CARGO_LOCK");
  console.log(releaseVersion(process.argv[2], process.argv[3], readFileSync(process.argv[4], "utf8")));
}
