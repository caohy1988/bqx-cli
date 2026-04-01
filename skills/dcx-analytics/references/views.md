# Per-Event-Type Views

The `views` command creates BigQuery views that filter `agent_events` by event type.

## Create all views

```bash
dcx analytics views create-all \
  --project-id PROJECT --dataset-id DATASET \
  [--prefix <prefix>]
```

Creates 18 views, one per standard event type:

| View name | Event type |
|-----------|-----------|
| `llm_request` | `LLM_REQUEST` |
| `llm_response` | `LLM_RESPONSE` |
| `tool_starting` | `TOOL_STARTING` |
| `tool_completed` | `TOOL_COMPLETED` |
| `tool_error` | `TOOL_ERROR` |
| `tool_call` | `TOOL_CALL` |
| `tool_response` | `TOOL_RESPONSE` |
| `agent_run_start` | `AGENT_RUN_START` |
| `agent_run_end` | `AGENT_RUN_END` |
| `agent_run_error` | `AGENT_RUN_ERROR` |
| `invocation_start` | `INVOCATION_START` |
| `invocation_completed` | `INVOCATION_COMPLETED` |
| `invocation_error` | `INVOCATION_ERROR` |
| `human_input_required` | `HUMAN_INPUT_REQUIRED` |
| `human_input_received` | `HUMAN_INPUT_RECEIVED` |
| `session_start` | `SESSION_START` |
| `session_end` | `SESSION_END` |
| `session_error` | `SESSION_ERROR` |

With `--prefix adk_`, views are named `adk_llm_request`, `adk_tool_completed`, etc.

## Create a single view

```bash
dcx analytics views create <EVENT_TYPE> \
  --project-id PROJECT --dataset-id DATASET \
  [--prefix <prefix>]
```

Creates one view for the specified event type. Custom event types are accepted
with a warning.

## Examples

```bash
# Create all 18 views with prefix
dcx analytics views create-all --project-id my-proj --dataset-id analytics --prefix adk_

# Create a single view
dcx analytics views create LLM_REQUEST --project-id my-proj --dataset-id analytics

# Query a view
dcx jobs query --project-id my-proj \
  --query "SELECT session_id, timestamp FROM \`my-proj.analytics.adk_tool_error\` LIMIT 20" \
  --format table
```

## Listing views

```bash
dcx tables list --project-id PROJECT --dataset-id DATASET --format table
```

Views appear alongside tables with type `VIEW`.

## Notes

- Use `--prefix` to namespace views and avoid collisions
- Views are SQL-based filters, not materialized — query cost applies on each read
- View creation requires `bigquery.tables.create` IAM permission
- Use `--dry-run` to validate SQL before executing
