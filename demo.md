# bqx v0.0 MVP Demo Guide

This guide walks you through the `bqx` demo end-to-end: from setup to
running all four commands against a real BigQuery dataset.

## Prerequisites

1. **Rust toolchain** (1.70+)

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

2. **Google Cloud CLI** with a project that has BigQuery enabled

   ```bash
   gcloud auth application-default login
   ```

3. **A BigQuery dataset** with an `agent_events` table (see
   [Data Contract](#data-contract) below)

## Quick Start

```bash
# Clone and build
git clone https://github.com/haiyuan-eng-google/bqx-cli.git
cd bqx-cli
cargo build

# Set your project and dataset
export BQX_PROJECT="your-project-id"
export BQX_DATASET="agent_analytics"

# Run the full demo
bash scripts/demo.sh
```

Or run each command individually — see the [Walkthrough](#walkthrough) below.

---

## Data Contract

`bqx analytics` commands expect a table called `agent_events` with these
columns:

| Column | Type | Required | Description |
|--------|------|----------|-------------|
| `session_id` | STRING | Yes | Groups all events in one user interaction |
| `agent` | STRING | Yes | Name of the agent that generated the event |
| `event_type` | STRING | Yes | Category: `LLM_REQUEST`, `LLM_RESPONSE`, `TOOL_STARTING`, `TOOL_COMPLETED`, `TOOL_ERROR`, etc. |
| `timestamp` | TIMESTAMP | Yes | When the event occurred (UTC) |
| `status` | STRING | No | `OK` or `ERROR` |
| `error_message` | STRING | No | Error details if status is `ERROR` |
| `latency_ms` | JSON | No | `{"total_ms": 3200}` — latency measurement |
| `content` | JSON | No | Event payload (prompt text, tool output, etc.) |

This is the standard schema used by the
[ADK BigQuery exporter](https://github.com/haiyuan-eng-google/BigQuery-Agent-Analytics-SDK).
If your table has extra columns, that's fine — `bqx` ignores columns it
doesn't need.

---

## Walkthrough

The demo proves three things:

1. `bqx` is **JSON-first** — every command defaults to structured JSON
2. `bqx` covers **both raw BigQuery access and analytics** in one CLI
3. `bqx` **feels better than `bq`** for agent analytics workflows

### Step 1: Raw SQL Query (JSON-first)

`bqx jobs query` executes arbitrary SQL and returns structured JSON.
This is the raw BigQuery access layer — same capability as `bq query`,
but JSON by default instead of ASCII tables.

```bash
bqx jobs query \
  --query "SELECT session_id, agent, event_type, timestamp
           FROM \`${BQX_PROJECT}.${BQX_DATASET}.agent_events\`
           LIMIT 5"
```

**Output:**

```json
{
  "total_rows": 5,
  "rows": [
    {
      "agent": "yahoo_sales_agent",
      "event_type": "AGENT_COMPLETED",
      "session_id": "adcp-f5a6b8bd92e8",
      "timestamp": "2026-03-05 08:42:32.323 UTC"
    },
    {
      "agent": "yahoo_sales_agent",
      "event_type": "AGENT_COMPLETED",
      "session_id": "adcp-87820945dd00",
      "timestamp": "2026-03-05 08:58:10.590 UTC"
    },
    {
      "agent": "yahoo_sales_agent",
      "event_type": "AGENT_COMPLETED",
      "session_id": "adcp-033c95d7a97d",
      "timestamp": "2026-03-05 08:42:10.678 UTC"
    },
    {
      "agent": "yahoo_sales_agent",
      "event_type": "AGENT_COMPLETED",
      "session_id": "adcp-819759bc861c",
      "timestamp": "2026-03-05 09:19:02.703 UTC"
    },
    {
      "agent": "yahoo_sales_agent",
      "event_type": "AGENT_COMPLETED",
      "session_id": "adcp-c9aa93e81f22",
      "timestamp": "2026-03-05 09:18:47.135 UTC"
    }
  ]
}
```

You can also preview the request without executing it:

```bash
bqx jobs query --query "SELECT 1" --dry-run
```

```json
{
  "dry_run": true,
  "url": "https://bigquery.googleapis.com/bigquery/v2/projects/your-project-id/queries",
  "method": "POST",
  "body": {
    "query": "SELECT 1",
    "useLegacySql": false,
    "location": "US"
  }
}
```

### Step 2: Health Check

`bqx analytics doctor` validates your dataset is set up correctly.
It checks that the table exists, has all required columns, and reports
row counts, data freshness, and null rates.

```bash
bqx analytics doctor
```

**Output:**

```json
{
  "status": "warning",
  "table": "test-project-0728-467323.agent_analytics.agent_events",
  "total_rows": 296,
  "distinct_sessions": 12,
  "distinct_agents": 1,
  "earliest_event": "2026-03-05 08:41:57.918 UTC",
  "latest_event": "2026-03-05 09:27:54.474 UTC",
  "minutes_since_last_event": 5659,
  "null_checks": {
    "session_id": 0,
    "agent": 0,
    "event_type": 0,
    "timestamp": 0
  },
  "distinct_event_types": 9,
  "columns": [
    "timestamp", "event_type", "agent", "session_id",
    "invocation_id", "user_id", "trace_id", "span_id",
    "parent_span_id", "content", "content_parts", "attributes",
    "latency_ms", "status", "error_message", "is_truncated"
  ],
  "missing_required_columns": [],
  "warnings": [
    "No recent data — last event was 5659 minutes ago."
  ]
}
```

**What `status` means:**
- `healthy` — table exists, all columns present, data is fresh, no nulls
  in required columns
- `warning` — functional but something is off (e.g., stale data, null
  values in optional columns)
- `error` — missing required columns, empty table, or null values in
  required columns

### Step 3: Evaluate Agent Latency

`bqx analytics evaluate` runs a pass/fail evaluation across all sessions
in a time window. The latency evaluator checks maximum per-session latency
against your threshold.

```bash
# JSON output (default)
bqx analytics evaluate \
  --evaluator latency \
  --threshold 5000 \
  --last 30d

# Table output (for quick visual scanning)
bqx analytics evaluate \
  --evaluator latency \
  --threshold 5000 \
  --last 30d \
  --format table
```

**JSON output:**

```json
{
  "evaluator": "latency",
  "threshold": 5000.0,
  "time_window": "30d",
  "agent_id": null,
  "total_sessions": 12,
  "passed": 0,
  "failed": 12,
  "pass_rate": 0.0,
  "sessions": [
    {"session_id": "adcp-a20d176b82af", "agent": "yahoo_sales_agent", "passed": false, "score": 32135.0},
    {"session_id": "adcp-affa5aab2ee0", "agent": "yahoo_sales_agent", "passed": false, "score": 26848.0},
    {"session_id": "adcp-033c95d7a97d", "agent": "yahoo_sales_agent", "passed": false, "score": 26784.0}
  ]
}
```

**Table output:**

```
┌───────────────────┬────────┬─────────┬───────────────────┐
│ agent             ┆ passed ┆ score   ┆ session_id        │
╞═══════════════════╪════════╪═════════╪═══════════════════╡
│ yahoo_sales_agent ┆ false  ┆ 32135.0 ┆ adcp-a20d176b82af │
│ yahoo_sales_agent ┆ false  ┆ 26848.0 ┆ adcp-affa5aab2ee0 │
│ yahoo_sales_agent ┆ false  ┆ 26784.0 ┆ adcp-033c95d7a97d │
│ yahoo_sales_agent ┆ false  ┆ 25925.0 ┆ adcp-87820945dd00 │
│ yahoo_sales_agent ┆ false  ┆ 16483.0 ┆ adcp-e80ed00dd884 │
│ yahoo_sales_agent ┆ false  ┆ 16119.0 ┆ adcp-d9d7cfaa5693 │
│ yahoo_sales_agent ┆ false  ┆ 15361.0 ┆ adcp-c9aa93e81f22 │
│ yahoo_sales_agent ┆ false  ┆ 15171.0 ┆ adcp-819759bc861c │
│ yahoo_sales_agent ┆ false  ┆ 15146.0 ┆ adcp-040c04837251 │
│ yahoo_sales_agent ┆ false  ┆ 15068.0 ┆ adcp-2c401a645c40 │
│ yahoo_sales_agent ┆ false  ┆ 14564.0 ┆ adcp-7d9855e7a71b │
│ yahoo_sales_agent ┆ false  ┆ 14345.0 ┆ adcp-f5a6b8bd92e8 │
└───────────────────┴────────┴─────────┴───────────────────┘
```

The `score` column is max latency in milliseconds. All 12 sessions exceed
the 5000ms threshold — the worst at 32,135ms (32 seconds).

You can filter to a specific agent:

```bash
bqx analytics evaluate \
  --evaluator latency \
  --threshold 5000 \
  --last 30d \
  --agent-id yahoo_sales_agent
```

### Step 4: Inspect a Failing Session

Take the worst session from step 3 (`adcp-a20d176b82af`, 32s latency)
and drill into its event timeline:

```bash
bqx analytics get-trace \
  --session-id adcp-a20d176b82af \
  --format table
```

**Output:**

```
Session: adcp-a20d176b82af  Agent: yahoo_sales_agent  Events: 32  Errors: false
Time:    2026-03-05 09:26:59.267 UTC → 2026-03-05 09:27:23.812 UTC

┌─────────────────────────────┬───────────────────────┬────────┬────────────┬───────────────┐
│ timestamp                   ┆ event_type            ┆ status ┆ latency_ms ┆ error_message │
╞═════════════════════════════╪═══════════════════════╪════════╪════════════╪═══════════════╡
│ 2026-03-05 09:26:59.267 UTC ┆ USER_MESSAGE_RECEIVED ┆ OK     ┆ -          ┆ -             │
│ 2026-03-05 09:26:59.267 UTC ┆ INVOCATION_STARTING   ┆ OK     ┆ -          ┆ -             │
│ 2026-03-05 09:26:59.268 UTC ┆ AGENT_STARTING        ┆ OK     ┆ -          ┆ -             │
│ 2026-03-05 09:26:59.270 UTC ┆ LLM_REQUEST           ┆ OK     ┆ -          ┆ -             │
│ 2026-03-05 09:27:03.208 UTC ┆ LLM_RESPONSE          ┆ OK     ┆ 3938       ┆ -             │
│ 2026-03-05 09:27:03.289 UTC ┆ TOOL_STARTING         ┆ OK     ┆ -          ┆ -             │
│ 2026-03-05 09:27:03.289 UTC ┆ TOOL_COMPLETED        ┆ OK     ┆ 0          ┆ -             │
│ 2026-03-05 09:27:03.289 UTC ┆ TOOL_STARTING         ┆ OK     ┆ -          ┆ -             │
│ 2026-03-05 09:27:03.289 UTC ┆ TOOL_COMPLETED        ┆ OK     ┆ 0          ┆ -             │
│ 2026-03-05 09:27:03.291 UTC ┆ LLM_REQUEST           ┆ OK     ┆ -          ┆ -             │
│ 2026-03-05 09:27:05.728 UTC ┆ LLM_RESPONSE          ┆ OK     ┆ 2437       ┆ -             │
│ ...                         ┆ ...                   ┆ ...    ┆ ...        ┆ ...           │
│ 2026-03-05 09:27:17.493 UTC ┆ LLM_RESPONSE          ┆ OK     ┆ 9298       ┆ -             │
│ 2026-03-05 09:27:17.494 UTC ┆ AGENT_COMPLETED       ┆ OK     ┆ 18226      ┆ -             │
│ 2026-03-05 09:27:17.494 UTC ┆ INVOCATION_COMPLETED  ┆ OK     ┆ 32135      ┆ -             │
└─────────────────────────────┴───────────────────────┴────────┴────────────┴───────────────┘
```

**What the trace reveals:** This session had 4 LLM round-trips (3.9s +
2.4s + 2.5s + 9.3s) before the agent completed. The 32s total latency
at `INVOCATION_COMPLETED` is the cumulative wall-clock time. The bottleneck
was the 4th LLM call at 9.3 seconds.

For the full event payload (tool inputs, LLM prompts, etc.), use JSON:

```bash
bqx analytics get-trace --session-id adcp-a20d176b82af
```

### Step 5: Error Rate Evaluation (CI Gate)

The error-rate evaluator counts sessions with error events (any event
where `event_type` ends with `_ERROR`, `status = 'ERROR'`, or
`error_message IS NOT NULL`).

The `--exit-code` flag makes the command return exit code 1 if any
session fails the threshold — designed for CI/CD pipelines:

```bash
# In a GitHub Actions step:
bqx analytics evaluate \
  --evaluator error-rate \
  --threshold 0.05 \
  --last 30d \
  --exit-code
```

**Output (all passing):**

```json
{
  "evaluator": "error_rate",
  "threshold": 0.05,
  "time_window": "30d",
  "total_sessions": 12,
  "passed": 12,
  "failed": 0,
  "pass_rate": 1.0,
  "sessions": [
    {"session_id": "adcp-f5a6b8bd92e8", "agent": "yahoo_sales_agent", "passed": true, "score": 0.0},
    {"session_id": "adcp-87820945dd00", "agent": "yahoo_sales_agent", "passed": true, "score": 0.0}
  ]
}
```

Exit code behavior:
- `0` — all sessions pass the threshold (CI gate passes)
- `1` — at least one session fails (CI gate fails)

---

## Composing Commands (Agent Workflow)

The real power of `bqx` is that these commands compose. An AI agent
(or an SRE) can chain them:

```bash
# 1. Is the system healthy?
bqx analytics doctor

# 2. Any sessions breaching SLA?
bqx analytics evaluate --evaluator latency --threshold 5000 --last 1h

# 3. Drill into the worst one
bqx analytics evaluate --evaluator latency --threshold 5000 --last 1h \
  | jq -r '.sessions[0].session_id' \
  | xargs -I{} bqx analytics get-trace --session-id {}

# 4. Run ad-hoc SQL for custom analysis
bqx jobs query --query "
  SELECT event_type, COUNT(*) as count
  FROM \`${BQX_PROJECT}.${BQX_DATASET}.agent_events\`
  WHERE session_id = 'adcp-a20d176b82af'
  GROUP BY event_type
  ORDER BY count DESC"
```

Because every command outputs structured JSON, the output can be piped
to `jq`, consumed by other tools, or parsed by an AI agent in its
tool-use loop.

---

## Configuration Reference

### Global Flags

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--project-id` | `BQX_PROJECT` | (required) | GCP project ID |
| `--dataset-id` | `BQX_DATASET` | — | BigQuery dataset (required for `analytics` commands) |
| `--location` | `BQX_LOCATION` | `US` | BigQuery location |
| `--table` | — | `agent_events` | Table name |
| `--format` | — | `json` | Output format: `json` or `table` |

### Evaluate Flags

| Flag | Description |
|------|-------------|
| `--evaluator` | `latency` or `error-rate` |
| `--threshold` | Milliseconds for latency, 0-1 ratio for error-rate |
| `--last` | Time window: `1h`, `24h`, `7d`, `30d` |
| `--agent-id` | Filter to a specific agent |
| `--exit-code` | Exit 1 if any session fails |

### Authentication

The MVP uses Application Default Credentials only. Run this once:

```bash
gcloud auth application-default login
```

For CI/CD, use a service account:

```bash
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/sa-key.json
bqx analytics evaluate --evaluator latency --threshold 5000 --last 24h --exit-code
```

---

## Troubleshooting

**"--project-id or BQX_PROJECT is required"**
Set the env var or pass the flag:
```bash
export BQX_PROJECT="your-project-id"
```

**"Failed to initialize ADC authentication"**
Run `gcloud auth application-default login` and try again.

**"BigQuery API error 404"**
The dataset or table doesn't exist. Check with:
```bash
bq ls $BQX_PROJECT:$BQX_DATASET
```

**"BigQuery API error 403"**
Your account doesn't have BigQuery access. You need at least
`roles/bigquery.dataViewer` and `roles/bigquery.jobUser`.

**Empty evaluate results**
The `--last` window may be too narrow. Try `--last 30d` to include
older data.
