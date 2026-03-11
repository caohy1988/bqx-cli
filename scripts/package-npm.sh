#!/usr/bin/env bash
#
# Build bqx binaries for all supported platforms and stage them into
# the npm platform packages, then run `npm pack` on each.
#
# Usage:
#   ./scripts/package-npm.sh           # build all targets
#   ./scripts/package-npm.sh --local   # build only the current platform (for smoke testing)
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NPM_DIR="$REPO_ROOT/npm"

# Map: npm-package-dir -> rust-target-triple
declare -A TARGETS=(
  [bqx-darwin-arm64]=aarch64-apple-darwin
  [bqx-darwin-x64]=x86_64-apple-darwin
  [bqx-linux-x64]=x86_64-unknown-linux-gnu
  [bqx-linux-arm64]=aarch64-unknown-linux-gnu
)

LOCAL_ONLY=false
if [[ "${1:-}" == "--local" ]]; then
  LOCAL_ONLY=true
fi

detect_local_target() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "$os-$arch" in
    darwin-arm64)  echo "bqx-darwin-arm64" ;;
    darwin-x86_64) echo "bqx-darwin-x64" ;;
    linux-x86_64)  echo "bqx-linux-x64" ;;
    linux-aarch64) echo "bqx-linux-arm64" ;;
    *) echo "ERROR: unsupported platform $os-$arch" >&2; exit 1 ;;
  esac
}

build_and_stage() {
  local pkg_dir="$1"
  local target="${TARGETS[$pkg_dir]}"

  echo ">>> Building for $target ..."
  cargo build --release --target "$target"

  local bin_path="$REPO_ROOT/target/$target/release/bqx"
  if [[ ! -f "$bin_path" ]]; then
    echo "ERROR: binary not found at $bin_path" >&2
    exit 1
  fi

  cp "$bin_path" "$NPM_DIR/$pkg_dir/bqx"
  chmod +x "$NPM_DIR/$pkg_dir/bqx"
  echo "    Staged binary into npm/$pkg_dir/"
}

pack_packages() {
  local out_dir="$REPO_ROOT/dist"
  mkdir -p "$out_dir"

  # Pack platform packages
  for pkg_dir in "$@"; do
    echo ">>> Packing npm/$pkg_dir ..."
    (cd "$NPM_DIR/$pkg_dir" && npm pack --pack-destination "$out_dir")
  done

  # Pack root package
  echo ">>> Packing npm/ (root) ..."
  (cd "$NPM_DIR" && npm pack --pack-destination "$out_dir")

  echo ""
  echo "Tarballs written to dist/:"
  ls -1 "$out_dir"/*.tgz
}

if $LOCAL_ONLY; then
  local_pkg="$(detect_local_target)"
  echo "=== Local-only mode: building $local_pkg ==="
  build_and_stage "$local_pkg"
  pack_packages "$local_pkg"
else
  echo "=== Building all platform packages ==="
  for pkg_dir in "${!TARGETS[@]}"; do
    build_and_stage "$pkg_dir"
  done
  pack_packages "${!TARGETS[@]}"
fi

echo ""
echo "Done. To publish:"
echo "  npm publish dist/bqx-cli-*.tgz"
echo "  npm publish dist/bqx-cli-darwin-arm64-*.tgz"
echo "  npm publish dist/bqx-cli-darwin-x64-*.tgz"
echo "  npm publish dist/bqx-cli-linux-x64-*.tgz"
echo "  npm publish dist/bqx-cli-linux-arm64-*.tgz"
