---
name: persona-sre
description: On-call SRE workflows for monitoring and triaging AI agent issues using dcx analytics, conversational analytics, and raw queries.
---

## When to use this skill

Use when the user is:
- On-call and triaging an AI agent incident
- Running daily or weekly health checks on agent infrastructure
- Investigating error spikes, latency regressions, or drift
- Setting up monitoring and alerting for AI agents

## Prerequisites

Load the following skills: `dcx-analytics`, `dcx-ca`, `dcx-query`

See **dcx-shared** for authentication and global flags.

## Incident Triage Workflow

1. Check overall health:
   `dcx analytics doctor`
2. Look for error spikes:
   `dcx analytics evaluate --evaluator=error-rate --threshold=0.05 --last=1h`
3. Identify failing sessions:
   `dcx analytics evaluate --evaluator=latency --threshold=5000 --last=1h --format=table`
4. Inspect a specific failure:
   `dcx analytics get-trace --session-id=<ID_FROM_STEP_3>`
5. Ask follow-up in natural language:
   `dcx ca ask "What tools failed most in the last hour?" --agent=agent-analytics`

## Daily Health Check

```bash
dcx analytics doctor && \
dcx analytics evaluate --evaluator=error-rate --threshold=0.05 --last=24h && \
dcx analytics evaluate --evaluator=latency --threshold=5000 --last=24h
```

## Deep Dive Workflow

When the health check surfaces issues, drill deeper:

```bash
# Get insights summary
dcx analytics insights --last=1h --format=text

# Check event distribution for anomalies
dcx analytics distribution --last=1h --format=table

# Check HITL escalation rate
dcx analytics hitl-metrics --last=1h --format=table

# Drift check against golden questions
dcx analytics drift --golden-dataset=golden_questions --last=24h --format=text
```

## Natural Language Investigation

Use CA to ask ad-hoc questions during an incident:

```bash
# BigQuery agent — ask about agent events
dcx ca ask "Which agents have error rate above 5% in the last hour?" \
  --agent=agent-analytics

dcx ca ask "What's causing high latency in the last hour?" \
  --agent=agent-analytics

dcx ca ask "Which tools failed most in the last 24 hours?" \
  --agent=agent-analytics
```

### Cross-source investigation with profiles

When operational data lives in database sources, use profiles:

```bash
# AlloyDB — check application database health
dcx ca ask --profile ops-alloydb.yaml "show active connections"
dcx ca ask --profile ops-alloydb.yaml "any blocked queries right now?"

# Spanner — check transaction health
dcx ca ask --profile finance-spanner.yaml "failed transactions last hour"
```

## Weekly Review Script

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=== Weekly Agent Health Report ==="

echo "--- Insights ---"
dcx analytics insights --last=7d --format=text

echo ""
echo "--- Drift Check ---"
dcx analytics drift --golden-dataset=golden_questions --last=7d --exit-code || \
  echo "WARNING: Drift detected!"

echo ""
echo "--- HITL Metrics ---"
dcx analytics hitl-metrics --last=7d --format=table

echo ""
echo "--- Event Distribution ---"
dcx analytics distribution --last=7d --format=table
```

## Tips

- Use `--format=table` for quick visual scans during incidents.
- Pipe `--format=json` output to `jq` for scripted analysis.
- Set `DCX_PROJECT` and `DCX_DATASET` env vars to avoid repetitive flags.
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
- BigQuery CA requires a configured data agent (see `dcx-ca-create-agent`)
- Database CA (AlloyDB, Spanner, Cloud SQL) requires profiles (see `dcx-ca-database`)
- Agent events must already be flowing to BigQuery — dcx does not handle ingestion
- Alerting integrations (Slack, PagerDuty) must be configured outside dcx
