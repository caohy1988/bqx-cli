#!/usr/bin/env node

"use strict";

const { execFileSync } = require("child_process");
const path = require("path");
const os = require("os");

const PLATFORM_PACKAGES = {
  "darwin-arm64": "@bqx-cli/darwin-arm64",
  "darwin-x64": "@bqx-cli/darwin-x64",
  "linux-x64": "@bqx-cli/linux-x64",
  "linux-arm64": "@bqx-cli/linux-arm64",
  "win32-x64": "@bqx-cli/win32-x64",
  "win32-arm64": "@bqx-cli/win32-arm64",
};

function getBinaryPath() {
  const key = `${os.platform()}-${os.arch()}`;
  const pkg = PLATFORM_PACKAGES[key];
  if (!pkg) {
    throw new Error(
      `Unsupported platform: ${key}. bqx supports: ${Object.keys(PLATFORM_PACKAGES).join(", ")}`
    );
  }

  try {
    const pkgDir = path.dirname(require.resolve(`${pkg}/package.json`));
    const ext = os.platform() === "win32" ? ".exe" : "";
    return path.join(pkgDir, `bqx${ext}`);
  } catch {
    throw new Error(
      `Could not find the bqx binary. The platform package ${pkg} may not be installed.\n` +
        `Try reinstalling with: npm install bqx`
    );
  }
}

const binary = getBinaryPath();
const result = require("child_process").spawnSync(binary, process.argv.slice(2), {
  stdio: "inherit",
});

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error(`bqx binary not found at: ${binary}`);
  } else if (result.error.code === "EACCES") {
    console.error(`Permission denied. Try: chmod +x ${binary}`);
  } else {
    console.error(`Failed to run bqx: ${result.error.message}`);
  }
  process.exit(1);
}

process.exit(result.status ?? 1);
