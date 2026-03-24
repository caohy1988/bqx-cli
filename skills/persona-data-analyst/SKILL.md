---
name: persona-data-analyst
description: Persona for SQL analysts using dcx to explore BigQuery data, inspect schemas, and run ad-hoc queries. Guides through data exploration workflows.
---

## When to use this skill

Use when the user is:
- Exploring BigQuery datasets and tables for the first time
- Running ad-hoc SQL queries to investigate data
- Looking for specific tables or columns in a project
- Trying to understand data shapes before building dashboards or reports

## Prerequisites

Load the following skills: `dcx-query`, `dcx-schema`, `dcx-datasets`

See **dcx-shared** for authentication and global flags.

## Persona context

The data analyst uses SQL to explore and analyze data stored in BigQuery. They
need to discover available datasets and tables, understand schemas, and run
queries — all through the dcx CLI without switching to the GCP console.

## Workflow: Data exploration

### 1. Discover available datasets

```bash
dcx datasets list \
  --project-id <PROJECT_ID> \
  --format table
```

### 2. List tables in a dataset

```bash
dcx tables list \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --format table
```

### 3. Inspect a table schema

```bash
dcx tables get \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --table-id <TABLE_ID> \
  --format table
```

### 4. Preview data

```bash
dcx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT * FROM \`<PROJECT_ID>.<DATASET_ID>.<TABLE_ID>\` LIMIT 10" \
  --format table
```

### 5. Run an analytical query

```bash
dcx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT col_a, COUNT(*) AS cnt FROM \`<PROJECT_ID>.<DATASET_ID>.<TABLE_ID>\` GROUP BY col_a ORDER BY cnt DESC LIMIT 20" \
  --format table
```

## Workflow: Schema comparison

Compare column layouts across tables to find join keys:

```bash
dcx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT table_name, column_name, data_type FROM \`<PROJECT_ID>.<DATASET_ID>\`.INFORMATION_SCHEMA.COLUMNS WHERE table_name IN ('table_a', 'table_b') ORDER BY table_name, ordinal_position" \
  --format table
```

## Decision rules

- Start with `datasets list` → `tables list` → `tables get` to orient yourself
- Use `--format table` for visual exploration in the terminal
- Use `--format json` when saving output for scripts or notebooks
- Use `--dry-run` to validate query syntax before running expensive queries
- Use INFORMATION_SCHEMA for metadata queries across tables

## Constraints

- This persona covers read-only data exploration, not ETL or data engineering
- All queries go through `dcx jobs query` — there is no interactive SQL REPL
- Query costs apply to all non-dry-run executions
- Table references in SQL must be fully qualified: `project.dataset.table`
