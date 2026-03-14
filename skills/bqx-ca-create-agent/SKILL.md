---
name: bqx-ca-create-agent
description: Create a Conversational Analytics data agent with table references, views, verified queries, and system instructions.
---

## When to use this skill

Use when the user wants to:
- Create a new data agent for Conversational Analytics
- Set up an agent with specific tables and instructions
- Configure verified queries for improved CA accuracy
- Bootstrap the agent-analytics agent for agent observability

## Prerequisites

Load the following skills: `bqx-ca`

See **bqx-shared** for authentication and global flags.

## Usage

```bash
bqx ca create-agent \
  --name=<AGENT_NAME> \
  --tables=<TABLE_REFS> \
  [--views=<VIEW_REFS>] \
  [--verified-queries=<PATH>] \
  [--instructions=<TEXT>]
```

## Flags

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `--name` | Yes | — | Agent ID (alphanumeric, hyphens, underscores, dots) |
| `--tables` | Yes | — | Comma-separated table references (`project.dataset.table`) |
| `--views` | No | — | Comma-separated view references as additional data sources |
| `--verified-queries` | No | Bundled | Path to verified queries YAML file |
| `--instructions` | No | — | System instructions for the agent |

## Examples

### Basic agent creation

```bash
bqx ca create-agent \
  --name=my-analytics-agent \
  --tables=myproject.analytics.agent_events
```

### Full agent with views, verified queries, and instructions

```bash
bqx ca create-agent \
  --name=agent-analytics \
  --tables=myproject.analytics.agent_events \
  --views=myproject.analytics.adk_llm_response,myproject.analytics.adk_tool_completed \
  --verified-queries=./deploy/ca/verified_queries.yaml \
  --instructions="You help analyze AI agent performance. The agent_events
    table stores traces from ADK agents. Key event types: LLM_REQUEST,
    LLM_RESPONSE, TOOL_STARTING, TOOL_COMPLETED, TOOL_ERROR."
```

### Using with bqx analytics views

Create per-event-type views first, then include them in the agent:

```bash
# Step 1: Create views
bqx analytics views create-all \
  --project-id myproject \
  --dataset-id analytics \
  --prefix adk_

# Step 2: Create agent with views
bqx ca create-agent \
  --name=agent-analytics \
  --tables=myproject.analytics.agent_events \
  --views=myproject.analytics.adk_llm_response,myproject.analytics.adk_tool_completed
```

## Verified queries format

The `--verified-queries` flag accepts a YAML file mapping natural language questions to SQL:

```yaml
verified_queries:
  - question: "What is the error rate for {agent}?"
    query: |
      SELECT
        SAFE_DIVIDE(
          COUNTIF(ENDS_WITH(event_type, '_ERROR')),
          COUNT(DISTINCT session_id)
        ) AS error_rate
      FROM `{project}.{dataset}.agent_events`
      WHERE agent = @agent
        AND timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 24 HOUR)
```

If `--verified-queries` is not provided, the bundled `deploy/ca/verified_queries.yaml` is used.

## Decision rules

- Always provide `--instructions` to give the agent context about table semantics
- Include views for pre-filtered data access (e.g., per-event-type views from `bqx analytics views`)
- Add verified queries for frequently asked questions to improve accuracy
- Use `bqx ca add-verified-query` to add queries incrementally after creation

## Constraints

- Agent names must be alphanumeric with hyphens, underscores, or dots
- At least one `--tables` reference is required
- Table references must be fully qualified: `project.dataset.table`
- This command **creates resources** — it requires appropriate IAM permissions
