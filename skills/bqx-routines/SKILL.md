---
name: bqx-routines
description: Use bqx to manage BigQuery routines via the list, get commands. Generated from the BigQuery v2 Discovery API.
---

## When to use this skill

Use when the user asks about:
- "list a BigQuery routine"
- "get a BigQuery routine"
- "show me BigQuery routines"
- "what routines are in my project"

Do not use when the user wants analytics workflows (doctor, evaluate, get-trace)
— use bqx-analytics instead.

## Prerequisites

See **bqx-shared** for authentication and global flags.

Requires: `--project-id` and `--dataset-id`

## Commands

### routines list

Lists all routines in the specified dataset. Requires `--project-id` and `--dataset-id`.

```bash
bqx routines list \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  [--max-results] \
  [--page-token] \
  [--read-mask] \
  [--filter] \
  [--dry-run] \
  [--format json|table|text]
```

| Flag | Required | Description |
|------|----------|-------------|
| `--project-id` | Yes | GCP project ID (global flag) |
| `--dataset-id` | Yes | BigQuery dataset (global flag) |
| `--max-results` | No | The maximum number of results to return in a single response page |
| `--page-token` | No | Page token, returned by a previous call, to request the next page of results |
| `--read-mask` | No | If set, then only the Routine fields in the field mask are returned |
| `--filter` | No | If set, then only the Routines matching this filter are returned |

### routines get

Gets the specified routine resource by routine ID. Requires `--project-id`, `--dataset-id`, and `--routine-id`.

```bash
bqx routines get \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --routine-id <routineId> \
  [--read-mask] \
  [--dry-run] \
  [--format json|table|text]
```

| Flag | Required | Description |
|------|----------|-------------|
| `--project-id` | Yes | GCP project ID (global flag) |
| `--dataset-id` | Yes | BigQuery dataset (global flag) |
| `--routine-id` | Yes | Routine ID of the requested routine |
| `--read-mask` | No | If set, only the Routine fields in the field mask are returned |

## Decision rules

- Use `--dry-run` to see the API request without executing it
- Use `--format table` for scanning results visually in a terminal
- Use `--format json` when piping output to other tools or scripts
- Use `routines list` to discover available routines in a dataset
- Use `routines get` to inspect a specific routine's metadata

## Examples

```bash
# List routines
bqx routines list \
  --project-id my-proj \
  --dataset-id my_dataset \
  --format table

# Get routine
bqx routines get \
  --project-id my-proj \
  --dataset-id my_dataset \
  --routine-id my_routine \
  --format table
```

## Constraints

- These commands are generated from the BigQuery v2 Discovery API
- Only read operations are supported in Phase 2
- Nested response objects are summarized in table format; use `--format json` for full detail
- Reference objects (e.g. routinesReference) are automatically flattened in table output
