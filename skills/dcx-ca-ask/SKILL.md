---
name: dcx-ca-ask
description: Ask natural language questions over Data Cloud sources (BigQuery, Looker, AlloyDB, Spanner, Cloud SQL) using Conversational Analytics. Translates plain English to SQL, runs it, and returns structured results.
---

## When to use this skill

Use when the user wants to:
- Ask a question about their data in natural language
- Get SQL generated from a plain English question
- Query through a pre-configured data agent (BigQuery)
- Run ad-hoc natural language queries against specific tables (BigQuery)
- Query Looker, AlloyDB, Spanner, or Cloud SQL via a profile

## Prerequisites

Load the following skills: `dcx-ca`

See **dcx-shared** for authentication and global flags.

## Usage

```bash
# BigQuery (flags)
dcx ca ask "<question>" [--agent=<AGENT>] [--tables=<TABLE_REFS>]

# Any source (profile)
dcx ca ask --profile <PROFILE> "<question>"
```

## Flags

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `<question>` | Yes | — | Natural language question (positional argument) |
| `--agent` | No | — | Data agent to route the question through (BigQuery) |
| `--tables` | No | — | Comma-separated table references for ad-hoc context (BigQuery) |
| `--profile` | No | — | Path to a source profile YAML file |
| `--format` | No | `json` | Output format: `json`, `text`, or `table` |

## Examples

### BigQuery with a data agent

```bash
dcx ca ask "What were the top errors for support_bot yesterday?" \
  --agent=agent-analytics
```

### BigQuery with inline tables

```bash
dcx ca ask "How many sessions were there yesterday?" \
  --tables=myproject.analytics.agent_events
```

### Looker via profile

```bash
dcx ca ask --profile sales-looker.yaml "top selling products last month"
```

### Spanner via profile

```bash
dcx ca ask --profile finance-spanner.yaml "total revenue by region"
```

### AlloyDB via profile

```bash
dcx ca ask --profile ops-alloydb.yaml "show all tables in the database"
```

### Cloud SQL via profile

```bash
dcx ca ask --profile app-cloudsql.yaml "active users today"
```

### Output formats

```bash
# JSON (default) — best for piping
dcx ca ask --profile finance-spanner.yaml --format json "top customers" | jq '.results'

# Human-readable text
dcx ca ask --profile ops-alloydb.yaml --format text "largest tables"
```

## Response structure

The JSON response includes:
- `question` — the original question
- `sql` — the generated SQL query
- `results` — query result rows
- `explanation` — natural language explanation of the results
- `agent` — the agent used (BigQuery only, when applicable)

## Decision rules

- Use `--agent` when a BigQuery data agent has been set up with verified queries
- Use `--tables` for one-off BigQuery queries without agent setup
- Use `--profile` for Looker, AlloyDB, Spanner, Cloud SQL, or BigQuery profiles
- Never combine `--profile` with `--agent` or `--tables`
- Never combine `--agent` and `--tables`
- Pipe `--format json` output to `jq` for scripted analysis
- Use `--format text` for interactive exploration

## Constraints

- Questions must not be empty
- Agent names are validated (alphanumeric, hyphens, underscores, dots)
- CA API must be available in your project's region
- `--profile` cannot be combined with `--agent` or `--tables`
- This is a **read-only** command — safe to run without confirmation
