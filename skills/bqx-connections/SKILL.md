---
name: bqx-connections
description: Inspect BigQuery external connections and their metadata. Use this when the user wants to discover or understand external connections configured in their project.
---

## When to use this skill

Use when the user asks about:
- "list BigQuery connections"
- "what connections are in my project"
- "show me remote function connections"
- "check connection configuration"

Do not use when the user wants to create or modify connections — that requires
the BigQuery Connection API which is outside the bigquery v2 scope.

## Prerequisites

See **bqx-shared** for authentication and global flags.

Requires: `--project-id`

## Approach

BigQuery connections are managed by the separate BigQuery Connection API
(`bigqueryconnection.googleapis.com`), not the core BigQuery v2 API. To inspect
connection metadata through bqx, use `bqx query` with `INFORMATION_SCHEMA` views.

## Inspecting connections

### List connections in a region

```bash
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT * FROM \`region-us\`.INFORMATION_SCHEMA.OBJECT_PRIVILEGES WHERE object_type = 'CONNECTION'" \
  --format table
```

### List remote functions and their connections

```bash
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT routine_name, routine_type, remote_function_info FROM \`<PROJECT_ID>.<DATASET_ID>\`.INFORMATION_SCHEMA.ROUTINES WHERE routine_type = 'FUNCTION' AND remote_function_info IS NOT NULL" \
  --format table
```

### Check connection details via routine metadata

```bash
bqx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT routine_name, remote_function_info.connection FROM \`<PROJECT_ID>.<DATASET_ID>\`.INFORMATION_SCHEMA.ROUTINES WHERE remote_function_info.connection IS NOT NULL" \
  --format json
```

## Decision rules

- Use `INFORMATION_SCHEMA` queries to discover connection usage
- Use `--format json` to capture connection references for downstream scripts
- Use `--format table` for visual inspection of connection metadata
- Connection creation/deletion requires the BigQuery Connection API directly

## Examples

```bash
# Find all remote functions and their connection references
bqx jobs query \
  --project-id my-proj \
  --query "SELECT routine_name, remote_function_info FROM \`my-proj.my_dataset\`.INFORMATION_SCHEMA.ROUTINES WHERE remote_function_info IS NOT NULL" \
  --format table
```

## Constraints

- Connection management (create/delete/update) is not available through bqx — it requires the BigQuery Connection API
- INFORMATION_SCHEMA queries require appropriate IAM permissions on the project
- Connection details visible through INFORMATION_SCHEMA are read-only metadata
