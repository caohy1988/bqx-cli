---
name: bqx-schema
description: Inspect BigQuery table and view schemas, including column names, types, and descriptions. Use this when the user wants to understand the structure of a table before writing queries.
---

## When to use this skill

Use when the user asks about:
- "what columns does this table have"
- "show me the schema"
- "describe a BigQuery table"
- "what fields are in agent_events"

Do not use for listing tables in a dataset — use bqx-tables instead.
Do not use for querying data — use bqx-query or bqx-jobs instead.

## Prerequisites

See **bqx-shared** for authentication and global flags.

Requires: `--project-id` and `--dataset-id`

## Inspecting a table schema

### Using bqx tables get

The fastest way to inspect a schema is `bqx tables get`:

```bash
bqx tables get \
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
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT column_name, data_type, is_nullable, column_default FROM \`<PROJECT_ID>.<DATASET_ID>\`.INFORMATION_SCHEMA.COLUMNS WHERE table_name = '<TABLE_ID>' ORDER BY ordinal_position" \
  --format table
```

### Comparing schemas across tables

```bash
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT table_name, column_name, data_type FROM \`<PROJECT_ID>.<DATASET_ID>\`.INFORMATION_SCHEMA.COLUMNS WHERE table_name IN ('<TABLE_A>', '<TABLE_B>') ORDER BY table_name, ordinal_position" \
  --format table
```

## Decision rules

- Use `bqx tables get --format json` for a single table's full schema
- Use `bqx tables get --format table` for a quick visual overview
- Use INFORMATION_SCHEMA queries when comparing schemas or filtering by column properties
- Use `--selected-fields` with `bqx tables get` to limit response size for wide tables

## Examples

```bash
# Get schema for agent_events table
bqx tables get \
  --project-id my-proj \
  --dataset-id analytics \
  --table-id agent_events \
  --format table

# JSON schema for scripting
bqx tables get \
  --project-id my-proj \
  --dataset-id analytics \
  --table-id agent_events \
  --format json

# Find all STRING columns in a table
bqx jobs query \
  --project-id my-proj \
  --query "SELECT column_name FROM \`my-proj.analytics\`.INFORMATION_SCHEMA.COLUMNS WHERE table_name = 'agent_events' AND data_type = 'STRING'" \
  --format table
```

## Constraints

- `bqx tables get` returns the schema as part of the full table metadata response
- INFORMATION_SCHEMA queries require `bigquery.tables.get` IAM permission
- Nested/repeated field structures are shown in JSON format; table format flattens them
