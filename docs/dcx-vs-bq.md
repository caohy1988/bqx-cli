# DCX vs. BQ: Technical Comparison and Architectural Rationale

This document compares `dcx` with the standard `bq` CLI and explains why
agent-native BigQuery workflows require a new architecture.

## 1. Technical Comparison

### Core Capabilities

| Capability | `bq` CLI | `dcx` CLI |
|---|---|---|
| **Output format** | Mixed text/JSON; `--format=json` behavior varies by command | JSON by default on every command; `--format json\|table\|text` consistent everywhere |
| **Error output** | Free-text stderr, no structured error codes | All errors emitted as `{"error":"..."}` JSON on stderr; `--exit-code` distinguishes eval failure (exit 1) from other errors |
| **Input validation** | Errors surface after API call | Validates inputs before authentication or network calls |
| **Startup time** | ~300-500ms (Python interpreter) | ~5-10ms (compiled Rust binary) |
| **Distribution** | Bundled with gcloud SDK (~1GB) | `npx dcx` — standalone 10MB binary, 6 platforms |
| **Auth model** | Coupled to `gcloud auth` | 5-level priority: `DCX_TOKEN` > `DCX_CREDENTIALS_FILE` > OAuth > ADC > gcloud |
| **Extensibility** | None — monolithic Python binary | Skills (SKILL.md) discoverable by Claude Code, Gemini CLI, Cursor, Codex |
| **Agent analytics** | Not available — requires custom SQL | Built-in: `doctor`, `evaluate`, `get-trace` |
| **CI/CD integration** | Manual exit code handling | `--exit-code` flag returns process exit code 1 on failure |
| **Dry-run** | `--dry_run` returns estimated bytes only | `--dry-run` returns full structured request (URL, method, body) |

### Agent-Specific Capabilities

These are the three capabilities from the issue that `bq` fundamentally lacks:

#### Schema Discovery

An agent exploring a dataset needs to understand table structure before writing SQL.

**`bq` approach:**
```bash
$ bq show --schema --format=prettyjson myproject:analytics.agent_events
```
Returns a JSON array of field definitions, but:
- No column-level statistics (null counts, distinct values)
- No health assessment — agent cannot tell if the table has data problems
- Requires the agent to parse raw schema and infer which columns matter

**`dcx` approach:**
```bash
$ dcx --project-id myproject --dataset-id analytics analytics doctor
```
```json
{
  "status": "healthy",
  "table": "myproject.analytics.agent_events",
  "total_rows": 296,
  "distinct_sessions": 12,
  "distinct_agents": 1,
  "earliest_event": "2026-03-01 00:00:00.000 UTC",
  "latest_event": "2026-03-05 09:27:54.474 UTC",
  "minutes_since_last_event": 30,
  "null_checks": {
    "session_id": 0,
    "agent": 0,
    "event_type": 0,
    "timestamp": 0
  },
  "distinct_event_types": 5,
  "columns": ["session_id", "agent", "event_type", "timestamp", "status",
               "error_message", "latency_ms", "content"],
  "missing_required_columns": [],
  "warnings": []
}
```

The agent gets one call that answers: "Is this table usable? How much data is there? Are required columns present? Are there null-integrity problems?" No SQL writing needed.

#### Self-Correction

When an agent's query fails, it needs structured error information to decide what to do next.

**`bq` approach:**
```bash
$ bq query --use_legacy_sql=false "SELECT bad_column FROM analytics.agent_events"
BigQuery error in query operation: Error processing job ...: Unrecognized name: bad_column
```
- Error is free-text on stderr
- No structured field indicating which column failed
- Agent must regex-parse the error message to extract the column name

**`dcx` approach:**
```bash
$ dcx --project-id myproject --dataset-id analytics jobs query \
    --query "SELECT bad_column FROM analytics.agent_events"
{"error":"...Unrecognized name: bad_column..."}
```
Errors are always JSON (`{"error":"<message>"}`) on stderr, so agents can
parse them without regex. The message is currently a string (not a structured
object with BigQuery reason/location fields), but the JSON envelope means
agents can reliably detect and extract errors.

Additionally, `dcx` validates inputs *before* making any API call:
```bash
$ dcx analytics evaluate --evaluator latency --threshold 5000 --last "bogus"
Error: Invalid duration: "bogus". Expected format: 1h, 24h, 7d, 30d
```
The agent gets a clear, parseable message without burning an API round-trip.

