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
#   tests/fixtures/analytics_sdk_contract.json
#   docs/analytics_sdk_contract.md

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UPSTREAM_REPO="haiyuan-eng-google/BigQuery-Agent-Analytics-SDK"
UPSTREAM_BRANCH="main"
FIXTURE_DIR="${REPO_ROOT}/tests/fixtures/upstream_sdk_latest"
CONTRACT_JSON="${REPO_ROOT}/tests/fixtures/analytics_sdk_contract.json"
CONTRACT_MD="${REPO_ROOT}/docs/analytics_sdk_contract.md"

LOCAL_ONLY=false
if [[ "${1:-}" == "--local" ]]; then
    LOCAL_ONLY=true
fi

# ── Step 1: Fetch upstream sources ───────────────────────────────────

if [[ "$LOCAL_ONLY" == false ]]; then
    echo "Fetching upstream SDK sources from ${UPSTREAM_REPO}@${UPSTREAM_BRANCH}..."
    mkdir -p "${FIXTURE_DIR}"

    base_url="https://raw.githubusercontent.com/${UPSTREAM_REPO}/${UPSTREAM_BRANCH}"

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
fi

# ── Step 2: Validate fetched files ───────────────────────────────────

if [[ ! -f "${FIXTURE_DIR}/cli.py" ]]; then
    echo "Error: ${FIXTURE_DIR}/cli.py not found. Run without --local first." >&2
    exit 1
fi

# ── Step 3: Generate contract ────────────────────────────────────────

echo ""
echo "Generating compatibility contract..."
python3 "${REPO_ROOT}/scripts/parse_sdk_cli.py" \
    --cli-py "${FIXTURE_DIR}/cli.py" \
    --out-json "${CONTRACT_JSON}" \
    --out-md "${CONTRACT_MD}"

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
