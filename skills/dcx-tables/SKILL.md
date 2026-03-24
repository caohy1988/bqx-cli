---
name: dcx-tables
description: Use dcx to manage BigQuery tables via the get, list commands. Generated from the BigQuery v2 Discovery API.
---

## When to use this skill

Use when the user asks about:
- "get a BigQuery table"
- "list a BigQuery table"
- "show me BigQuery tables"
- "what tables are in my project"

Do not use when the user wants analytics workflows (doctor, evaluate, get-trace) — use dcx-analytics instead.

## Prerequisites

See **dcx-shared** for authentication and global flags.

Requires: `--project-id` and `--dataset-id`

## Commands

### tables get

Gets the specified table resource by table ID. This method does not return the data in the table, it only returns the table resource, which describes the structure of this table.

```bash
dcx tables get \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --table-id <tableId> \
  [--selected-fields] \
  [--view] \
  [--dry-run] \
  [--format json|table|text]
```

| Flag | Required | Description |
|------|----------|-------------|
| `--project-id` | Yes | GCP project ID (global flag) |
| `--dataset-id` | Yes | BigQuery dataset (global flag) |
| `--table-id` | Yes | Required |
| `--selected-fields` | No | List of table schema fields to return (comma-separated) |
| `--view` | No | Optional |

### tables list

Lists all tables in the specified dataset. Requires the READER dataset role.

```bash
dcx tables list \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  [--max-results] \
  [--page-token] \
  [--dry-run] \
  [--format json|table|text]
```

| Flag | Required | Description |
|------|----------|-------------|
| `--project-id` | Yes | GCP project ID (global flag) |
| `--dataset-id` | Yes | BigQuery dataset (global flag) |
| `--max-results` | No | The maximum number of results to return in a single response page |
| `--page-token` | No | Page token, returned by a previous call, to request the next page of results |

## Decision rules

- Use `--dry-run` to see the API request without executing it
- Use `--format table` for scanning results visually in a terminal
- Use `--format json` when piping output to other tools or scripts
- Use `tables list` to discover available tables in a project
- Use `tables get` to inspect a specific table's metadata

## Examples

```bash
# Get tables
dcx tables get \
  --project-id my-proj \
  --dataset-id my_dataset \
  --table-id my_tableid \
  --format table

# List tables
dcx tables list \
  --project-id my-proj \
  --dataset-id my_dataset \
  --format table

```

## Constraints

- These commands are generated from the BigQuery v2 Discovery API
- Only read operations are supported in Phase 2
- Nested response objects are summarized in table format; use `--format json` for full detail
- Reference objects (e.g. tablesReference) are automatically flattened in table output
