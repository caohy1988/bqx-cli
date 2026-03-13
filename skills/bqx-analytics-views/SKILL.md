---
name: bqx-analytics-views
description: Manage and query per-event-type BigQuery views over the agent_events table. Use this when the user wants to create, inspect, or query views that filter agent events by type.
---

## When to use this skill

Use when the user asks about:
- "create a view for error events"
- "query only latency events"
- "set up views for agent event types"
- "filter agent_events by event_type"

Do not use for raw SQL queries without event-type filtering — use bqx-query instead.

## Prerequisites

See **bqx-shared** for authentication and global flags.

Requires: `--project-id` and `--dataset-id`

## Approach

Per-event-type views provide filtered access to the `agent_events` table. Each
view selects rows matching a specific `event_type`, making it easier to query
subsets of events without repeating WHERE clauses.

## Creating event-type views

Use `bqx query` to create views for common event types:

### Session start events

```bash
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "CREATE OR REPLACE VIEW \`<PROJECT_ID>.<DATASET_ID>.v_session_start\` AS SELECT * FROM \`<PROJECT_ID>.<DATASET_ID>.agent_events\` WHERE event_type = 'SESSION_START'" \
  --format text
```

### Error events

```bash
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "CREATE OR REPLACE VIEW \`<PROJECT_ID>.<DATASET_ID>.v_errors\` AS SELECT * FROM \`<PROJECT_ID>.<DATASET_ID>.agent_events\` WHERE event_type LIKE '%_ERROR' OR status = 'ERROR'" \
  --format text
```

### Latency events

```bash
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "CREATE OR REPLACE VIEW \`<PROJECT_ID>.<DATASET_ID>.v_latency\` AS SELECT * FROM \`<PROJECT_ID>.<DATASET_ID>.agent_events\` WHERE latency_ms IS NOT NULL" \
  --format text
```

## Querying views

Once views exist, query them directly:

```bash
# Recent errors
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT session_id, timestamp, error_message FROM \`<PROJECT_ID>.<DATASET_ID>.v_errors\` ORDER BY timestamp DESC LIMIT 20" \
  --format table

# Latency distribution
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT agent, AVG(latency_ms) AS avg_latency, MAX(latency_ms) AS max_latency FROM \`<PROJECT_ID>.<DATASET_ID>.v_latency\` GROUP BY agent" \
  --format table
```

## Listing existing views

Use `bqx tables list` to discover views in a dataset:

```bash
bqx tables list \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --format table
```

Views appear alongside tables in the listing with type `VIEW`.

## Decision rules

- Create views for event types you query frequently
- Use `CREATE OR REPLACE VIEW` to update view definitions safely
- Use `bqx tables list` to discover existing views
- Use `--dry-run` on view creation queries to validate syntax first
- Prefix view names with `v_` to distinguish them from base tables

## Examples

```bash
# Create a view for all completion events
bqx jobs query \
  --project-id my-proj \
  --query "CREATE OR REPLACE VIEW \`my-proj.analytics.v_completions\` AS SELECT * FROM \`my-proj.analytics.agent_events\` WHERE event_type = 'COMPLETION'" \
  --format text

# Query the view
bqx jobs query \
  --project-id my-proj \
  --query "SELECT session_id, timestamp, content FROM \`my-proj.analytics.v_completions\` ORDER BY timestamp DESC LIMIT 10" \
  --format table
```

## Constraints

- View creation requires `bigquery.tables.create` IAM permission on the dataset
- Views are SQL-based filters, not materialized — query cost applies on each read
- View names must follow BigQuery naming rules (alphanumeric + underscores)
- Views created via `bqx query` are standard SQL views, not authorized views
