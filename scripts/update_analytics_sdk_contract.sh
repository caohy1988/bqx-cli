#!/usr/bin/env bash
# Fetch latest BigQuery Agent Analytics SDK sources and regenerate the
# compatibility contract.
#
# Usage:
#   ./scripts/update_analytics_sdk_contract.sh          # fetch + generate
#   ./scripts/update_analytics_sdk_contract.sh --local   # skip fetch, regenerate from cached files
#
# Outputs:
#   tests/fixtures/upstream_sdk_latest/cli.py
#   tests/fixtures/upstream_sdk_latest/SDK.md
#   tests/fixtures/upstream_sdk_latest/UPSTREAM_SHA
#   tests/fixtures/analytics_sdk_contract.json
#   docs/analytics_sdk_contract.md

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UPSTREAM_REPO="haiyuan-eng-google/BigQuery-Agent-Analytics-SDK"
UPSTREAM_BRANCH="main"
FIXTURE_DIR="${REPO_ROOT}/tests/fixtures/upstream_sdk_latest"
CONTRACT_JSON="${REPO_ROOT}/tests/fixtures/analytics_sdk_contract.json"
CONTRACT_MD="${REPO_ROOT}/docs/analytics_sdk_contract.md"
CLI_RS="${REPO_ROOT}/src/cli.rs"
SHA_FILE="${FIXTURE_DIR}/UPSTREAM_SHA"

LOCAL_ONLY=false
if [[ "${1:-}" == "--local" ]]; then
    LOCAL_ONLY=true
fi

# ── Step 1: Fetch upstream sources ───────────────────────────────────

UPSTREAM_SHA=""

if [[ "$LOCAL_ONLY" == false ]]; then
    echo "Fetching upstream SDK sources from ${UPSTREAM_REPO}@${UPSTREAM_BRANCH}..."
    mkdir -p "${FIXTURE_DIR}"

    # Resolve the current HEAD SHA of the upstream branch
    UPSTREAM_SHA=$(git ls-remote "https://github.com/${UPSTREAM_REPO}.git" "${UPSTREAM_BRANCH}" | awk '{print $1}')
    if [[ -z "$UPSTREAM_SHA" ]]; then
        echo "Error: could not resolve HEAD SHA for ${UPSTREAM_REPO}@${UPSTREAM_BRANCH}" >&2
        exit 1
    fi
    echo "  Resolved SHA: ${UPSTREAM_SHA}"
    echo "$UPSTREAM_SHA" > "${SHA_FILE}"

    # Fetch at the resolved SHA for reproducibility
    base_url="https://raw.githubusercontent.com/${UPSTREAM_REPO}/${UPSTREAM_SHA}"

    curl -sSfL "${base_url}/src/bigquery_agent_analytics/cli.py" \
        -o "${FIXTURE_DIR}/cli.py"
    echo "  ✓ cli.py"

    curl -sSfL "${base_url}/SDK.md" \
        -o "${FIXTURE_DIR}/SDK.md"
    echo "  ✓ SDK.md"

    # README is optional — useful for context but not required for contract
    if curl -sSfL "${base_url}/README.md" -o "${FIXTURE_DIR}/README.md" 2>/dev/null; then
        echo "  ✓ README.md"
    else
        echo "  ⚠ README.md not fetched (non-fatal)"
    fi
else
    echo "Local mode — skipping fetch, using cached files in ${FIXTURE_DIR}"
    if [[ -f "${SHA_FILE}" ]]; then
        UPSTREAM_SHA=$(cat "${SHA_FILE}")
        echo "  Cached SHA: ${UPSTREAM_SHA}"
    fi
fi

# ── Step 2: Validate inputs ──────────────────────────────────────────

if [[ ! -f "${FIXTURE_DIR}/cli.py" ]]; then
    echo "Error: ${FIXTURE_DIR}/cli.py not found. Run without --local first." >&2
    exit 1
fi

if [[ ! -f "${CLI_RS}" ]]; then
    echo "Error: ${CLI_RS} not found. Run from the repo root." >&2
    exit 1
fi

# ── Step 3: Generate contract ────────────────────────────────────────

echo ""
echo "Generating compatibility contract..."

sha_arg=""
if [[ -n "$UPSTREAM_SHA" ]]; then
    sha_arg="--upstream-sha ${UPSTREAM_SHA}"
fi

python3 "${REPO_ROOT}/scripts/parse_sdk_cli.py" \
    --cli-py "${FIXTURE_DIR}/cli.py" \
    --cli-rs "${CLI_RS}" \
    --out-json "${CONTRACT_JSON}" \
    --out-md "${CONTRACT_MD}" \
    ${sha_arg}

echo ""
echo "Done. Review:"
echo "  ${CONTRACT_JSON}"
echo "  ${CONTRACT_MD}"

# ── Step 4: Check for changes ────────────────────────────────────────

if git -C "${REPO_ROOT}" diff --quiet -- "${CONTRACT_JSON}" "${CONTRACT_MD}" 2>/dev/null; then
    echo ""
    echo "No contract changes detected."
else
    echo ""
    echo "Contract has changed — review the diff and commit if appropriate."
    git -C "${REPO_ROOT}" diff --stat -- "${CONTRACT_JSON}" "${CONTRACT_MD}" 2>/dev/null || true
fi
