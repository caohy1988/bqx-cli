---
name: bqx-analytics-trace
description: Use bqx to retrieve and inspect the event trace for a specific agent session. Use this when the user wants to debug a failed session, inspect event ordering, or understand what happened during a session.
---

## When to use this skill

Use when the user asks about:
- "show me the trace for this session"
- "inspect session adcp-a20d176b82af"
- "debug a failed session"
- "what happened in session X"
- "show me the events for a session"

Do not use when the user wants to evaluate many sessions at once — use bqx-analytics-evaluate instead.

## Prerequisites

See **bqx-shared** for authentication and global flags.

Requires: `--project-id`, `--dataset-id`

## Core workflow

```bash
bqx analytics get-trace \
  --session-id <session-id> \
  [--format json|table|text]
```

### Getting a session ID

Session IDs typically come from evaluate output. Run evaluate first to find sessions that failed:

```bash
# Find failing sessions
bqx analytics evaluate \
  --evaluator latency \
  --threshold 5000 \
  --last 24h \
  --project-id my-proj \
  --dataset-id analytics_demo \
  --format json
```

The `sessions` array in the output contains `session_id` values you can pass to `get-trace`.

### Running the trace

```bash
bqx analytics get-trace \
  --session-id adcp-a20d176b82af \
  --project-id my-proj \
  --dataset-id analytics_demo \
  --format text
```

## Decision rules

- Use `--format text` for a timeline view of events with latency and status
- Use `--format table` for a columnar event grid (timestamp, event_type, status, latency, errors)
- Use `--format json` for the full structured trace (includes content field if present)
- The table format shows a summary header (session, agent, event count, errors) followed by an event grid

## Reading the output

**Text format** shows:
- Session ID and agent name
- Event count and whether errors were detected
- One line per event: timestamp, event type, status, and latency (if present)

**Table format** shows:
- Summary line: session, agent, event count, error flag, time range
- Columnar grid: timestamp, event_type, status, latency_ms, error_message

**JSON format** includes:
- `session_id`, `agent`, `event_count`, `started_at`, `ended_at`, `has_errors`
- `events` array with full detail per event (including `content` and `latency_ms` objects)

## Examples

```bash
# Text timeline view
bqx analytics get-trace \
  --session-id adcp-a20d176b82af \
  --project-id my-proj \
  --dataset-id analytics_demo \
  --format text

# Table event grid
bqx analytics get-trace \
  --session-id adcp-a20d176b82af \
  --project-id my-proj \
  --dataset-id analytics_demo \
  --format table
```

## Constraints

- Session IDs must be alphanumeric with underscores, dots, and hyphens
- Returns an error if no events are found for the given session ID
- Events are ordered by timestamp ascending
- The `latency_ms` field may contain a JSON object (e.g. `{"total_ms": 3938}`) or a scalar
