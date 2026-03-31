---
name: dcx-analytics
description: Agent analytics workflows — health checks, session evaluation, trace debugging, drift detection, and event views over BigQuery agent_events tables.
---

## When to use this skill

Use when the user wants to:
- Check agent health or table setup
- Evaluate sessions against latency or error-rate thresholds
- Debug a specific agent session trace
- Detect performance drift across time windows
- Create or query per-event-type BigQuery views
- Build monitoring or CI gates around agent quality

## Prerequisites

All analytics commands require `--project-id` and `--dataset-id`.
See **dcx-bigquery** for authentication and global flags.

## Command routing

| User goal | Command |
|-----------|---------|
| Health check on table setup | `dcx analytics doctor` |
| Gate sessions against a threshold | `dcx analytics evaluate --evaluator <latency\|error-rate> --threshold N --last <duration>` |
| Inspect a specific session | `dcx analytics get-trace --session-id <ID>` |
| Comprehensive insights report | `dcx analytics insights` |
| Event distribution analysis | `dcx analytics distribution` |
| Human-in-the-loop metrics | `dcx analytics hitl-metrics` |
| List recent traces | `dcx analytics list-traces` |

## Standard workflow

1. **Doctor** — verify table exists, has required columns, contains recent data
2. **Evaluate** — run latency or error-rate evaluator against recent sessions
3. **Get-trace** — inspect a specific session that failed evaluation

```bash
dcx analytics doctor --project-id my-proj --dataset-id analytics_demo --format text

dcx analytics evaluate --evaluator latency --threshold 5000 --last 24h \
  --project-id my-proj --dataset-id analytics_demo --format text

dcx analytics get-trace --session-id adcp-a20d176b82af \
  --project-id my-proj --dataset-id analytics_demo --format table
```

## Drift detection

Compare metrics across time windows to detect regressions. See `references/drift.md`.

```bash
dcx analytics evaluate --evaluator latency --threshold 5000 --last 7d --format json > baseline.json
dcx analytics evaluate --evaluator latency --threshold 5000 --last 24h --format json > current.json
```

A pass-rate drop >10% or average latency increase >2x indicates drift.

## Decision rules

- Start with `doctor` if unsure whether the table is set up
- Use `evaluate` to find failing sessions, `get-trace` to dig into one
- Add `--exit-code` to `evaluate` in CI pipelines to fail builds on threshold violations
- Add `--agent-id` to scope evaluation to a specific agent
- `--format text` for interactive work; `--format json` for automation

## Constraints

- Table defaults to `agent_events` (override with `--table`)
- Required columns: `session_id`, `agent`, `event_type`, `timestamp`
- Location defaults to `US`; duration format: `<number><unit>` (h/d/m)

## References

- `references/evaluate.md` — evaluate command flags and output formats
- `references/trace.md` — get-trace command and output formats
- `references/drift.md` — drift detection workflow
- `references/views.md` — per-event-type BigQuery view creation
