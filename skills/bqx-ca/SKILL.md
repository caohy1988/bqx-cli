---
name: bqx-ca
description: Top-level routing for bqx Conversational Analytics commands. Use when the user wants to ask natural language questions over BigQuery data, create data agents, or manage verified queries.
---

## When to use this skill

Use when the user asks about:
- "use bqx to ask questions in natural language"
- "how do I use conversational analytics"
- "what CA commands are available"
- "set up a data agent for my BigQuery data"
- "query BigQuery with plain English"

Do not use when the user already knows which specific command they need — use `bqx-ca-ask` or `bqx-ca-create-agent` instead.

## Prerequisites

See **bqx-shared** for authentication and global flags.

All CA commands require:
- `--project-id` (or `BQX_PROJECT`)

CA commands use `--location` (defaults to `US`) but do **not** require `--dataset-id`.

## Command routing

| User goal | Command | Skill |
|-----------|---------|-------|
| Ask a natural language question | `ca ask` | bqx-ca-ask |
| Create a data agent | `ca create-agent` | bqx-ca-create-agent |
| List existing data agents | `ca list-agents` | (this skill) |
| Add a verified query to an agent | `ca add-verified-query` | (this skill) |

## Core workflow

The standard CA workflow is:

1. **Create agent** — set up a data agent with table references and instructions
2. **Ask** — query data using natural language through the agent
3. **Refine** — add verified queries to improve agent accuracy over time

### Step 1: Create a data agent

```bash
bqx ca create-agent \
  --name=agent-analytics \
  --tables=myproject.analytics.agent_events \
  --instructions="You help analyze AI agent performance."
```

### Step 2: Ask questions

```bash
bqx ca ask "What is the error rate for support_bot?" \
  --agent=agent-analytics
```

Returns structured JSON with the generated SQL, results, and explanation.

### Step 3: List agents

```bash
bqx ca list-agents \
  --project-id my-proj
```

### Step 4: Add verified queries

Improve accuracy by adding question/SQL pairs the agent should use:

```bash
bqx ca add-verified-query \
  --agent=agent-analytics \
  --question="What is the error rate for {agent}?" \
  --query="SELECT SAFE_DIVIDE(COUNTIF(ENDS_WITH(event_type, '_ERROR')), COUNT(DISTINCT session_id)) AS error_rate FROM \`{project}.{dataset}.agent_events\` WHERE agent = @agent AND timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 24 HOUR)"
```

## Decision rules

- Use `--agent` when you have a pre-configured data agent with context and verified queries
- Use `--tables` for ad-hoc queries against specific tables without an agent
- `--agent` and `--tables` cannot be used together
- Verified queries improve CA accuracy — add them for frequently asked questions
- Use `--format text` for interactive exploration; `--format json` for scripts

## Constraints

- CA depends on the BigQuery Conversational Analytics API (currently in preview)
- Data agents are project-scoped — they cannot span multiple projects
- Agent names must be alphanumeric with hyphens, underscores, or dots
- `--agent` and `--tables` are mutually exclusive
