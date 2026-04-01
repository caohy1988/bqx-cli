---
name: dcx-analytics
description: Agent analytics workflows — health checks, session evaluation, trace debugging, drift detection, event views, and categorical evaluation over BigQuery agent_events tables.
---

## When to use this skill

Use when the user wants to:
- Check agent health or table setup
- Evaluate sessions against thresholds (latency, error-rate, turn-count, token-efficiency, ttft, cost)
- Debug a specific agent session trace
- Detect performance drift against a golden question set
- Analyze event distribution patterns
- Monitor human-in-the-loop interactions
- Create or query per-event-type BigQuery views
- Run categorical (LLM-based) evaluation over traces
- Build monitoring or CI gates around agent quality

## Prerequisites

All analytics commands require `--project-id` and `--dataset-id`.
See **dcx-bigquery** for authentication and global flags.

## Command routing

| User goal | Command |
|-----------|---------|
| Health check on table setup | `dcx analytics doctor` |
| Gate sessions against a threshold | `dcx analytics evaluate --evaluator <type> --threshold N --last <duration>` |
| Inspect a specific session | `dcx analytics get-trace --session-id <ID>` |
| List recent sessions | `dcx analytics list-traces --last <duration>` |
| Comprehensive insights report | `dcx analytics insights --last <duration>` |
| Drift detection vs golden dataset | `dcx analytics drift --golden-dataset <table> --last <duration>` |
| Event distribution analysis | `dcx analytics distribution --last <duration>` |
| Human-in-the-loop metrics | `dcx analytics hitl-metrics --last <duration>` |
| Create all event-type views | `dcx analytics views create-all [--prefix <prefix>]` |
| Create single event-type view | `dcx analytics views create <EVENT_TYPE> [--prefix <prefix>]` |
| Categorical (LLM) evaluation | `dcx analytics categorical-eval --metrics-file <path>` |
| Categorical dashboard views | `dcx analytics categorical-views` |

## Evaluators

| Evaluator | Threshold unit | Description |
|-----------|---------------|-------------|
| `latency` | milliseconds | Max session latency vs threshold |
| `error-rate` | ratio (0–1) | Session error ratio vs threshold |
| `turn-count` | count | Human turns per session vs threshold |
| `token-efficiency` | count | Total tokens used per session vs threshold |
| `ttft` | milliseconds | Time-to-first-token vs threshold |
| `cost` | USD | Session cost vs threshold |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success (all sessions passed, or command completed normally) |
| 1 | Evaluation failure (one or more sessions exceeded threshold; requires `--exit-code`) |
| 2 | Infrastructure error (connection, auth, bad input) |

## Standard workflow

1. **Doctor** — verify table exists, has required columns, contains recent data
2. **Evaluate** — run evaluator against recent sessions
3. **Get-trace** — inspect a specific session that failed evaluation

```bash
dcx analytics doctor --project-id my-proj --dataset-id analytics_demo --format text

dcx analytics evaluate --evaluator latency --threshold 5000 --last 24h \
  --project-id my-proj --dataset-id analytics_demo --exit-code

dcx analytics get-trace --session-id adcp-a20d176b82af \
  --project-id my-proj --dataset-id analytics_demo --format table
```

## Drift detection

Compare against a golden question set:

```bash
dcx analytics drift --golden-dataset golden_questions --last 7d \
  --min-coverage 0.8 --exit-code \
  --project-id PROJECT --dataset-id DATASET
```

See `references/drift.md`.

## Decision rules

- Start with `doctor` if unsure whether the table is set up
- Use `evaluate` to find failing sessions, `get-trace` to dig into one
- Add `--exit-code` to `evaluate` or `drift` in CI to fail builds on threshold violations
- Add `--agent-id` to scope evaluation to a specific agent
- Use `--limit` to cap the number of sessions evaluated
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
