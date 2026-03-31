# Per-Event-Type Views

Views provide filtered access to `agent_events` without repeating WHERE clauses.

## Creating views

```bash
# Session start events
dcx jobs query --project-id PROJECT \
  --query "CREATE OR REPLACE VIEW \`PROJECT.DATASET.v_session_start\` AS
    SELECT * FROM \`PROJECT.DATASET.agent_events\` WHERE event_type = 'SESSION_START'" \
  --format text

# Error events
dcx jobs query --project-id PROJECT \
  --query "CREATE OR REPLACE VIEW \`PROJECT.DATASET.v_errors\` AS
    SELECT * FROM \`PROJECT.DATASET.agent_events\`
    WHERE event_type LIKE '%_ERROR' OR status = 'ERROR'" \
  --format text

# Latency events
dcx jobs query --project-id PROJECT \
  --query "CREATE OR REPLACE VIEW \`PROJECT.DATASET.v_latency\` AS
    SELECT * FROM \`PROJECT.DATASET.agent_events\` WHERE latency_ms IS NOT NULL" \
  --format text
```

## Querying views

```bash
dcx jobs query --project-id PROJECT \
  --query "SELECT session_id, timestamp, error_message
    FROM \`PROJECT.DATASET.v_errors\` ORDER BY timestamp DESC LIMIT 20" \
  --format table
```

## Listing views

```bash
dcx tables list --project-id PROJECT --dataset-id DATASET --format table
```

Views appear alongside tables with type `VIEW`.

## Notes

- Prefix view names with `v_` to distinguish from base tables
- Use `--dry-run` to validate view creation SQL first
- Views are SQL-based filters, not materialized — query cost applies on each read
- View creation requires `bigquery.tables.create` IAM permission