#### Stateful Interaction

Agent workflows often span multiple commands that share context.

**`bq` approach:**
Each invocation is fully independent. The agent must pass `--project_id` and
construct full table references (`project:dataset.table`) in every command.
There is no concept of a "current dataset" or "current table."

**`dcx` approach:**
```bash
# Set once via environment
export DCX_PROJECT=myproject
export DCX_DATASET=analytics

# All subsequent commands inherit context
dcx analytics doctor
dcx analytics evaluate --evaluator latency --threshold 5000 --last 1h
dcx analytics get-trace --session-id sess-042
```

The `--project-id`, `--dataset-id`, and `--location` flags read from
environment variables (`DCX_PROJECT`, `DCX_DATASET`, `DCX_LOCATION`).
`--table` defaults to `agent_events` and is set via CLI flag only.
This allows a session to maintain implicit state across commands without
repeating project and dataset on every invocation.

## 2. Sample Workflow Gallery

Each scenario shows real commands you can run today (Phase 1). All output
shapes are actual `dcx` data structures.

### Scenario A: "Is my agent table healthy?"

**The `bq` way** — 3 manual queries, agent must interpret each:

```bash
# Step 1: Check if table exists and has the right columns
$ bq show --schema --format=prettyjson myproject:analytics.agent_events
# → raw JSON array of fields; agent must check for session_id, agent, etc.

# Step 2: Check row count and freshness
$ bq query --use_legacy_sql=false \
  "SELECT COUNT(*) as cnt, MAX(timestamp) as latest
   FROM \`myproject.analytics.agent_events\`"
# → text table; agent must parse the row count and compare timestamps

# Step 3: Check for null integrity issues
$ bq query --use_legacy_sql=false \
  "SELECT COUNTIF(session_id IS NULL) as null_sessions,
          COUNTIF(agent IS NULL) as null_agents
   FROM \`myproject.analytics.agent_events\`"
# → another text table to parse
```

The agent writes 3 queries, parses 3 different text outputs, and synthesizes
a health assessment.

**The `dcx` way** — 1 command, structured verdict:

```bash
$ dcx analytics doctor
{
  "status": "healthy",
  "table": "myproject.analytics.agent_events",
  "total_rows": 296,
  "distinct_sessions": 12,
  "distinct_agents": 1,
  ...
  "missing_required_columns": [],
  "warnings": []
}
```

The agent reads `status` and is done. If `status` is `"error"` or
`"warning"`, the `warnings` and `missing_required_columns` arrays tell it
exactly what's wrong.

**Demo command:**
```bash
export DCX_PROJECT=<your-project> DCX_DATASET=<your-dataset>
dcx analytics doctor
dcx analytics doctor --format text
dcx analytics doctor --format table
```

### Scenario B: "CI gate — fail the deploy if latency exceeds 5s"

**The `bq` way** — custom SQL + manual exit code:

```bash
# Write the evaluation SQL yourself
$ RESULT=$(bq query --use_legacy_sql=false --format=json \
  "WITH s AS (
     SELECT session_id,
            MAX(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS FLOAT64)) AS max_lat
     FROM \`myproject.analytics.agent_events\`
     WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 1 HOUR)
     GROUP BY session_id
   )
   SELECT COUNTIF(max_lat > 5000) AS failed FROM s")

# Parse the JSON output with jq
$ FAILED=$(echo "$RESULT" | jq '.[0].failed')

# Manually set exit code
$ if [ "$FAILED" -gt 0 ]; then exit 1; fi
```

This is 15+ lines of bash that every team reimplements differently.

**The `dcx` way** — 1 command:

```bash
$ dcx analytics evaluate \
    --evaluator latency \
    --threshold 5000 \
    --last 1h \
    --exit-code
```
```json
{
  "evaluator": "latency",
  "threshold": 5000.0,
  "time_window": "1h",
  "total_sessions": 10,
  "passed": 7,
  "failed": 3,
  "pass_rate": 0.7,
  "sessions": [
    {
      "session_id": "sess-042",
      "agent": "support_bot",
      "passed": false,
      "score": 8200.0
    },
    ...
  ]
}
# Exit code: 1 (because failed > 0)
```

The `--exit-code` flag makes the process return exit code 1 when any session
fails the threshold. GitHub Actions, Jenkins, or any CI system treats this as
a step failure — no wrapper scripts needed.

