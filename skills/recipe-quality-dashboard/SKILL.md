---
name: recipe-quality-dashboard
description: Recipe for building an agent quality dashboard using BigQuery views, scheduled queries, and Looker Studio (or similar) connected via dcx-managed data.
---

## When to use this skill

Use when the user wants to:
- Build a dashboard for monitoring agent quality over time
- Create summary tables for visualization tools
- Set up scheduled aggregations of agent metrics

## Prerequisites

Load the following skills: `dcx-analytics`, `dcx-query`, `dcx-analytics-views`

See **dcx-shared** for authentication and global flags.

## Recipe

### Step 1: Create summary views

Create views that aggregate agent metrics for dashboard consumption.

#### Daily latency summary

```bash
dcx jobs query \
  --project-id <PROJECT_ID> \
  --query "CREATE OR REPLACE VIEW \`<PROJECT_ID>.<DATASET_ID>.v_daily_latency\` AS SELECT DATE(timestamp) AS day, agent, COUNT(DISTINCT session_id) AS sessions, AVG(latency_ms) AS avg_latency_ms, MAX(latency_ms) AS max_latency_ms, APPROX_QUANTILES(latency_ms, 100)[OFFSET(95)] AS p95_latency_ms FROM \`<PROJECT_ID>.<DATASET_ID>.agent_events\` WHERE latency_ms IS NOT NULL GROUP BY day, agent" \
  --format text
```

#### Daily error summary

```bash
dcx jobs query \
  --project-id <PROJECT_ID> \
  --query "CREATE OR REPLACE VIEW \`<PROJECT_ID>.<DATASET_ID>.v_daily_errors\` AS SELECT DATE(timestamp) AS day, agent, COUNT(DISTINCT session_id) AS total_sessions, COUNTIF(event_type LIKE '%_ERROR' OR status = 'ERROR') AS error_events, SAFE_DIVIDE(COUNTIF(event_type LIKE '%_ERROR' OR status = 'ERROR'), COUNT(*)) AS error_rate FROM \`<PROJECT_ID>.<DATASET_ID>.agent_events\` GROUP BY day, agent" \
  --format text
```

### Step 2: Verify views return data

```bash
dcx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT * FROM \`<PROJECT_ID>.<DATASET_ID>.v_daily_latency\` ORDER BY day DESC LIMIT 10" \
  --format table

dcx jobs query \
  --project-id <PROJECT_ID> \
  --query "SELECT * FROM \`<PROJECT_ID>.<DATASET_ID>.v_daily_errors\` ORDER BY day DESC LIMIT 10" \
  --format table
```

### Step 3: Connect to a visualization tool

Point Looker Studio, Grafana, or Metabase at the summary views:

- **Data source**: BigQuery
- **Project**: `<PROJECT_ID>`
- **Tables**: `v_daily_latency`, `v_daily_errors`
- **Dimensions**: `day`, `agent`
- **Metrics**: `avg_latency_ms`, `p95_latency_ms`, `error_rate`, `sessions`

### Step 4: Set up scheduled refresh (optional)

For materialized summaries instead of views, create a scheduled query in
BigQuery that writes to a destination table daily. Use `dcx jobs query --dry-run`
to validate the aggregation SQL before scheduling.

## Decision rules

- Use views for real-time dashboards (query cost on each refresh)
- Use materialized tables with scheduled queries for cost-sensitive setups
- Include p95 latency alongside average for realistic performance picture
- Group by `agent` to compare agents side-by-side on the dashboard
- Use `--format json` to export sample data for dashboard prototyping

## Constraints

- Views are computed on read — dashboard refresh triggers BigQuery queries
- Scheduled queries require `bigquery.jobs.create` and `bigquery.tables.update` permissions
- Dashboard tool configuration is outside dcx scope
- Data freshness depends on how often agent events are written to the table
