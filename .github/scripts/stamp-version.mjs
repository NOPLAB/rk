// Write the release version into the Tauri config.
//
// The installers take their version from tauri.conf.json, and NSIS uses it to
// decide whether an install is an upgrade. Left at whatever is committed,
// every release would ship as the same version and refuse to replace itself.
//
//   node .github/scripts/stamp-version.mjs 0.2.0

import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error(`stamp-version: '${version}' is not a version`);
  process.exit(1);
}

const path = "apps/desktop/src-tauri/tauri.conf.json";
const config = JSON.parse(readFileSync(path, "utf8"));
config.version = version;
writeFileSync(path, `${JSON.stringify(config, null, 2)}\n`);

console.log(`stamp-version: ${path} is now ${version}`);
