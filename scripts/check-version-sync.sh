#!/usr/bin/env bash
#
# Verify that all npm package.json versions match the Cargo.toml version.
# Optionally verify a git tag matches too.
#
# Usage:
#   ./scripts/check-version-sync.sh             # check npm vs Cargo.toml
#   ./scripts/check-version-sync.sh --tag v0.0.1 # also verify tag matches
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TAG_VERSION=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      TAG_VERSION="${2:?--tag requires a version argument}"
      shift 2
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# Extract version from Cargo.toml
CARGO_VERSION="$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')"

echo "Cargo.toml version: $CARGO_VERSION"
echo ""

ERRORS=0

# If a tag was provided, verify it matches Cargo.toml
if [[ -n "$TAG_VERSION" ]]; then
  # Strip leading 'v' from tag
  TAG_BARE="${TAG_VERSION#v}"
  if [[ "$TAG_BARE" == "$CARGO_VERSION" ]]; then
    echo "  OK    tag $TAG_VERSION matches Cargo.toml ($CARGO_VERSION)"
  else
    echo "  FAIL  tag $TAG_VERSION ($TAG_BARE) != Cargo.toml ($CARGO_VERSION)"
    ERRORS=$((ERRORS + 1))
  fi
  echo ""
fi

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
for dir in dcx-darwin-arm64 dcx-darwin-x64 dcx-linux-x64 dcx-linux-arm64 dcx-win32-x64 dcx-win32-arm64; do
  check_package "$REPO_ROOT/npm/$dir/package.json"
done

echo ""
if [[ $ERRORS -gt 0 ]]; then
  echo "FAILED: $ERRORS version mismatch(es) found."
  exit 1
else
  echo "All versions in sync."
fi
