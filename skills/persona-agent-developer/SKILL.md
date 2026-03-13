---
name: persona-agent-developer
description: Persona for developers building and debugging AI agents that log to BigQuery. Guides through the full agent development lifecycle using bqx.
---

## When to use this skill

Use when the user is:
- Building an AI agent that logs events to BigQuery
- Setting up agent observability for the first time
- Debugging agent behavior or performance issues
- Designing an evaluation pipeline for agent sessions

## Prerequisites

Load the following skills: `bqx-analytics`, `bqx-query`, `bqx-schema`

See **bqx-shared** for authentication and global flags.

## Persona context

The agent developer builds AI agents (LLM-based, tool-calling, or multi-step)
that write structured events to a BigQuery `agent_events` table. They need to:
1. Verify their logging pipeline works
2. Evaluate agent performance against quality thresholds
3. Debug failing or slow sessions
4. Set up CI gates to catch regressions

## Workflow: First-time setup

### 1. Verify the events table exists and is healthy

```bash
bqx analytics doctor \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --format table
```

Doctor checks that required columns exist (`session_id`, `agent`, `event_type`,
`timestamp`), counts rows and sessions, and reports data freshness.

### 2. Inspect the table schema

```bash
bqx tables get \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --table-id agent_events \
  --format table
```

### 3. Run a baseline evaluation

```bash
bqx analytics evaluate \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --evaluator latency \
  --threshold 5000 \
  --last 24h \
  --format table
```

### 4. Debug a failing session

```bash
bqx analytics get-trace \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --session-id <SESSION_ID> \
  --format table
```

## Workflow: CI evaluation gate

Add to your CI pipeline to fail builds when agent quality drops:

```bash
bqx analytics evaluate \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --evaluator error-rate \
  --threshold 0.05 \
  --last 24h \
  --exit-code \
  --format json
```

The `--exit-code` flag returns exit code 1 when the evaluation fails, which
stops CI pipelines.

## Decision rules

- Start with `doctor` to verify data pipeline health
- Use `evaluate` with `--last 24h` for daily checks, `--last 7d` for weekly
- Use `get-trace` to drill into specific sessions identified by `evaluate`
- Use `--exit-code` in CI pipelines for automated quality gates
- Use `--agent-id` to focus evaluation on a specific agent when you have multiple

## Constraints

- This persona covers the development lifecycle, not SRE/on-call scenarios
- Agent events must already be flowing to BigQuery — bqx does not handle ingestion
- The `agent_events` table must have the required schema columns
