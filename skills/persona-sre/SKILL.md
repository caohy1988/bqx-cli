---
name: persona-sre
description: On-call SRE workflows for monitoring and triaging AI agent issues using bqx analytics, conversational analytics, and raw queries.
---

## When to use this skill

Use when the user is:
- On-call and triaging an AI agent incident
- Running daily or weekly health checks on agent infrastructure
- Investigating error spikes, latency regressions, or drift
- Setting up monitoring and alerting for AI agents

## Prerequisites

Load the following skills: `bqx-analytics`, `bqx-ca`, `bqx-query`

See **bqx-shared** for authentication and global flags.

## Incident Triage Workflow

1. Check overall health:
   `bqx analytics doctor`
2. Look for error spikes:
   `bqx analytics evaluate --evaluator=error_rate --threshold=0.05 --last=1h`
3. Identify failing sessions:
   `bqx analytics evaluate --evaluator=latency --threshold=5000 --last=1h --format=table`
4. Inspect a specific failure:
   `bqx analytics get-trace --session-id=<ID_FROM_STEP_3>`
5. Ask follow-up in natural language:
   `bqx ca ask "What tools failed most in the last hour?" --agent=agent-analytics`

## Daily Health Check

```bash
bqx analytics doctor && \
bqx analytics evaluate --evaluator=error_rate --threshold=0.05 --last=24h && \
bqx analytics evaluate --evaluator=latency --threshold=5000 --last=24h
```

## Deep Dive Workflow

When the health check surfaces issues, drill deeper:

```bash
# Get insights summary
bqx analytics insights --last=1h --format=text

# Check event distribution for anomalies
bqx analytics distribution --last=1h --format=table

# Check HITL escalation rate
bqx analytics hitl-metrics --last=1h --format=table

# Drift check against golden questions
bqx analytics drift --golden-dataset=golden_questions --last=24h --format=text
```

## Natural Language Investigation

Use CA to ask ad-hoc questions during an incident:

```bash
# Which agents are failing?
bqx ca ask "Which agents have error rate above 5% in the last hour?" \
  --agent=agent-analytics

# What's causing latency?
bqx ca ask "What's causing high latency in the last hour?" \
  --agent=agent-analytics

# Tool failure analysis
bqx ca ask "Which tools failed most in the last 24 hours?" \
  --agent=agent-analytics
```

## Weekly Review Script

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=== Weekly Agent Health Report ==="

echo "--- Insights ---"
bqx analytics insights --last=7d --format=text

echo ""
echo "--- Drift Check ---"
bqx analytics drift --golden-dataset=golden_questions --last=7d --exit-code || \
  echo "WARNING: Drift detected!"

echo ""
echo "--- HITL Metrics ---"
bqx analytics hitl-metrics --last=7d --format=table

echo ""
echo "--- Event Distribution ---"
bqx analytics distribution --last=7d --format=table
```

## Tips

- Use `--format=table` for quick visual scans during incidents.
- Pipe `--format=json` output to `jq` for scripted analysis.
- Set `BQX_PROJECT` and `BQX_DATASET` env vars to avoid repetitive flags.
- Use `--exit-code` with `evaluate` and `drift` in automated health checks.
- Use `--agent-id` to focus on the specific agent mentioned in the alert.

## Decision rules

- Start with `doctor` to rule out data pipeline issues
- Use short windows (`--last=1h`) during active incidents
- Use longer windows (`--last=7d`) for trend analysis and weekly reviews
- Combine `evaluate` (quantitative) with `ca ask` (exploratory) for complete triage
- Use `drift` to check whether golden question coverage has regressed

## Constraints

- This persona covers SRE/on-call scenarios, not agent development (see `persona-agent-developer`)
- CA commands require a configured data agent (see `bqx-ca-create-agent`)
- Agent events must already be flowing to BigQuery — bqx does not handle ingestion
- Alerting integrations (Slack, PagerDuty) must be configured outside bqx