**Demo commands:**
```bash
# Succeeds (exit 0) — set a very high threshold
dcx analytics evaluate --evaluator latency --threshold 999999 --last 1h --exit-code
echo "Exit code: $?"

# Fails (exit 1) — set a low threshold
dcx analytics evaluate --evaluator latency --threshold 1 --last 1h --exit-code
echo "Exit code: $?"

# Error rate evaluator
dcx analytics evaluate --evaluator error-rate --threshold 0.05 --last 1h --exit-code

# Human-readable output
dcx analytics evaluate --evaluator latency --threshold 5000 --last 1h --format text
```

### Scenario C: "Debug a slow session"

**The `bq` way** — multi-step manual investigation:

```bash
# Step 1: Find slow sessions (write SQL from scratch)
$ bq query --use_legacy_sql=false \
  "SELECT session_id, agent,
          MAX(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS FLOAT64)) AS max_lat
   FROM \`myproject.analytics.agent_events\`
   WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 1 HOUR)
   GROUP BY session_id, agent
   ORDER BY max_lat DESC
   LIMIT 5"
# → text table; manually pick a session_id

# Step 2: Get the trace (write more SQL)
$ bq query --use_legacy_sql=false \
  "SELECT event_type, timestamp, status, error_message, latency_ms
   FROM \`myproject.analytics.agent_events\`
   WHERE session_id = 'sess-042'
   ORDER BY timestamp"
# → text table; manually scan for errors
```

**The `dcx` way** — evaluate finds the slow sessions, get-trace shows the detail:

```bash
# Step 1: Find slow sessions (structured output with session IDs)
$ dcx analytics evaluate --evaluator latency --threshold 5000 --last 1h
{
  "evaluator": "latency",
  "threshold": 5000.0,
  "sessions": [
    { "session_id": "sess-042", "agent": "support_bot", "passed": false, "score": 32135.0 },
    { "session_id": "sess-017", "agent": "support_bot", "passed": false, "score": 8200.0 },
    { "session_id": "sess-091", "agent": "support_bot", "passed": true, "score": 1200.0 }
  ]
}

# Step 2: Drill into the worst session
$ dcx analytics get-trace --session-id sess-042
{
  "session_id": "sess-042",
  "agent": "support_bot",
  "event_count": 5,
  "started_at": "2026-03-05 09:26:59.270 UTC",
  "ended_at": "2026-03-05 09:27:17.494 UTC",
  "has_errors": false,
  "events": [
    { "event_type": "LLM_REQUEST",  "timestamp": "...", "status": "OK" },
    { "event_type": "LLM_RESPONSE", "timestamp": "...", "status": "OK", "latency_ms": {"total_ms": 3938} },
    { "event_type": "TOOL_CALL",    "timestamp": "...", "status": "OK" },
    { "event_type": "TOOL_ERROR",   "timestamp": "...", "status": "ERROR", "error_message": "Connection timeout" },
    { "event_type": "INVOCATION_COMPLETED", "timestamp": "...", "latency_ms": {"total_ms": 32135} }
  ]
}
```

An agent (or human) can immediately see: the session took 32s total because
a TOOL_ERROR (connection timeout) occurred after the LLM response.

**Demo commands:**
```bash
# End-to-end: find slow sessions → inspect the worst one
dcx analytics evaluate --evaluator latency --threshold 5000 --last 24h

# Pick a session_id from the output, then:
dcx analytics get-trace --session-id <session-id-from-above>

# Table format for quick visual scan
dcx analytics get-trace --session-id <session-id> --format table

# Text format for compact terminal output
dcx analytics get-trace --session-id <session-id> --format text
```

### Bonus: Dry-Run Inspection

Agents (and humans) can preview exactly what API call `dcx` will make:

```bash
$ dcx --project-id demo-project jobs query \
    --query "SELECT session_id, agent FROM analytics.agent_events LIMIT 5" \
    --dry-run
{
  "dry_run": true,
  "url": "https://bigquery.googleapis.com/bigquery/v2/projects/demo-project/queries",
  "method": "POST",
  "body": {
    "query": "SELECT session_id, agent FROM analytics.agent_events LIMIT 5",
    "useLegacySql": false,
    "location": "US"
  }
}
```

`bq` has `--dry_run` but it only returns estimated bytes processed — not the
full request that would be sent to the API.

## 3. Integration Roadmap: Why Skills Cannot Be Decoupled from the CLI

