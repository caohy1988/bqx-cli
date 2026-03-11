#!/usr/bin/env bash
#
# Build bqx binaries for all supported platforms and stage them into
# the npm platform packages, then run `npm pack` on each.
#
# Usage:
#   ./scripts/package-npm.sh --local             # build only the current platform (smoke test)
#   ./scripts/package-npm.sh --build             # build all targets, stage, and pack
#   ./scripts/package-npm.sh --from-artifacts DIR # stage prebuilt binaries from DIR and pack
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NPM_DIR="$REPO_ROOT/npm"

# Platform definitions: "pkg_dir:rust_target"
PLATFORMS=(
  "bqx-darwin-arm64:aarch64-apple-darwin"
  "bqx-darwin-x64:x86_64-apple-darwin"
  "bqx-linux-x64:x86_64-unknown-linux-gnu"
  "bqx-linux-arm64:aarch64-unknown-linux-gnu"
  "bqx-win32-x64:x86_64-pc-windows-msvc"
  "bqx-win32-arm64:aarch64-pc-windows-msvc"
)

pkg_dir_of()  { echo "${1%%:*}"; }
target_of()   { echo "${1##*:}"; }

bin_ext() {
  case "$1" in
    *windows*|*win32*) echo ".exe" ;;
    *) echo "" ;;
  esac
}

detect_local_pkg() {
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

find_platform_entry() {
  local want_pkg="$1"
  for entry in "${PLATFORMS[@]}"; do
    if [[ "$(pkg_dir_of "$entry")" == "$want_pkg" ]]; then
      echo "$entry"
      return 0
    fi
  done
  echo "ERROR: unknown platform package: $want_pkg" >&2
  return 1
}

build_and_stage() {
  local entry="$1"
  local use_default_target="${2:-false}"
  local pkg_dir target ext
  pkg_dir="$(pkg_dir_of "$entry")"
  target="$(target_of "$entry")"
  ext="$(bin_ext "$target")"

  if [[ "$use_default_target" == "true" ]]; then
    echo ">>> Building (default target, staging as $pkg_dir) ..."
    cargo build --release
    local bin_path="$REPO_ROOT/target/release/bqx${ext}"
  else
    echo ">>> Building for $target ..."
    cargo build --release --target "$target"
    local bin_path="$REPO_ROOT/target/$target/release/bqx${ext}"
  fi

  if [[ ! -f "$bin_path" ]]; then
    echo "ERROR: binary not found at $bin_path" >&2
    exit 1
  fi

  cp "$bin_path" "$NPM_DIR/$pkg_dir/bqx${ext}"
  chmod +x "$NPM_DIR/$pkg_dir/bqx${ext}"
  echo "    Staged binary into npm/$pkg_dir/"
}

stage_from_artifacts() {
  local artifact_dir="$1"
  local pkg_dir="$2"
  local ext
  ext="$(bin_ext "$pkg_dir")"

  local src=""
  if [[ -f "$artifact_dir/$pkg_dir/bqx${ext}" ]]; then
    src="$artifact_dir/$pkg_dir/bqx${ext}"
  elif [[ -f "$artifact_dir/bqx${ext}" ]]; then
    src="$artifact_dir/bqx${ext}"
  else
    echo "ERROR: binary not found for $pkg_dir in $artifact_dir" >&2
    echo "  Looked for: $artifact_dir/$pkg_dir/bqx${ext}" >&2
    echo "          or: $artifact_dir/bqx${ext}" >&2
    exit 1
  fi

  cp "$src" "$NPM_DIR/$pkg_dir/bqx${ext}"
  chmod +x "$NPM_DIR/$pkg_dir/bqx${ext}"
  echo "    Staged $src -> npm/$pkg_dir/bqx${ext}"
}

pack_packages() {
  local out_dir="$REPO_ROOT/dist"
  mkdir -p "$out_dir"

  for pkg_dir in "$@"; do
    echo ">>> Packing npm/$pkg_dir ..."
    (cd "$NPM_DIR/$pkg_dir" && npm pack --pack-destination "$out_dir")
  done

  echo ">>> Packing npm/ (root) ..."
  (cd "$NPM_DIR" && npm pack --pack-destination "$out_dir")

  echo ""
  echo "Tarballs written to dist/:"
  ls -1 "$out_dir"/*.tgz
}

MODE="${1:-}"
case "$MODE" in
  --local)
    local_pkg="$(detect_local_pkg)"
    entry="$(find_platform_entry "$local_pkg")"
    echo "=== Local-only mode: building $local_pkg ==="
    build_and_stage "$entry" true
    pack_packages "$local_pkg"
    ;;

  --build)
    echo "=== Build mode: building all platform packages ==="
    all_pkgs=()
    for entry in "${PLATFORMS[@]}"; do
      build_and_stage "$entry"
      all_pkgs+=("$(pkg_dir_of "$entry")")
    done
    pack_packages "${all_pkgs[@]}"
    ;;

  --from-artifacts)
    ARTIFACT_DIR="${2:?Usage: $0 --from-artifacts <dir>}"
    if [[ ! -d "$ARTIFACT_DIR" ]]; then
      echo "ERROR: artifact directory not found: $ARTIFACT_DIR" >&2
      exit 1
    fi
    echo "=== CI mode: staging from $ARTIFACT_DIR ==="
    staged=()
    for entry in "${PLATFORMS[@]}"; do
      pkg_dir="$(pkg_dir_of "$entry")"
      ext="$(bin_ext "$pkg_dir")"
      if [[ -f "$ARTIFACT_DIR/$pkg_dir/bqx${ext}" ]] || [[ -f "$ARTIFACT_DIR/bqx${ext}" ]]; then
        stage_from_artifacts "$ARTIFACT_DIR" "$pkg_dir"
        staged+=("$pkg_dir")
      else
        echo "    SKIP $pkg_dir (no artifact found)"
      fi
    done
    if [[ ${#staged[@]} -eq 0 ]]; then
      echo "ERROR: no artifacts found in $ARTIFACT_DIR" >&2
      exit 1
    fi
    pack_packages "${staged[@]}"
    ;;

  ""|--help|-h)
    echo "Usage:"
    echo "  $0 --local               Build current platform only (smoke test)"
    echo "  $0 --build               Build all targets, stage, and pack"
    echo "  $0 --from-artifacts DIR  Stage prebuilt binaries from DIR and pack"
    exit 0
    ;;

  *)
    echo "ERROR: unknown option: $MODE" >&2
    echo "Run $0 --help for usage." >&2
    exit 1
    ;;
esac

echo ""
echo "Done."
