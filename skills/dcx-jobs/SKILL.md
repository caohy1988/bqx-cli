---
name: dcx-jobs
description: Use dcx to execute and manage BigQuery jobs, including running SQL queries. Use this when the user wants to run queries or check job status via dcx.
---

## When to use this skill

Use when the user asks about:
- "run a SQL query through dcx"
- "execute a BigQuery query"
- "check a BigQuery job"

Do not use when the user wants analytics workflows (doctor, evaluate, get-trace)
— use dcx-analytics instead.

## Prerequisites

See **dcx-shared** for authentication and global flags.

Requires: `--project-id`

## Commands

### jobs query

Execute a SQL query against BigQuery.

```bash
dcx jobs query \
  --project-id <PROJECT_ID> \
  --query <SQL> \
  [--use-legacy-sql <BOOL>] \
  [--dry-run] \
  [--format json|table|text]
```

| Flag | Required | Description |
|------|----------|-------------|
| `--project-id` | Yes | GCP project ID (global flag) |
| `--query` | Yes | SQL query string to execute |
| `--use-legacy-sql` | No | Use legacy SQL syntax (default: false) |
| `--dry-run` | No | Show API request without executing |
| `--format` | No | Output format: json, table, or text |

## Decision rules

- Use `--dry-run` to verify the query plan without running it
- Use `--format table` for scanning results visually in a terminal
- Use `--format json` when piping output to other tools or scripts
- Prefer the `dcx-query` skill for guidance on simple one-off queries
- Use `dcx jobs query` when you need explicit control over legacy SQL mode

## Examples

```bash
# Run a simple query
dcx jobs query \
  --project-id my-proj \
  --query "SELECT COUNT(*) AS cnt FROM \`my-proj.my_dataset.my_table\`" \
  --format table

# Dry-run to check query plan
dcx jobs query \
  --project-id my-proj \
  --query "SELECT * FROM \`my-proj.logs.events\` LIMIT 10" \
  --dry-run

# Use text format for quick output
dcx jobs query \
  --project-id my-proj \
  --query "SELECT session_id, event_type FROM \`my-proj.analytics.agent_events\` LIMIT 5" \
  --format text
```

## Constraints

- Only `jobs query` is available in Phase 2; job listing and management commands are planned
- The `--query` flag requires the full SQL string including fully-qualified table references
- See the `dcx-query` skill for query-focused guidance
