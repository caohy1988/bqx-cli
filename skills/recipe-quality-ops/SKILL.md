---
name: recipe-quality-ops
description: Recipes for agent quality operations — CI evaluation gates, drift monitoring, error alerting, quality dashboards, and self-diagnostic loops.
---

## When to use this skill

Use when the user wants to:
- Set up CI/CD evaluation gates for agent quality
- Build automated drift monitoring
- Configure error alerting pipelines
- Create quality dashboards with BigQuery views
- Build self-diagnostic agent loops

## Recipe: CI evaluation gate

Block deployments when agent quality drops below thresholds.

### Step 1: Add evaluation step to CI

```bash
dcx analytics evaluate \
  --evaluator latency --threshold 3000 --last 1h \
  --exit-code \
  --project-id PROJECT --dataset-id DATASET --format json
```

`--exit-code` returns exit code 1 if any session fails — use as CI gate.

### Step 2: Add error-rate gate

```bash
dcx analytics evaluate \
  --evaluator error-rate --threshold 0.05 --last 1h \
  --exit-code \
  --project-id PROJECT --dataset-id DATASET --format json
```

### Step 3: Capture results as artifacts

```bash
dcx analytics evaluate --evaluator latency --threshold 3000 --last 1h \
  --format json > eval-results.json
```

## Recipe: Weekly drift monitoring

### Step 1: Capture baseline

```bash
dcx analytics evaluate --evaluator latency --threshold 5000 --last 30d \
  --format json > baseline.json
```

### Step 2: Capture current window

```bash
dcx analytics evaluate --evaluator latency --threshold 5000 --last 7d \
  --format json > current.json
```

### Step 3: Compare and alert

Compare pass rates between baseline and current. Alert if:
- Pass rate drops >10%
- Average latency increases >2x
- Error rate increases >5 percentage points

### Cron setup

Run weekly via cron or CI scheduled pipeline:

```bash
# drift-check.sh
#!/bin/bash
dcx analytics evaluate --evaluator latency --threshold 5000 --last 7d \
  --project-id PROJECT --dataset-id DATASET --format json > /tmp/drift.json

PASS_RATE=$(jq '.pass_rate' /tmp/drift.json)
if (( $(echo "$PASS_RATE < 0.9" | bc -l) )); then
  echo "DRIFT ALERT: pass rate $PASS_RATE" | # pipe to Slack/PagerDuty
fi
```

## Recipe: Error alerting

### Step 1: Check error rate

```bash
dcx analytics evaluate --evaluator error-rate --threshold 0.1 --last 1h \
  --project-id PROJECT --dataset-id DATASET --format json
```

### Step 2: Get failing sessions

```bash
dcx analytics evaluate --evaluator error-rate --threshold 0.1 --last 1h \
  --format json | jq '.sessions[] | select(.passed == false) | .session_id'
```

### Step 3: Inspect worst session

```bash
dcx analytics get-trace --session-id SESSION_ID \
  --project-id PROJECT --dataset-id DATASET --format text
```

### Step 4: Natural language investigation

```bash
dcx ca ask "what are the most common errors in the last hour?" \
  --agent=agent-analytics --format text
```

## Recipe: Quality dashboard views

Create BigQuery views for dashboarding:

```bash
# Hourly error rate
dcx jobs query --project-id PROJECT --query "
  CREATE OR REPLACE VIEW \`PROJECT.DATASET.v_hourly_errors\` AS
  SELECT
    TIMESTAMP_TRUNC(timestamp, HOUR) AS hour,
    agent,
    COUNTIF(status = 'ERROR') AS errors,
    COUNT(*) AS total,
    SAFE_DIVIDE(COUNTIF(status = 'ERROR'), COUNT(*)) AS error_rate
  FROM \`PROJECT.DATASET.agent_events\`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 30 DAY)
  GROUP BY 1, 2
" --format text

# Session latency summary
dcx jobs query --project-id PROJECT --query "
  CREATE OR REPLACE VIEW \`PROJECT.DATASET.v_session_latency\` AS
  SELECT
    session_id, agent,
    MAX(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS FLOAT64)) AS max_latency_ms,
    MIN(timestamp) AS started, MAX(timestamp) AS ended
  FROM \`PROJECT.DATASET.agent_events\`
  WHERE latency_ms IS NOT NULL
  GROUP BY 1, 2
" --format text
```

Connect these views to Looker Studio or any BI tool.

## Decision rules

- Use `--exit-code` for CI gates — it fails the step on threshold violation
- Use 7d baseline vs 24h current for daily drift; 30d vs 7d for weekly
- Fix structural issues first (doctor), then evaluate, then trace
- Use `--format json` for all automated pipelines
- Create views for metrics you query repeatedly
