# Evaluate Command Reference

## Usage

```bash
dcx analytics evaluate \
  --evaluator <latency|error-rate|turn-count|token-efficiency|ttft|cost> \
  --threshold <number> \
  --last <duration> \
  [--agent-id <name>] \
  [--limit <n>] \
  [--exit-code] \
  [--criterion <name>] \
  [--strict] \
  [--format json|table|text]
```

## Required flags

| Flag | Description |
|------|-------------|
| `--evaluator` | One of: `latency`, `error-rate`, `turn-count`, `token-efficiency`, `ttft`, `cost` |
| `--threshold` | Threshold value (unit depends on evaluator; see below) |
| `--last` | Time window: `1h`, `24h`, `7d`, `30d`, etc. |

## Optional flags

| Flag | Default | Description |
|------|---------|-------------|
| `--agent-id` | — | Filter to a specific agent |
| `--limit` | 100 | Maximum number of sessions to evaluate |
| `--exit-code` | — | Return exit code 1 if any session fails (for CI) |
| `--criterion` | `correctness` | Accepted for SDK CLI parity; no effect on code evaluators (applies to llm-judge only) |
| `--strict` | — | Accepted for SDK CLI parity; no effect on code evaluators (applies to llm-judge only) |
| `--format` | `json` | `json`, `table`, or `text` |

## Evaluator definitions

| Evaluator | Threshold unit | Pass condition |
|-----------|---------------|----------------|
| `latency` | milliseconds | Max session latency ≤ threshold |
| `error-rate` | ratio (0–1) | Session error ratio ≤ threshold |
| `turn-count` | count | Human turns in session ≤ threshold |
| `token-efficiency` | count | Total tokens used ≤ threshold |
| `ttft` | milliseconds | Time-to-first-token ≤ threshold |
| `cost` | USD | Session cost ≤ threshold |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All sessions passed the threshold |
| 1 | One or more sessions failed (requires `--exit-code` flag) |
| 2 | Infrastructure error (auth, connection, bad input) |

## Output

**JSON** includes: `evaluator`, `threshold`, `time_window`, `agent_id`,
`total_sessions`, `passed`, `failed`, `pass_rate`, and a `sessions` array
with per-session `session_id`, `agent`, `passed`, `score`.

**Table** shows a columnar grid of per-session results.

**Text** shows evaluator name, threshold, time window, session counts, and worst sessions.

## Examples

```bash
# CI gate: fail if any session exceeds 3s latency in the last hour
dcx analytics evaluate --evaluator latency --threshold 3000 --last 1h \
  --exit-code --project-id my-proj --dataset-id analytics_demo

# Error rate for a specific agent over 7 days
dcx analytics evaluate --evaluator error-rate --threshold 0.1 --last 7d \
  --agent-id sales_agent --project-id my-proj --dataset-id analytics_demo

# TTFT check with limited sessions
dcx analytics evaluate --evaluator ttft --threshold 2000 --last 24h \
  --limit 50 --project-id my-proj --dataset-id analytics_demo

# Cost evaluation
dcx analytics evaluate --evaluator cost --threshold 1.0 --last 7d \
  --project-id my-proj --dataset-id analytics_demo --format table

# Token efficiency check
dcx analytics evaluate --evaluator token-efficiency --threshold 50000 \
  --last 24h --exit-code --project-id my-proj --dataset-id analytics_demo
```
