---
name: bqx-query
description: Use the bqx CLI to run raw BigQuery SQL queries, including dry-run checks and json/table/text output. Use this when the user wants direct SQL execution through bqx rather than analytics workflows.
---

## When to use this skill

Use when the user asks about:
- "run this SQL through bqx"
- "dry-run this BigQuery query"
- "query BigQuery directly"
- "execute a SQL query with bqx"
- "check what a query would send"

Do not use when the user wants analytics workflows (doctor, evaluate, get-trace) — use bqx-analytics instead.

## Prerequisites

See **bqx-shared** for authentication and global flags.

Requires: `--project-id`

Does **not** require `--dataset-id` (unlike analytics commands).

## Core workflow

```bash
bqx jobs query \
  --query "<SQL>" \
  [--dry-run] \
  [--use-legacy-sql] \
  [--format json|table|text]
```

### Flags

| Flag | Description |
|------|-------------|
| `--query` | SQL query string (required) |
| `--dry-run` | Show the request that would be sent without executing |
| `--use-legacy-sql` | Use BigQuery legacy SQL dialect (default: standard SQL) |
| `--format` | `json` (default), `table`, or `text` |

## Decision rules

- Use `--dry-run` to verify what bqx will send before executing
- Use `--format table` when scanning result rows visually
- Use `--format text` for a compact summary with row-by-row output
- Use `--format json` when piping output to other tools or scripts
- Text format preserves column order from the BigQuery schema
- Dry-run does not require authentication

## Examples

```bash
# Simple query
bqx jobs query \
  --project-id my-proj \
  --query "SELECT 1"

# Query with table format for visual scanning
bqx jobs query \
  --project-id my-proj \
  --query "SELECT session_id, agent FROM \`my_proj.ds.agent_events\` LIMIT 5" \
  --format table

# Dry-run to see the request without executing
bqx jobs query \
  --project-id my-proj \
  --query "SELECT * FROM \`my_proj.ds.agent_events\` LIMIT 10" \
  --dry-run \
  --format text

# Query with text output
bqx jobs query \
  --project-id my-proj \
  --query "SELECT COUNT(*) as cnt FROM \`my_proj.ds.agent_events\`" \
  --format text
```

## Reading the output

**Text format** shows:
- "Query complete: N rows"
- Column names
- One line per row with pipe-separated values

**Dry-run text** shows:
- Request URL and method
- Query string
- Legacy SQL flag and location

**Table format** renders results as a bordered ASCII table.

**JSON format** returns `{"total_rows": N, "rows": [...]}` with each row as a key-value object.

## Constraints

- TIMESTAMP values are automatically converted from epoch seconds to ISO 8601 format
- The query runs against the project specified by `--project-id`
- Location defaults to `US` (override with `--location`)
- Long-running queries are polled automatically until completion
- Large result sets are paginated automatically
