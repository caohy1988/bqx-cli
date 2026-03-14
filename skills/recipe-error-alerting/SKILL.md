---
name: recipe-error-alerting
description: Recipe for setting up automated error alerting using bqx analytics evaluate, CA natural language queries, and CI/CD integration for proactive agent monitoring.
---

## When to use this skill

Use when the user wants to:
- Set up automated alerts when agent error rates exceed thresholds
- Build a proactive monitoring pipeline for AI agents
- Integrate bqx with Slack, PagerDuty, or email for error notifications
- Create a scheduled job that checks agent health and alerts on failures

## Prerequisites

Load the following skills: `bqx-analytics`, `bqx-ca`

See **bqx-shared** for authentication and global flags.

## Recipe

### Step 1: Define alert thresholds

Decide on your alert thresholds:

| Metric | Warning | Critical |
|--------|---------|----------|
| Error rate | > 5% | > 15% |
| P95 latency | > 5000ms | > 15000ms |
| Drift coverage | < 90% | < 70% |

### Step 2: Create an alerting script

```bash
#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID="${1:?Usage: alert-check.sh <project-id> <dataset-id>}"
DATASET_ID="${2:?Usage: alert-check.sh <project-id> <dataset-id>}"
ALERT_OUTPUT="/tmp/bqx-alert-$(date +%s).json"

FAILURES=0

echo "Checking error rate..."
if ! bqx analytics evaluate \
  --project-id "$PROJECT_ID" \
  --dataset-id "$DATASET_ID" \
  --evaluator error-rate \
  --threshold 0.05 \
  --last 1h \
  --exit-code \
  --format json > "$ALERT_OUTPUT" 2>&1; then
  echo "ALERT: Error rate exceeded 5% threshold"
  FAILURES=$((FAILURES + 1))
fi

echo "Checking latency..."
if ! bqx analytics evaluate \
  --project-id "$PROJECT_ID" \
  --dataset-id "$DATASET_ID" \
  --evaluator latency \
  --threshold 5000 \
  --last 1h \
  --exit-code \
  --format json >> "$ALERT_OUTPUT" 2>&1; then
  echo "ALERT: Latency exceeded 5000ms threshold"
  FAILURES=$((FAILURES + 1))
fi

echo "Checking drift coverage..."
if ! bqx analytics drift \
  --project-id "$PROJECT_ID" \
  --dataset-id "$DATASET_ID" \
  --golden-dataset golden_questions \
  --min-coverage 0.80 \
  --last 24h \
  --exit-code \
  --format json >> "$ALERT_OUTPUT" 2>&1; then
  echo "ALERT: Drift coverage below 80%"
  FAILURES=$((FAILURES + 1))
fi

if [ "$FAILURES" -gt 0 ]; then
  echo "$FAILURES alert(s) triggered. Details in $ALERT_OUTPUT"
  exit 1
fi

echo "All checks passed."
```

### Step 3: Add notification integration

#### Slack webhook

```bash
if [ "$FAILURES" -gt 0 ]; then
  SUMMARY=$(bqx analytics insights --project-id "$PROJECT_ID" --dataset-id "$DATASET_ID" --last 1h --format json | jq -r '.summary | "Sessions: \(.total_sessions), Errors: \(.error_events), Error rate: \(.error_rate)"')

  curl -s -X POST "$SLACK_WEBHOOK_URL" \
    -H 'Content-type: application/json' \
    -d "{\"text\": \"Agent Alert: $FAILURES check(s) failed.\n$SUMMARY\"}"
fi
```

#### PagerDuty event

```bash
if [ "$FAILURES" -gt 0 ]; then
  curl -s -X POST "https://events.pagerduty.com/v2/enqueue" \
    -H 'Content-type: application/json' \
    -d "{
      \"routing_key\": \"$PAGERDUTY_KEY\",
      \"event_action\": \"trigger\",
      \"payload\": {
        \"summary\": \"bqx: $FAILURES agent health check(s) failed\",
        \"severity\": \"warning\",
        \"source\": \"bqx-alerting\"
      }
    }"
fi
```

### Step 4: Schedule with cron or GitHub Actions

#### Cron (every 15 minutes)

```bash
*/15 * * * * /path/to/alert-check.sh my-proj analytics >> /var/log/bqx-alerts.log 2>&1
```

#### GitHub Actions

```yaml
on:
  schedule:
    - cron: '*/15 * * * *'

jobs:
  alert-check:
    runs-on: ubuntu-latest
    steps:
      - name: Install bqx
        run: npm install -g bqx

      - uses: google-github-actions/auth@v2
        with:
          workload_identity_provider: ${{ vars.WIF_PROVIDER }}
          service_account: ${{ vars.SA_EMAIL }}

      - name: Run health checks
        run: |
          bqx analytics evaluate \
            --project-id ${{ secrets.GCP_PROJECT }} \
            --dataset-id ${{ secrets.BQX_DATASET }} \
            --evaluator error-rate \
            --threshold 0.05 \
            --last 1h \
            --exit-code

      - name: Notify on failure
        if: failure()
        run: |
          curl -s -X POST "${{ secrets.SLACK_WEBHOOK }}" \
            -H 'Content-type: application/json' \
            -d '{"text": "Agent health check failed — investigate immediately."}'
```

### Step 5: Use CA for automated diagnosis

When an alert fires, use CA to generate a human-readable diagnosis:

```bash
bqx ca ask "Summarize the errors in the last hour and suggest root causes" \
  --agent=agent-analytics \
  --format text
```

## Decision rules

- Use `--last 1h` for real-time alerting, `--last 24h` for daily summaries
- Start with `error-rate` and `latency` checks — add `drift` once golden questions exist
- Use `--exit-code` so script exit status reflects check results
- Keep alert scripts idempotent — they may run on overlapping schedules
- Use `--agent-id` to set per-agent thresholds for high-priority agents

## Constraints

- Alerting integrations (Slack, PagerDuty) must be configured externally
- bqx is read-only — it detects issues but does not remediate them
- The CI service account needs `bigquery.dataViewer` and `bigquery.jobUser` roles only
- Alert frequency should match your data freshness — don't alert faster than events arrive
