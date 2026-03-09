#!/usr/bin/env bash
set -euo pipefail

# bqx MVP Demo Script
# Prerequisites:
#   - gcloud auth application-default login
#   - BQX_PROJECT and BQX_DATASET env vars set
#   - agent_events table with sample data

BQX="${BQX:-cargo run --quiet --}"

echo "=== bqx MVP Demo ==="
echo ""

# 1. Raw BigQuery query — JSON-first output
echo "--- Step 1: bqx jobs query (JSON-first raw SQL) ---"
$BQX jobs query --query "SELECT session_id, agent, event_type, timestamp FROM \`${BQX_PROJECT}.${BQX_DATASET}.agent_events\` LIMIT 5"
echo ""

# 2. Doctor — health check
echo "--- Step 2: bqx analytics doctor ---"
$BQX analytics doctor
echo ""

# 3. Evaluate latency — find bad sessions
echo "--- Step 3: bqx analytics evaluate (latency, table format) ---"
$BQX analytics evaluate --evaluator latency --threshold 5000 --last 24h --format table
echo ""

# 4. Get trace for a bad session
echo "--- Step 4: bqx analytics get-trace (table format) ---"
# Replace with an actual session_id from your dataset
SESSION_ID="${DEMO_SESSION_ID:-sess-042}"
$BQX analytics get-trace --session-id "$SESSION_ID" --format table
echo ""

# 5. Error rate evaluation with exit code
echo "--- Step 5: bqx analytics evaluate (error-rate, CI gate) ---"
$BQX analytics evaluate --evaluator error-rate --threshold 0.05 --last 24h --exit-code || echo "(exit code: $?)"
echo ""

echo "=== Demo complete ==="
