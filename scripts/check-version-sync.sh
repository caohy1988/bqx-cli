#!/usr/bin/env bash
#
# Verify that all npm package.json versions match the Cargo.toml version.
#
# Usage:
#   ./scripts/check-version-sync.sh
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Extract version from Cargo.toml
CARGO_VERSION="$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')"

echo "Cargo.toml version: $CARGO_VERSION"
echo ""

ERRORS=0

check_package() {
  local pkg_path="$1"
  local rel_path="${pkg_path#"$REPO_ROOT/"}"

  if [[ ! -f "$pkg_path" ]]; then
    echo "  SKIP  $rel_path (not found)"
    return
  fi

  local npm_version
  npm_version="$(node -e "console.log(require('./$rel_path').version)")"

  if [[ "$npm_version" == "$CARGO_VERSION" ]]; then
    echo "  OK    $rel_path  ($npm_version)"
  else
    echo "  FAIL  $rel_path  ($npm_version != $CARGO_VERSION)"
    ERRORS=$((ERRORS + 1))
  fi
}

# Check root package
check_package "$REPO_ROOT/npm/package.json"

# Check optionalDependencies versions in root package
for dep_version in $(node -e "
  const pkg = require('./npm/package.json');
  const deps = pkg.optionalDependencies || {};
  Object.values(deps).forEach(v => console.log(v));
"); do
  if [[ "$dep_version" != "$CARGO_VERSION" ]]; then
    echo "  FAIL  npm/package.json optionalDependencies version ($dep_version != $CARGO_VERSION)"
    ERRORS=$((ERRORS + 1))
  fi
done

# Check platform packages
for dir in bqx-darwin-arm64 bqx-darwin-x64 bqx-linux-x64 bqx-linux-arm64; do
  check_package "$REPO_ROOT/npm/$dir/package.json"
done

echo ""
if [[ $ERRORS -gt 0 ]]; then
  echo "FAILED: $ERRORS version mismatch(es) found."
  exit 1
else
  echo "All versions in sync."
fi
