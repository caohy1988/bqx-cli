---
name: dcx-bigquery-api
description: Discovery-generated BigQuery v2 API commands — datasets, tables, routines, and models get/list operations.
---

## When to use this skill

Use when the user wants to:
- List or get BigQuery datasets, tables, routines, or models
- Inspect resource metadata via the BigQuery v2 API
- Use `--dry-run` to see API requests without executing

## Prerequisites

Requires `--project-id`. Most commands also require `--dataset-id`.
See **dcx-bigquery** for authentication.

## Commands

### Datasets

```bash
dcx datasets list --project-id P [--all] [--filter] [--max-results] --format json
dcx datasets get --project-id P --dataset-id D --format json
```

### Tables

```bash
dcx tables list --project-id P --dataset-id D [--max-results] --format json
dcx tables get --project-id P --dataset-id D --table-id T [--selected-fields] --format json
```

### Routines

```bash
dcx routines list --project-id P --dataset-id D [--filter] [--max-results] --format json
dcx routines get --project-id P --dataset-id D --routine-id R [--read-mask] --format json
```

### Models

```bash
dcx models list --project-id P --dataset-id D [--max-results] --format json
dcx models get --project-id P --dataset-id D --model-id M --format json
```

## Decision rules

- Use `--dry-run` to see the API request without executing
- Use `--format table` for visual scanning; `--format json` for automation
- Use `datasets list` to discover datasets, then `tables list` for tables within
- Use `tables get --format json` for schema inspection (includes `schema.fields`)

## Constraints

- Generated from the BigQuery v2 Discovery document
- Read-only operations only
- Nested response objects are summarized in table format; use `--format json` for full detail
- Reference objects (e.g. `datasetReference`) are flattened in table output
