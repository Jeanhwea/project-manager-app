const fs = require("fs");
const path = require("path");

const version = process.argv[2];
if (!version) {
  console.error("Usage: node version.js <version>");
  process.exit(1);
}

const PACKAGES = [
  "pma",
  "pma-win32-x64",
  "pma-win32-arm64",
  "pma-linux-x64",
  "pma-darwin-x64",
  "pma-darwin-arm64",
];

for (const pkg of PACKAGES) {
  const pkgPath = path.join(__dirname, pkg, "package.json");
  const json = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
  json.version = version;

  if (json.optionalDependencies) {
    for (const dep of Object.keys(json.optionalDependencies)) {
      json.optionalDependencies[dep] = version;
    }
  }

  fs.writeFileSync(pkgPath, JSON.stringify(json, null, 2) + "\n");
  console.log(`Updated ${pkg} to ${version}`);
}
