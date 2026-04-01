# Trace Command Reference

## Usage

```bash
dcx analytics get-trace \
  --session-id <session-id> \
  [--trace-id <trace-id>] \
  [--format json|table|text]
```

## Flags

| Flag | Description |
|------|-------------|
| `--session-id` | Session ID to retrieve (required unless `--trace-id` is used) |
| `--trace-id` | Alias for `--session-id`; warns that dedicated trace-id lookup is planned |
| `--format` | `json` (default), `table`, or `text` |

## Getting a session ID

Session IDs come from `evaluate` or `list-traces` output:

```bash
# From evaluate
dcx analytics evaluate --evaluator latency --threshold 5000 --last 24h \
  --project-id my-proj --dataset-id analytics_demo --format json

# From list-traces
dcx analytics list-traces --last 24h \
  --project-id my-proj --dataset-id analytics_demo --format json
```

The `sessions` array (evaluate) or `traces` array (list-traces) contains
`session_id` values.

## Output formats

**JSON** — full structured trace with `session_id`, `agent`, `event_count`,
`started_at`, `ended_at`, `has_errors`, and `events` array. Each event
includes `event_type`, `timestamp`, `status`, `error_message`, `latency_ms`,
and `content`.

**Table** — columnar grid: summary header followed by timestamp, event_type,
status, latency_ms, error_message.

**Text** — timeline view: one line per event with timestamp, event type,
status, latency.

## Examples

```bash
dcx analytics get-trace --session-id adcp-a20d176b82af \
  --project-id my-proj --dataset-id analytics_demo --format text

dcx analytics get-trace --session-id adcp-a20d176b82af \
  --project-id my-proj --dataset-id analytics_demo --format table

# Using --trace-id (alias, warns at runtime)
dcx analytics get-trace --trace-id adcp-a20d176b82af \
  --project-id my-proj --dataset-id analytics_demo
```

## Notes

- Events are ordered by timestamp ascending
- `latency_ms` may be a JSON object (`{"total_ms": 3938}`) or scalar
- Returns error (exit code 2) if no events found for the session ID
