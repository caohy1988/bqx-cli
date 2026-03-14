---
name: bqx-ca-ask
description: Ask natural language questions over BigQuery data using Conversational Analytics. Translates plain English to SQL, runs it, and returns structured results.
---

## When to use this skill

Use when the user wants to:
- Ask a question about their BigQuery data in natural language
- Get SQL generated from a plain English question
- Query through a pre-configured data agent
- Run ad-hoc natural language queries against specific tables

## Prerequisites

Load the following skills: `bqx-ca`

See **bqx-shared** for authentication and global flags.

## Usage

```bash
bqx ca ask "<question>" [--agent=<AGENT>] [--tables=<TABLE_REFS>]
```

## Flags

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `<question>` | Yes | — | Natural language question (positional argument) |
| `--agent` | No | — | Data agent to route the question through |
| `--tables` | No | — | Comma-separated table references for ad-hoc context |

## Examples

### With a data agent

```bash
# Ask through a pre-configured agent
bqx ca ask "What were the top errors for support_bot yesterday?" \
  --agent=agent-analytics

# Chain with other commands
bqx ca ask "Which agent had the worst performance today?" \
  --agent=agent-analytics \
  --format json \
  | jq -r '.results[0].agent' \
  | xargs -I{} bqx analytics evaluate --agent-id={} --evaluator=latency --threshold=5000 --last=24h
```

### With inline tables (no agent)

```bash
bqx ca ask "How many sessions were there yesterday?" \
  --tables=myproject.analytics.agent_events
```

### Output formats

```bash
# Structured JSON (default) — best for piping
bqx ca ask "error rate by agent" --agent=agent-analytics --format json

# Human-readable text
bqx ca ask "error rate by agent" --agent=agent-analytics --format text
```

## Response structure

The JSON response includes:
- `question` — the original question
- `sql` — the generated SQL query
- `results` — query result rows
- `explanation` — natural language explanation of the results

## Decision rules

- Use `--agent` when a data agent has been set up with context and verified queries
- Use `--tables` for one-off exploratory queries without agent setup
- Never combine `--agent` and `--tables` — they are mutually exclusive
- Pipe `--format json` output to `jq` for scripted analysis
- Use `--format text` for interactive exploration

## Constraints

- Questions must not be empty
- Agent names are validated (alphanumeric, hyphens, underscores, dots)
- CA API must be available in your project's region
- This is a **read-only** command — safe to run without confirmation
