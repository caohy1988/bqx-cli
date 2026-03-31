# Evaluate Command Reference

## Usage

```bash
dcx analytics evaluate \
  --evaluator <latency|error-rate> \
  --threshold <number> \
  --last <duration> \
  [--agent-id <name>] \
  [--exit-code] \
  [--format json|table|text]
```

## Required flags

| Flag | Description |
|------|-------------|
| `--evaluator` | `latency` or `error-rate` |
| `--threshold` | Milliseconds for latency (e.g. `5000`), ratio 0-1 for error-rate (e.g. `0.1`) |
| `--last` | Time window: `1h`, `24h`, `7d`, `30m`, etc. |

## Optional flags

| Flag | Description |
|------|-------------|
| `--agent-id` | Filter to a specific agent |
| `--exit-code` | Return exit code 1 if any session fails (for CI) |
| `--format` | `json` (default), `table`, or `text` |

## Evaluator definitions

**Latency**: compares each session's maximum `latency_ms.total_ms` against threshold.
A session passes if max latency is at or below threshold.

**Error-rate**: computes each session's error ratio (`error_events / total_events`).
A session passes if error rate is at or below threshold.

## Output

**Text** shows evaluator name, threshold, time window, session counts (total/passed/failed/pass rate), and worst sessions.

**JSON** includes all fields plus a `sessions` array with per-session detail.

## Examples

```bash
# CI gate: fail if any session exceeds 3s in the last hour
dcx analytics evaluate --evaluator latency --threshold 3000 --last 1h \
  --exit-code --project-id my-proj --dataset-id analytics_demo --format json

# Error rate for a specific agent over 7 days
dcx analytics evaluate --evaluator error-rate --threshold 0.1 --last 7d \
  --agent-id sales_agent --project-id my-proj --dataset-id analytics_demo --format text
```
