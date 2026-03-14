---
name: recipe-ca-data-agent-setup
description: Step-by-step recipe for setting up a Conversational Analytics data agent for agent observability, including table prep, agent creation, verified queries, and validation.
---

## When to use this skill

Use when the user wants to:
- Set up a CA data agent from scratch for agent analytics
- Configure verified queries for common agent performance questions
- Connect bqx analytics views to a CA agent
- Bootstrap the full observability stack (events table + views + CA agent)

## Prerequisites

Load the following skills: `bqx-ca`, `bqx-ca-create-agent`, `bqx-analytics-views`

See **bqx-shared** for authentication and global flags.

## Recipe

### Step 1: Verify the events table is healthy

```bash
bqx analytics doctor \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --format text
```

Confirm the table has data, required columns, and recent events.

### Step 2: Create per-event-type views

Views give the CA agent focused access to specific event types:

```bash
bqx analytics views create-all \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --prefix adk_
```

This creates 18 views (e.g., `adk_llm_request`, `adk_tool_completed`, `adk_tool_error`).

### Step 3: Create the data agent

```bash
bqx ca create-agent \
  --name=agent-analytics \
  --tables=<PROJECT_ID>.<DATASET_ID>.agent_events \
  --views=<PROJECT_ID>.<DATASET_ID>.adk_llm_responses,<PROJECT_ID>.<DATASET_ID>.adk_tool_completions \
  --verified-queries=./deploy/ca/verified_queries.yaml \
  --instructions="You help analyze AI agent performance. The agent_events
    table stores traces from ADK agents. Key event types: LLM_REQUEST,
    LLM_RESPONSE, TOOL_STARTING, TOOL_COMPLETED, TOOL_ERROR.
    Error detection: event_type ends with _ERROR OR error_message IS NOT NULL
    OR status = 'ERROR'."
```

### Step 4: Validate the agent works

```bash
# Simple test question
bqx ca ask "How many sessions were there in the last 24 hours?" \
  --agent=agent-analytics

# Test a verified query
bqx ca ask "What is the error rate for support_bot?" \
  --agent=agent-analytics

# Test an exploratory question
bqx ca ask "Which tools are used most frequently?" \
  --agent=agent-analytics
```

### Step 5: Add custom verified queries

Add domain-specific questions your team frequently asks:

```bash
bqx ca add-verified-query \
  --agent=agent-analytics \
  --question="How many errors occurred in the last hour?" \
  --query="SELECT COUNT(*) AS error_count FROM \`<PROJECT_ID>.<DATASET_ID>.agent_events\` WHERE (ENDS_WITH(event_type, '_ERROR') OR error_message IS NOT NULL OR status = 'ERROR') AND timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 1 HOUR)"
```

### Step 6: Share with your team

Set `BQX_PROJECT` and `BQX_DATASET` env vars and share the agent name:

```bash
export BQX_PROJECT=<PROJECT_ID>
export BQX_DATASET=<DATASET_ID>

# Anyone on the team can now ask questions
bqx ca ask "What's the p95 latency today?" --agent=agent-analytics
```

## Decision rules

- Always run `doctor` first to confirm the events table is set up correctly
- Create views before the agent so views can be included as data sources
- Include `--instructions` to give the agent semantic context about your table
- Use the bundled verified queries as a starting point, then add custom ones
- Test with both verified-query-style questions and open-ended questions

## Constraints

- The CA API must be available in your project's region
- The agent creator needs sufficient IAM permissions to create CA resources
- Views must exist before referencing them in `--views`
- Verified queries use `@agent` parameterized syntax — do not hardcode agent names
