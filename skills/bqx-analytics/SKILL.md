---
name: bqx-analytics
description: Top-level routing for bqx agent analytics workflows. Use this when the user wants to check agent health, evaluate sessions, or debug agent behavior in BigQuery, and you need to decide which analytics subcommand to use.
---

## When to use this skill

Use when the user asks about:
- "use bqx to check agent analytics"
- "find bad sessions"
- "debug agent behavior in BigQuery"
- "what analytics commands are available"
- "how do I monitor agents with bqx"
- "check my agent health"

Do not use when the user already knows which specific command they need — use bqx-analytics-evaluate or bqx-analytics-trace instead.

## Prerequisites

See **bqx-shared** for authentication and global flags.

All analytics commands require:
- `--project-id` (or `BQX_PROJECT`)
- `--dataset-id` (or `BQX_DATASET`)

## Command routing

| User goal | Command | Skill |
|-----------|---------|-------|
| Health check on table setup | `analytics doctor` | (this skill) |
| Policy gate: pass/fail sessions against a threshold | `analytics evaluate` | bqx-analytics-evaluate |
| Session-level debugging | `analytics get-trace` | bqx-analytics-trace |
| Drift detection against golden questions | `analytics drift` | bqx-analytics-drift |
| Per-event-type BigQuery views | `analytics views create-all` | bqx-analytics-views |
| Comprehensive insights report | `analytics insights` | (this skill) |
| Event distribution analysis | `analytics distribution` | (this skill) |
| Human-in-the-loop metrics | `analytics hitl-metrics` | (this skill) |
| List recent traces | `analytics list-traces` | bqx-analytics-trace |

## Core workflow

The standard analytics workflow is:

1. **Doctor** — verify the table exists, has required columns, and contains recent data
2. **Evaluate** — run a latency or error-rate evaluator against recent sessions
3. **Get-trace** — inspect a specific session that failed evaluation

### Step 1: Doctor

Checks table schema, row counts, null values, and data freshness.

```bash
bqx analytics doctor \
  --project-id my-proj \
  --dataset-id analytics_demo \
  --format text
```

Returns: status (healthy/warning/error), row counts, column validation, warnings.

### Step 2: Evaluate

Runs a latency or error-rate evaluator against sessions in a time window.

```bash
bqx analytics evaluate \
  --evaluator latency \
  --threshold 5000 \
  --last 24h \
  --project-id my-proj \
  --dataset-id analytics_demo \
  --format text
```

Returns: pass/fail for each session, aggregate pass rate.

### Step 3: Get-trace

Retrieves the full event trace for a specific session.

```bash
bqx analytics get-trace \
  --session-id adcp-a20d176b82af \
  --project-id my-proj \
  --dataset-id analytics_demo \
  --format table
```

Returns: ordered events with timestamps, status, latency, errors.

## Decision rules

- Start with `doctor` if unsure whether the table is set up correctly
- Use `evaluate` to find which sessions are failing a threshold
- Use `get-trace` to dig into a specific session ID from evaluate output
- `--format text` is best for interactive exploration; `--format json` for scripts

## Constraints

- The table defaults to `agent_events` (override with `--table`)
- Required table columns: `session_id`, `agent`, `event_type`, `timestamp`
- Location defaults to `US` (override with `--location`)
