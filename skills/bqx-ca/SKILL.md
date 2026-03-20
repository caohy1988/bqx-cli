---
name: bqx-ca
description: Top-level routing for bqx Conversational Analytics commands. Use when the user wants to ask natural language questions over Data Cloud sources (BigQuery, Looker, AlloyDB, Spanner, Cloud SQL), create data agents, or manage verified queries.
---

## When to use this skill

Use when the user asks about:
- "use bqx to ask questions in natural language"
- "how do I use conversational analytics"
- "what CA commands are available"
- "query my data with plain English"
- "set up a data agent"
- "ask questions over Looker / AlloyDB / Spanner / Cloud SQL"

Do not use when the user already knows which specific command they need — use `bqx-ca-ask` or source-specific skills instead.

## Prerequisites

See **bqx-shared** for authentication and global flags.

CA commands require either:
- `--project-id` (or `BQX_PROJECT`) for BigQuery
- `--profile` for all other sources (Looker, AlloyDB, Spanner, Cloud SQL)

CA commands use `--location` (defaults to `US`) but do **not** require `--dataset-id`.

## Supported data sources

| Source | API Family | Access method |
|--------|-----------|---------------|
| BigQuery | Chat/DataAgent | `--agent`, `--tables`, or `--profile` |
| Looker | Chat/DataAgent | `--profile` only |
| Looker Studio | Chat/DataAgent | `--profile` only |
| AlloyDB | QueryData | `--profile` only |
| Spanner | QueryData | `--profile` only |
| Cloud SQL | QueryData | `--profile` only |

## Command routing

| User goal | Command | Skill |
|-----------|---------|-------|
| Ask a natural language question | `ca ask` | bqx-ca-ask |
| Create a data agent | `ca create-agent` | bqx-ca-create-agent |
| List existing data agents | `ca list-agents` | (this skill) |
| Add a verified query to an agent | `ca add-verified-query` | (this skill) |

## Source-specific skills

| Data source | Skill |
|-------------|-------|
| Looker | bqx-ca-looker |
| AlloyDB | bqx-ca-alloydb |
| Spanner | bqx-ca-spanner |
| Database sources (overview) | bqx-ca-database |

## Core workflows

### BigQuery workflow (agents + inline tables)

1. **Create agent** — set up a data agent with table references and instructions
2. **Ask** — query data using natural language through the agent
3. **Refine** — add verified queries to improve agent accuracy over time

```bash
# Create agent
bqx ca create-agent \
  --name=agent-analytics \
  --tables=myproject.analytics.agent_events \
  --instructions="You help analyze AI agent performance."

# Ask questions
bqx ca ask "What is the error rate for support_bot?" \
  --agent=agent-analytics
```

### Profile-based workflow (Looker, databases)

1. **Create profile** — YAML file with source-specific config
2. **Ask** — `ca ask --profile <name>` routes to the right API automatically

```bash
# Looker
bqx ca ask --profile sales-looker.yaml "top selling products"

# Spanner
bqx ca ask --profile finance-spanner.yaml "revenue by region"

# AlloyDB
bqx ca ask --profile ops-alloydb.yaml "show all tables"

# Cloud SQL
bqx ca ask --profile app-cloudsql.yaml "active users today"
```

### List agents

```bash
bqx ca list-agents --project-id my-proj
```

### Add verified queries

```bash
bqx ca add-verified-query \
  --agent=agent-analytics \
  --question="What is the error rate for {agent}?" \
  --query="SELECT SAFE_DIVIDE(COUNTIF(ENDS_WITH(event_type, '_ERROR')), COUNT(DISTINCT session_id)) AS error_rate FROM \`{project}.{dataset}.agent_events\` WHERE agent = @agent AND timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 24 HOUR)"
```

## Decision rules

- Use `--agent` when you have a pre-configured BigQuery data agent
- Use `--tables` for ad-hoc BigQuery queries without an agent
- Use `--profile` for Looker, AlloyDB, Spanner, Cloud SQL, or BigQuery profiles
- `--profile` cannot be combined with `--agent` or `--tables`
- Verified queries improve CA accuracy — add them for frequently asked questions
- Use `--format text` for interactive exploration; `--format json` for scripts

## Constraints

- CA depends on the Conversational Analytics API (currently in preview)
- Data agents are project-scoped — they cannot span multiple projects
- Agent names must be alphanumeric with hyphens, underscores, or dots
- `--agent` and `--tables` are mutually exclusive
- Data agent creation is only supported for BigQuery, Looker, and Looker Studio
- Database sources (AlloyDB, Spanner, Cloud SQL) do not support data agents or visualizations