### The core argument

BQ Skills (SKILL.md files that agents discover and use) are tightly coupled
to the CLI's output contract. You cannot wrap `bq` with skill files and get
the same result, for four concrete reasons:

### Reason 1: Skills depend on structured output contracts

Each SKILL.md references specific JSON shapes that agents parse
programmatically. For example, the `dcx-analytics` skill's
`references/evaluate.md` tells agents to look for `pass_rate`,
`sessions[].passed`, and `sessions[].score` in the evaluate output.

`bq` has no guaranteed output schema. Its JSON output changes shape between
commands and sometimes between versions. A skill that wraps `bq` would need
a translation layer for every command — at which point you've built a new CLI.

**Status:** Skills are defined on the `skills-m6` branch (PR #10) and will
ship in `skills/` once merged. The output contracts they reference are
available now in Phase 1.

### Reason 2: Skills require predictable error semantics

When a skill tells an agent "run `dcx analytics evaluate --exit-code`", the
agent needs to know:
- Exit code 0 = all sessions passed
- Exit code 1 = evaluation failure (sessions exceeded threshold)
- Exit code 2 = infrastructure error (connection, auth, bad input)
- All errors emit `{"error":"<message>"}` JSON on stderr

`bq` returns exit code 1 for all errors (auth failure, query syntax error,
permission denied) with free-text stderr. An agent wrapping `bq` cannot
distinguish "the evaluation found failures" from "the query was malformed."
`dcx` uses distinct exit codes: eval failure (exit 1 via `--exit-code`) vs
infrastructure errors (exit 2), matching the upstream SDK semantics. It
always uses JSON-formatted stderr so agents can parse errors without regex.

Note: the current error contract is a JSON string envelope, not structured
error objects with typed codes. Richer error typing is a future improvement.

**Status:** Available now (Phase 1). `--exit-code` works on `evaluate`.

### Reason 3: Skills need composable command chains

The `evaluate → get-trace` workflow is a natural two-step that agents perform
constantly. `dcx` designs these as complementary commands sharing the same
`--project-id`, `--dataset-id`, and `--table` context:

```
evaluate (returns session_ids with scores)
    ↓
get-trace (takes a session_id, returns event timeline)
```

With `bq`, each step requires the agent to write custom SQL from scratch.
There is no concept of "evaluate" or "trace" — the agent must know the table
schema, the latency field JSON structure, and the error detection logic.

**Status:** Available now (Phase 1). `evaluate` + `get-trace` + `doctor`.

### Reason 4: Skills require a distributable binary

The SKILL.md format supports declaring binary dependencies (e.g.,
`requires.bins: ["dcx"]`), which agent runtimes check before activating a
skill. `dcx` distributes via `npx dcx` (npm platform packages for 6
targets), making it installable in any CI environment in seconds.

`bq` requires the full gcloud SDK (~1GB install). In CI environments where
`gcloud` isn't pre-installed, adding it takes minutes and requires manual
auth configuration.

**Status:** npm distribution available (Phase 1). `npx dcx --help` works on
macOS, Linux, and Windows. 14 consolidated skills shipped (Phase 5), with 5
generated from Discovery API metadata and 9 curated.

### Phase roadmap for deeper integration

| Phase | What it adds | Why it can't be bq |
|---|---|---|
| Phase 1 (complete) | `evaluate`, `get-trace`, `doctor`, npm distribution, 5 core skills | `bq` has no analytics commands or skill format |
| Phase 2 (complete) | Dynamic BigQuery API commands from Discovery Document, `generate-skills`, 19 skills, `--sanitize` (Model Armor), Gemini extension manifest | `bq` commands are static Python; cannot generate skills from API metadata |
| Phase 3 (complete) | Conversational Analytics (`dcx ca ask`), natural language → SQL for BigQuery, 26 skills | `bq` has no CA integration; requires a new command domain |
| Phase 4 (complete) | Multi-source CA (Looker, AlloyDB, Spanner, Cloud SQL), source profiles | `bq` is BigQuery-only; cannot span Data Cloud sources |
| Phase 5 (complete) | Native Data Cloud commands, SDK alignment (all 12 SDK commands, 6 evaluators, exit-code parity, drift automation), 14 consolidated skills, 526 tests | `bq` has no analytics SDK, no skill format, no drift detection |

Each phase increases the gap between what `dcx` can do natively and what
would need to be shimmed on top of `bq`.
