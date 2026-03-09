#!/usr/bin/env bash
set -euo pipefail

# bqx MVP Demo Script
#
# Prerequisites:
#   - Rust toolchain installed (https://rustup.rs)
#   - gcloud auth application-default login
#   - BQX_PROJECT and BQX_DATASET env vars set
#   - agent_events table with sample data (see demo.md)
#
# Usage:
#   export BQX_PROJECT="your-project-id"
#   export BQX_DATASET="agent_analytics"
#   bash scripts/demo.sh

: "${BQX_PROJECT:?Set BQX_PROJECT to your GCP project ID}"
: "${BQX_DATASET:?Set BQX_DATASET to your BigQuery dataset}"

BQX="${BQX:-cargo run --quiet --}"

echo "=== bqx v0.0 MVP Demo ==="
echo "Project: $BQX_PROJECT"
echo "Dataset: $BQX_DATASET"
echo ""

# 1. Raw BigQuery query — JSON-first output
echo "--- Step 1: bqx jobs query (JSON-first raw SQL) ---"
echo "\$ bqx jobs query --query \"SELECT session_id, agent, event_type, timestamp FROM ... LIMIT 5\""
echo ""
$BQX jobs query --query "SELECT session_id, agent, event_type, timestamp FROM \`${BQX_PROJECT}.${BQX_DATASET}.agent_events\` LIMIT 5"
echo ""

# 2. Doctor — health check
echo "--- Step 2: bqx analytics doctor ---"
echo "\$ bqx analytics doctor"
echo ""
$BQX analytics doctor
echo ""

# 3. Evaluate latency — find bad sessions
echo "--- Step 3: bqx analytics evaluate (latency, table format) ---"
echo "\$ bqx analytics evaluate --evaluator latency --threshold 5000 --last 30d --format table"
echo ""
$BQX analytics evaluate --evaluator latency --threshold 5000 --last 30d --format table
echo ""

# 4. Get trace for the worst session
# Pick the worst session from step 3 output
WORST_SESSION=$($BQX analytics evaluate --evaluator latency --threshold 5000 --last 30d 2>/dev/null \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['sessions'][0]['session_id'])" 2>/dev/null \
  || echo "")

if [ -z "$WORST_SESSION" ]; then
  echo "--- Step 4: skipped (could not determine worst session) ---"
else
  echo "--- Step 4: bqx analytics get-trace (table format) ---"
  echo "Worst session from step 3: $WORST_SESSION"
  echo "\$ bqx analytics get-trace --session-id $WORST_SESSION --format table"
  echo ""
  $BQX analytics get-trace --session-id "$WORST_SESSION" --format table
fi
echo ""

# 5. Error rate evaluation with exit code (CI gate demo)
echo "--- Step 5: bqx analytics evaluate (error-rate, CI gate) ---"
echo "\$ bqx analytics evaluate --evaluator error-rate --threshold 0.05 --last 30d --exit-code"
echo ""
if $BQX analytics evaluate --evaluator error-rate --threshold 0.05 --last 30d --exit-code; then
  echo ""
  echo "Exit code: 0 (all sessions passed — CI gate would PASS)"
else
  EXIT=$?
  echo ""
  echo "Exit code: $EXIT (some sessions failed — CI gate would FAIL)"
fi
echo ""

echo "=== Demo complete ==="
