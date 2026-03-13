---
name: recipe-drift-monitoring
description: Recipe for setting up weekly drift detection that compares agent performance across time windows and alerts on regressions.
---

## When to use this skill

Use when the user wants to:
- Set up automated weekly drift checks
- Detect performance regressions before they reach users
- Create a scheduled drift monitoring script

## Prerequisites

Load the following skills: `bqx-analytics`, `bqx-analytics-drift`

See **bqx-shared** for authentication and global flags.

## Recipe

### Step 1: Create a drift-check script

```bash
#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID="${1:?Usage: drift-check.sh <project-id> <dataset-id>}"
DATASET_ID="${2:?Usage: drift-check.sh <project-id> <dataset-id>}"
LATENCY_THRESHOLD="${3:-5000}"
ERROR_THRESHOLD="${4:-0.10}"

echo "=== Latency drift check (7d window) ==="
bqx analytics evaluate \
  --project-id "$PROJECT_ID" \
  --dataset-id "$DATASET_ID" \
  --evaluator latency \
  --threshold "$LATENCY_THRESHOLD" \
  --last 7d \
  --format table

echo ""
echo "=== Error rate drift check (7d window) ==="
bqx analytics evaluate \
  --project-id "$PROJECT_ID" \
  --dataset-id "$DATASET_ID" \
  --evaluator error-rate \
  --threshold "$ERROR_THRESHOLD" \
  --last 7d \
  --format table

echo ""
echo "=== Health check ==="
bqx analytics doctor \
  --project-id "$PROJECT_ID" \
  --dataset-id "$DATASET_ID" \
  --format table
```

### Step 2: Schedule weekly execution

#### Cron (Linux/macOS)

```bash
# Run every Monday at 9am
0 9 * * 1 /path/to/drift-check.sh my-proj analytics >> /var/log/drift-check.log 2>&1
```

#### GitHub Actions (scheduled)

```yaml
on:
  schedule:
    - cron: '0 9 * * 1'  # Every Monday at 9am UTC

jobs:
  drift-check:
    runs-on: ubuntu-latest
    steps:
      - name: Install bqx
        run: npm install -g @bqx-cli/linux-x64

      - name: Run drift check
        run: |
          bqx analytics evaluate \
            --project-id ${{ secrets.GCP_PROJECT }} \
            --dataset-id ${{ secrets.BQX_DATASET }} \
            --evaluator latency \
            --threshold 5000 \
            --last 7d \
            --exit-code \
            --format json
```

### Step 3: Compare across time windows

For deeper drift analysis, capture JSON output from two time windows:

```bash
# Baseline: previous 30 days
bqx analytics evaluate \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --evaluator latency \
  --threshold 5000 \
  --last 30d \
  --format json > baseline.json

# Current: last 7 days
bqx analytics evaluate \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --evaluator latency \
  --threshold 5000 \
  --last 7d \
  --format json > current.json

# Compare pass rates (using jq)
echo "Baseline pass rate:"
jq '.pass_rate' baseline.json
echo "Current pass rate:"
jq '.pass_rate' current.json
```

### Step 4: Add alerting

Combine `--exit-code` with notification tools:

```bash
if ! bqx analytics evaluate \
  --project-id "$PROJECT_ID" \
  --dataset-id "$DATASET_ID" \
  --evaluator error-rate \
  --threshold 0.10 \
  --last 7d \
  --exit-code \
  --format json > /tmp/eval.json 2>&1; then
  # Send alert (e.g., Slack webhook, PagerDuty, email)
  echo "Drift detected — error rate exceeded 10% threshold"
  cat /tmp/eval.json
fi
```

## Decision rules

- Use `--last 7d` for weekly drift checks
- Use `--last 30d` as a stable baseline for comparison
- Start with relaxed thresholds and tighten as you collect data
- Run `doctor` alongside evaluate to catch data pipeline issues
- Use `--exit-code` in scheduled jobs to trigger alerts on failure
- Use `--agent-id` to monitor high-priority agents separately

## Constraints

- Drift detection relies on consistent event volume — gaps in data may cause false positives
- The `--last` flag measures from the current time, not fixed calendar windows
- Alerting integrations (Slack, PagerDuty) must be configured outside bqx
- For sub-daily monitoring, use shorter windows with `recipe-eval-pipeline` instead
