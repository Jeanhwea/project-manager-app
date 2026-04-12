const fs = require("fs");
const path = require("path");

const PLATFORM_MAP = {
  "win32-x64": "@jeansoft/pma-win32-x64",
  "win32-arm64": "@jeansoft/pma-win32-arm64",
  "linux-x64": "@jeansoft/pma-linux-x64",
  "darwin-x64": "@jeansoft/pma-darwin-x64",
  "darwin-arm64": "@jeansoft/pma-darwin-arm64",
};

function getPlatformPackage() {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PLATFORM_MAP[key];
  if (!pkg) {
    throw new Error(`Unsupported platform: ${key}`);
  }
  return pkg;
}

function install() {
  const pkg = getPlatformPackage();
  const binName = process.platform === "win32" ? "pma.exe" : "pma";

  let srcPath;
  try {
    const pkgDir = path.dirname(require.resolve(`${pkg}/package.json`));
    srcPath = path.join(pkgDir, "bin", binName);
  } catch {
    throw new Error(
      `Failed to find platform package ${pkg}. ` +
        `Your platform (${process.platform}-${process.arch}) may not be supported.`
    );
  }

  const destDir = path.join(__dirname, "bin");
  const destPath = path.join(destDir, binName);

  if (!fs.existsSync(destDir)) {
    fs.mkdirSync(destDir, { recursive: true });
  }

  fs.copyFileSync(srcPath, destPath);
  fs.chmodSync(destPath, 0o755);
}

install();
