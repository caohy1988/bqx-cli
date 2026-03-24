---
name: dcx-schema
description: Inspect BigQuery table and view schemas, including column names, types, and descriptions. Use this when the user wants to understand the structure of a table before writing queries.
---

## When to use this skill

Use when the user asks about:
- "what columns does this table have"
- "show me the schema"
- "describe a BigQuery table"
- "what fields are in agent_events"

Do not use for listing tables in a dataset — use dcx-tables instead.
Do not use for querying data — use dcx-query or dcx-jobs instead.

## Prerequisites

See **dcx-shared** for authentication and global flags.

Requires: `--project-id` and `--dataset-id`

## Inspecting a table schema

### Using dcx tables get

The fastest way to inspect a schema is `dcx tables get`:

```bash
dcx tables get \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --table-id <TABLE_ID> \
  --format json
```

The response includes a `schema.fields` array with column name, type, mode,
and description for each field.

### Using INFORMATION_SCHEMA

For richer metadata or cross-table schema queries:

```bash
dcx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT column_name, data_type, is_nullable, column_default FROM \`<PROJECT_ID>.<DATASET_ID>\`.INFORMATION_SCHEMA.COLUMNS WHERE table_name = '<TABLE_ID>' ORDER BY ordinal_position" \
  --format table
```

### Comparing schemas across tables

```bash
dcx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT table_name, column_name, data_type FROM \`<PROJECT_ID>.<DATASET_ID>\`.INFORMATION_SCHEMA.COLUMNS WHERE table_name IN ('<TABLE_A>', '<TABLE_B>') ORDER BY table_name, ordinal_position" \
  --format table
```

## Decision rules

- Use `dcx tables get --format json` for a single table's full schema
- Use `dcx tables get --format table` for a quick visual overview
- Use INFORMATION_SCHEMA queries when comparing schemas or filtering by column properties
- Use `--selected-fields` with `dcx tables get` to limit response size for wide tables

## Examples

```bash
# Get schema for agent_events table
dcx tables get \
  --project-id my-proj \
  --dataset-id analytics \
  --table-id agent_events \
  --format table

# JSON schema for scripting
dcx tables get \
  --project-id my-proj \
  --dataset-id analytics \
  --table-id agent_events \
  --format json

# Find all STRING columns in a table
dcx jobs query \
  --project-id my-proj \
  --query "SELECT column_name FROM \`my-proj.analytics\`.INFORMATION_SCHEMA.COLUMNS WHERE table_name = 'agent_events' AND data_type = 'STRING'" \
  --format table
```

## Constraints

- `dcx tables get` returns the schema as part of the full table metadata response
- INFORMATION_SCHEMA queries require `bigquery.tables.get` IAM permission
- Nested/repeated field structures are shown in JSON format; table format flattens them
