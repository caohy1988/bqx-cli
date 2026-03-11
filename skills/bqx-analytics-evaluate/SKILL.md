---
name: bqx-analytics-evaluate
description: Use bqx to evaluate agent sessions against latency or error-rate thresholds. Use this when the user wants to check session quality, gate CI on agent performance, or find failing sessions.
---

## When to use this skill

Use when the user asks about:
- "check latency for the last 24h"
- "fail CI if error rate is too high"
- "evaluate agent sessions"
- "which sessions are too slow"
- "set a latency threshold for agents"

Do not use when the user wants to inspect a single session — use bqx-analytics-trace instead.

## Prerequisites

See **bqx-shared** for authentication and global flags.

Requires: `--project-id`, `--dataset-id`

## Core workflow

```bash
bqx analytics evaluate \
  --evaluator <latency|error-rate> \
  --threshold <number> \
  --last <duration> \
  [--agent-id <name>] \
  [--exit-code] \
  [--format json|table|text]
```

### Required flags

| Flag | Description |
|------|-------------|
| `--evaluator` | `latency` or `error-rate` |
| `--threshold` | Milliseconds for latency (e.g. `5000`), ratio 0-1 for error-rate (e.g. `0.1`) |
| `--last` | Time window: `1h`, `24h`, `7d`, `30m`, etc. |

### Optional flags

| Flag | Description |
|------|-------------|
| `--agent-id` | Filter to a specific agent name |
| `--exit-code` | Return exit code 1 if any session fails (for CI) |
| `--format` | `json` (default), `table`, or `text` |

## Evaluator definitions

**Latency**: compares each session's maximum `latency_ms.total_ms` against the threshold. A session passes if its max latency is at or below the threshold.

**Error-rate**: computes each session's error ratio (`error_events / total_events`). A session passes if its error rate is at or below the threshold.

## Decision rules

- Use `latency` to catch slow sessions (threshold in milliseconds)
- Use `error-rate` to catch unreliable sessions (threshold as a decimal, e.g. `0.1` = 10%)
- Add `--agent-id` to scope evaluation to a single agent
- Add `--exit-code` in CI pipelines to fail the build on threshold violations
- Use `--format text` for interactive review; `--format json` for automation
- Sessions with no latency data are marked as failed by the latency evaluator

## Examples

```bash
# Check latency over the last 24 hours
bqx analytics evaluate \
  --evaluator latency \
  --threshold 5000 \
  --last 24h \
  --project-id my-proj \
  --dataset-id analytics_demo

# Check error rate for a specific agent over 7 days
bqx analytics evaluate \
  --evaluator error-rate \
  --threshold 0.1 \
  --last 7d \
  --agent-id sales_agent \
  --project-id my-proj \
  --dataset-id analytics_demo \
  --format text

# CI gate: fail if any session exceeds 3s latency in the last hour
bqx analytics evaluate \
  --evaluator latency \
  --threshold 3000 \
  --last 1h \
  --exit-code \
  --project-id my-proj \
  --dataset-id analytics_demo \
  --format json
```

## Reading the output

**Text format** shows:
- Evaluator name, threshold, and time window
- Session counts: total, passed, failed, pass rate
- List of worst (failed) sessions with their scores

**JSON format** includes all fields plus a `sessions` array with per-session detail.

## Constraints

- Duration format: `<number><unit>` where unit is `h` (hours), `d` (days), or `m` (minutes)
- Agent IDs must be alphanumeric with underscores, dots, and hyphens
- The evaluator queries the table specified by `--table` (default: `agent_events`)
