---
name: dcx-models
description: Use dcx to manage BigQuery models via the get, list commands. Generated from the BigQuery v2 Discovery API.
---

## When to use this skill

Use when the user asks about:
- "get a BigQuery model"
- "list a BigQuery model"
- "show me BigQuery models"
- "what models are in my project"

Do not use when the user wants analytics workflows (doctor, evaluate, get-trace) — use dcx-analytics instead.

## Prerequisites

See **dcx-shared** for authentication and global flags.

Requires: `--project-id` and `--dataset-id`

## Commands

### models get

Gets the specified model resource by model ID.

```bash
dcx models get \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --model-id <modelId> \
  [--dry-run] \
  [--format json|table|text]
```

| Flag | Required | Description |
|------|----------|-------------|
| `--project-id` | Yes | GCP project ID (global flag) |
| `--dataset-id` | Yes | BigQuery dataset (global flag) |
| `--model-id` | Yes | Required |

### models list

Lists all models in the specified dataset. Requires the READER dataset role. After retrieving the list of models, you can get information about a particular model by calling the models.get method.

```bash
dcx models list \
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
| `--page-token` | No | Page token, returned by a previous call to request the next page of results |

## Decision rules

- Use `--dry-run` to see the API request without executing it
- Use `--format table` for scanning results visually in a terminal
- Use `--format json` when piping output to other tools or scripts
- Use `models list` to discover available models in a project
- Use `models get` to inspect a specific model's metadata

## Examples

```bash
# Get models
dcx models get \
  --project-id my-proj \
  --dataset-id my_dataset \
  --model-id my_modelid \
  --format table

# List models
dcx models list \
  --project-id my-proj \
  --dataset-id my_dataset \
  --format table

```

## Constraints

- These commands are generated from the BigQuery v2 Discovery API
- Only read operations are supported in Phase 2
- Nested response objects are summarized in table format; use `--format json` for full detail
- Reference objects (e.g. modelsReference) are automatically flattened in table output
