---
name: dcx-analytics-drift
description: Detect behavioral drift in agent sessions over time. Use this when the user wants to compare agent performance across time windows or detect regressions.
---

## When to use this skill

Use when the user asks about:
- "has my agent's performance changed"
- "detect drift in agent behavior"
- "compare agent metrics across time windows"
- "check for regressions in agent latency or error rate"

Do not use for one-time evaluations — use dcx-analytics-evaluate instead.

## Prerequisites

See **dcx-shared** for authentication and global flags.

Requires: `--project-id` and `--dataset-id`

## Approach

Drift detection compares agent metrics across two time windows: a **baseline**
period and a **current** period. Use `dcx analytics evaluate` to capture metrics
for each window, then compare results to identify regressions.

## Workflow

### Step 1: Establish baseline metrics

```bash
dcx analytics evaluate \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --evaluator latency \
  --threshold 5000 \
  --last 7d \
  --format json > baseline.json
```

### Step 2: Capture current metrics

```bash
dcx analytics evaluate \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --evaluator latency \
  --threshold 5000 \
  --last 24h \
  --format json > current.json
```

### Step 3: Compare windows

Compare the pass rate, average latency, and error counts between baseline and
current output files. A significant degradation (e.g., pass rate drops >10% or
average latency increases >2x) indicates drift.

### Error rate drift

Repeat the same workflow with `--evaluator error-rate`:

```bash
# Baseline error rate
dcx analytics evaluate \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --evaluator error-rate \
  --threshold 0.1 \
  --last 7d \
  --format json > baseline_errors.json

# Current error rate
dcx analytics evaluate \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --evaluator error-rate \
  --threshold 0.1 \
  --last 24h \
  --format json > current_errors.json
```

## Decision rules

- Use 7d baseline vs 24h current for daily drift checks
- Use 30d baseline vs 7d current for weekly drift reports
- A pass rate drop of >10% warrants investigation
- Use `--agent-id` to isolate drift to a specific agent
- Use `--format json` to enable programmatic comparison

## Examples

```bash
# Weekly drift check for a specific agent
dcx analytics evaluate \
  --project-id my-proj \
  --dataset-id analytics \
  --evaluator latency \
  --threshold 3000 \
  --last 7d \
  --agent-id my-agent \
  --format json

# Quick 24h latency snapshot
dcx analytics evaluate \
  --project-id my-proj \
  --dataset-id analytics \
  --evaluator latency \
  --threshold 5000 \
  --last 24h \
  --format table
```

## Constraints

- Drift detection is a workflow pattern using `dcx analytics evaluate`, not a standalone command
- Comparison logic must be done outside dcx (e.g., in a script or CI pipeline)
- Time window granularity depends on the density of events in the agent_events table
- The `--last` flag accepts durations like 1h, 24h, 7d, 30d
