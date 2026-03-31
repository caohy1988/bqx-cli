# Trace Command Reference

## Usage

```bash
dcx analytics get-trace \
  --session-id <session-id> \
  [--format json|table|text]
```

## Getting a session ID

Session IDs come from `evaluate` output:

```bash
dcx analytics evaluate --evaluator latency --threshold 5000 --last 24h \
  --project-id my-proj --dataset-id analytics_demo --format json
```

The `sessions` array contains `session_id` values.

## Output formats

**Text** — timeline view: one line per event with timestamp, event type, status, latency.

**Table** — columnar grid: summary header followed by timestamp, event_type, status, latency_ms, error_message.

**JSON** — full structured trace with `session_id`, `agent`, `event_count`, `started_at`, `ended_at`, `has_errors`, and `events` array.

## Examples

```bash
dcx analytics get-trace --session-id adcp-a20d176b82af \
  --project-id my-proj --dataset-id analytics_demo --format text

dcx analytics get-trace --session-id adcp-a20d176b82af \
  --project-id my-proj --dataset-id analytics_demo --format table
```

## Notes

- Events are ordered by timestamp ascending
- `latency_ms` may be a JSON object (`{"total_ms": 3938}`) or scalar
- Returns error if no events found for the session ID
